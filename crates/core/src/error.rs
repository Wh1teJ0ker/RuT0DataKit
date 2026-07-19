//! Core 错误类型。
//!
//! 所有公共 API 返回 `Result<T, CoreError>`，用 `thiserror` 派生。
//! v0.1.0 即提前定义 `DependencyMissing`（v0.1.2 tshark 用）。

use std::io;
use thiserror::Error;

/// 核心 crate 统一错误类型。
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),

    #[error("yaml error: {0}")]
    Yaml(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("missing dependency: {0}")]
    DependencyMissing(String),

    #[error("other error: {0}")]
    Other(String),
}
