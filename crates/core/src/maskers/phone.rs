//! 手机号脱敏：保留前 3 位 + 后 4 位，中间 4 位替换为 `*`。

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 手机号脱敏器。
pub struct PhoneMask;

impl PhoneMask {
    /// 构造一个 `PhoneMask`。`params` 当前未使用。
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self
    }
}

impl Masker for PhoneMask {
    fn mask(&self, value: &str) -> String {
        // 仅处理 11 位；非 11 位原样返回，不 panic。
        if value.chars().count() != 11 {
            return value.to_string();
        }
        let chars: Vec<char> = value.chars().collect();
        let head: String = chars[..3].iter().collect();
        let tail: String = chars[7..].iter().collect();
        format!("{head}****{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_11_digits() {
        let m = PhoneMask::new(HashMap::new());
        assert_eq!(m.mask("13812345678"), "138****5678");
    }

    #[test]
    fn non_11_passthrough() {
        let m = PhoneMask::new(HashMap::new());
        assert_eq!(m.mask("1381234567"), "1381234567"); // 10 位
        assert_eq!(m.mask("138123456789"), "138123456789"); // 12 位
    }

    #[test]
    fn empty_input() {
        let m = PhoneMask::new(HashMap::new());
        assert_eq!(m.mask(""), "");
    }
}
