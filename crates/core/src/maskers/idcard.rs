//! 身份证号脱敏：保留前 6 位 + 后 4 位，中间 8 位替换为 `*`。

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 身份证号脱敏器。
pub struct IdCardMask;

impl IdCardMask {
    /// 构造一个 `IdCardMask`。`params` 当前未使用，保留以与其它脱敏器签名一致。
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self
    }
}

impl Masker for IdCardMask {
    fn mask(&self, value: &str) -> String {
        // 仅处理 18 位；非 18 位原样返回，不 panic。
        if value.chars().count() != 18 {
            return value.to_string();
        }
        let chars: Vec<char> = value.chars().collect();
        let head: String = chars[..6].iter().collect();
        let tail: String = chars[14..].iter().collect();
        format!("{head}********{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_18_digits() {
        let m = IdCardMask::new(HashMap::new());
        assert_eq!(m.mask("110101199001011234"), "110101********1234");
    }

    #[test]
    fn last_char_x_still_masked() {
        let m = IdCardMask::new(HashMap::new());
        assert_eq!(m.mask("11010119900101123X"), "110101********123X");
    }

    #[test]
    fn non_18_passthrough() {
        let m = IdCardMask::new(HashMap::new());
        assert_eq!(m.mask("11010119900101"), "11010119900101");
        assert_eq!(m.mask("1234567890"), "1234567890");
        assert_eq!(m.mask(""), "");
    }

    #[test]
    fn empty_input() {
        let m = IdCardMask::new(HashMap::new());
        assert_eq!(m.mask(""), "");
    }
}
