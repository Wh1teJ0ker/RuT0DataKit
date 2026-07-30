//! SQLite 持久层错误类型。
//!
//! v1.0.0: 三类错误（SQLite / IO / Migration），供 `DbManager` 与命令层统一返回。

use std::io;

/// 持久层错误。
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// SQLite 底层错误。
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// 文件系统 IO 错误（DB 目录创建/备份）。
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// 迁移失败（版本不兼容且备份重建失败等）。
    #[error("migration error: {0}")]
    Migration(String),
}
