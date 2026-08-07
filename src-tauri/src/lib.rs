//! Tauri v2 应用入口。
//!
//! 装配 `tauri-plugin-dialog`、`tauri-plugin-fs`、`tauri-plugin-updater` 与 `DbManager`（SQLite 持久层）。
//!
//! v1.1.0: 规则持久化到 DB（`rules` 表），启动时 `seed_builtin_rules`；
//! 注册 6 个 processor IPC（`mask_column` / `validate_column` / `extract_column`
//! / `list_rules` / `toggle_rule` / `update_rule_params`）。
//! v1.1.1: 新增 3 个撤销/重做 IPC（`undo_operation` / `redo_operation`
//! / `list_undoable_operations`）；`mask_column` 改用
//! `log_operation_with_snapshot` 存 before/after 快照。

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
            commands::processor::mask_column,
            commands::processor::validate_column,
            commands::processor::extract_column,
            commands::processor::list_rules,
            commands::processor::toggle_rule,
            commands::processor::update_rule_params,
            commands::processor::undo_operation,
            commands::processor::redo_operation,
            commands::processor::list_undoable_operations,
        ])
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_manager = DbManager::new(&dir)?;
            // v1.1.0：启动时若 DB 无规则则 seed 三条内置姓名规则。
            db_manager.seed_builtin_rules()?;
            app.manage(db_manager);
            // 启动时加载 tshark 路径并注入 core 运行时（同步执行）。
            let settings = commands::read_settings(app.handle());
            ruT0_data_kit_core::pcap::set_tshark_path(settings.tshark_path.clone());
            Ok(())
        })
        // 命令注册见上方 generate_handler! 列表。
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
