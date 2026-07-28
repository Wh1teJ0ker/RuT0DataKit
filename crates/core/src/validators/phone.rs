//! 手机号校验器（11 位数字）。
//!
//! v0.4.3 重构：删除原 CTF_PREFIXES / REAL_PREFIXES 硬编码号段白名单
//! （用户要求"删除绝对化内容"），仅按 spec 校验。
//! v0.5.0：正则源改引用 `rules::patterns::PHONE.validate`，消除散布。
//!
//! v0.6.8（修订）：`phone` 与 `pinfo_phone` 两个 scope 统一本校验器，
//! 支持 `params.prefixes`（前 1-3 位号段集合，逗号分隔字符串序列）。
//! - `prefixes=None`（缺省/空）：默认 1 开头正常号码（`^1\d{10}$` 语义）
//! - `prefixes=Some(set)`：11 位数字且前缀命中集合中任一项（前缀长度 1-3）
//!
//! 上一轮 v0.6.8 默认 52 虚假号段的设计不正确（用户反馈），改为默认 1 开头。
//! `PInfoPhoneValidator`（原 `validators/pinfo_phone.rs`）已删除，scope
//! `pinfo_phone` 在 `build_validator` 中路由到本校验器。

use std::collections::HashMap;
use std::collections::HashSet;

use regex::Regex;
use serde_yml::Value;

use crate::validators::{ValidationResult, Validator};

pub struct PhoneValidator {
    /// 基础长度校验正则：`^\d{11}$`。
    re: Regex,
    /// 自定义前缀集合。`None` = 默认 1 开头；`Some(set)` = 命中集合中任一前缀。
    prefixes: Option<HashSet<String>>,
}

impl PhoneValidator {
    /// 从 params 构造。识别 `prefixes`（YAML 字符串序列，前 1-3 位号段）；
    /// 未提供 / 空 / 全空白时 `prefixes=None`（走默认 1 开头）。
    pub fn new(params: HashMap<String, Value>) -> Self {
        let prefixes = params
            .get("prefixes")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|item| item.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect::<HashSet<String>>()
            })
            .filter(|s| !s.is_empty());
        Self {
            re: Regex::new(r"^\d{11}$").expect("valid regex"),
            prefixes,
        }
    }
}

impl Validator for PhoneValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if !self.re.is_match(v) {
            return ValidationResult::fail("phone must be 11 digits");
        }
        match &self.prefixes {
            None => {
                // 缺省：1 开头正常号码（与 v0.6.7 行为一致）。
                if v.starts_with('1') {
                    ValidationResult::ok()
                } else {
                    ValidationResult::fail("phone must start with 1")
                }
            }
            Some(set) => {
                // 自定义前 1-3 位号段集合：任一 prefix 是 v 的前缀即通过。
                if set.iter().any(|p| v.starts_with(p.as_str())) {
                    ValidationResult::ok()
                } else {
                    ValidationResult::fail("phone prefix not in allowed set")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> PhoneValidator {
        PhoneValidator::new(HashMap::new())
    }

    fn make_params(prefixes: &[&str]) -> HashMap<String, Value> {
        let seq = Value::Sequence(
            prefixes
                .iter()
                .map(|s| Value::String((*s).to_string()))
                .collect(),
        );
        let mut m = HashMap::new();
        m.insert("prefixes".to_string(), seq);
        m
    }

    // -------- 缺省（1 开头）路径 --------

    #[test]
    fn starts_with_1_positive() {
        assert!(v().validate("13812345678").valid);
        assert!(v().validate("15500001111").valid); // 合成测试号，非真实号段
    }

    #[test]
    fn non_1_prefix_fails_by_default() {
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

    #[test]
    fn empty_fails() {
        assert!(!v().validate("").valid);
    }

    // -------- 自定义 prefixes 路径 --------

    #[test]
    fn custom_prefixes_override_default() {
        let v = PhoneValidator::new(make_params(&["138", "159", "734"]));
        assert!(v.validate("13812345678").valid);
        assert!(v.validate("15987654321").valid);
        assert!(v.validate("73412345678").valid);
        // 788 不在自定义集合 → 失败
        assert!(!v.validate("78813630178").valid);
        // 1 开头但前三位不在自定义集合 → 失败（自定义覆盖默认 1 开头）
        assert!(!v.validate("18812345678").valid);
    }

    #[test]
    fn custom_prefixes_supports_short_prefix() {
        // 前缀长度可 1-3 位：放 "7" → 所有 7 开头 11 位号码通过。
        let v = PhoneValidator::new(make_params(&["7"]));
        assert!(v.validate("73412345678").valid);
        assert!(v.validate("78813630178").valid);
        assert!(!v.validate("13812345678").valid);
    }

    #[test]
    fn empty_prefixes_param_falls_back_to_default() {
        // prefixes=[] 或全空白 → 视作未提供，走默认 1 开头。
        let empty_seq = Value::Sequence(vec![]);
        let mut m = HashMap::new();
        m.insert("prefixes".to_string(), empty_seq);
        let v = PhoneValidator::new(m);
        assert!(v.validate("13812345678").valid, "empty prefixes should fall back");
        assert!(!v.validate("78813630178").valid);
    }

    #[test]
    fn whitespace_only_prefixes_falls_back_to_default() {
        let seq = Value::Sequence(vec![Value::String("  ".into()), Value::String("".into())]);
        let mut m = HashMap::new();
        m.insert("prefixes".to_string(), seq);
        let v = PhoneValidator::new(m);
        assert!(v.validate("13812345678").valid);
        assert!(!v.validate("78813630178").valid);
    }

    #[test]
    fn no_prefixes_param_uses_default() {
        let v = PhoneValidator::new(HashMap::new());
        assert!(v.validate("13812345678").valid);
        assert!(!v.validate("78813630178").valid);
    }

    #[test]
    fn custom_prefixes_trims_whitespace() {
        // 前端 TextArea 逗号分隔输入可能带空格，构造时已 trim。
        let v = PhoneValidator::new(make_params(&["  138  ", "159"]));
        assert!(v.validate("13812345678").valid);
        assert!(v.validate("15987654321").valid);
        assert!(!v.validate("78813630178").valid);
    }
}
