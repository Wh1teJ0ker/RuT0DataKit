//! 用户名校验器：仅字母数字，`^[A-Za-z0-9]+$`。
//!
//! v0.5.0：正则源改引用 `rules::patterns::USERNAME.validate`，消除散布。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::rules::patterns::USERNAME;
use crate::validators::{ValidationResult, Validator};

/// 用户名校验器。
pub struct UsernameValidator {
    re: Regex,
}

impl UsernameValidator {
    pub fn new(_params: HashMap<String, Value>) -> Self {
        let re = Regex::new(USERNAME.validate).expect("valid regex");
        Self { re }
    }
}

impl Validator for UsernameValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if v.is_empty() {
            return ValidationResult::fail("username must not be empty");
        }
        if self.re.is_match(v) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail("username must be alphanumeric only")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> UsernameValidator {
        UsernameValidator::new(HashMap::new())
    }

    #[test]
    fn alphanumeric_positive() {
        assert!(v().validate("n0tr00t").valid);
    }

    #[test]
    fn percent_fails() {
        assert!(!v().validate("MrRdarke%er").valid);
    }

    #[test]
    fn empty_fails() {
        assert!(!v().validate("").valid);
    }
}
