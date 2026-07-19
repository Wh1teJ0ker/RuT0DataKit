//! 中文姓名校验器：全中文，`^[\u4e00-\u9fa5]+$`（Rust 正则写作 `[\x{4e00}-\x{9fa5}]`）。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::validators::{ValidationResult, Validator};

/// 中文姓名校验器。
pub struct NameValidator {
    re: Regex,
}

impl NameValidator {
    pub fn new(_params: HashMap<String, Value>) -> Self {
        let re = Regex::new(r"^[\x{4e00}-\x{9fa5}]+$").expect("valid regex");
        Self { re }
    }
}

impl Validator for NameValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if v.is_empty() {
            return ValidationResult::fail("name must not be empty");
        }
        if self.re.is_match(v) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail("name must be all CJK characters")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> NameValidator {
        NameValidator::new(HashMap::new())
    }

    #[test]
    fn two_char_positive() {
        assert!(v().validate("张三").valid);
    }

    #[test]
    fn three_char_positive() {
        assert!(v().validate("李四海").valid);
    }

    #[test]
    fn english_fails() {
        assert!(!v().validate("Zhang San").valid);
    }

    #[test]
    fn mixed_digit_fails() {
        assert!(!v().validate("张三1").valid);
    }

    #[test]
    fn empty_fails() {
        assert!(!v().validate("").valid);
    }
}
