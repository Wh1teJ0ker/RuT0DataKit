//! 个人信息校验：手机号 / 用户名 / 性别。

/// 手机号前缀白名单校验。
///
/// - `allowed` 空 → 不过滤前缀（任何开头都接受）。
/// - `allowed` 非空 → 前 3 位必须在列表内（支持非标准前缀如 7xx）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::check_phone_prefix;
/// assert!(check_phone_prefix("13412345678", &[]));                    // 空 = 不过滤
/// assert!(check_phone_prefix("79912345678", &[]));                    // 空 = 不过滤
/// assert!(check_phone_prefix("13412345678", &["134".into()]));        // 命中白名单
/// assert!(!check_phone_prefix("15912345678", &["134".into()]));       // 不命中
/// assert!(check_phone_prefix("79912345678", &["799".into()]));        // 非标准前缀
/// ```
pub fn check_phone_prefix(s: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    allowed.iter().any(|p| s.starts_with(p.as_str()))
}

/// 手机号校验：11 位、纯 ASCII 数字 + 前缀白名单。
///
/// - `allowed` 空 → 不过滤前缀（任意开头，只要 11 位纯数字）。
/// - `allowed` 非空 → 前 3 位必须在列表内（复用 [`check_phone_prefix`]），
///   支持非标准前缀（如 7xx）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_phone;
/// assert!(is_valid_phone("13412345678", &[]));
/// assert!(is_valid_phone("79912345678", &[]));             // 空 = 不过滤前缀
/// assert!(is_valid_phone("13412345678", &["134".into()]));
/// assert!(!is_valid_phone("13412345678", &["159".into()])); // 前缀不匹配
/// assert!(!is_valid_phone("1341234567", &[]));              // 长度不足
/// assert!(!is_valid_phone("1341234567a", &[]));             // 含非数字
/// ```
pub fn is_valid_phone(s: &str, allowed: &[String]) -> bool {
    s.len() == 11
        && s.bytes().all(|b| b.is_ascii_digit())
        && check_phone_prefix(s, allowed)
}

/// 用户名校验：纯字母数字（非空）。
///
/// 规则：`^[a-zA-Z0-9]+$`。有效 `admin` / `lufe1jian` / `91xxev`；
/// 无效 `ab.cd` / `ad_1in` / `a-123`（含 `.` / `_` / `-`）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_username;
/// assert!(is_valid_username("admin"));
/// assert!(is_valid_username("lufe1jian"));
/// assert!(is_valid_username("91xxev"));
/// assert!(!is_valid_username("ab.cd"));  // 含点
/// assert!(!is_valid_username("ad_1in")); // 含下划线
/// assert!(!is_valid_username("a-123"));  // 含连字符
/// assert!(!is_valid_username(""));       // 空串
/// ```
pub fn is_valid_username(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// 性别校验：仅「男」或「女」（trim 后精确匹配，区分大小写）。
///
/// 不接受 `male` / `m` / `1` 等别名（行级校验对原值严格匹配；性别归一化
/// 用于跨字段比对，见 `normalize_gender` in commands::processor）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_sex;
/// assert!(is_valid_sex("男"));
/// assert!(is_valid_sex("女"));
/// assert!(!is_valid_sex("male"));
/// assert!(!is_valid_sex(""));
/// ```
pub fn is_valid_sex(s: &str) -> bool {
    matches!(s.trim(), "男" | "女")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- check_phone_prefix ----

    #[test]
    fn phone_prefix_empty_list_passes() {
        // 空前缀列表 = 不过滤前缀，任何开头都接受
        assert!(check_phone_prefix("13412345678", &[]));
        assert!(check_phone_prefix("15987654321", &[]));
        assert!(check_phone_prefix("19898765432", &[]));
        assert!(check_phone_prefix("79912345678", &[]));
        assert!(check_phone_prefix("23412345678", &[]));
    }

    #[test]
    fn phone_prefix_match() {
        let allowed = vec!["134".to_string(), "159".to_string()];
        assert!(check_phone_prefix("13412345678", &allowed));
        assert!(check_phone_prefix("15987654321", &allowed));
    }

    #[test]
    fn phone_prefix_no_match() {
        let allowed = vec!["134".to_string()];
        assert!(!check_phone_prefix("15987654321", &allowed));
        assert!(!check_phone_prefix("19898765432", &allowed));
    }

    // ---- is_valid_phone ----

    #[test]
    fn phone_valid_samples() {
        // 空前缀列表 → 11 位 + 纯数字即通过（不过滤前缀）
        assert!(is_valid_phone("13412345678", &[]));
        assert!(is_valid_phone("15987654321", &[]));
        assert!(is_valid_phone("19898765432", &[]));
        assert!(is_valid_phone("79912345678", &[])); // 非 1 开头也通过
        // 前缀白名单命中
        assert!(is_valid_phone("13412345678", &["134".into()]));
        assert!(is_valid_phone("15987654321", &["134".into(), "159".into()]));
    }

    #[test]
    fn phone_invalid_samples() {
        // 前缀不匹配
        assert!(!is_valid_phone("13412345678", &["159".into()]));
        // 非标准前缀但白名单不匹配
        assert!(!is_valid_phone("79912345678", &["134".into()]));
        // 长度不足
        assert!(!is_valid_phone("1341234567", &[]));
        // 长度超
        assert!(!is_valid_phone("134123456789", &[]));
        // 含非数字
        assert!(!is_valid_phone("1341234567a", &[]));
        // 空串
        assert!(!is_valid_phone("", &[]));
    }

    #[test]
    fn phone_valid_non_standard_prefix() {
        // 非标准前缀（如 7xx）：空名单不过滤前缀，11 位纯数字即通过
        assert!(is_valid_phone("79996258889", &[]));              // 空名单放行
        assert!(is_valid_phone("79996258889", &["799".into()]));  // 白名单放行
        assert!(is_valid_phone("78638972987", &["786".into()])); // 白名单放行
    }

    // ---- is_valid_username ----

    #[test]
    fn username_valid_samples() {
        // 用户给的有效样例
        assert!(is_valid_username("admin"));
        assert!(is_valid_username("lufe1jian"));
        assert!(is_valid_username("91xxev"));
        // 混合大小写 + 数字
        assert!(is_valid_username("AbCd123"));
        assert!(is_valid_username("Z9"));
    }

    #[test]
    fn username_invalid_samples() {
        // 用户给的无效样例
        assert!(!is_valid_username("ab.cd")); // 含点
        assert!(!is_valid_username("ad_1in")); // 含下划线
        assert!(!is_valid_username("a-123")); // 含连字符
                                              // 其他非法
        assert!(!is_valid_username("")); // 空串
        assert!(!is_valid_username("用户名")); // 非字母数字
        assert!(!is_valid_username("abc def")); // 含空格
        assert!(!is_valid_username("a@b")); // 含特殊字符
    }

    // ---- is_valid_sex ----

    #[test]
    fn sex_valid_samples() {
        assert!(is_valid_sex("男"));
        assert!(is_valid_sex("女"));
        // trim 后匹配
        assert!(is_valid_sex(" 男 "));
        assert!(is_valid_sex("\t女"));
    }

    #[test]
    fn sex_invalid_samples() {
        assert!(!is_valid_sex("male"));
        assert!(!is_valid_sex("female"));
        assert!(!is_valid_sex("m"));
        assert!(!is_valid_sex("f"));
        assert!(!is_valid_sex("1"));
        assert!(!is_valid_sex("2"));
        assert!(!is_valid_sex(""));
        assert!(!is_valid_sex("未知"));
    }
}
