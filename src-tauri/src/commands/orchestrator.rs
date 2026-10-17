use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, State};
use log::{info, error};

use crate::DbState;
use crate::db;
use crate::gguf;
use crate::gpu;
use crate::orchestrator::{
    self, AgentConfig, AgentRole, AgentStatus, Artifact, Directive,
    OrchestratorState, OrchestratorStatus, TeamConfig,
    AgentStartPayload, AgentCompletePayload, AgentSwapPayload,
    CheckpointPayload, OrchestratorCompletePayload, OrchestratorErrorPayload,
    VramEstimate, AgentVramEstimate,
};

// --- Tauri command input types ---

#[derive(Debug, Deserialize)]
pub struct AgentConfigInput {
    pub role: AgentRole,
    pub model_path: String,
    pub ngl: Option<i32>,
    pub context_size: Option<u32>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TeamConfigInput {
    pub agents: Vec<AgentConfigInput>,
    pub task_description: String,
    pub working_directory: Option<String>,
}

impl TeamConfigInput {
    fn to_team_config(&self) -> TeamConfig {
        let wd = self.working_directory.as_deref();
        TeamConfig {
            agents: self.agents.iter().map(|a| AgentConfig {
                role: a.role,
                model_path: a.model_path.clone(),
                ngl: a.ngl,
                context_size: a.context_size.unwrap_or(8192),
                system_prompt: a.system_prompt.clone()
                    .unwrap_or_else(|| orchestrator::default_system_prompt(a.role, wd)),
            }).collect(),
            task_description: self.task_description.clone(),
            working_directory: self.working_directory.clone(),
        }
    }
}

// --- Commands ---

#[tauri::command]
pub fn create_orchestrator_session(
    db: State<'_, DbState>,
    team_config: TeamConfigInput,
) -> Result<String, String> {
    let conn = db.0.lock().unwrap();
    let user_id = db::default_user_id();
    let session = db::create_orchestrator_session(&conn, &user_id, &team_config.task_description)
        .map_err(|e| e.to_string())?;

    // Store agent configs in DB
    for agent in &team_config.agents {
        let role_str = serde_json::to_value(&agent.role)
            .map(|v| v.as_str().unwrap_or("unknown").to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        db::insert_orchestrator_agent(
            &conn,
            &session.id,
            &role_str,
            &agent.model_path,
            agent.ngl,
            agent.context_size.unwrap_or(8192) as i32,
            agent.role.port() as i32,
        ).map_err(|e| e.to_string())?;
    }

    Ok(session.id)
}

#[tauri::command]
pub async fn start_orchestrator(
    app: AppHandle,
    db_state: State<'_, DbState>,
    orch: State<'_, OrchestratorState>,
    session_id: String,
    team_config: TeamConfigInput,
) -> Result<(), String> {
    let config = team_config.to_team_config();
    let task = config.task_description.clone();

    // Get server path from settings
    let server_path = {
        let conn = db_state.0.lock().unwrap();
        db::get_setting(&conn, "llama_server_path")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
                format!("{}/src/llama.cpp/build/bin/llama-server", home)
            })
    };

    // Store session ID and working directory
    *orch.session_id.lock().unwrap() = Some(session_id.clone());
    *orch.working_directory.lock().unwrap() = config.working_directory.clone();
    orch.artifacts.lock().unwrap().clear();

    // Update DB status
    {
        let conn = db_state.0.lock().unwrap();
        let _ = db::update_session_status(&conn, &session_id, "setting_up");
    }

    // Start all agent servers
    orchestrator::start_team(&orch, &config, &server_path).await?;

    // Update DB status
    {
        let conn = db_state.0.lock().unwrap();
        let _ = db::update_session_status(&conn, &session_id, "running");
    }

    // Spawn the orchestration loop in background
    let app_clone = app.clone();
    let session_id_clone = session_id.clone();

