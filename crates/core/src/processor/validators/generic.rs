//! 通用校验：字符类白名单 + 长度范围。

/// 通用校验：按字符类白名单 + 长度范围校验（v1.1.4 续轮 T70 新增；T77 改为
/// 自定义特殊字符白名单）。
///
/// - `allow_digits`：允许 0-9
/// - `allow_letters`：允许 a-zA-Z
/// - `allow_special_chars`：用户自定义的特殊字符白名单（空串 = 不允许任何
///   特殊字符；非空如 `"_-.@"` = 仅允许这些字符）
/// - `min_len` / `max_len`：长度范围（None = 不限）
///
/// 规则：
/// - `allow_digits`/`allow_letters` 全 false 且 `allow_special_chars` 为空
///   → 直接返回 false（无任何允许的字符类）
/// - 长度按 `s.chars().count()`（支持中文等多字节字符）
/// - min_len 非空且 count < min → false
/// - max_len 非空且 count > max → false
/// - 逐字符检查：每个 char 必须属于至少一个"允许"的字符类
///   - `c.is_ascii_digit()` → digits 类
///   - `c.is_ascii_alphabetic()` → letters 类
///   - `allow_special_chars.contains(c)` → 自定义特殊字符白名单
///   - 若字符不属于任何允许的类 → false
/// - 空串 → false（即使三个字符类都允许，空串无字符也判为无效）
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_generic;
/// assert!(is_valid_generic("abc123", true, true, "", None, None));
/// assert!(!is_valid_generic("abc123", true, false, "", None, None)); // 有字母
/// assert!(!is_valid_generic("ab", true, true, "", Some(3), None));   // 长度<3
/// assert!(is_valid_generic("a@b", false, true, "@", None, None));    // 字母+白名单@
/// assert!(!is_valid_generic("a#b", false, true, "@", None, None));   // #不在白名单
/// assert!(!is_valid_generic("", true, true, "", None, None));        // 空串
/// assert!(!is_valid_generic("abc", false, false, "", None, None));   // 全空
/// ```
pub fn is_valid_generic(
    s: &str,
    allow_digits: bool,
    allow_letters: bool,
    allow_special_chars: &str,
    min_len: Option<usize>,
    max_len: Option<usize>,
) -> bool {
    // 三个字符类全空 → 无任何允许的字符类
    if !allow_digits && !allow_letters && allow_special_chars.is_empty() {
        return false;
    }
    // 空串 → false（空串无字符，视为无效）
    if s.is_empty() {
        return false;
    }
    let count = s.chars().count();
    if let Some(min) = min_len {
        if count < min {
            return false;
        }
    }
    if let Some(max) = max_len {
        if count > max {
            return false;
        }
    }
    // 逐字符检查：每个 char 必须属于至少一个允许的类
    for c in s.chars() {
        let is_digit = c.is_ascii_digit();
        let is_letter = c.is_ascii_alphabetic();
        let allowed = (is_digit && allow_digits)
            || (is_letter && allow_letters)
            || allow_special_chars.contains(c);
        if !allowed {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_generic_valid_samples() {
        // 数字 + 字母（两者都允许）
        assert!(is_valid_generic("abc123", true, true, "", None, None));
        // 纯数字
        assert!(is_valid_generic("12345", true, false, "", None, None));
        // 纯字母
        assert!(is_valid_generic("abcde", false, true, "", None, None));
        // 字母 + 白名单特殊字符 @
        assert!(is_valid_generic("a@b", false, true, "@", None, None));
        // 数字 + 字母 + 白名单特殊字符 @
        assert!(is_valid_generic("a1@", true, true, "@", None, None));
        // 带长度范围
        assert!(is_valid_generic(
            "abc123",
            true,
            true,
            "",
            Some(1),
            Some(10)
        ));
        // T77：多字符白名单
        assert!(is_valid_generic("a-b.c_d", false, true, "-._", None, None));
    }

    #[test]
    fn is_valid_generic_invalid_samples() {
        // 有字母但只允许数字
        assert!(!is_valid_generic("abc123", true, false, "", None, None));
        // 长度不足（min=3，但只有 2 字符）
        assert!(!is_valid_generic("ab", true, true, "", Some(3), None));
        // 长度超（max=3，但有 6 字符）
        assert!(!is_valid_generic("abc123", true, true, "", None, Some(3)));
        // 含特殊字符但白名单为空
        assert!(!is_valid_generic("a@b", true, true, "", None, None));
        // 含特殊字符但不在白名单中
        assert!(!is_valid_generic("a#b", false, true, "@", None, None));
        // 空串
        assert!(!is_valid_generic("", true, true, "", None, None));
        // 三个字符类全空
        assert!(!is_valid_generic("abc", false, false, "", None, None));
    }
}
