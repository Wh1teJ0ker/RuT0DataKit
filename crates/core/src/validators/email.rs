//! 邮箱校验器（标准邮箱正则）。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::validators::{ValidationResult, Validator};

/// 邮箱校验器。
pub struct EmailValidator {
    re: Regex,
}

impl EmailValidator {
    pub fn new(_params: HashMap<String, Value>) -> Self {
        let re = Regex::new(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$")
            .expect("valid regex");
        Self { re }
    }
}

impl Validator for EmailValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if self.re.is_match(v) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail("email format invalid")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> EmailValidator {
        EmailValidator::new(HashMap::new())
    }

    #[test]
    fn standard_positive() {
        assert!(v().validate("zhangsan@example.com").valid);
    }

    #[test]
    fn short_tld_positive() {
        assert!(v().validate("zs@x.cn").valid);
    }

    #[test]
    fn plain_word_fails() {
        assert!(!v().validate("invalid").valid);
    }

    #[test]
    fn no_tld_fails() {
        assert!(!v().validate("a@b").valid);
    }

    #[test]
    fn no_local_part_fails() {
        assert!(!v().validate("@x.com").valid);
    }
}
