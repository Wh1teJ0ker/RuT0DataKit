//! RuT0DataKit core engine.
//!
//! v1.0.0: 仅骨架 + trait 占位。具体 masker/validator/extractor/rules/reader
//! 实现推迟到 v1.1+，以「实现 trait + 注册到 registry」方式增量落地。

// crate 名 `ruT0-data-kit-core` 为品牌命名（非 snake_case），有意保留；
// 改名将牵动 workspace dep / Cargo.toml `[lib].name` / 所有 `use`，得不偿失。
#![allow(non_snake_case)]

pub mod datasource;
pub mod error;
pub mod model;
pub mod pcap;
pub mod processor;

pub use error::CoreError;
