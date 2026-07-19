//! 脱敏 / 校验算子元信息（v0.1.0 重构：删除所有预置别名，只保留通用算子）。
//!
//! v0.1.0 重构：用户要求「只做规则模版」，因此本模块不再维护旧 masker /
//! validator 名 → 算子配置的映射表，预置别名（`idcard_mask` / `phone_mask` /
//! `email` / `username` / `name` / `idcard` / `bankcard` / `phone` / `mac` 等）
//! 全部删除。所有规则均通过通用算子 + 显式 params 配置：
//! - 脱敏：`template` / `split_template` / `regex_replace` / `const_replace`
//! - 校验：`regex` / `algorithm` / `regex_with_guard`
//!
//! [`list_mask_op_types`] / [`list_validate_op_types`] 仅返回这 4 + 3 个通用
//! 算子，供前端下拉源使用。每条规则的所有参数（keep_prefix / keep_suffix /
//! mask_char / min_len / max_len / cjk / pattern / replacement / match_mode /
//! algo / guard / prefix_set / prefix / message 等）均由调用方显式提供。
//!
//! 旧正则常量（EMAIL_REGEX / USERNAME_REGEX 等）作为「常用正则样例」保留导出，
//! 便于 [`crate::rules::operator::RegexOp`] 使用方在 params.pattern 中引用，
//! 但不再绑定任何预置别名。

/// 邮箱常用正则样例（供用户在 `regex` 算子的 params.pattern 中直接引用）。
pub const EMAIL_REGEX: &str = r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$";
/// 用户名常用正则样例（字母数字）。
pub const USERNAME_REGEX: &str = r"^[A-Za-z0-9]+$";
/// 中文姓名常用正则样例。
pub const NAME_REGEX: &str = r"^[\x{4e00}-\x{9fa5}]+$";
/// 11 位手机号格式正则样例。
pub const PHONE_REGEX: &str = r"^\d{11}$";
/// MAC 地址格式正则样例。
pub const MAC_REGEX: &str = r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$";

/// 生日格式正则样例（YYYYMMDD，月份 01-12、日期 01-31，不精确校验闰年）。
pub const BIRTH_REGEX: &str = r"^\d{4}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])$";
/// 中文地址正则样例（包含中文字符 + 省/市/区/县/镇/村/街/路/号 任一关键字）。
pub const ADDRESS_REGEX: &str = r"^[\x{4e00}-\x{9fa5}]+.*(省|市|区|县|镇|村|街|路|号).*$";
/// 密码格式正则样例（长度 8-32，仅字母数字）。
///
/// 注：`regex` crate 不支持 look-around，无法在单条正则中同时强制
/// 「至少含字母 + 至少含数字」。本样例仅约束长度与字符集；如需强制
/// 字母+数字混合，请配合 `algorithm` / `regex_with_guard` 算子做附加检查。
pub const PASSWORD_REGEX: &str = r"^[A-Za-z0-9]{8,32}$";
/// IPv4 格式正则样例（四段 0-255）。
pub const IP_REGEX: &str =
    r"^((25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\.){3}(25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)$";

/// 返回脱敏算子类型清单（供前端下拉源）。
///
/// v0.1.0 重构后仅含 4 个通用算子（无预置别名）：
/// - `template` / `split_template` / `regex_replace` / `const_replace`
///
/// 元组格式：`(op_name, label)`。
pub fn list_mask_op_types() -> Vec<(&'static str, &'static str)> {
    vec![
        ("template", "通用模板脱敏"),
        ("split_template", "切分模板脱敏"),
        ("regex_replace", "正则替换脱敏"),
        ("const_replace", "常量替换脱敏"),
    ]
}

/// 返回校验算子类型清单（供前端下拉源）。
///
/// v0.1.0 重构后仅含 3 个通用算子（无预置别名）：
/// - `regex` / `algorithm` / `regex_with_guard`
pub fn list_validate_op_types() -> Vec<(&'static str, &'static str)> {
    vec![
        ("regex", "正则校验"),
        ("algorithm", "算法校验"),
        ("regex_with_guard", "守卫+正则校验"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_op_types_only_general_operators() {
        let mask = list_mask_op_types();
        let mask_names: Vec<&str> = mask.iter().map(|(n, _)| *n).collect();
        assert_eq!(mask_names, ["template", "split_template", "regex_replace", "const_replace"]);

        let val = list_validate_op_types();
        let val_names: Vec<&str> = val.iter().map(|(n, _)| *n).collect();
        assert_eq!(val_names, ["regex", "algorithm", "regex_with_guard"]);
    }

    #[test]
    fn regex_constants_still_available() {
        // 常用正则常量保留导出，便于用户在 regex 算子 params.pattern 中引用。
        assert!(EMAIL_REGEX.contains("@"));
        assert!(PHONE_REGEX.starts_with(r"^\d{11}$"));
        assert!(MAC_REGEX.contains("A-Fa-f"));
        assert!(USERNAME_REGEX.chars().any(|c| c == '+'));
        assert!(NAME_REGEX.contains("4e00"));
    }

    #[test]
    fn birth_regex_matches_valid_and_invalid() {
        use regex::Regex;
        let re = Regex::new(BIRTH_REGEX).unwrap();
        assert!(re.is_match("20240115"));
        assert!(re.is_match("19991231"));
        assert!(!re.is_match("20241301")); // 月份 13 非法
        assert!(!re.is_match("20241232")); // 日期 32 非法
        assert!(!re.is_match("2024-01-15")); // 含分隔符
        assert!(!re.is_match("abcd0101")); // 非数字
    }

    #[test]
    fn address_regex_matches_valid_and_invalid() {
        use regex::Regex;
        let re = Regex::new(ADDRESS_REGEX).unwrap();
        assert!(re.is_match("广东省深圳市南山区"));
        assert!(re.is_match("北京市朝阳区"));
        assert!(re.is_match("四川省成都市武侯区科华路1号"));
        assert!(re.is_match("广东省")); // 关键字「省」+ 中文，符合样例约束
        assert!(!re.is_match("123456")); // 纯数字
        assert!(!re.is_match("New York City")); // 非中文
    }

    #[test]
    fn password_regex_matches_valid_and_invalid() {
        use regex::Regex;
        let re = Regex::new(PASSWORD_REGEX).unwrap();
        // 正例：长度 8-32 且仅字母数字
        assert!(re.is_match("abc12345"));
        assert!(re.is_match("Password12345"));
        assert!(re.is_match("A1B2C3D4"));
        // 反例：长度/字符集不符
        assert!(!re.is_match("abc123")); // 长度 < 8
        assert!(!re.is_match("abc 12345")); // 含空格（不在字符集）
        assert!(!re.is_match("abcdefghijklmnopqrstuvwxyz1234567")); // 长度 > 32
        assert!(!re.is_match("password!@#")); // 含非字母数字
    }

    #[test]
    fn ip_regex_matches_valid_and_invalid() {
        use regex::Regex;
        let re = Regex::new(IP_REGEX).unwrap();
        assert!(re.is_match("192.168.1.1"));
        assert!(re.is_match("255.255.255.255"));
        assert!(re.is_match("0.0.0.0"));
        assert!(re.is_match("10.0.0.1"));
        assert!(!re.is_match("256.1.1.1")); // 段 > 255
        assert!(!re.is_match("192.168.1")); // 段数不足
        assert!(!re.is_match("192.168.1.1.1")); // 段数过多
        assert!(!re.is_match("a.b.c.d")); // 非数字
    }
}
