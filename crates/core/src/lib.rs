//! RuT0DataKit core engine.
//!
//! v1.0.0: 仅 datasource / pcap / model 骨架。具体 masker/validator/extractor/rules
//! 实现推迟到 v1.1+，届时应在具备真实实现时新建 `processor` 模块（不再保留空骨架）。

// crate 名 `ruT0-data-kit-core` 为品牌命名（非 snake_case），有意保留；
// 改名将牵动 workspace dep / Cargo.toml `[lib].name` / 所有 `use`，得不偿失。
#![allow(non_snake_case)]

pub mod datasource;
pub mod error;
pub mod model;
pub mod pcap;

pub use error::CoreError;
