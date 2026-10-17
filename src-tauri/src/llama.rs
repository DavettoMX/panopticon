use std::sync::Mutex;
use tokio::process::{Child, Command};
use tokio::time::{Duration, sleep};
use log::{info, error};

pub struct LlamaState {
    pub process: Mutex<Option<Child>>,
    pub port: Mutex<u16>,
    pub active_model: Mutex<Option<String>>,
    pub is_generating: Mutex<bool>,
}

impl LlamaState {
    pub fn new() -> Self {
        Self {
            process: Mutex::new(None),
            port: Mutex::new(8080),
            active_model: Mutex::new(None),
            is_generating: Mutex::new(false),
        }
    }
}

pub async fn start_server(
    state: &LlamaState,
    server_path: &str,
    model_path: &str,
    port: u16,
    context_size: u32,
    ngl: Option<i32>,
) -> Result<(), String> {
    // Stop any running server first
    stop_server(state).await?;

    info!("Starting llama-server with model: {}", model_path);

    // Log file for llama-server output
    let log_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let log_path = format!("{}/.local/share/com.panopticon.app/llama-server.log", log_dir);
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| format!("Failed to create log file: {}", e))?;
    let stderr_file = log_file.try_clone()
        .map_err(|e| format!("Failed to clone log file: {}", e))?;

    let child = Command::new(server_path)
        .arg("-m")
        .arg(model_path)
        .arg("--port")
        .arg(port.to_string())
        .arg("--host")
        .arg("127.0.0.1")
        .arg("-ngl")
        .arg(match ngl {
            Some(n) => n.to_string(),
            None => "auto".to_string(),
        })
        .arg("-c")
        .arg(context_size.to_string())
        .arg("--jinja")
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(stderr_file))
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start llama-server: {}", e))?;

    *state.process.lock().unwrap() = Some(child);
    *state.port.lock().unwrap() = port;
    *state.active_model.lock().unwrap() = Some(model_path.to_string());

    wait_for_ready(state, port).await?;

    info!("llama-server ready on port {}", port);
    Ok(())
}

pub async fn stop_server(state: &LlamaState) -> Result<(), String> {
    let mut child = {
        let mut process = state.process.lock().unwrap();
        process.take()
    };
    if let Some(ref mut child) = child {
        info!("Stopping llama-server");
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    *state.active_model.lock().unwrap() = None;
    Ok(())
}

async fn wait_for_ready(state: &LlamaState, port: u16) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::new();
    let timeout = Duration::from_secs(120);
    let start = tokio::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err("llama-server startup timed out after 120s".to_string());
        }

        // Check if the process is still alive
        {
            let mut process = state.process.lock().unwrap();
            if let Some(ref mut child) = *process {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        *process = None;
                        error!("llama-server exited with status: {}", status);
                        // Try to read log file for details
                        let log_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                        let log_path = format!("{}/.local/share/com.panopticon.app/llama-server.log", log_dir);
                        let log_content = std::fs::read_to_string(&log_path).unwrap_or_default();
                        let last_lines: String = log_content.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
                        return Err(format!("llama-server crashed (exit {}). Last log:\n{}", status, last_lines));
                    }
                    Ok(None) => {} // still running
                    Err(e) => {
                        return Err(format!("Failed to check llama-server status: {}", e));
                    }
                }
            } else {
                return Err("llama-server process not found".to_string());
            }
        }

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                return Ok(());
            }
            _ => {
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

/// Standalone server launch — returns Child directly without storing in LlamaState.
/// Used by the orchestrator to spawn multiple concurrent llama-server instances.
pub async fn start_server_standalone(
    server_path: &str,
    model_path: &str,
    port: u16,
    context_size: u32,
    ngl: Option<i32>,
) -> Result<Child, String> {
    info!("Starting standalone llama-server on port {} with model: {}", port, model_path);

    let log_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let log_path = format!("{}/.local/share/com.panopticon.app/llama-server-{}.log", log_dir, port);
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| format!("Failed to create log file: {}", e))?;
    let stderr_file = log_file.try_clone()
        .map_err(|e| format!("Failed to clone log file: {}", e))?;

    let child = Command::new(server_path)
        .arg("-m")
        .arg(model_path)
        .arg("--port")
        .arg(port.to_string())
        .arg("--host")
        .arg("127.0.0.1")
        .arg("-ngl")
        .arg(match ngl {
            Some(n) => n.to_string(),
            None => "auto".to_string(),
        })
        .arg("-c")
        .arg(context_size.to_string())
        .arg("--jinja")
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(stderr_file))
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start llama-server on port {}: {}", port, e))?;

    Ok(child)
}

/// Poll /health endpoint on an arbitrary port until server is ready.
pub async fn wait_for_ready_on_port(port: u16, timeout_secs: u64) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::new();
    let timeout = Duration::from_secs(timeout_secs);
    let start = tokio::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(format!("llama-server on port {} timed out after {}s", port, timeout_secs));
        }

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("llama-server on port {} is ready", port);
                return Ok(());
            }
            _ => {
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
}
