//! 姓名脱敏（BMP CJK 字符）：
//! - 1 字：原样返回
//! - 2 字：首字符 + `*`
//! - ≥3 字：首字符 + `*`×(n-2) + 末字符

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 姓名脱敏器。
pub struct NameMask;

impl NameMask {
    /// 构造一个 `NameMask`。`params` 当前未使用。
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self
    }
}

impl Masker for NameMask {
    fn mask(&self, value: &str) -> String {
        let chars: Vec<char> = value.chars().collect();
        let n = chars.len();
        match n {
            0 => String::new(),
            1 => value.to_string(),
            2 => format!("{}*", chars[0]),
            _ => {
                let head = chars[0];
                let tail = chars[n - 1];
                let stars = "*".repeat(n - 2);
                format!("{head}{stars}{tail}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_chars_cjk() {
        let m = NameMask::new(HashMap::new());
        assert_eq!(m.mask("张三"), "张*");
    }

    #[test]
    fn three_chars_cjk() {
        let m = NameMask::new(HashMap::new());
        assert_eq!(m.mask("李四海"), "李*海");
    }

    #[test]
    fn four_chars_cjk() {
        let m = NameMask::new(HashMap::new());
        assert_eq!(m.mask("欧阳四海"), "欧**海");
    }

    #[test]
    fn one_char_passthrough() {
        let m = NameMask::new(HashMap::new());
        assert_eq!(m.mask("张"), "张");
    }

    #[test]
    fn empty_input() {
        let m = NameMask::new(HashMap::new());
        assert_eq!(m.mask(""), "");
    }
}