    tokio::spawn(async move {
        let orch_state = app_clone.state::<OrchestratorState>();
        let db_state = app_clone.state::<DbState>();

        match run_orchestration_loop(&app_clone, &orch_state, &db_state, &session_id_clone, &task, &config).await {
            Ok(()) => {
                info!("Orchestration session {} completed", session_id_clone);
                let conn = db_state.0.lock().unwrap();
                let _ = db::update_session_status(&conn, &session_id_clone, "complete");
            }
            Err(e) => {
                error!("Orchestration session {} failed: {}", session_id_clone, e);
                let _ = app_clone.emit("orchestrator-error", OrchestratorErrorPayload {
                    error: e.clone(),
                    role: None,
                });
                let conn = db_state.0.lock().unwrap();
                let _ = db::update_session_status(&conn, &session_id_clone, "error");
            }
        }

        // Stop all servers after completion
        let _ = orchestrator::stop_team(&orch_state).await;
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_orchestrator(
    app: AppHandle,
) -> Result<(), String> {
    let orch = app.state::<OrchestratorState>();
    let session_id = orch.session_id.lock().unwrap().clone();
    orchestrator::stop_team(&orch).await?;
    if let Some(sid) = session_id {
        let db_state = app.state::<DbState>();
        let conn = db_state.0.lock().unwrap();
        let _ = db::update_session_status(&conn, &sid, "stopped");
    }
    Ok(())
}

#[tauri::command]
pub fn pause_orchestrator(orch: State<'_, OrchestratorState>) -> Result<(), String> {
    *orch.is_paused.lock().unwrap() = true;
    *orch.status.lock().unwrap() = OrchestratorStatus::Paused;
    Ok(())
}

#[tauri::command]
pub fn resume_orchestrator(orch: State<'_, OrchestratorState>) -> Result<(), String> {
    *orch.is_paused.lock().unwrap() = false;
    *orch.status.lock().unwrap() = OrchestratorStatus::Running;
    Ok(())
}

#[tauri::command]
pub fn inject_user_message(
    orch: State<'_, OrchestratorState>,
    content: String,
) -> Result<(), String> {
    *orch.pending_user_message.lock().unwrap() = Some(content);
    // Also resume if paused
    *orch.is_paused.lock().unwrap() = false;
    *orch.status.lock().unwrap() = OrchestratorStatus::Running;
    Ok(())
}

#[tauri::command]
pub fn get_orchestrator_status(
    orch: State<'_, OrchestratorState>,
) -> Result<serde_json::Value, String> {
    let status = orch.status.lock().unwrap().clone();
    let session_id = orch.session_id.lock().unwrap().clone();
    let is_paused = *orch.is_paused.lock().unwrap();
    let step = *orch.step_counter.lock().unwrap();

    let agent_statuses: Vec<serde_json::Value> = {
        let agents = orch.agents.lock().unwrap();
        agents.iter().map(|(role, instance)| {
            serde_json::json!({
                "role": role.label(),
                "status": instance.status,
                "model_path": instance.config.model_path,
            })
        }).collect()
    };

    Ok(serde_json::json!({
        "status": status,
        "session_id": session_id,
        "is_paused": is_paused,
        "step": step,
        "agents": agent_statuses,
    }))
}

#[tauri::command]
pub fn estimate_team_vram(
    team_config: TeamConfigInput,
) -> Result<VramEstimate, String> {
    let gpu_info = gpu::detect_gpu().ok();
    let gpu_available_mb = gpu_info.as_ref().map(|g| g.vram_free_mb).unwrap_or(0);

    let mut agent_estimates = Vec::new();

    for agent in &team_config.agents {
        let meta = gguf::read_gguf_metadata(&agent.model_path)?;
        let file_size_mb = std::fs::metadata(&agent.model_path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);

        let total_layers = meta.layer_count + 1; // +1 for output layer
        let ngl = agent.ngl.unwrap_or(total_layers as i32);
        let ngl = ngl.min(total_layers as i32).max(0);
        let gpu_layers = ngl as f64;
        let cpu_layers = total_layers as f64 - gpu_layers;

        let layer_size_mb = file_size_mb / total_layers as f64;

        // KV cache estimation (FP16)
        let head_dim = if meta.head_count > 0 {
            meta.embedding_length / meta.head_count
        } else {
            128
        };
        let ctx = agent.context_size.unwrap_or(8192) as f64;
        let kv_layers = gpu_layers.min(meta.layer_count as f64);
        let kv_cache_mb = (2.0 * kv_layers * meta.head_count_kv as f64 * head_dim as f64 * ctx * 2.0)
            / (1024.0 * 1024.0);

        let cuda_overhead = if gpu_layers > 0.0 { 300.0 } else { 0.0 };

        let gpu_mem_mb = gpu_layers * layer_size_mb + kv_cache_mb + cuda_overhead;
        let cpu_mem_mb = cpu_layers * layer_size_mb;

        let model_filename = std::path::Path::new(&agent.model_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        agent_estimates.push(AgentVramEstimate {
            role: serde_json::to_value(&agent.role)
                .map(|v| v.as_str().unwrap_or("unknown").to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            model_filename,
            gpu_mem_mb,
            cpu_mem_mb,
            needs_cpu_offload: gpu_mem_mb > gpu_available_mb as f64,
            ngl,
            total_layers,
        });
    }

    // Swap-on-demand: VRAM = Orchestrator (fixed) + max(other agents in rotating slot)
    let orch_gpu = agent_estimates.iter()
        .find(|a| a.role == "orchestrator")
        .map(|a| a.gpu_mem_mb)
        .unwrap_or(0.0);
    let orch_cpu = agent_estimates.iter()
        .find(|a| a.role == "orchestrator")
        .map(|a| a.cpu_mem_mb)
        .unwrap_or(0.0);
    let max_other_gpu = agent_estimates.iter()
        .filter(|a| a.role != "orchestrator")
        .map(|a| a.gpu_mem_mb)
        .fold(0.0f64, f64::max);
    let max_other_cpu = agent_estimates.iter()
        .filter(|a| a.role != "orchestrator")
        .map(|a| a.cpu_mem_mb)
        .fold(0.0f64, f64::max);

    let total_gpu_mb = orch_gpu + max_other_gpu;
    let total_cpu_mb = orch_cpu + max_other_cpu;

    let will_overflow = total_gpu_mb > gpu_available_mb as f64;
    let overflow_mb = if will_overflow {
        total_gpu_mb - gpu_available_mb as f64
    } else {
        0.0
    };

    Ok(VramEstimate {
        agents: agent_estimates,
        total_gpu_mb,
        total_cpu_mb,
        gpu_available_mb,
        will_overflow,
        overflow_mb,
    })
}

#[tauri::command]
pub fn list_orchestrator_sessions(
    db: State<'_, DbState>,
) -> Result<Vec<db::OrchestratorSession>, String> {
    let conn = db.0.lock().unwrap();
    let user_id = db::default_user_id();
    db::list_orchestrator_sessions(&conn, &user_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_orchestrator_log(
    db: State<'_, DbState>,
    session_id: String,
) -> Result<Vec<db::OrchestratorMessageRow>, String> {
    let conn = db.0.lock().unwrap();
    db::get_orchestrator_messages(&conn, &session_id).map_err(|e| e.to_string())
}


#[tauri::command]
pub fn delete_orchestrator_session(
    db: State<'_, DbState>,
    session_id: String,
) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    db::delete_orchestrator_session(&conn, &session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_working_directory(path: String) -> Result<serde_json::Value, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Ok(serde_json::json!({ "valid": false, "error": "Directory does not exist" }));
    }
    if !p.is_dir() {
        return Ok(serde_json::json!({ "valid": false, "error": "Path is not a directory" }));
    }
    // Check read/write permissions by attempting to read the directory
    match std::fs::read_dir(p) {
        Ok(_) => Ok(serde_json::json!({ "valid": true })),
        Err(e) => Ok(serde_json::json!({ "valid": false, "error": format!("Cannot access directory: {}", e) })),
    }
}

#[tauri::command]
pub fn approve_file_writes(
    orch: State<'_, OrchestratorState>,
) -> Result<(), String> {
    *orch.file_approval.lock().unwrap() = Some(true);
    Ok(())
}

#[tauri::command]
pub fn deny_file_writes(
    orch: State<'_, OrchestratorState>,
) -> Result<(), String> {
    *orch.file_approval.lock().unwrap() = Some(false);
    *orch.pending_file_proposals.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub fn list_working_directory(
    orch: State<'_, OrchestratorState>,
    relative_path: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let working_dir = orch.working_directory.lock().unwrap().clone()
        .ok_or("No working directory set")?;

    let target = match relative_path {
        Some(ref rel) => {
            orchestrator::validate_path_in_working_dir(&working_dir, rel)?
        }
        None => std::path::PathBuf::from(&working_dir),
    };

    let entries = std::fs::read_dir(&target)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let metadata = entry.metadata().map_err(|e| format!("Failed to read metadata: {}", e))?;
        result.push(serde_json::json!({
            "name": entry.file_name().to_string_lossy(),
            "is_dir": metadata.is_dir(),
            "size": metadata.len(),
        }));
    }

    result.sort_by(|a, b| {
        let a_dir = a["is_dir"].as_bool().unwrap_or(false);
        let b_dir = b["is_dir"].as_bool().unwrap_or(false);
        b_dir.cmp(&a_dir).then_with(|| {
            a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
        })
    });

    Ok(result)
}

// --- Orchestration Loop ---

async fn run_orchestration_loop(
    app: &AppHandle,
    orch: &OrchestratorState,
    db_state: &State<'_, DbState>,
    session_id: &str,
    task: &str,
    config: &TeamConfig,
) -> Result<(), String> {
    let orchestrator_config = config.agents.iter()
        .find(|a| a.role == AgentRole::Orchestrator)
        .ok_or("No Orchestrator agent configured")?;

    let mut orchestrator_messages: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "role": "system",
            "content": orchestrator_config.system_prompt,
        }),
        serde_json::json!({
            "role": "user",
            "content": format!("Task: {}", task),
        }),
    ];

    let max_iterations = 50;
    let mut architect_produced_plan = false;

    for iteration in 0..max_iterations {
        // Check if stopped
        {
            let status = orch.status.lock().unwrap().clone();
            if status == OrchestratorStatus::Idle || status == OrchestratorStatus::Complete {
                return Ok(());
            }
        }

        // Check pause and wait
        loop {
            let paused = *orch.is_paused.lock().unwrap();
            if !paused {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let user_msg = orch.pending_user_message.lock().unwrap().take();
            if let Some(msg) = user_msg {
                orchestrator_messages.push(serde_json::json!({
                    "role": "user",
                    "content": format!("[USER INTERVENTION] {}", msg),
                }));
                let conn = db_state.0.lock().unwrap();
                let step = *orch.step_counter.lock().unwrap();
                let _ = db::insert_orchestrator_message(
                    &conn, session_id, "user", "orchestrator", &msg, "user_intervention", step as i32,
                );
                break;
            }
        }

        let step = {
            let mut s = orch.step_counter.lock().unwrap();
            *s += 1;
            *s
        };

        // Send to Orchestrator agent
        {
            let mut agents = orch.agents.lock().unwrap();
            if let Some(instance) = agents.get_mut(&AgentRole::Orchestrator) {
                instance.status = AgentStatus::Thinking;
            }
        }

        let _ = app.emit("orchestrator-agent-start", AgentStartPayload {
            role: "Orchestrator".to_string(),
            instruction: if iteration == 0 { task.to_string() } else { "Continuing...".to_string() },
            step_number: step,
        });

        // Sliding window: trim orchestrator_messages to fit context budget (~8000 chars ≈ 2000 tokens)
        let context_budget: usize = 8000;
        {
            let total_chars: usize = orchestrator_messages.iter()
                .map(|m| m["content"].as_str().map(|s| s.len()).unwrap_or(0))
                .sum();
            if total_chars > context_budget && orchestrator_messages.len() > 3 {
                // Keep: system (0), task (1), and trim from the front of the middle
                let system_msg = orchestrator_messages[0].clone();
                let task_msg = orchestrator_messages[1].clone();
                let mut trimmed = vec![system_msg, task_msg];

                // Count how many recent messages fit in the remaining budget
                let fixed_chars = trimmed.iter()
                    .map(|m| m["content"].as_str().map(|s| s.len()).unwrap_or(0))
                    .sum::<usize>();
                let mut remaining_budget = context_budget.saturating_sub(fixed_chars);
                let middle = &orchestrator_messages[2..];

                let mut keep_from = middle.len();
                for (i, msg) in middle.iter().enumerate().rev() {
                    let msg_len = msg["content"].as_str().map(|s| s.len()).unwrap_or(0);
                    if msg_len > remaining_budget {
                        break;
                    }
                    remaining_budget -= msg_len;
                    keep_from = i;
                }

                let trimmed_count = keep_from;
                if trimmed_count > 0 {
                    trimmed.push(serde_json::json!({
                        "role": "user",
                        "content": format!("[{} earlier messages trimmed. Reference Artifacts by ID for previous context.]", trimmed_count),
                    }));
                }
                trimmed.extend_from_slice(&middle[keep_from..]);
                orchestrator_messages = trimmed;
            }
        }

        let orch_response = orchestrator::send_to_agent(
            AgentRole::Orchestrator,
            orchestrator_messages.clone(),
            app,
        ).await?;

        {
            let mut agents = orch.agents.lock().unwrap();
            if let Some(instance) = agents.get_mut(&AgentRole::Orchestrator) {
                instance.status = AgentStatus::Done;
            }
        }

        let _ = app.emit("orchestrator-agent-complete", AgentCompletePayload {
            role: "Orchestrator".to_string(),
            response: orch_response.clone(),
            step_number: step,
        });

        {
            let conn = db_state.0.lock().unwrap();
            let _ = db::insert_orchestrator_message(
                &conn, session_id, "orchestrator", "system", &orch_response, "response", step as i32,
            );
        }

        orchestrator_messages.push(serde_json::json!({
            "role": "assistant",
            "content": orch_response.clone(),
        }));

        let directives = orchestrator::parse_directives(&orch_response);

        if directives.iter().all(|d| matches!(d, Directive::Text(_))) {
            orchestrator_messages.push(serde_json::json!({
                "role": "user",
                "content": "Please use the [DELEGATE:AgentName], [CHECKPOINT], or [COMPLETE] markers to indicate your next action.",
            }));
            continue;
        }

        let mut should_break = false;

        for directive in &directives {
            match directive {
                Directive::Delegate { role, instruction } => {
                    let step = {
                        let mut s = orch.step_counter.lock().unwrap();
                        *s += 1;
                        *s
                    };

                    {
                        let mut agents = orch.agents.lock().unwrap();
                        if let Some(instance) = agents.get_mut(role) {
                            instance.status = AgentStatus::Thinking;
                        }
                    }

                    let _ = app.emit("orchestrator-agent-start", AgentStartPayload {
                        role: role.label().to_string(),
                        instruction: instruction.clone(),
                        step_number: step,
                    });

                    {
                        let conn = db_state.0.lock().unwrap();
                        let _ = db::insert_orchestrator_message(
                            &conn, session_id, "orchestrator", &role.label().to_lowercase(),
                            instruction, "instruction", step as i32,
                        );
                    }

                    // Build agent messages: system prompt + artifact context + instruction
                    let agent_config = config.agents.iter().find(|a| a.role == *role);
                    let system_prompt = agent_config
                        .map(|c| c.system_prompt.clone())
                        .unwrap_or_else(|| orchestrator::default_system_prompt(*role, config.working_directory.as_deref()));

                    // Inject artifact context into the system prompt
                    let artifacts = orch.artifacts.lock().unwrap().clone();
                    let artifact_context = orchestrator::build_artifact_context(&artifacts, 4000);
                    let full_system = if artifact_context.is_empty() {
                        system_prompt
                    } else {
                        format!("{}{}", system_prompt, artifact_context)
                    };

                    let mut agent_messages = vec![
                        serde_json::json!({
                            "role": "system",
                            "content": full_system,
                        }),
                        serde_json::json!({
                            "role": "user",
                            "content": instruction,
                        }),
                    ];

                    // Swap the rotating slot to load this agent's model
                    if *role != AgentRole::Orchestrator {
                        let model_filename = {
                            let agents = orch.agents.lock().unwrap();
                            agents.get(role)
                                .map(|i| std::path::Path::new(&i.config.model_path)
                                    .file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_default())
                                .unwrap_or_default()
                        };
                        let _ = app.emit("orchestrator-agent-swap", AgentSwapPayload {
                            role: role.label().to_string(),
                            model_filename: model_filename.clone(),
                        });
                        orchestrator::swap_agent(orch, *role).await?;
                    }

                    // Use tool-calling for Constructor/Auditor when working directory is set
                    let has_working_dir = config.working_directory.is_some();
                    let use_tools = has_working_dir && matches!(role, AgentRole::Constructor | AgentRole::Auditor);

                    // Pre-list working directory so agents don't waste tool calls on list_directory(".")
                    if use_tools {
                        if let Some(ref wd) = config.working_directory {
                            let dir_listing = match std::fs::read_dir(wd) {
                                Ok(entries) => {
                                    let mut items: Vec<String> = Vec::new();
                                    for entry in entries.flatten() {
                                        let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
                                        let name = entry.file_name().to_string_lossy().to_string();
                                        items.push(if is_dir { format!("{}/", name) } else { name });
                                    }
                                    items.sort();
                                    if items.is_empty() {
                                        "(empty directory — this is a new project)".to_string()
                                    } else {
                                        items.join("\n")
                                    }
                                }
                                Err(e) => format!("(could not list directory: {})", e),
                            };
                            // Append to the last system message
                            if let Some(sys_msg) = agent_messages.get_mut(0) {
                                if let Some(content) = sys_msg["content"].as_str() {
                                    *sys_msg = serde_json::json!({
                                        "role": "system",
                                        "content": format!("{}\n\n=== Project Directory Contents ===\n{}\n=== End Directory ===", content, dir_listing),
                                    });
                                }
                            }
                        }
                    }

                    let agent_response = if use_tools {
                        let tools = orchestrator::filesystem_tools();
                        orchestrator::send_to_agent_with_tools(*role, agent_messages, &tools, orch, app).await?
                    } else {
                        orchestrator::send_to_agent(*role, agent_messages, app).await?
                    };

                    {
                        let mut agents = orch.agents.lock().unwrap();
                        if let Some(instance) = agents.get_mut(role) {
                            instance.status = AgentStatus::Done;
                        }
                    }

                    let _ = app.emit("orchestrator-agent-complete", AgentCompletePayload {
                        role: role.label().to_string(),
                        response: agent_response.clone(),
                        step_number: step,
                    });

                    {
                        let conn = db_state.0.lock().unwrap();
                        let _ = db::insert_orchestrator_message(
                            &conn, session_id, &role.label().to_lowercase(), "orchestrator",
                            &agent_response, "response", step as i32,
                        );
                    }

                    // Store as artifact for future agents
                    {
                        let mut arts = orch.artifacts.lock().unwrap();
                        let artifact_id = arts.len() as u32 + 1;
                        arts.push(Artifact {
                            id: artifact_id,
                            from_role: *role,
                            step_number: step,
                            content: agent_response.clone(),
                        });
                    }

                    // Feed truncated response back to orchestrator (full content is in Artifact)
                    let artifact_id = orch.artifacts.lock().unwrap().len() as u32;
                    let summary = orchestrator::truncate_for_context(&agent_response, 500);
                    orchestrator_messages.push(serde_json::json!({
                        "role": "user",
                        "content": format!(
                            "[RESPONSE FROM {} — stored as Artifact #{}]\n{}",
                            role.label().to_uppercase(),
                            artifact_id,
                            summary,
                        ),
                    }));

                    // Track if Architect produced a plan (for forced Judge review)
                    if *role == AgentRole::Architect {
                        architect_produced_plan = true;
                    }

                    if architect_produced_plan && *role == AgentRole::Architect {
                        let has_judge = directives.iter().any(|d| matches!(d, Directive::Delegate { role: AgentRole::Judge, .. }));
                        if !has_judge {
                            orchestrator_messages.push(serde_json::json!({
                                "role": "user",
                                "content": "The Architect has produced a plan. You MUST now delegate to Judge for review before proceeding to construction.",
                            }));
                        }
                    }
                }

                Directive::Checkpoint { summary } => {
                    let _ = app.emit("orchestrator-checkpoint", CheckpointPayload {
                        summary: summary.clone(),
                        step_number: step,
                    });

                    *orch.status.lock().unwrap() = OrchestratorStatus::Paused;
                    *orch.is_paused.lock().unwrap() = true;

                    {
                        let conn = db_state.0.lock().unwrap();
                        let _ = db::insert_orchestrator_message(
                            &conn, session_id, "orchestrator", "system",
                            summary, "checkpoint", step as i32,
                        );
                    }

                    architect_produced_plan = false;
                }

                Directive::Complete { summary } => {
                    let _ = app.emit("orchestrator-complete", OrchestratorCompletePayload {
                        final_summary: summary.clone(),
                    });

                    *orch.status.lock().unwrap() = OrchestratorStatus::Complete;

                    {
                        let conn = db_state.0.lock().unwrap();
                        let _ = db::insert_orchestrator_message(
                            &conn, session_id, "orchestrator", "system",
                            summary, "complete", step as i32,
                        );
                    }

                    should_break = true;
                }

                Directive::Text(_) => {}
            }
        }

        if should_break {
            break;
        }
    }

    Ok(())
}
