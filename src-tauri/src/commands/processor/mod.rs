//! v1.1.0 处理器类 IPC 命令（脱敏 / 校验 / 提取 / 规则管理）。
//!
//! v1.2.0 T92：从单文件 `processor.rs` 拆分为 4 个子模块，按功能域聚合：
//! - [`rules_ops`]：规则管理（list / toggle / update_params / update_template /
//!   update_extract_config）
//! - [`mask_ops`]：脱敏命令（mask_column）+ 脱敏/校验/提取结果结构体
//! - [`extract_ops`]：校验 / 提取 / 提取+校验→新 Tab / 行级多字段校验 / 多规则校验
//! - [`undo_ops`]：撤销 / 重做 / 可撤销列表
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（列读写 + 规则持久化）。

pub mod extract_ops;
pub mod mask_ops;
pub mod rules_ops;
pub mod undo_ops;

pub use extract_ops::*;
pub use mask_ops::*;
pub use rules_ops::*;
pub use undo_ops::*;
