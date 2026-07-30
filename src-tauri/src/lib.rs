//! Tauri v2 应用入口。
//!
//! v1.0.0: 装配 `tauri-plugin-dialog` 与 `DbManager`（SQLite 持久层）。
//! - `tauri-plugin-updater` 装配与 `check_update`/`install_update` 命令 → T6。
//! - `import_file` / `get_sheet_data` / `ai_suggest` 等命令 → T5/T7。

mod db;

use db::DbManager;
use tauri::Manager;

/// 启动 Tauri 应用。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // .plugin(tauri_plugin_updater::Builder::new().build())  // T6 装配
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_manager = DbManager::new(&dir)?;
            app.manage(db_manager);
            Ok(())
        })
        // .invoke_handler(tauri::generate_handler![...])         // T5/T7 注册命令
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
