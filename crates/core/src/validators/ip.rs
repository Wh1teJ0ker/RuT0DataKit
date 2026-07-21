//! IPv4 校验器（四段 0-255，拒绝前导 0）。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::rules::presets::IP_REGEX;
use crate::validators::{ValidationResult, Validator};

pub struct IpValidator {
    re: Regex,
}

impl IpValidator {
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self {
            re: Regex::new(IP_REGEX).expect("valid regex"),
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
        assert!(v().validate("163.211.48.156").valid);
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
