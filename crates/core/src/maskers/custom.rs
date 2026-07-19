//! 通用脱敏器：保留前 `keep_prefix` 字符 + 后 `keep_suffix` 字符，中间替换为 `mask_char`。
//!
//! 支持参数：
//! - `keep_prefix`: usize，默认 0
//! - `keep_suffix`: usize，默认 0
//! - `mask_char`: String，默认 `*`（取首字符）
//! - `mask_min_len`: usize，默认 1
//!
//! 脱敏段字符数 = max(原中间长度, mask_min_len)。
//! 当 `keep_prefix + keep_suffix >= 总长度` 时，至少插入 `mask_min_len` 个 `mask_char`。

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 通用脱敏器参数。
#[derive(Clone, Debug)]
pub struct CustomMask {
    keep_prefix: usize,
    keep_suffix: usize,
    mask_char: char,
    mask_min_len: usize,
}

impl CustomMask {
    /// 从 params 构造。未提供时使用默认值。
    pub fn new(params: HashMap<String, Value>) -> Self {
        let keep_prefix = parse_usize(&params, "keep_prefix").unwrap_or(0);
        let keep_suffix = parse_usize(&params, "keep_suffix").unwrap_or(0);
        let mask_min_len = parse_usize(&params, "mask_min_len").unwrap_or(1);
        let mask_char = params
            .get("mask_char")
            .and_then(|v| v.as_str())
            .and_then(|s| s.chars().next())
            .unwrap_or('*');
        Self {
            keep_prefix,
            keep_suffix,
            mask_char,
            mask_min_len,
        }
    }
}

fn parse_usize(params: &HashMap<String, Value>, key: &str) -> Option<usize> {
    match params.get(key)? {
        Value::Number(n) => n.as_u64().map(|n| n as usize),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

impl Masker for CustomMask {
    fn mask(&self, value: &str) -> String {
        let chars: Vec<char> = value.chars().collect();
        let n = chars.len();
        let head_end = self.keep_prefix.min(n);
        let tail_start = n.saturating_sub(self.keep_suffix);
        if tail_start <= head_end {
            // keep_prefix + keep_suffix >= n：保留段重叠（含相等边界，
            // 此时中间段长度为 0，仍按规格「至少插入 mask_min_len 个
            // mask_char」处理，避免 head+mask+空 tail 错误拼接）。
            // 按规格“至少插入 mask_min_len 个 mask_char”，仅输出脱敏段。
            let mask: String = std::iter::repeat(self.mask_char)
                .take(self.mask_min_len)
                .collect();
            return mask;
        }
        let mid_len = tail_start - head_end;
        let mask_len = mid_len.max(self.mask_min_len);
        let mask: String = std::iter::repeat(self.mask_char)
            .take(mask_len)
            .collect();
        let head: String = chars[..head_end].iter().collect();
        let tail: String = chars[tail_start..].iter().collect();
        format!("{head}{mask}{tail}")
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
    fn spec_example_pad_to_three() {
        // keep_prefix=2, keep_suffix=2, mask_char="#", mask_min_len=3
        // 对 "abcdef"（中间 "cd" 2 字）→ "ab###ef"
        let p = params(&[
            ("keep_prefix", "2"),
            ("keep_suffix", "2"),
            ("mask_char", "#"),
            ("mask_min_len", "3"),
        ]);
        let m = CustomMask::new(p);
        assert_eq!(m.mask("abcdef"), "ab###ef");
    }

    #[test]
    fn default_all_mask() {
        // 全部默认：keep_prefix=0, keep_suffix=0, mask_char='*', mask_min_len=1
        // 中间 = "hello"(5)，max(5,1)=5 → "*****"
        let m = CustomMask::new(HashMap::new());
        assert_eq!(m.mask("hello"), "*****");
    }

    #[test]
    fn keep_prefix_only() {
        let p = params(&[("keep_prefix", "3")]);
        let m = CustomMask::new(p);
        assert_eq!(m.mask("abcdef"), "abc***");
    }

    #[test]
    fn keep_suffix_only() {
        let p = params(&[("keep_suffix", "2")]);
        let m = CustomMask::new(p);
        assert_eq!(m.mask("abcdef"), "****ef");
    }

    #[test]
    fn overlap_uses_min_len() {
        // keep_prefix=5, keep_suffix=5, mask_min_len=2 → 保留段重叠
        // 头尾全空，插入 mask_min_len 个 *。
        let p = params(&[
            ("keep_prefix", "5"),
            ("keep_suffix", "5"),
            ("mask_min_len", "2"),
        ]);
        let m = CustomMask::new(p);
        assert_eq!(m.mask("abcdef"), "**");
    }

    #[test]
    fn empty_input() {
        let m = CustomMask::new(HashMap::new());
        assert_eq!(m.mask(""), "*"); // 至少 mask_min_len=1 个 *
    }

    #[test]
    fn numeric_params() {
        let mut p = HashMap::new();
        p.insert("keep_prefix".to_string(), Value::Number(2.into()));
        p.insert("keep_suffix".to_string(), Value::Number(2.into()));
        let m = CustomMask::new(p);
        assert_eq!(m.mask("abcdef"), "ab**ef");
    }
}
