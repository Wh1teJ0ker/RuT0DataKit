//! IPv4 校验器（四段 0-255，拒绝前导 0）。
//!
//! v0.5.0：正则源改引用 `rules::patterns::IP.validate`，消除散布。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::rules::patterns::IP;
use crate::validators::{ValidationResult, Validator};

pub struct IpValidator {
    re: Regex,
}

impl IpValidator {
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self {
            re: Regex::new(IP.validate).expect("valid regex"),
        }
    }
}

impl Validator for IpValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if self.re.is_match(v) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail("ip format invalid")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> IpValidator {
        IpValidator::new(HashMap::new())
    }

    #[test]
    fn standard_positive() {
        assert!(v().validate("192.168.1.1").valid);
    }

    #[test]
    fn fixture_positive() {
        // 用合成测试 IP（非 tips/ fixture 真实 PII），符合「样本不上传」约束
        assert!(v().validate("192.168.100.200").valid);
    }

    #[test]
    fn over_255_fails() {
        assert!(!v().validate("256.1.1.1").valid);
    }

    #[test]
    fn leading_zero_fails() {
        assert!(!v().validate("01.2.3.4").valid);
    }

    #[test]
    fn too_few_segments_fails() {
        assert!(!v().validate("1.2.3").valid);
    }
}
