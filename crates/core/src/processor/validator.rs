//! 校验器 trait 骨架。v1.0.0 仅声明，不实现逻辑。

use crate::error::CoreResult;
use crate::model::Record;

/// 校验器 trait。v1.1+ 实现具体校验规则。
pub trait Validator: Send + Sync {
    /// 对单条记录执行校验，返回校验不通过的列名列表。v1.0.0 不实现。
    fn validate(&self, record: &Record) -> CoreResult<Vec<String>>;
}

/// 占位空实现结构体，不含业务逻辑。
pub struct PlaceholderValidator;
