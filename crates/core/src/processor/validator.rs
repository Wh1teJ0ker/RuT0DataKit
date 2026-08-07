//! 校验器 trait + 正则校验实现。
//!
//! v1.1.0：`Validator` trait + `RegexValidator`（用 `rule.pattern` 编译 regex，
//! 匹配则 `passed=true`）。姓名校验规则由 `RegexValidator` 承载。

use regex::Regex;

use crate::error::{CoreError, CoreResult};
use crate::processor::rules::{Rule, RuleKind};

/// 单行校验结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    /// 是否通过。
    pub passed: bool,
    /// 不通过时的说明（通过时为空串）。
    pub message: String,
    /// 触发规则 ID。
    pub rule_id: String,
}

/// 校验器 trait。纯逻辑，不持有状态。
pub trait Validator {
    /// 校验单值。`rule.kind` 应为 `Validate`，否则返回 `Err`。
    fn validate(&self, input: &str, rule: &Rule) -> CoreResult<ValidationResult>;
}

/// 正则校验器。用 `rule.pattern` 编译 regex，匹配则 `passed=true`。
pub struct RegexValidator;

impl Validator for RegexValidator {
    fn validate(&self, input: &str, rule: &Rule) -> CoreResult<ValidationResult> {
        if rule.kind != RuleKind::Validate {
            return Err(CoreError::Processor(format!(
                "rule `{}` is not a validate rule (kind={})",
                rule.id, rule.kind
            )));
        }
        let pattern = rule.pattern.as_deref().ok_or_else(|| {
            CoreError::Processor(format!("validate rule `{}` has no pattern", rule.id))
        })?;
        let re = Regex::new(pattern)
            .map_err(|e| CoreError::Processor(format!("invalid regex `{}`: {e}", pattern)))?;
        let passed = re.is_match(input);
        let message = if passed {
            String::new()
        } else {
            format!("值 `{input}` 不匹配规则 `{}`", rule.name)
        };
        Ok(ValidationResult {
            passed,
            message,
            rule_id: rule.id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::rules::RuleRegistry;

    fn name_rule() -> Rule {
        RuleRegistry::name_validate_rule()
    }

    #[test]
    fn validates_chinese_name_2_to_4_chars() {
        let v = RegexValidator;
        let rule = name_rule();
        assert!(v.validate("张三", &rule).unwrap().passed);
        assert!(v.validate("张三丰", &rule).unwrap().passed);
        assert!(v.validate("欧阳修", &rule).unwrap().passed);
    }

    #[test]
    fn rejects_non_chinese_name() {
        let v = RegexValidator;
        let rule = name_rule();
        assert!(!v.validate("Zhang San", &rule).unwrap().passed);
        assert!(!v.validate("张3", &rule).unwrap().passed);
    }

    #[test]
    fn rejects_too_long_chinese_name() {
        let v = RegexValidator;
        let rule = name_rule();
        assert!(!v.validate("张三丰四丰五", &rule).unwrap().passed);
    }

    #[test]
    fn rejects_empty_input() {
        let v = RegexValidator;
        let rule = name_rule();
        assert!(!v.validate("", &rule).unwrap().passed);
    }

    #[test]
    fn rejects_non_validate_rule_kind() {
        let v = RegexValidator;
        let mut rule = name_rule();
        rule.kind = RuleKind::Mask;
        assert!(v.validate("张三", &rule).is_err());
    }

    #[test]
    fn rejects_rule_without_pattern() {
        let v = RegexValidator;
        let mut rule = name_rule();
        rule.pattern = None;
        assert!(v.validate("张三", &rule).is_err());
    }
}
