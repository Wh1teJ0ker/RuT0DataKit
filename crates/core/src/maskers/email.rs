//! 邮箱脱敏：本地部分首尾字符保留，中间替换为 `*`；域名完整保留。
//!
//! - 本地 1 字：原样返回
//! - 本地 2 字：首字符 + 一个 `*`
//! - 本地 ≥3 字：首字符 + `*`×(n-2) + 末字符
//! - 无 `@` 或本地为空：原样返回

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 邮箱脱敏器。
pub struct EmailMask;

impl EmailMask {
    /// 构造一个 `EmailMask`。`params` 当前未使用。
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self
    }
}

impl Masker for EmailMask {
    fn mask(&self, value: &str) -> String {
        let at = match value.rfind('@') {
            Some(idx) => idx,
            None => return value.to_string(),
        };
        let (local, domain) = value.split_at(at);
        // domain 包含起始 `@`
        if local.is_empty() {
            return value.to_string();
        }
        let chars: Vec<char> = local.chars().collect();
        let n = chars.len();
        let masked_local = match n {
            1 => local.to_string(),
            2 => format!("{}*", chars[0]),
            _ => {
                let head = chars[0];
                let tail = chars[n - 1];
                let stars = "*".repeat(n - 2);
                format!("{head}{stars}{tail}")
            }
        };
        format!("{masked_local}{domain}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_8_chars() {
        let m = EmailMask::new(HashMap::new());
        assert_eq!(m.mask("zhangsan@example.com"), "z******n@example.com");
    }

    #[test]
    fn local_2_chars() {
        let m = EmailMask::new(HashMap::new());
        assert_eq!(m.mask("zs@x.com"), "z*@x.com");
    }

    #[test]
    fn local_1_char() {
        let m = EmailMask::new(HashMap::new());
        assert_eq!(m.mask("z@x.com"), "z@x.com");
    }

    #[test]
    fn local_3_chars() {
        let m = EmailMask::new(HashMap::new());
        assert_eq!(m.mask("abc@x.com"), "a*c@x.com");
    }

    #[test]
    fn no_at_passthrough() {
        let m = EmailMask::new(HashMap::new());
        assert_eq!(m.mask("notanemail"), "notanemail");
    }

    #[test]
    fn empty_local_passthrough() {
        let m = EmailMask::new(HashMap::new());
        assert_eq!(m.mask("@x.com"), "@x.com");
    }

    #[test]
    fn empty_input() {
        let m = EmailMask::new(HashMap::new());
        assert_eq!(m.mask(""), "");
    }

    #[test]
    fn multiple_at_uses_last() {
        let m = EmailMask::new(HashMap::new());
        // rfind：最后一个 @ 为分隔；本地 = "a@b"
        assert_eq!(m.mask("a@b@example.com"), "a*b@example.com");
    }
}
