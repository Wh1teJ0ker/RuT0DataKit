//! RuT0DataKit core engine.
//!
//! v1.0.0: datasource / pcap / model 骨架。
//! v1.1.0: 新增 `processor` 模块——脱敏/校验/提取原型 + 规则管理基础结构
//! （三条姓名相关内置规则：脱敏/校验/提取各一条，持久化到 DB）。

// crate 名 `ruT0-data-kit-core` 为品牌命名（非 snake_case），有意保留；
// 改名将牵动 workspace dep / Cargo.toml `[lib].name` / 所有 `use`，得不偿失。
#![allow(non_snake_case)]

pub mod datasource;
pub mod error;
pub mod model;
pub mod pcap;
pub mod processor;

pub use error::CoreError;
