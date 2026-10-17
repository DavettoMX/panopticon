use std::collections::HashMap;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tokio::process::Child;
use log::{info, error};

use crate::llama;

// --- Agent Role ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    Orchestrator,
    Architect,
    Auditor,
    Constructor,
    Judge,
}

impl AgentRole {
    pub fn port(&self) -> u16 {
        match self {
            AgentRole::Orchestrator => 8090,
            _ => 8091, // rotating slot — only one non-orchestrator agent loaded at a time
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AgentRole::Orchestrator => "Orchestrator",
            AgentRole::Architect => "Architect",
            AgentRole::Auditor => "Auditor",
            AgentRole::Constructor => "Constructor",
            AgentRole::Judge => "Judge",
        }
    }

    pub fn all() -> &'static [AgentRole] {
        &[
            AgentRole::Orchestrator,
            AgentRole::Architect,
            AgentRole::Auditor,
            AgentRole::Constructor,
            AgentRole::Judge,
        ]
    }
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// --- Agent Status ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Thinking,
    Done,
    Error,
}

// --- Orchestrator Status ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrchestratorStatus {
    Idle,
    SettingUp,
    Running,
    Paused,
    Complete,
    Error,
}

// --- Config ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub role: AgentRole,
    pub model_path: String,
    pub ngl: Option<i32>,
    pub context_size: u32,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub agents: Vec<AgentConfig>,
    pub task_description: String,
    pub working_directory: Option<String>,
}

// --- Artifacts (context passing between agents) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: u32,
    pub from_role: AgentRole,
    pub step_number: u32,
    pub content: String,
}

// --- File proposals (Constructor output parsing) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProposal {
    pub path: String,
    pub content: String,
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileProposalPayload {
    pub files: Vec<FileProposal>,
    pub step_number: u32,
}

// --- Messages ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Instruction,
    Response,
    Question,
    Review,
    Checkpoint,
    Complete,
    UserIntervention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorMessage {
    pub id: String,
    pub from_role: String,
    pub to_role: String,
    pub content: String,
    pub message_type: MessageType,
    pub step_number: u32,
    pub timestamp: String,
}

// --- Runtime Instance ---

pub struct AgentInstance {
    pub config: AgentConfig,
    pub process: Option<Child>,
    pub status: AgentStatus,
    pub history: Vec<OrchestratorMessage>,
}

// --- Tauri Managed State ---

pub struct OrchestratorState {
    pub agents: Mutex<HashMap<AgentRole, AgentInstance>>,
    pub session_id: Mutex<Option<String>>,
    pub status: Mutex<OrchestratorStatus>,
    pub is_paused: Mutex<bool>,
    pub step_counter: Mutex<u32>,
    pub pending_user_message: Mutex<Option<String>>,
    pub active_rotating_role: Mutex<Option<AgentRole>>,
    pub server_path: Mutex<Option<String>>,
    pub rotating_process: Mutex<Option<Child>>,
    pub artifacts: Mutex<Vec<Artifact>>,
    pub working_directory: Mutex<Option<String>>,
    pub pending_file_proposals: Mutex<Option<Vec<FileProposal>>>,
    pub file_approval: Mutex<Option<bool>>,
}

impl OrchestratorState {
    pub fn new() -> Self {
        Self {
            agents: Mutex::new(HashMap::new()),
            session_id: Mutex::new(None),
            status: Mutex::new(OrchestratorStatus::Idle),
            is_paused: Mutex::new(false),
            step_counter: Mutex::new(0),
            pending_user_message: Mutex::new(None),
            active_rotating_role: Mutex::new(None),
            server_path: Mutex::new(None),
            rotating_process: Mutex::new(None),
            artifacts: Mutex::new(Vec::new()),
            working_directory: Mutex::new(None),
            pending_file_proposals: Mutex::new(None),
            file_approval: Mutex::new(None),
        }
    }
}

// --- Event Payloads ---

