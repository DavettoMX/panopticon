use rusqlite::{Connection, Result, params, ffi::sqlite3_auto_extension};
use serde::{Deserialize, Serialize};
use sqlite_vec::sqlite3_vec_init;
use std::path::Path;
use log::info;

pub fn init_db(db_path: &Path) -> Result<Connection> {
    // Register sqlite-vec as auto-extension before opening the connection
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }

    let conn = Connection::open(db_path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    create_tables(&conn)?;
    run_migrations(&conn)?;

    info!("Database initialized at {:?}", db_path);
    Ok(conn)
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            display_name TEXT,
            avatar_path TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            title TEXT NOT NULL DEFAULT 'New conversation',
            model TEXT,
            system_prompt TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
            content TEXT NOT NULL,
            tokens_input INTEGER,
            tokens_output INTEGER,
            model TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            filename TEXT NOT NULL,
            content TEXT,
            mime_type TEXT,
            chunk_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS document_chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            content TEXT NOT NULL,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_conversations_user ON conversations(user_id);
        CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
        CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);
        CREATE TABLE IF NOT EXISTS model_tags (
            model_path TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY (model_path, tag)
        );

        CREATE INDEX IF NOT EXISTS idx_documents_user ON documents(user_id);
        CREATE INDEX IF NOT EXISTS idx_chunks_document ON document_chunks(document_id);

        CREATE TABLE IF NOT EXISTS orchestrator_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            task_description TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'created',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            completed_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS orchestrator_agents (
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            model_path TEXT NOT NULL,
            ngl INTEGER,
            context_size INTEGER NOT NULL DEFAULT 8192,
            port INTEGER NOT NULL,
            PRIMARY KEY (session_id, role),
            FOREIGN KEY (session_id) REFERENCES orchestrator_sessions(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS orchestrator_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            from_role TEXT NOT NULL,
            to_role TEXT NOT NULL,
            content TEXT NOT NULL,
            message_type TEXT NOT NULL,
            step_number INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (session_id) REFERENCES orchestrator_sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_orch_messages_session ON orchestrator_messages(session_id, step_number);
        CREATE INDEX IF NOT EXISTS idx_orch_sessions_user ON orchestrator_sessions(user_id);
        "
    )?;

    // Create virtual table for vector embeddings (sqlite-vec)
    // 384 dimensions = default for common embedding models (e.g. all-MiniLM-L6-v2)
    conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS embeddings USING vec0(
            chunk_id TEXT PRIMARY KEY,
            embedding float[384]
        );
        "
    )?;

    Ok(())
}

fn run_migrations(conn: &Connection) -> Result<()> {
    // Add deleted_at column for soft-delete / trash
    let has_deleted_at: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('conversations') WHERE name = 'deleted_at'")?
        .query_row([], |row| row.get::<_, i64>(0))
        .map(|count| count > 0)?;

    if !has_deleted_at {
        conn.execute_batch("ALTER TABLE conversations ADD COLUMN deleted_at TEXT")?;
        info!("Migration: added deleted_at column to conversations");
    }

    Ok(())
}

/// Permanently delete conversations that have been in trash for more than 30 days.
/// Called once on app startup.
pub fn purge_expired_trash(conn: &Connection) -> Result<usize> {
    let deleted = conn.execute(
        "DELETE FROM conversations WHERE deleted_at IS NOT NULL AND datetime(deleted_at, '+30 days') < datetime('now')",
        [],
    )?;
    if deleted > 0 {
        info!("Purged {} expired conversations from trash", deleted);
    }
    Ok(deleted)
}

// --- Data structs ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub model: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub created_at: String,
}

// --- User ---

const DEFAULT_USER_ID: &str = "00000000-0000-0000-0000-000000000000";

pub fn ensure_default_user(conn: &Connection) -> Result<String> {
    conn.execute(
        "INSERT OR IGNORE INTO users (id, username, display_name) VALUES (?1, ?2, ?3)",
        params![DEFAULT_USER_ID, "local", "User"],
    )?;
    Ok(DEFAULT_USER_ID.to_string())
}

pub fn default_user_id() -> String {
    DEFAULT_USER_ID.to_string()
}

// --- Settings ---

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
        params![key, value],
    )?;
    Ok(())
}

