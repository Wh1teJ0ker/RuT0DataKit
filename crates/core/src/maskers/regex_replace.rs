//! 正则替换脱敏器：用 `pattern` 正则匹配，按 `replacement`（支持 `$1`/`$2`
//! 捕获组语法）替换所有匹配子串。
//!
//! 参数：
//! - `pattern`（String，必填）：正则表达式。非法或缺失时 masker 退化为
//!   原值返回（即不脱敏），不 panic。
//! - `replacement`（String，必填）：替换模板。regex crate 使用 `$1`/`$2`
//!   命名捕获组替换语法（非 JS 的 `$1` 形式也兼容）。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::maskers::Masker;

/// 正则替换脱敏器。
#[derive(Clone, Debug)]
pub struct RegexReplaceMask {
    re: Option<Regex>,
    replacement: String,
}

impl RegexReplaceMask {
    /// 从 params 构造。`pattern` 缺失或非法时 `re=None`（退化为原值返回）。
    pub fn new(params: HashMap<String, Value>) -> Self {
        let pattern = params.get("pattern").and_then(|v| v.as_str());
        let replacement = params
            .get("replacement")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let re = pattern.and_then(|p| Regex::new(p).ok());
        Self {
            re,
            replacement,
        }
    }
}

impl Masker for RegexReplaceMask {
    fn mask(&self, value: &str) -> String {
        match self.re.as_ref() {
            Some(re) => re.replace_all(value, self.replacement.as_str()).into_owned(),
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
    fn spec_example_phone_mask() {
        // 对 "13812345678"（11 位）用 pattern="(\d{3})\d{4}(\d{4})"
        // replacement="$1****$2" → "138****5678"
        //
        // 注：HANDOFF acceptance 原文写的是 `(\d{4})\d{4}(\d{4})`（12 位），
        // 但输入为 11 位，原模式无法匹配；按预期输出 "138****5678"
        // 反推正确模式为 `(\d{3})\d{4}(\d{4})`，此处采用反推后的模式。
        let p = params(&[
            ("pattern", r"(\d{3})\d{4}(\d{4})"),
            ("replacement", "$1****$2"),
        ]);
        let m = RegexReplaceMask::new(p);
        assert_eq!(m.mask("13812345678"), "138****5678");
    }

    #[test]
    fn replaces_all_matches() {
        // 多个匹配都替换
        let p = params(&[("pattern", r"\d"), ("replacement", "*")]);
        let m = RegexReplaceMask::new(p);
        assert_eq!(m.mask("a1b22c333"), "a*b**c***");
    }

    #[test]
    fn no_match_returns_original() {
        let p = params(&[("pattern", r"\d{4}"), ("replacement", "####")]);
        let m = RegexReplaceMask::new(p);
        assert_eq!(m.mask("abc"), "abc");
    }

    #[test]
    fn invalid_pattern_passthrough() {
        // pattern 非法 → 退化为原值返回，不 panic
        let p = params(&[("pattern", "("), ("replacement", "x")]);
        let m = RegexReplaceMask::new(p);
        assert_eq!(m.mask("abc"), "abc");
    }

    #[test]
    fn missing_pattern_passthrough() {
        let p = params(&[("replacement", "x")]);
        let m = RegexReplaceMask::new(p);
        assert_eq!(m.mask("abc"), "abc");
    }
}