#[derive(Debug, Clone, Serialize)]
pub struct AgentStartPayload {
    pub role: String,
    pub instruction: String,
    pub step_number: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTokenPayload {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentCompletePayload {
    pub role: String,
    pub response: String,
    pub step_number: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckpointPayload {
    pub summary: String,
    pub step_number: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrchestratorCompletePayload {
    pub final_summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrchestratorErrorPayload {
    pub error: String,
    pub role: Option<String>,
}

// --- VRAM Estimation ---

#[derive(Debug, Clone, Serialize)]
pub struct AgentVramEstimate {
    pub role: String,
    pub model_filename: String,
    pub gpu_mem_mb: f64,
    pub cpu_mem_mb: f64,
    pub needs_cpu_offload: bool,
    pub ngl: i32,
    pub total_layers: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VramEstimate {
    pub agents: Vec<AgentVramEstimate>,
    pub total_gpu_mb: f64,
    pub total_cpu_mb: f64,
    pub gpu_available_mb: u64,
    pub will_overflow: bool,
    pub overflow_mb: f64,
}

// --- Directive Parsing ---

#[derive(Debug, Clone)]
pub enum Directive {
    Delegate { role: AgentRole, instruction: String },
    Checkpoint { summary: String },
    Complete { summary: String },
    Text(String),
}

pub fn parse_directives(response: &str) -> Vec<Directive> {
    let mut directives = Vec::new();
    let mut current_text = String::new();
    let mut lines = response.lines().peekable();

    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix("[DELEGATE:") {
            // Flush accumulated text
            if !current_text.trim().is_empty() {
                directives.push(Directive::Text(current_text.trim().to_string()));
                current_text.clear();
            }

            if let Some(bracket_end) = rest.find(']') {
                let role_str = &rest[..bracket_end];
                let inline_instruction = rest[bracket_end + 1..].trim().to_string();

                if let Some(role) = parse_role(role_str) {
                    // Collect lines until next directive marker
                    let mut instruction = inline_instruction;
                    while let Some(next) = lines.peek() {
                        if next.starts_with("[DELEGATE:") || next.starts_with("[CHECKPOINT]") || next.starts_with("[COMPLETE]") {
                            break;
                        }
                        if !instruction.is_empty() {
                            instruction.push('\n');
                        }
                        instruction.push_str(lines.next().unwrap());
                    }
                    directives.push(Directive::Delegate { role, instruction: instruction.trim().to_string() });
                }
            }
        } else if let Some(rest) = line.strip_prefix("[CHECKPOINT]") {
            if !current_text.trim().is_empty() {
                directives.push(Directive::Text(current_text.trim().to_string()));
                current_text.clear();
            }
            let mut summary = rest.trim().to_string();
            while let Some(next) = lines.peek() {
                if next.starts_with("[DELEGATE:") || next.starts_with("[CHECKPOINT]") || next.starts_with("[COMPLETE]") {
                    break;
                }
                summary.push('\n');
                summary.push_str(lines.next().unwrap());
            }
            directives.push(Directive::Checkpoint { summary: summary.trim().to_string() });
        } else if let Some(rest) = line.strip_prefix("[COMPLETE]") {
            if !current_text.trim().is_empty() {
                directives.push(Directive::Text(current_text.trim().to_string()));
                current_text.clear();
            }
            let mut summary = rest.trim().to_string();
            while let Some(next) = lines.next() {
                summary.push('\n');
                summary.push_str(next);
            }
            directives.push(Directive::Complete { summary: summary.trim().to_string() });
        } else {
            current_text.push_str(line);
            current_text.push('\n');
        }
    }

    if !current_text.trim().is_empty() {
        directives.push(Directive::Text(current_text.trim().to_string()));
    }

    directives
}

fn parse_role(s: &str) -> Option<AgentRole> {
    match s.to_lowercase().trim() {
        "architect" => Some(AgentRole::Architect),
        "auditor" => Some(AgentRole::Auditor),
        "constructor" => Some(AgentRole::Constructor),
        "judge" => Some(AgentRole::Judge),
        _ => None,
    }
}

// --- Event Payload for Swap ---

#[derive(Debug, Clone, Serialize)]
pub struct AgentSwapPayload {
    pub role: String,
    pub model_filename: String,
}

// --- Team Lifecycle ---

/// Starts only the Orchestrator server (port 8090). Other agents are stored
/// as configs without processes — they'll be swapped in on demand via `swap_agent`.
pub async fn start_team(
    state: &OrchestratorState,
    team_config: &TeamConfig,
    server_path: &str,
) -> Result<(), String> {
    *state.status.lock().unwrap() = OrchestratorStatus::SettingUp;

    // Store server_path for later use in swap_agent
    *state.server_path.lock().unwrap() = Some(server_path.to_string());

    // Find the orchestrator config
    let orch_config = team_config.agents.iter()
        .find(|a| a.role == AgentRole::Orchestrator)
        .ok_or("No Orchestrator agent in team config")?;

    // Launch only the Orchestrator server
    let sp = server_path.to_string();
    let mp = orch_config.model_path.clone();
    let ctx = orch_config.context_size;
    let ngl = orch_config.ngl;

    let child = llama::start_server_standalone(&sp, &mp, 8090, ctx, ngl).await
        .map_err(|e| {
            error!("Failed to start Orchestrator: {}", e);
            e
        })?;
    llama::wait_for_ready_on_port(8090, 120).await?;
    info!("Orchestrator ready on port 8090");

    // Build agents map: Orchestrator with process, others without
    let mut agents_map = HashMap::new();
    agents_map.insert(AgentRole::Orchestrator, AgentInstance {
        config: orch_config.clone(),
        process: Some(child),
        status: AgentStatus::Idle,
        history: Vec::new(),
    });

    for agent_config in &team_config.agents {
        if agent_config.role == AgentRole::Orchestrator {
            continue;
        }
        agents_map.insert(agent_config.role, AgentInstance {
            config: agent_config.clone(),
            process: None, // no process — loaded on demand
            status: AgentStatus::Idle,
            history: Vec::new(),
        });
        info!("Agent {} registered (on-demand swap)", agent_config.role);
    }

    *state.agents.lock().unwrap() = agents_map;
    *state.status.lock().unwrap() = OrchestratorStatus::Running;
    *state.step_counter.lock().unwrap() = 0;

    Ok(())
}

/// Swaps the rotating slot (port 8091) to load the model for `role`.
/// Skips the swap if the same model is already loaded.
pub async fn swap_agent(state: &OrchestratorState, role: AgentRole) -> Result<(), String> {
    if role == AgentRole::Orchestrator {
        return Ok(()); // Orchestrator is always loaded on 8090
    }

    // Get the model_path for the requested role
    let (model_path, ctx, ngl) = {
        let agents = state.agents.lock().unwrap();
        let instance = agents.get(&role)
            .ok_or(format!("Agent {} not found in team config", role))?;
        (
            instance.config.model_path.clone(),
            instance.config.context_size,
            instance.config.ngl,
        )
    };

    // Check if the same model is already loaded in the rotating slot
    let active_role = { state.active_rotating_role.lock().unwrap().clone() };
    if let Some(active) = active_role {
        let active_model_path = {
            let agents = state.agents.lock().unwrap();
            agents.get(&active).map(|i| i.config.model_path.clone())
        };
        if active_model_path.as_deref() == Some(&model_path) {
            info!("Skipping swap: {} uses same model as active {}", role, active);
            *state.active_rotating_role.lock().unwrap() = Some(role);
            return Ok(());
        }
    }

    // Kill existing rotating process
    let old_process = { state.rotating_process.lock().unwrap().take() };
    if let Some(mut child) = old_process {
        info!("Killing rotating slot process for swap to {}", role);
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    // Get server_path
    let server_path = {
        state.server_path.lock().unwrap().clone()
            .ok_or("Server path not set — was start_team called?")?
    };

    // Launch the new model on port 8091
    info!("Swapping rotating slot to {} (model: {})", role, model_path);
    let child = llama::start_server_standalone(&server_path, &model_path, 8091, ctx, ngl).await
        .map_err(|e| format!("Failed to swap to agent {}: {}", role, e))?;
    llama::wait_for_ready_on_port(8091, 120).await
        .map_err(|e| format!("Agent {} failed health check after swap: {}", role, e))?;

    *state.rotating_process.lock().unwrap() = Some(child);
    *state.active_rotating_role.lock().unwrap() = Some(role);

    info!("Agent {} ready on rotating slot (port 8091)", role);
    Ok(())
}

pub async fn stop_team(state: &OrchestratorState) -> Result<(), String> {
    // Kill the Orchestrator process from agents map
    let mut children: Vec<(AgentRole, Child)> = {
        let mut agents = state.agents.lock().unwrap();
        agents.iter_mut()
            .filter_map(|(role, instance)| {
                instance.process.take().map(|child| (*role, child))
            })
            .collect()
    };

    for (role, mut child) in children.drain(..) {
        info!("Stopping agent {} on port {}", role, role.port());
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    // Kill the rotating slot process
    let rotating = { state.rotating_process.lock().unwrap().take() };
    if let Some(mut child) = rotating {
        info!("Stopping rotating slot process (port 8091)");
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    {
        let mut agents = state.agents.lock().unwrap();
        agents.clear();
    }
    *state.status.lock().unwrap() = OrchestratorStatus::Idle;
    *state.session_id.lock().unwrap() = None;
    *state.is_paused.lock().unwrap() = false;
    *state.active_rotating_role.lock().unwrap() = None;
    *state.server_path.lock().unwrap() = None;
    state.artifacts.lock().unwrap().clear();
    *state.working_directory.lock().unwrap() = None;
    *state.pending_file_proposals.lock().unwrap() = None;
    *state.file_approval.lock().unwrap() = None;
    Ok(())
}

// --- Filesystem Tools Definition ---

pub fn filesystem_tools() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file. Creates parent directories if needed. The user will approve each write. Use relative paths only (e.g. 'src/main.py').",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path (e.g. 'src/main.py', 'app/routes.py'). Never use absolute paths."
                        },
                        "content": {
                            "type": "string",
                            "description": "The complete file content to write"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the content of a file. Use relative paths only. Only call once per file — do not re-read a file you already read.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path (e.g. 'src/main.py'). Never use absolute paths."
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "list_directory",
                "description": "List files and subdirectories. Only call once per directory — do not re-list a directory you already listed. After listing, use the results to answer or call other tools.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path, or '.' for the project root. Never use absolute paths."
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
    ]
}

// --- Tool Call Result ---

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallPayload {
    pub role: String,
    pub tool_name: String,
    pub arguments: String,
    pub step_number: u32,
}

// --- Streaming Response with Tool Call Detection ---

#[derive(Debug, Clone)]
pub struct StreamedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub enum AgentResponse {
    Text(String),
    ToolCalls(Vec<StreamedToolCall>),
}

/// Stream a single completion from an agent, returning either text content or tool calls.
async fn stream_completion(
    role: AgentRole,
    messages: &[serde_json::Value],
    tools: Option<&[serde_json::Value]>,
    app: &tauri::AppHandle,
) -> Result<AgentResponse, String> {
    use futures_util::StreamExt;
    use tauri::Emitter;

    let port = role.port();
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/v1/chat/completions", port);

    let mut body = serde_json::json!({
        "messages": messages,
        "stream": true,
    });
    if let Some(t) = tools {
        body["tools"] = serde_json::json!(t);
    }

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send to agent {}: {}", role, e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Agent {} error ({}): {}", role, status, text));
    }

    let mut stream = response.bytes_stream();
    let mut full_content = String::new();
    let mut buffer = String::new();
    let mut tool_calls: Vec<StreamedToolCall> = Vec::new();
    let mut finish_reason = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error from {}: {}", role, e))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    continue;
                }

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(err_msg) = parsed["error"]["message"].as_str() {
                        return Err(format!("Agent {} error: {}", role, err_msg));
                    }

                    let choice = &parsed["choices"][0];

                    // Capture finish_reason
                    if let Some(fr) = choice["finish_reason"].as_str() {
                        finish_reason = fr.to_string();
                    }

                    let delta = &choice["delta"];

                    // Text content
                    if let Some(text) = delta["content"].as_str() {
                        full_content.push_str(text);
                        let _ = app.emit("orchestrator-agent-token", AgentTokenPayload {
                            role: role.label().to_string(),
                            content: text.to_string(),
                        });
                    }

                    // Tool calls (streamed incrementally)
                    if let Some(tc_arr) = delta["tool_calls"].as_array() {
                        for tc in tc_arr {
                            let idx = tc["index"].as_u64().unwrap_or(0) as usize;

                            // Extend tool_calls vector if needed
                            while tool_calls.len() <= idx {
                                tool_calls.push(StreamedToolCall {
                                    id: String::new(),
                                    name: String::new(),
                                    arguments: String::new(),
                                });
                            }

                            if let Some(id) = tc["id"].as_str() {
                                tool_calls[idx].id = id.to_string();
                            }
                            if let Some(func) = tc["function"].as_object() {
                                if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                    tool_calls[idx].name = name.to_string();
                                }
                                if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                                    tool_calls[idx].arguments.push_str(args);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Determine what the model returned
    if !tool_calls.is_empty() && (finish_reason == "tool" || finish_reason == "tool_calls" || tool_calls.iter().any(|tc| !tc.name.is_empty())) {
        Ok(AgentResponse::ToolCalls(tool_calls))
    } else {
        Ok(AgentResponse::Text(full_content))
    }
}

// --- Send to Agent (simple, no tools) ---

pub async fn send_to_agent(
    role: AgentRole,
    messages: Vec<serde_json::Value>,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    match stream_completion(role, &messages, None, app).await? {
        AgentResponse::Text(text) => Ok(text),
        AgentResponse::ToolCalls(_) => Ok(String::new()), // shouldn't happen without tools
    }
}

// --- Send to Agent with Tools (handles tool call loop) ---

pub async fn send_to_agent_with_tools(
    role: AgentRole,
    mut messages: Vec<serde_json::Value>,
    tools: &[serde_json::Value],
    state: &OrchestratorState,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    use tauri::Emitter;

    let max_tool_rounds = 6;
    let mut tool_cache: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut consecutive_failures: u32 = 0;

    for _round in 0..max_tool_rounds {
        let result = stream_completion(role, &messages, Some(tools), app).await?;

        match result {
            AgentResponse::Text(text) => return Ok(text),
            AgentResponse::ToolCalls(tool_calls) => {
                let tc_json: Vec<serde_json::Value> = tool_calls.iter().map(|tc| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                }).collect();

                messages.push(serde_json::json!({
                    "role": "assistant",
                    "tool_calls": tc_json,
                }));

                for tc in &tool_calls {
                    let _ = app.emit("orchestrator-tool-call", ToolCallPayload {
                        role: role.label().to_string(),
                        tool_name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                        step_number: *state.step_counter.lock().unwrap(),
                    });

                    let cache_key = format!("{}:{}", tc.name, tc.arguments);
                    let is_dedup = tool_cache.contains_key(&cache_key);

                    let tool_result = if is_dedup {
                        let cached = tool_cache.get(&cache_key).unwrap();
                        format!(
                            "{}\n\n[You already called {}() with the same arguments. Use the results above to formulate your answer. Do not call this tool again with the same arguments.]",
                            cached, tc.name
                        )
                    } else {
                        let result = execute_tool(tc, state, app).await?;
                        tool_cache.insert(cache_key, result.clone());
                        result
                    };

                    let is_error = tool_result.starts_with("Error ") || tool_result.contains("os error");
                    if is_dedup || is_error {
                        consecutive_failures += 1;
                    } else {
                        consecutive_failures = 0;
                    }

                    let _ = app.emit("orchestrator-agent-token", AgentTokenPayload {
                        role: role.label().to_string(),
                        content: format!("\n[Tool: {} -> {}]\n", tc.name,
                            if tool_result.len() > 200 { format!("{}...", &tool_result[..200]) } else { tool_result.clone() }),
                    });

                    messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": tool_result,
                    }));
                }

                // If too many consecutive failures, force a text response
                if consecutive_failures >= 3 {
                    info!("Agent {} hit {} consecutive tool failures, forcing text response", role, consecutive_failures);
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": "STOP calling tools. You have had multiple failed or duplicate calls. Respond NOW with your answer using only the information you already have."
                    }));
                    match stream_completion(role, &messages, None, app).await? {
                        AgentResponse::Text(text) => return Ok(text),
                        _ => return Ok("[Agent could not produce a text response after tool failures]".to_string()),
                    }
                }
            }
        }
    }

    // Graceful degradation: force text response instead of fatal error
    info!("Agent {} exceeded {} tool rounds, forcing final response", role, max_tool_rounds);
    messages.push(serde_json::json!({
        "role": "user",
        "content": "Tool call limit reached. Summarize your findings NOW based on what you have gathered so far."
    }));
    match stream_completion(role, &messages, None, app).await? {
        AgentResponse::Text(text) => Ok(text),
        _ => Ok(format!("[Agent {} exceeded tool limit without producing a response]", role)),
    }
}

