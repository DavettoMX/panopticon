use tauri::State;
use crate::DbState;
use crate::db;

#[tauri::command]
pub fn get_setting(db: State<'_, DbState>, key: String) -> Result<Option<String>, String> {
    let conn = db.0.lock().unwrap();
    db::get_setting(&conn, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(db: State<'_, DbState>, key: String, value: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    db::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}
