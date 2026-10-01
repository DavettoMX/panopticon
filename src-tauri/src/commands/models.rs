use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use tauri::State;
use crate::DbState;
use crate::db;
use crate::gguf::{self, GgufMetadata};
use crate::gpu::{self, GpuInfo};

#[derive(Debug, Serialize, Clone)]
pub struct ModelInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub parameter_count: u64,
}

#[tauri::command]
pub fn list_models(db: State<'_, DbState>) -> Result<Vec<ModelInfo>, String> {
    let conn = db.0.lock().unwrap();
    let models_dir = db::get_setting(&conn, "models_dir")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
            format!("{}/models", home)
        });

    let entries = fs::read_dir(&models_dir)
        .map_err(|e| format!("Cannot read models directory '{}': {}", models_dir, e))?;

    let mut models: Vec<ModelInfo> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                let metadata = entry.metadata().ok()?;
                let path_str = path.to_string_lossy().to_string();
                let parameter_count = gguf::read_gguf_metadata(&path_str)
                    .map(|m| m.parameter_count)
                    .unwrap_or(0);
                Some(ModelInfo {
                    filename: entry.file_name().to_string_lossy().to_string(),
                    path: path_str,
                    size_bytes: metadata.len(),
                    parameter_count,
                })
            } else {
                None
            }
        })
        .collect();

    models.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(models)
}

#[tauri::command]
pub fn get_model_details(model_path: String) -> Result<GgufMetadata, String> {
    gguf::read_gguf_metadata(&model_path)
}

#[tauri::command]
pub fn get_gpu_info() -> Result<GpuInfo, String> {
    gpu::detect_gpu()
}

#[tauri::command]
pub fn get_system_ram_mb() -> u64 {
    gpu::get_system_ram_mb()
}

#[tauri::command]
pub fn get_all_model_tags(db: State<'_, DbState>) -> Result<HashMap<String, Vec<String>>, String> {
    let conn = db.0.lock().unwrap();
    db::get_all_model_tags(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_model_tags(db: State<'_, DbState>, model_path: String, tags: Vec<String>) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    db::set_model_tags(&conn, &model_path, &tags).map_err(|e| e.to_string())
}
