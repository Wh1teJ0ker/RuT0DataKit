//! Tauri v2 应用入口。
//!
//! v1.0.0: 仅装配 `tauri-plugin-dialog`，验证 dev 链路。
//! - `tauri-plugin-updater` 装配与 `check_update`/`install_update` 命令 → T6。
//! - `rusqlite` / `DbManager` 装配与 `manage` 注入 → T4。
//! - `import_file` / `get_sheet_data` / `ai_suggest` 等命令 → T5/T7。

/// 启动 Tauri 应用。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // .plugin(tauri_plugin_updater::Builder::new().build())  // T6 装配
        // .manage(DbManager::new(...).unwrap())                  // T4 注入
        // .invoke_handler(tauri::generate_handler![...])         // T5/T7 注册命令
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
