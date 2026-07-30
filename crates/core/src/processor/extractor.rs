//! 提取器 trait 骨架。v1.0.0 仅声明，不实现逻辑。

use crate::error::CoreResult;
use crate::model::Record;

/// 提取器 trait。v1.1+ 实现字段/模式提取。
pub trait Extractor: Send + Sync {
    /// 从单条记录提取信息。v1.0.0 不实现。
    fn extract(&self, record: &Record) -> CoreResult<Record>;
}

/// 占位空实现结构体，不含业务逻辑。
pub struct PlaceholderExtractor;
