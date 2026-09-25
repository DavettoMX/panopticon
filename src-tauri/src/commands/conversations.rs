use tauri::State;
use crate::DbState;
use crate::db::{self, Conversation};

#[tauri::command]
pub fn list_conversations(db: State<'_, DbState>) -> Result<Vec<Conversation>, String> {
    let conn = db.0.lock().unwrap();
    let user_id = db::default_user_id();
    db::list_conversations(&conn, &user_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_conversation(db: State<'_, DbState>, title: Option<String>) -> Result<Conversation, String> {
    let conn = db.0.lock().unwrap();
    let user_id = db::default_user_id();
    let title = title.unwrap_or_else(|| "New conversation".to_string());
    db::create_conversation(&conn, &user_id, &title, None).map_err(|e| e.to_string())
}

/// Soft-delete: move conversation to trash
#[tauri::command]
pub fn delete_conversation(db: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    db::soft_delete_conversation(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_conversation(db: State<'_, DbState>, id: String, title: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    db::update_conversation_title(&conn, &id, &title).map_err(|e| e.to_string())
}

/// List conversations in trash
#[tauri::command]
pub fn list_trash(db: State<'_, DbState>) -> Result<Vec<Conversation>, String> {
    let conn = db.0.lock().unwrap();
    let user_id = db::default_user_id();
    db::list_trash(&conn, &user_id).map_err(|e| e.to_string())
}

/// Restore conversation from trash
#[tauri::command]
pub fn restore_conversation(db: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    db::restore_conversation(&conn, &id).map_err(|e| e.to_string())
}

/// Permanently delete conversation (from trash)
#[tauri::command]
pub fn permanent_delete_conversation(db: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    db::delete_conversation(&conn, &id).map_err(|e| e.to_string())
}
