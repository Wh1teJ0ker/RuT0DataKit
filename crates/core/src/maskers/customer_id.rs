//! 客户号脱敏：首字符保留，其余替换为 `*`。

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 客户号脱敏器。
pub struct CustomerIdMask;

impl CustomerIdMask {
    /// 构造一个 `CustomerIdMask`。`params` 当前未使用。
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self
    }
}

impl Masker for CustomerIdMask {
    fn mask(&self, value: &str) -> String {
        let chars: Vec<char> = value.chars().collect();
        if chars.is_empty() {
            return String::new();
        }
        let head = chars[0];
        let stars = "*".repeat(chars.len() - 1);
        format!("{head}{stars}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_8_digits() {
        let m = CustomerIdMask::new(HashMap::new());
        assert_eq!(m.mask("12345678"), "1*******");
    }

    #[test]
    fn single_char_passthrough() {
        let m = CustomerIdMask::new(HashMap::new());
        assert_eq!(m.mask("A"), "A");
    }

    #[test]
    fn empty_input() {
        let m = CustomerIdMask::new(HashMap::new());
        assert_eq!(m.mask(""), "");
    }

    #[test]
    fn mixed_chars() {
        let m = CustomerIdMask::new(HashMap::new());
        assert_eq!(m.mask("C1234"), "C****");
    }
}
