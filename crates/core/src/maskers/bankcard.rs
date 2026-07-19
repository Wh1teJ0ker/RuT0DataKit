//! 银行卡号脱敏：保留前 6 位 + 后 4 位，中间替换为 `*`（数量 = len - 10）。

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 银行卡号脱敏器。
pub struct BankCardMask;

impl BankCardMask {
    /// 构造一个 `BankCardMask`。`params` 当前未使用。
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self
    }
}

impl Masker for BankCardMask {
    fn mask(&self, value: &str) -> String {
        let len = value.chars().count();
        // 不足 10 位原样返回（前 6 + 后 4 = 10，至少需要 10 位才有脱敏空间）。
        if len < 10 {
            return value.to_string();
        }
        let chars: Vec<char> = value.chars().collect();
        let head: String = chars[..6].iter().collect();
        let tail: String = chars[len - 4..].iter().collect();
        let mask_len = len - 10;
        let mask: String = "*".repeat(mask_len);
        format!("{head}{mask}{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_16_digits() {
        let m = BankCardMask::new(HashMap::new());
        assert_eq!(m.mask("6225887654321098"), "622588******1098");
    }

    #[test]
    fn minimum_10_digits() {
        let m = BankCardMask::new(HashMap::new());
        // 10 位：前 6 + 后 4，中间 0 个 *
        assert_eq!(m.mask("6225881234"), "6225881234");
    }

    #[test]
    fn less_than_10_passthrough() {
        let m = BankCardMask::new(HashMap::new());
        assert_eq!(m.mask("622588123"), "622588123"); // 9 位
        assert_eq!(m.mask("12345"), "12345");
        assert_eq!(m.mask(""), "");
    }

    #[test]
    fn longer_card() {
        let m = BankCardMask::new(HashMap::new());
        // 19 位：前 6 + 后 4，中间 9 个 *
        assert_eq!(m.mask("6225887654321098765"), "622588*********8765");
    }
}
