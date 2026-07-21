//! 手机号校验器（11 位数字、首位 1）。
//!
//! v0.4.3 重构：删除原 CTF_PREFIXES / REAL_PREFIXES 硬编码号段白名单
//! （用户要求"删除绝对化内容"），仅按 PDF spec 校验 `^1\d{10}$`。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::validators::{ValidationResult, Validator};

pub struct PhoneValidator {
    re: Regex,
}

impl PhoneValidator {
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self {
            re: Regex::new(r"^1\d{10}$").expect("valid regex"),
        }
    }
}

impl Validator for PhoneValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if self.re.is_match(v) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail("phone must be 11 digits starting with 1")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> PhoneValidator {
        PhoneValidator::new(HashMap::new())
    }

    #[test]
    fn starts_with_1_positive() {
        assert!(v().validate("13812345678").valid);
        assert!(v().validate("15560728076").valid); // 非旧白名单号段，现在通过
    }

    #[test]
    fn non_1_prefix_fails() {
        assert!(!v().validate("22345678901").valid); // 首位非 1
        assert!(!v().validate("73012345678").valid); // 首位非 1
    }

    #[test]
    fn wrong_length_fails() {
        assert!(!v().validate("1381234567").valid); // 10
        assert!(!v().validate("138123456789").valid); // 12
    }

    #[test]
    fn non_digit_fails() {
        assert!(!v().validate("1381234567a").valid);
    }
}
