//! 规则引擎 trait 骨架。v1.0.0 仅声明，不实现逻辑。

use crate::error::CoreResult;
use crate::model::Record;

/// 规则引擎 trait。v1.1+ 实现规则编排与批量执行。
pub trait RuleEngine: Send + Sync {
    /// 按规则集对单条记录执行。v1.0.0 不实现。
    fn apply(&self, record: &Record) -> CoreResult<Record>;
}

/// 占位空实现结构体，不含业务逻辑。
pub struct PlaceholderRuleEngine;