/// Execute a single tool call
async fn execute_tool(
    tc: &StreamedToolCall,
    state: &OrchestratorState,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    use tauri::Emitter;

    let working_dir = state.working_directory.lock().unwrap().clone()
        .ok_or("No working directory set for tool execution")?;

    let args: serde_json::Value = serde_json::from_str(&tc.arguments)
        .map_err(|e| format!("Invalid tool arguments: {}", e))?;

    match tc.name.as_str() {
        "read_file" => {
            let path = args["path"].as_str()
                .ok_or("read_file requires 'path' argument")?;
            let full_path = validate_path_in_working_dir(&working_dir, path)?;
            match std::fs::read_to_string(&full_path) {
                Ok(content) => Ok(content),
                Err(e) => Ok(format!("Error reading file: {}", e)),
            }
        }

        "list_directory" => {
            let path = args["path"].as_str().unwrap_or(".");
            let target = if path == "." || path.is_empty() {
                std::path::PathBuf::from(&working_dir)
            } else {
                validate_path_in_working_dir(&working_dir, path)?
            };

            match std::fs::read_dir(&target) {
                Ok(entries) => {
                    let mut items: Vec<String> = Vec::new();
                    for entry in entries {
                        if let Ok(e) = entry {
                            let meta = e.metadata();
                            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                            let name = e.file_name().to_string_lossy().to_string();
                            items.push(if is_dir { format!("{}/", name) } else { name });
                        }
                    }
                    items.sort();
                    Ok(items.join("\n"))
                }
                Err(e) => Ok(format!("Error listing directory: {}", e)),
            }
        }

        "write_file" => {
            let path = args["path"].as_str()
                .ok_or("write_file requires 'path' argument")?;
            let content = args["content"].as_str()
                .ok_or("write_file requires 'content' argument")?;

            let full_path = validate_path_in_working_dir(&working_dir, path)?;
            let is_new = !full_path.exists();

            // Create a file proposal and wait for user approval
            let proposal = vec![FileProposal {
                path: path.to_string(),
                content: content.to_string(),
                is_new,
            }];

            let step = *state.step_counter.lock().unwrap();
            let _ = app.emit("orchestrator-file-proposal", FileProposalPayload {
                files: proposal.clone(),
                step_number: step,
            });

            *state.pending_file_proposals.lock().unwrap() = Some(proposal);
            *state.file_approval.lock().unwrap() = None;

            // Wait for user decision
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

                let approval = state.file_approval.lock().unwrap().take();
                if let Some(approved) = approval {
                    *state.pending_file_proposals.lock().unwrap() = None;
                    if approved {
                        // Write the file
                        if let Some(parent) = full_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        std::fs::write(&full_path, content)
                            .map_err(|e| format!("Failed to write {}: {}", path, e))?;
                        info!("Wrote file: {:?}", full_path);

                        let _ = app.emit("orchestrator-files-written", serde_json::json!({
                            "success": true,
                            "summary": format!("File written: {}", path),
                        }));

                        return Ok(format!("File '{}' written successfully ({} bytes)", path, content.len()));
                    } else {
                        return Ok(format!("User denied writing file '{}'. Try a different approach or ask the user.", path));
                    }
                }

                // Check if orchestrator was stopped
                let status = state.status.lock().unwrap().clone();
                if status == OrchestratorStatus::Idle {
                    return Err("Orchestrator stopped".to_string());
                }
            }
        }

        _ => Ok(format!("Unknown tool: {}", tc.name)),
    }
}

