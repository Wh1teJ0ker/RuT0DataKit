//! Tauri v2 应用入口。
//!
//! v1.0.0: 装配 `tauri-plugin-dialog`、`tauri-plugin-updater` 与 `DbManager`（SQLite 持久层）。
//! - `check_update` / `install_update` 命令 → `commands`（本任务 T6 装配）。
//! - `import_file` / `get_sheet_data` / `ai_suggest` 等命令 → T5/T7。

mod commands;
mod db;

use db::DbManager;
use tauri::Manager;

/// 启动 Tauri 应用。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::check_update,
            commands::install_update,
            commands::ai_suggest,
            commands::invoke_ai_op,
            commands::import_file,
            commands::get_sheet_data,
        ])
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_manager = DbManager::new(&dir)?;
            app.manage(db_manager);
            Ok(())
        })
        // import_file / get_sheet_data / ai_suggest 等命令 → T5/T7 注册
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
