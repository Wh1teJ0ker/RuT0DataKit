//! Core engine error type.
//!
//! v1.0.0: 仅定义枚举骨架，被 src-tauri 命令层 `map_err(|e| e.to_string())`
//! 转换为 `Result<_, String>` 后返回前端。

use thiserror::Error;

/// core 引擎统一错误类型。v1.1+ 各能力按需扩展变体。
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("data source error: {0}")]
    DataSource(String),

    #[error("processor error: {0}")]
    Processor(String),

    #[error("not implemented until v1.1+: {0}")]
    NotImplemented(&'static str),
}

pub type CoreResult<T> = Result<T, CoreError>;