// --- System Prompts ---

pub fn system_prompt_orchestrator(working_dir: Option<&str>) -> String {
    let working_dir_section = if working_dir.is_some() {
        "\n\nA working directory is configured. Agents (Auditor, Constructor) have filesystem tools to read and write files. The Constructor can create files after user approval. The Auditor can explore the existing codebase.".to_string()
    } else {
        String::new()
    };

    format!(r#"You are the Orchestrator in a multi-agent software development team. Your role is to coordinate work between specialized agents.

Available agents:
- Architect: Designs architecture, researches solutions, creates technical plans
- Auditor: Reviews existing codebase, finds files, answers questions about code structure
- Constructor: Writes code following the Architect's directives
- Judge: Reviews plans for gaps, edge cases, and missing requirements

Communication format — you MUST use these markers:
[DELEGATE:Architect] Your instruction to the Architect
[DELEGATE:Auditor] Your instruction to the Auditor
[DELEGATE:Constructor] Your instruction to the Constructor
[DELEGATE:Judge] Your instruction to the Judge
[CHECKPOINT] Summary of progress so far (user can review and intervene)
[COMPLETE] Final summary of what was accomplished

IMPORTANT — Context passing:
Previous agent responses are stored as numbered Artifacts. You see only a SUMMARY of each response — the full content is passed to the next agent automatically. Do NOT copy or repeat previous agent outputs. Simply reference by ID:
  Example: "[DELEGATE:Judge] Review Artifact #1 (Architect's plan) for completeness and edge cases."
Older messages may be trimmed from your history to save context space. Always reference Artifacts by ID.

Rules:
1. You do NOT write code. You coordinate.
2. After the Architect produces a plan, ALWAYS delegate to Judge for review before proceeding to construction.
3. Break complex tasks into phases. Use [CHECKPOINT] between phases.
4. If an agent's response is insufficient, re-delegate with more specific instructions.
5. Start by delegating to Auditor to understand existing code, then to Architect to design, then Judge to review, then Constructor to implement.
6. Reference previous agent outputs by Artifact ID instead of repeating them.{}"#, working_dir_section)
}

pub fn system_prompt_architect() -> String {
    r#"You are the Architect in a software development team. Your role is to design architecture and create detailed technical plans.

Your responsibilities:
- Analyze requirements and design solutions
- Create detailed implementation plans with specific files, functions, and data structures
- Consider edge cases, performance, and maintainability
- Research and recommend approaches

Output detailed, actionable plans that a Constructor can follow step by step. Include specific file paths, function signatures, and data structures."#.to_string()
}

pub fn system_prompt_auditor(working_dir: Option<&str>) -> String {
    let tools_section = if working_dir.is_some() {
        r#"

You have tools to explore the project filesystem:
- read_file(path): Read a file's content.
- list_directory(path): List subdirectories (only if you need to explore deeper).

The project root directory listing is provided below in your context. Use read_file to inspect specific files.
IMPORTANT: Always use RELATIVE paths (e.g., 'src/main.py'). Never use absolute paths."#.to_string()
    } else {
        String::new()
    };

    format!(r#"You are the Auditor/Explorer in a software development team. Your role is to review and explore the existing codebase.

Your responsibilities:
- Answer questions about where specific functionality exists in the codebase
- Identify existing patterns, conventions, and utilities that should be reused
- Find relevant files, functions, and data structures
- Report on the current state of code related to the task

Be precise: include file paths, line numbers, function names, and relevant code snippets in your responses.{}"#, tools_section)
}

pub fn system_prompt_constructor(working_dir: Option<&str>) -> String {
    let tools_section = if working_dir.is_some() {
        r#"

You have tools to interact with the project filesystem:
- write_file(path, content): Create or overwrite a file. The user will approve each write.
- read_file(path): Read an existing file.
- list_directory(path): List subdirectories (only if needed).

The project root directory listing is provided below in your context. Use write_file to create files.
IMPORTANT: Always use RELATIVE paths (e.g., 'src/main.py'). Never use absolute paths."#.to_string()
    } else {
        String::new()
    };

    format!(r#"You are the Constructor in a software development team. Your role is to write code following the Architect's directives.

Your responsibilities:
- Translate architectural plans into concrete code
- Follow existing patterns and conventions in the codebase
- Write clean, maintainable code
- Include all necessary imports, types, and error handling

Output complete, ready-to-use code. Do not leave placeholders or TODOs.{}"#, tools_section)
}

pub fn system_prompt_judge() -> String {
    r#"You are the Judge in a software development team. Your role is to critically review plans and identify gaps.

Your responsibilities:
- Review architectural plans for completeness and correctness
- Identify missing edge cases, security issues, and potential bugs
- Check for inconsistencies with existing codebase patterns
- Suggest improvements and alternatives

You MUST identify at least 3 potential issues or improvements in every plan you review. Be constructive but thorough."#.to_string()
}

pub fn default_system_prompt(role: AgentRole, working_dir: Option<&str>) -> String {
    match role {
        AgentRole::Orchestrator => system_prompt_orchestrator(working_dir),
        AgentRole::Architect => system_prompt_architect(),
        AgentRole::Auditor => system_prompt_auditor(working_dir),
        AgentRole::Constructor => system_prompt_constructor(working_dir),
        AgentRole::Judge => system_prompt_judge(),
    }
}

// --- File proposal parsing ---

/// Parse Constructor output for file blocks. Supports:
/// ```path/to/file.ext
/// content
/// ```
/// and
/// // FILE: path/to/file.ext
/// ```lang
/// content
/// ```
pub fn parse_file_proposals(response: &str, working_dir: &str) -> Vec<FileProposal> {
    let mut proposals = Vec::new();
    let lines: Vec<&str> = response.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Pattern 1: ```path/to/file.ext or ```lang:path/to/file.ext
        if line.starts_with("```") {
            let after_ticks = &line[3..];
            // Check if it looks like a file path (contains / or . but not just a language name)
            let possible_path = after_ticks.trim();
            let is_file_path = !possible_path.is_empty()
                && (possible_path.contains('/') || possible_path.contains('.'))
                && !["json", "yaml", "toml", "xml", "html", "css", "md"].contains(&possible_path);

            if is_file_path {
                let file_path = possible_path.to_string();
                let mut content = String::new();
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(lines[i]);
                    i += 1;
                }
                // Skip closing ```
                if i < lines.len() {
                    i += 1;
                }

                let full_path = std::path::Path::new(working_dir).join(&file_path);
                let is_new = !full_path.exists();

                proposals.push(FileProposal {
                    path: file_path,
                    content,
                    is_new,
                });
                continue;
            }
        }

        // Pattern 2: // FILE: path/to/file.ext or # FILE: path/to/file.ext
        let file_marker = if let Some(rest) = line.strip_prefix("// FILE:") {
            Some(rest.trim().to_string())
        } else if let Some(rest) = line.strip_prefix("# FILE:") {
            Some(rest.trim().to_string())
        } else if let Some(rest) = line.strip_prefix("<!-- FILE:") {
            Some(rest.trim_end_matches("-->").trim().to_string())
        } else {
            None
        };

        if let Some(file_path) = file_marker {
            // Next line should be opening ```
            i += 1;
            if i < lines.len() && lines[i].trim().starts_with("```") {
                i += 1;
                let mut content = String::new();
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(lines[i]);
                    i += 1;
                }
                // Skip closing ```
                if i < lines.len() {
                    i += 1;
                }

                let full_path = std::path::Path::new(working_dir).join(&file_path);
                let is_new = !full_path.exists();

                proposals.push(FileProposal {
                    path: file_path,
                    content,
                    is_new,
                });
                continue;
            }
        }

        i += 1;
    }

    proposals
}