pub fn ensure_default_settings(conn: &Connection) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    let defaults = [
        ("models_dir", format!("{}/models", home)),
        ("llama_server_path", format!("{}/src/llama.cpp/build/bin/llama-server", home)),
    ];
    for (key, value) in &defaults {
        if get_setting(conn, key)?.is_none() {
            set_setting(conn, key, value)?;
        }
    }
    Ok(())
}

// --- Conversations ---

pub fn create_conversation(conn: &Connection, user_id: &str, title: &str, model: Option<&str>) -> Result<Conversation> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO conversations (id, user_id, title, model) VALUES (?1, ?2, ?3, ?4)",
        params![id, user_id, title, model],
    )?;
    get_conversation(conn, &id)
}

pub fn list_conversations(conn: &Connection, user_id: &str) -> Result<Vec<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, model, created_at, updated_at, deleted_at FROM conversations
         WHERE user_id = ?1 AND deleted_at IS NULL ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Conversation {
            id: row.get(0)?,
            title: row.get(1)?,
            model: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn list_trash(conn: &Connection, user_id: &str) -> Result<Vec<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, model, created_at, updated_at, deleted_at FROM conversations
         WHERE user_id = ?1 AND deleted_at IS NOT NULL ORDER BY deleted_at DESC"
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Conversation {
            id: row.get(0)?,
            title: row.get(1)?,
            model: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn get_conversation(conn: &Connection, id: &str) -> Result<Conversation> {
    conn.query_row(
        "SELECT id, title, model, created_at, updated_at, deleted_at FROM conversations WHERE id = ?1",
        params![id],
        |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                model: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
            })
        },
    )
}

pub fn update_conversation_title(conn: &Connection, id: &str, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![title, id],
    )?;
    Ok(())
}

/// Soft-delete: move to trash
pub fn soft_delete_conversation(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET deleted_at = datetime('now') WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Restore from trash
pub fn restore_conversation(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET deleted_at = NULL WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Permanent delete
pub fn delete_conversation(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
    Ok(())
}

// --- Messages ---

pub fn insert_message(conn: &Connection, conversation_id: &str, role: &str, content: &str, model: Option<&str>) -> Result<Message> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO messages (id, conversation_id, role, content, model) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, conversation_id, role, content, model],
    )?;
    // Also touch conversation updated_at
    conn.execute(
        "UPDATE conversations SET updated_at = datetime('now') WHERE id = ?1",
        params![conversation_id],
    )?;
    conn.query_row(
        "SELECT id, conversation_id, role, content, model, created_at FROM messages WHERE id = ?1",
        params![id],
        |row| {
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                model: row.get(4)?,
                created_at: row.get(5)?,
            })
        },
    )
}

