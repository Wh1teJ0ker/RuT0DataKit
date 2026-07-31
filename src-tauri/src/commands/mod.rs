//! Tauri 命令层入口（模块聚合点）。
//!
//! v1.0.0 拆分为 3 类关注点子模块（原拟 4 类含 rules，但 baseline 无规则
//! CRUD 命令，故不建 rules.rs）：
//! - `commands::data`     → import_file / get_sheet_data / ai_suggest / invoke_ai_op
//! - `commands::settings` → tshark 设置 IO + detect_tshark / load_tshark_path / save_tshark_path
//! - `commands::update`   → check_update
//!
//! 这里不直接放命令实现，仅作子模块声明 + 类型重导出，便于 `lib.rs` 的
//! `invoke_handler!` 用 `commands::xxx` 全路径注册命令。
//!
//! updater 相关命令（`check_update`），委托 `tauri-plugin-updater` 完成
//! 版本检测、下载、Ed25519 验签与安装。

pub mod data;
pub mod settings;
pub mod update;

// `lib.rs` 的 `setup` 钩子需要 crate 内访问 `read_settings`；其在
// `settings` 模块内为 `pub(crate)`，通过 `pub(crate)` 重导出，避免 `pub`
// re-export `pub(crate)` 项报 E0364/E0365。
pub(crate) use settings::read_settings;
