//! 银行卡号校验器（13..19 位数字 + Luhn 校验）。
//!
//! Luhn 算法：从右起第 1 位为校验位，第 2 位起偶数位（0-indexed from right）乘 2
//! 超 9 减 9，全部求和 mod 10 == 0。
//! v0.5.0：正则源改引用 `rules::patterns::BANKCARD.validate`，消除散布。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::rules::patterns::BANKCARD;
use crate::validators::{ValidationResult, Validator};

/// 银行卡号校验器。
pub struct BankCardValidator {
    re: Regex,
}

impl BankCardValidator {
    pub fn new(_params: HashMap<String, Value>) -> Self {
        let re = Regex::new(BANKCARD.validate).expect("valid regex");
        Self { re }
    }
}

impl Validator for BankCardValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if !self.re.is_match(v) {
            return ValidationResult::fail("bankcard must be 13..19 digits");
        }
        if luhn_valid(v) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail("bankcard luhn checksum mismatch")
        }
    }
}

fn luhn_valid(s: &str) -> bool {
    let mut sum: u32 = 0;
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().rev().enumerate() {
        let mut n = (*b - b'0') as u32;
        if i % 2 == 1 {
            n *= 2;
            if n > 9 {
                n -= 9;
            }
        }
        sum += n;
    }
    sum % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> BankCardValidator {
        BankCardValidator::new(HashMap::new())
    }

    #[test]
    fn known_valid_visa() {
        assert!(v().validate("4532015112830366").valid);
    }

    #[test]
    fn generated_valid_cards() {
        // 13 位
        assert!(v().validate("6225887654322").valid);
        // 16 位
        assert!(v().validate("6225887654321096").valid);
        // 19 位
        assert!(v().validate("6225887654321234568").valid);
    }

    #[test]
    fn too_short_fails() {
        assert!(!v().validate("622588765432").valid); // 12
    }

    #[test]
    fn too_long_fails() {
        assert!(!v().validate("62258876543212345678").valid); // 20
    }

    #[test]
    fn non_digit_fails() {
        assert!(!v().validate("622588765432109a").valid);
    }

    #[test]
    fn checksum_tamper_fails() {
        // 16 位正例 6225887654321096 改校验位
        assert!(!v().validate("6225887654321097").valid);
    }
}