/// Truncate text to max_chars, cutting at the last newline before the limit.
pub fn truncate_for_context(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let cut = text[..max_chars].rfind('\n').unwrap_or(max_chars);
    format!("{}...\n[Truncated — full content in Artifact]", &text[..cut])
}

/// Build artifact context string for injecting into agent prompts.
/// Prioritizes recent artifacts and truncates each to fit within `max_chars` total budget.
pub fn build_artifact_context(artifacts: &[Artifact], max_chars: usize) -> String {
    if artifacts.is_empty() {
        return String::new();
    }

    let header = "\n\n=== Previous Agent Artifacts ===\n(Reference by Artifact ID. Content may be truncated.)\n\n";
    let footer = "=== End Artifacts ===\n";
    let overhead = header.len() + footer.len();
    if max_chars <= overhead {
        return String::new();
    }
    let budget = max_chars - overhead;

    // Per-artifact max: split budget among artifacts, minimum 200 chars each
    let per_artifact_max = (budget / artifacts.len()).max(200);
    let mut ctx = String::from(header);
    let mut used = 0;

    // Iterate in order (oldest first) but skip oldest if budget is tight
    for artifact in artifacts {
        if used >= budget {
            ctx.push_str(&format!(
                "[Artifact #{} — {} (step {})] [Truncated — budget exceeded]\n\n",
                artifact.id, artifact.from_role.label(), artifact.step_number,
            ));
            continue;
        }

        let header_line = format!(
            "[Artifact #{} — {} (step {})]\n",
            artifact.id, artifact.from_role.label(), artifact.step_number,
        );
        let remaining = budget.saturating_sub(used).saturating_sub(header_line.len() + 2);
        let content = truncate_for_context(&artifact.content, remaining.min(per_artifact_max));

        ctx.push_str(&header_line);
        ctx.push_str(&content);
        ctx.push_str("\n\n");
        used += header_line.len() + content.len() + 2;
    }

    ctx.push_str(footer);
    ctx
}

