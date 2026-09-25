use serde::Serialize;
use std::fs;
use tauri::State;
use crate::DbState;
use crate::db;

#[derive(Debug, Serialize, Clone)]
pub struct ModelInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
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
                Some(ModelInfo {
                    filename: entry.file_name().to_string_lossy().to_string(),
                    path: path.to_string_lossy().to_string(),
                    size_bytes: metadata.len(),
                })
            } else {
                None
            }
        })
        .collect();

    models.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(models)
}
