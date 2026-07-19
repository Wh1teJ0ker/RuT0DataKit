//! 正则提取脱敏器：返回第一个匹配的子串全文。
//!
//! 参数：
//! - `pattern`（String，必填）：正则表达式。非法或缺失时退化为原值返回。
//! 无匹配返回原值（避免静默丢数据）。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::maskers::Masker;

/// 正则提取脱敏器。
#[derive(Clone, Debug)]
pub struct RegexExtractMask {
    re: Option<Regex>,
}

impl RegexExtractMask {
    pub fn new(params: HashMap<String, Value>) -> Self {
        let pattern = params.get("pattern").and_then(|v| v.as_str());
        let re = pattern.and_then(|p| Regex::new(p).ok());
        Self { re }
    }
}

impl Masker for RegexExtractMask {
    fn mask(&self, value: &str) -> String {
        match self.re.as_ref() {
            Some(re) => match re.find(value) {
                Some(m) => m.as_str().to_string(),
                None => value.to_string(),
            },
            None => value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(p: &[(&str, &str)]) -> HashMap<String, Value> {
        p.iter()
            .map(|(k, v)| (k.to_string(), Value::String((*v).to_string())))
            .collect()
    }

    #[test]
    fn spec_example_extract_phone() {
        // 对 "tel:13812345678" 用 pattern="\d{11}" → "13812345678"
        let p = params(&[("pattern", r"\d{11}")]);
        let m = RegexExtractMask::new(p);
        assert_eq!(m.mask("tel:13812345678"), "13812345678");
    }

    #[test]
    fn no_match_returns_original() {
        let p = params(&[("pattern", r"\d{11}")]);
        let m = RegexExtractMask::new(p);
        assert_eq!(m.mask("no digits here"), "no digits here");
    }

    #[test]
    fn returns_first_match() {
        let p = params(&[("pattern", r"\d+")]);
        let m = RegexExtractMask::new(p);
        assert_eq!(m.mask("a123b456"), "123");
    }

    #[test]
    fn invalid_pattern_passthrough() {
        let p = params(&[("pattern", "(")]);
        let m = RegexExtractMask::new(p);
        assert_eq!(m.mask("abc"), "abc");
    }

    #[test]
    fn missing_pattern_passthrough() {
        let p = params(&[]);
        let m = RegexExtractMask::new(p);
        assert_eq!(m.mask("abc"), "abc");
    }
}
