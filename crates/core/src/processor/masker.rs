//! 脱敏器 trait 骨架。v1.0.0 仅声明，不实现逻辑。

use crate::error::CoreResult;
use crate::model::Record;

/// 脱敏器 trait。v1.1+ 以 `HashMasker`/`RegexMasker` 等实现接入。
pub trait Masker: Send + Sync {
    /// 对单条记录执行脱敏。v1.0.0 不实现。
    fn mask(&self, record: &Record) -> CoreResult<Record>;
}

/// 占位空实现结构体，仅为保证模块可编译；不含业务逻辑。
pub struct PlaceholderMasker;
