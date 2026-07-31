//! Tauri v2 应用入口。
//!
//! v1.0.0: 装配 `tauri-plugin-dialog`、`tauri-plugin-updater` 与 `DbManager`（SQLite 持久层）。
//! - `check_update` / `install_update` 命令 → `commands`（本任务 T6 装配）。
//! - `import_file` / `get_sheet_data` / `ai_suggest` 等命令 → T5/T7。

// crate 名 `ruT0-data-kit` 为品牌命名（非 snake_case），有意保留；
// 改名将牵动 workspace dep / Cargo.toml `[lib].name` / main.rs 调用，得不偿失。
#![allow(non_snake_case)]

mod commands;
mod db;

use db::DbManager;
use tauri::Manager;

/// 启动 Tauri 应用。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::update::check_update,
            commands::update::install_update,
            commands::data::ai_suggest,
            commands::data::invoke_ai_op,
            commands::data::import_file,
            commands::data::get_sheet_data,
            commands::settings::detect_tshark,
            commands::settings::load_tshark_path,
            commands::settings::save_tshark_path,
        ])
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_manager = DbManager::new(&dir)?;
            app.manage(db_manager);
            // 启动时加载 tshark 路径并注入 core 运行时（同步执行）。
            let settings = commands::read_settings(&app.handle());
            ruT0_data_kit_core::pcap::set_tshark_path(settings.tshark_path.clone());
            Ok(())
        })
        // import_file / get_sheet_data / ai_suggest 等命令 → T5/T7 注册
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
