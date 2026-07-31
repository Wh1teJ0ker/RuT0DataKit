//! Core engine error type.
//!
//! v1.0.0: 仅定义枚举骨架，被 src-tauri 命令层 `map_err(|e| e.to_string())`
//! 转换为 `Result<_, String>` 后返回前端。后续业务能力按需扩展变体。

use thiserror::Error;

/// core 引擎统一错误类型。后续业务能力按需扩展变体。
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

    #[error("not implemented yet (capability under development): {0}")]
    NotImplemented(&'static str),

    /// 外部依赖（如 tshark）缺失。仅本地探测，不外发数据。
    #[error("missing dependency: {0}")]
    DependencyMissing(String),

    /// 输入非法（如非 JSON/非 SQL）。
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// 其它运行时错误（如子进程退出码非 0）。
    #[error("{0}")]
    Other(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
