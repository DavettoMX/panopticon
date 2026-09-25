use serde::Serialize;
use tauri::State;
use crate::DbState;
use crate::db;
use crate::llama::{self, LlamaState};

#[derive(Debug, Serialize, Clone)]
pub struct ServerStatus {
    pub status: String,
    pub model: Option<String>,
}

#[tauri::command]
pub async fn start_model(
    db: State<'_, DbState>,
    llama: State<'_, LlamaState>,
    model_path: String,
) -> Result<(), String> {
    let (server_path, context_size) = {
        let conn = db.0.lock().unwrap();
        let path = db::get_setting(&conn, "llama_server_path")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
                format!("{}/src/llama.cpp/build/bin/llama-server", home)
            });
        let ctx = db::get_setting(&conn, "context_size")
            .map_err(|e| e.to_string())?
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(8192);
        (path, ctx)
    };

    let port = *llama.port.lock().unwrap();
    llama::start_server(&llama, &server_path, &model_path, port, context_size).await
}

#[tauri::command]
pub async fn stop_model(llama: State<'_, LlamaState>) -> Result<(), String> {
    llama::stop_server(&llama).await
}

#[tauri::command]
pub fn get_server_status(llama: State<'_, LlamaState>) -> ServerStatus {
    let model = llama.active_model.lock().unwrap().clone();
    let status = if model.is_some() { "ready" } else { "stopped" };
    ServerStatus {
        status: status.to_string(),
        model,
    }
}
