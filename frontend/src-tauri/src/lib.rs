mod db;

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

      app.manage(DbState(Mutex::new(conn)));
      log::info!("Database ready at {:?}", db_path);

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
