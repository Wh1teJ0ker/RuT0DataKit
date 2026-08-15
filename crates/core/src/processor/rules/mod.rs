//! 规则管理基础结构。
//!
//! v1.2.1：原 `rules/mod.rs`（627 源码）按职责拆分为 4 个子模块：
//! - [`rule`]：`Rule` 结构 + `RuleKind` 枚举 + Display/from_str
//! - [`registry`]：`RuleRegistry`（CRUD + `with_defaults`）
//! - [`builtins`]：18 条内置规则构造器（`BuiltinRules`）
//! - [`extract_params`]：`ExtractParams` 枚举（v1.2.0 T93 已拆出）
//! - [`template`]：`TemplateParams` / `SimpleTemplate` / `SegmentTemplate`
//!   + 预设构造器（v1.2.0 T93 已拆出）
//!
//! 本文件仅作 `mod` 声明 + `pub use` 再导出，保持路径兼容。
//!
//! 历史版本规则变更详见 `RuleRegistry::with_defaults` 与 `BuiltinRules` 各构造器文档。

pub mod builtins;
pub mod extract_params;
pub mod registry;
pub mod rule;
pub mod template;

pub use builtins::BuiltinRules;
pub use extract_params::ExtractParams;
pub use registry::RuleRegistry;
pub use rule::{Rule, RuleKind};
pub use template::{
    bankcard_preset, birthdate_preset, idcard_preset, phone_preset, SegmentMask, SegmentTemplate,
    SimpleTemplate, TemplateParams,
};
