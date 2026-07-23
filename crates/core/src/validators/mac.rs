//! MAC 地址校验器（`XX:XX:XX:XX:XX:XX` 或 `XX-XX-XX-XX-XX-XX`，大小写不敏感）。
//!
//! 可选 `params.prefix`（如 `"DE:AD:BE:"`）：存在时要求 value 以该前缀开头（大小写
//! 不敏感比较）。
//! v0.5.0：正则源改引用 `rules::patterns::MAC.validate`，消除散布。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::rules::patterns::MAC;
use crate::validators::{ValidationResult, Validator};

/// MAC 地址校验器。
pub struct MacValidator {
    re: Regex,
    prefix: Option<String>,
}

impl MacValidator {
    pub fn new(params: HashMap<String, Value>) -> Self {
        let re = Regex::new(MAC.validate).expect("valid regex");
        let prefix = params
            .get("prefix")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Self { re, prefix }
    }
}

impl Validator for MacValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if !self.re.is_match(v) {
            return ValidationResult::fail("mac format invalid");
        }
        if let Some(prefix) = &self.prefix {
            if !v
                .to_lowercase()
                .starts_with(&prefix.to_lowercase())
            {
                return ValidationResult::fail("mac prefix mismatch");
            }
        }
        ValidationResult::ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> MacValidator {
        MacValidator::new(HashMap::new())
    }

    #[test]
    fn colon_form_positive() {
        assert!(v().validate("DE:AD:BE:EF:01:23").valid);
    }

    #[test]
    fn dash_form_positive() {
        assert!(v().validate("de-ad-be-ef-01-23").valid);
    }

    #[test]
    fn too_few_segments_fails() {
        assert!(!v().validate("DE:AD:BE:EF:01").valid);
    }

    #[test]
    fn non_hex_fails() {
        assert!(!v().validate("DE:AD:BE:EF:01:ZZ").valid);
    }

    #[test]
    fn prefix_match_positive() {
        let mut p = HashMap::new();
        p.insert("prefix".to_string(), Value::String("DE:AD:BE:".to_string()));
        let v = MacValidator::new(p);
        assert!(v.validate("DE:AD:BE:EF:01:23").valid);
    }

    #[test]
    fn prefix_mismatch_fails() {
        let mut p = HashMap::new();
        p.insert("prefix".to_string(), Value::String("DE:AD:BE:".to_string()));
        let v = MacValidator::new(p);
        assert!(!v.validate("FF:AD:BE:EF:01:23").valid);
    }

    #[test]
    fn prefix_case_insensitive() {
        let mut p = HashMap::new();
        p.insert("prefix".to_string(), Value::String("de:ad:be:".to_string()));
        let v = MacValidator::new(p);
        assert!(v.validate("DE:AD:BE:EF:01:23").valid);
    }
}