pub fn get_messages(conn: &Connection, conversation_id: &str) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, model, created_at FROM messages
         WHERE conversation_id = ?1 ORDER BY created_at ASC"
    )?;
    let rows = stmt.query_map(params![conversation_id], |row| {
        Ok(Message {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            model: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

// --- Model Tags ---

pub fn get_all_model_tags(conn: &Connection) -> Result<std::collections::HashMap<String, Vec<String>>> {
    let mut stmt = conn.prepare("SELECT model_path, tag FROM model_tags ORDER BY model_path, tag")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for row in rows {
        let (path, tag) = row?;
        map.entry(path).or_default().push(tag);
    }
    Ok(map)
}

pub fn set_model_tags(conn: &Connection, model_path: &str, tags: &[String]) -> Result<()> {
    conn.execute("DELETE FROM model_tags WHERE model_path = ?1", params![model_path])?;
    let mut stmt = conn.prepare("INSERT INTO model_tags (model_path, tag) VALUES (?1, ?2)")?;
    for tag in tags {
        stmt.execute(params![model_path, tag])?;
    }
    Ok(())
}

// --- Orchestrator ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrchestratorSession {
    pub id: String,
    pub task_description: String,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrchestratorAgentRow {
    pub session_id: String,
    pub role: String,
    pub model_path: String,
    pub ngl: Option<i32>,
    pub context_size: i32,
    pub port: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrchestratorMessageRow {
    pub id: String,
    pub session_id: String,
    pub from_role: String,
    pub to_role: String,
    pub content: String,
    pub message_type: String,
    pub step_number: i32,
    pub created_at: String,
}

pub fn create_orchestrator_session(conn: &Connection, user_id: &str, task: &str) -> Result<OrchestratorSession> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO orchestrator_sessions (id, user_id, task_description) VALUES (?1, ?2, ?3)",
        params![id, user_id, task],
    )?;
    conn.query_row(
        "SELECT id, task_description, status, created_at, completed_at FROM orchestrator_sessions WHERE id = ?1",
        params![id],
        |row| Ok(OrchestratorSession {
            id: row.get(0)?,
            task_description: row.get(1)?,
            status: row.get(2)?,
            created_at: row.get(3)?,
            completed_at: row.get(4)?,
        }),
    )
}

pub fn update_session_status(conn: &Connection, session_id: &str, status: &str) -> Result<()> {
    let completed_at = if status == "complete" || status == "error" {
        "datetime('now')"
    } else {
        "NULL"
    };
    conn.execute(
        &format!(
            "UPDATE orchestrator_sessions SET status = ?1, completed_at = {} WHERE id = ?2",
            completed_at
        ),
        params![status, session_id],
    )?;
    Ok(())
}

pub fn insert_orchestrator_agent(
    conn: &Connection,
    session_id: &str,
    role: &str,
    model_path: &str,
    ngl: Option<i32>,
    context_size: i32,
    port: i32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO orchestrator_agents (session_id, role, model_path, ngl, context_size, port) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![session_id, role, model_path, ngl, context_size, port],
    )?;
    Ok(())
}

pub fn insert_orchestrator_message(
    conn: &Connection,
    session_id: &str,
    from_role: &str,
    to_role: &str,
    content: &str,
    message_type: &str,
    step_number: i32,
) -> Result<OrchestratorMessageRow> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO orchestrator_messages (id, session_id, from_role, to_role, content, message_type, step_number) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, session_id, from_role, to_role, content, message_type, step_number],
    )?;
    conn.query_row(
        "SELECT id, session_id, from_role, to_role, content, message_type, step_number, created_at FROM orchestrator_messages WHERE id = ?1",
        params![id],
        |row| Ok(OrchestratorMessageRow {
            id: row.get(0)?,
            session_id: row.get(1)?,
            from_role: row.get(2)?,
            to_role: row.get(3)?,
            content: row.get(4)?,
            message_type: row.get(5)?,
            step_number: row.get(6)?,
            created_at: row.get(7)?,
        }),
    )
}

pub fn get_orchestrator_messages(conn: &Connection, session_id: &str) -> Result<Vec<OrchestratorMessageRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, from_role, to_role, content, message_type, step_number, created_at
         FROM orchestrator_messages WHERE session_id = ?1 ORDER BY step_number ASC, created_at ASC"
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        Ok(OrchestratorMessageRow {
            id: row.get(0)?,
            session_id: row.get(1)?,
            from_role: row.get(2)?,
            to_role: row.get(3)?,
            content: row.get(4)?,
            message_type: row.get(5)?,
            step_number: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn list_orchestrator_sessions(conn: &Connection, user_id: &str) -> Result<Vec<OrchestratorSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_description, status, created_at, completed_at
         FROM orchestrator_sessions WHERE user_id = ?1 ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(OrchestratorSession {
            id: row.get(0)?,
            task_description: row.get(1)?,
            status: row.get(2)?,
            created_at: row.get(3)?,
            completed_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn get_orchestrator_agents(conn: &Connection, session_id: &str) -> Result<Vec<OrchestratorAgentRow>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, role, model_path, ngl, context_size, port
         FROM orchestrator_agents WHERE session_id = ?1"
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        Ok(OrchestratorAgentRow {
            session_id: row.get(0)?,
            role: row.get(1)?,
            model_path: row.get(2)?,
            ngl: row.get(3)?,
            context_size: row.get(4)?,
            port: row.get(5)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_init_db_in_memory() {
        let path = PathBuf::from(":memory:");
        let conn = init_db(&path).expect("Failed to init DB");

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"conversations".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"documents".to_string()));
        assert!(tables.contains(&"document_chunks".to_string()));
        assert!(tables.contains(&"embeddings".to_string()));
    }
}

pub fn delete_orchestrator_session(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM orchestrator_messages WHERE session_id = ?1", params![id])?;
    conn.execute("DELETE FROM orchestrator_agents WHERE session_id = ?1", params![id])?;
    conn.execute("DELETE FROM orchestrator_sessions WHERE id = ?1", params![id])?;
    Ok(())
}

