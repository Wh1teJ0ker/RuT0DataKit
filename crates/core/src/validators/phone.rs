//! 手机号校验器（11 位数字 + 号段前缀白名单）。
//!
//! 号段前缀集由 `params.prefix_set` 选择，取值 `"ctf"`（spec.pdf 列出的虚假 7xx
//! 号段，用于 CTF 题目样本）或 `"real"`（国内三大运营商真实号段，默认）。

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;
use serde_yml::Value;

use crate::validators::{ValidationResult, Validator};

const CTF_PREFIXES: &[&str] = &[
    "730", "731", "732", "733", "734", "735", "736", "737", "738", "739", "740", "745", "746",
    "747", "748", "749", "750", "751", "752", "753", "755", "756", "757", "758", "759", "766",
    "767", "771", "772", "773", "774", "775", "776", "777", "778", "780", "781", "782", "783",
    "784", "785", "786", "787", "788", "789", "790", "791", "793", "795", "796", "797", "798",
    "799",
];

const REAL_PREFIXES: &[&str] = &[
    // CMCC
    "134", "135", "136", "137", "138", "139", "147", "148", "150", "151", "152", "157", "158",
    "159", "178", "182", "183", "184", "187", "188", "198", // CUCC
    "130", "131", "132", "145", "146", "155", "156", "166", "171", "175", "176", "185", "186",
    "196", // CTCC
    "133", "149", "153", "173", "177", "180", "181", "189", "190", "191", "193", "199",
];

fn ctf_prefixes() -> &'static HashSet<&'static str> {
    static LOCK: OnceLock<HashSet<&'static str>> = OnceLock::new();
    LOCK.get_or_init(|| CTF_PREFIXES.iter().copied().collect())
}

fn real_prefixes() -> &'static HashSet<&'static str> {
    static LOCK: OnceLock<HashSet<&'static str>> = OnceLock::new();
    LOCK.get_or_init(|| REAL_PREFIXES.iter().copied().collect())
}

/// 手机号校验器。
pub struct PhoneValidator {
    ctf: bool,
    re: Regex,
}

impl PhoneValidator {
    /// `params.prefix_set`：`"ctf"` 使用 CTF 虚假号段集，其它值（含缺省）使用真实运营商号段集。
    pub fn new(params: HashMap<String, Value>) -> Self {
        let ctf = params
            .get("prefix_set")
            .and_then(|v| v.as_str())
            .map(|s| s == "ctf")
            .unwrap_or(false);
        let re = Regex::new(r"^\d{11}$").expect("valid regex");
        Self { ctf, re }
    }
}

impl Validator for PhoneValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if !self.re.is_match(v) {
            return ValidationResult::fail("phone must be 11 digits");
        }
        let prefix = &v[..3];
        let set = if self.ctf {
            ctf_prefixes()
        } else {
            real_prefixes()
        };
        if set.contains(prefix) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail("phone prefix not in allowed set")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> PhoneValidator {
        PhoneValidator::new(HashMap::new())
    }

    fn ctf() -> PhoneValidator {
        let mut p = HashMap::new();
        p.insert(
            "prefix_set".to_string(),
            Value::String("ctf".to_string()),
        );
        PhoneValidator::new(p)
    }

    #[test]
    fn ctf_positive() {
        let r = ctf().validate("73012345678");
        assert!(r.valid, "got {:?}", r.message);
    }

    #[test]
    fn real_positive() {
        let r = real().validate("13812345678");
        assert!(r.valid, "got {:?}", r.message);
    }

    #[test]
    fn wrong_prefix_fails() {
        assert!(!real().validate("12345678901").valid);
    }

    #[test]
    fn wrong_length_fails() {
        assert!(!real().validate("1381234567").valid); // 10
        assert!(!real().validate("138123456789").valid); // 12
    }

    #[test]
    fn non_digit_fails() {
        assert!(!real().validate("1381234567a").valid);
    }

    #[test]
    fn real_prefixes_span_carriers() {
        // CMCC / CUCC / CTCC 各取一
        assert!(real().validate("13400000000").valid);
        assert!(real().validate("13000000000").valid);
        assert!(real().validate("13300000000").valid);
    }
}
