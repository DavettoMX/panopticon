mod commands;
mod db;
mod gguf;
mod gpu;
mod llama;

use std::sync::Mutex;
use tauri::Manager;

pub struct DbState(pub Mutex<rusqlite::Connection>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let app_data_dir = app.path().app_data_dir()?;
      std::fs::create_dir_all(&app_data_dir)?;
      let db_path = app_data_dir.join("panopticon.db");

      let conn = db::init_db(&db_path)
        .expect("Failed to initialize database");

      db::ensure_default_user(&conn).expect("Failed to create default user");
      db::ensure_default_settings(&conn).expect("Failed to set default settings");
      db::purge_expired_trash(&conn).expect("Failed to purge expired trash");

      app.manage(DbState(Mutex::new(conn)));
      app.manage(llama::LlamaState::new());

      log::info!("Database ready at {:?}", db_path);

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::settings::get_setting,
      commands::settings::set_setting,
      commands::models::list_models,
      commands::models::get_model_details,
      commands::models::get_gpu_info,
      commands::models::get_system_ram_mb,
      commands::models::get_all_model_tags,
      commands::models::set_model_tags,
      commands::server::start_model,
      commands::server::stop_model,
      commands::server::get_server_status,
      commands::conversations::list_conversations,
      commands::conversations::create_conversation,
      commands::conversations::delete_conversation,
      commands::conversations::rename_conversation,
      commands::conversations::list_trash,
      commands::conversations::restore_conversation,
      commands::conversations::permanent_delete_conversation,
      commands::chat::get_messages,
      commands::chat::send_message,
      commands::chat::stop_generation,
    ])
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|app, event| {
      if let tauri::RunEvent::Exit = event {
        // kill_on_drop(true) is set on the child, so dropping it kills llama-server
        let llama_state = app.state::<llama::LlamaState>();
        let mut process = llama_state.process.lock().unwrap();
        if process.is_some() {
          log::info!("Dropping llama-server process on exit");
          *process = None;
        }
      }
    });
}