/// Validate that a path is safe (no traversal outside working dir)
pub fn validate_path_in_working_dir(working_dir: &str, relative_path: &str) -> Result<std::path::PathBuf, String> {
    let base = std::path::Path::new(working_dir).canonicalize()
        .map_err(|e| format!("Invalid working directory: {}", e))?;

    // If the model sent an absolute path, try to strip the working dir prefix
    let effective_path = if std::path::Path::new(relative_path).is_absolute() {
        let abs = std::path::Path::new(relative_path);
        let stripped = abs.strip_prefix(&base)
            .or_else(|_| abs.strip_prefix(std::path::Path::new(working_dir)))
            .map(|s| s.to_path_buf())
            .map_err(|_| format!(
                "Absolute path '{}' is outside the working directory '{}'",
                relative_path, working_dir
            ))?;
        // Model sent the working dir itself — return base directly
        if stripped.as_os_str().is_empty() {
            return Ok(base);
        }
        stripped
    } else {
        std::path::PathBuf::from(relative_path)
    };

    let target = base.join(&effective_path);

    // Canonicalize parent to check traversal (target file may not exist yet)
    let parent = target.parent()
        .ok_or_else(|| "Invalid file path".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create directories: {}", e))?;
    let canonical_parent = parent.canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    if !canonical_parent.starts_with(&base) {
        return Err(format!("Path traversal detected: {} escapes working directory", relative_path));
    }

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directives_delegate() {
        let input = "[DELEGATE:Architect] Design the user auth system\nUse JWT tokens for session management.";
        let directives = parse_directives(input);
        assert_eq!(directives.len(), 1);
        match &directives[0] {
            Directive::Delegate { role, instruction } => {
                assert_eq!(*role, AgentRole::Architect);
                assert!(instruction.contains("Design the user auth system"));
                assert!(instruction.contains("JWT tokens"));
            }
            _ => panic!("Expected Delegate"),
        }
    }

    #[test]
    fn test_parse_directives_multiple() {
        let input = "[DELEGATE:Auditor] Find auth patterns\n[DELEGATE:Architect] Design based on findings\n[CHECKPOINT] Phase 1 done\n[COMPLETE] All done";
        let directives = parse_directives(input);
        assert_eq!(directives.len(), 4);
        assert!(matches!(&directives[0], Directive::Delegate { role: AgentRole::Auditor, .. }));
        assert!(matches!(&directives[1], Directive::Delegate { role: AgentRole::Architect, .. }));
        assert!(matches!(&directives[2], Directive::Checkpoint { .. }));
        assert!(matches!(&directives[3], Directive::Complete { .. }));
    }

    #[test]
    fn test_parse_directives_no_markers() {
        let input = "Just some regular text without any markers.";
        let directives = parse_directives(input);
        assert_eq!(directives.len(), 1);
        assert!(matches!(&directives[0], Directive::Text(_)));
    }

    #[test]
    fn test_parse_role() {
        assert_eq!(parse_role("Architect"), Some(AgentRole::Architect));
        assert_eq!(parse_role("auditor"), Some(AgentRole::Auditor));
        assert_eq!(parse_role("CONSTRUCTOR"), Some(AgentRole::Constructor));
        assert_eq!(parse_role("Judge"), Some(AgentRole::Judge));
        assert_eq!(parse_role("unknown"), None);
    }

    #[test]
    fn test_parse_file_proposals_file_marker() {
        let response = r#"Here is the implementation:

// FILE: src/utils.ts
```typescript
export function add(a: number, b: number): number {
  return a + b;
}
```

// FILE: src/index.ts
```typescript
import { add } from './utils';
console.log(add(1, 2));
```
"#;
        let proposals = parse_file_proposals(response, "/tmp/nonexistent_test_dir");
        assert_eq!(proposals.len(), 2);
        assert_eq!(proposals[0].path, "src/utils.ts");
        assert!(proposals[0].content.contains("export function add"));
        assert_eq!(proposals[1].path, "src/index.ts");
        assert!(proposals[1].content.contains("import { add }"));
    }

    #[test]
    fn test_parse_file_proposals_path_in_ticks() {
        let response = r#"Here is the file:

```src/main.rs
fn main() {
    println!("Hello");
}
```
"#;
        let proposals = parse_file_proposals(response, "/tmp/nonexistent_test_dir");
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].path, "src/main.rs");
        assert!(proposals[0].content.contains("println!"));
    }

    #[test]
    fn test_build_artifact_context() {
        let artifacts = vec![
            Artifact {
                id: 1,
                from_role: AgentRole::Architect,
                step_number: 2,
                content: "Design plan here".to_string(),
            },
        ];
        let ctx = build_artifact_context(&artifacts, 6000);
        assert!(ctx.contains("Artifact #1"));
        assert!(ctx.contains("Architect"));
        assert!(ctx.contains("Design plan here"));
    }

    #[test]
    fn test_truncate_for_context_short() {
        let text = "Short text";
        assert_eq!(truncate_for_context(text, 100), "Short text");
    }

    #[test]
    fn test_truncate_for_context_long() {
        let text = "Line one\nLine two\nLine three\nLine four";
        let result = truncate_for_context(text, 20);
        assert!(result.contains("Line one"));
        assert!(result.contains("[Truncated"));
        assert!(!result.contains("Line four"));
    }

    #[test]
    fn test_build_artifact_context_budget_truncates() {
        let artifacts = vec![
            Artifact { id: 1, from_role: AgentRole::Architect, step_number: 1, content: "A".repeat(5000) },
            Artifact { id: 2, from_role: AgentRole::Auditor, step_number: 2, content: "B".repeat(5000) },
        ];
        let ctx = build_artifact_context(&artifacts, 2000);
        assert!(ctx.len() <= 2200); // some slack for headers
        assert!(ctx.contains("Artifact #1"));
        assert!(ctx.contains("Artifact #2"));
        assert!(ctx.contains("[Truncated"));
    }

    #[test]
    fn test_validate_path_relative() {
        let tmp = std::env::temp_dir().join("panopticon_test_validate");
        std::fs::create_dir_all(&tmp).unwrap();
        let result = validate_path_in_working_dir(tmp.to_str().unwrap(), "src/main.rs");
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("src/main.rs"));
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn test_validate_path_absolute_inside_working_dir() {
        let tmp = std::env::temp_dir().join("panopticon_test_validate_abs");
        let sub = tmp.join("src");
        std::fs::create_dir_all(&sub).unwrap();
        // Model sends absolute path that starts with working dir
        let abs_path = tmp.join("src/main.rs");
        let result = validate_path_in_working_dir(
            tmp.to_str().unwrap(),
            abs_path.to_str().unwrap(),
        );
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("src/main.rs"));
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn test_validate_path_absolute_outside_working_dir() {
        let tmp = std::env::temp_dir().join("panopticon_test_validate_out");
        std::fs::create_dir_all(&tmp).unwrap();
        let result = validate_path_in_working_dir(
            tmp.to_str().unwrap(),
            "/etc/passwd",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside the working directory"));
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn test_validate_path_absolute_equals_working_dir() {
        let tmp = std::env::temp_dir().join("panopticon_test_validate_eq");
        std::fs::create_dir_all(&tmp).unwrap();
        // Model sends the working dir itself as path
        let result = validate_path_in_working_dir(
            tmp.to_str().unwrap(),
            tmp.to_str().unwrap(),
        );
        assert!(result.is_ok());
        let canonical_tmp = tmp.canonicalize().unwrap();
        assert_eq!(result.unwrap(), canonical_tmp);
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn test_validate_path_traversal() {
        let tmp = std::env::temp_dir().join("panopticon_test_validate_trav");
        std::fs::create_dir_all(&tmp).unwrap();
        let result = validate_path_in_working_dir(
            tmp.to_str().unwrap(),
            "../../etc/passwd",
        );
        assert!(result.is_err());
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
