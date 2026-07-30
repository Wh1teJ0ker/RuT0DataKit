//! RuT0DataKit core engine.
//!
//! v1.0.0: 仅骨架 + trait 占位。具体 masker/validator/extractor/rules/reader
//! 实现推迟到 v1.1+，以「实现 trait + 注册到 registry」方式增量落地。

pub mod datasource;
pub mod error;
pub mod model;
pub mod processor;

pub use error::CoreError;
