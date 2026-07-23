//! 常用正则常量（v0.4.4 重构保留）。
//!
//! v0.4.4 重构：删除 `PRESET_SPECS` / `PresetSpec` / `PresetEntry` /
//! `list_tagged_presets` / `list_mask_op_types` / `list_validate_op_types`
//! 等预置模板元信息（用户要求「先删除，后续慢慢接入」）。仅保留 9 个常用
//! 正则常量，作为后续按数据类型（scope）接入提取/校验规则的基础设施。
//!
//! 后续接入时：按 scope（如 "phone"）映射到 `PHONE_REGEX` +
//! `ValidatorRegistry::get("phone")`，在 `MaskOp::from_rule` /
//! `ValidateOp::from_rule` 中实现 scope -> 算子 + 默认参数的映射表。

/// 邮箱常用正则样例（供后续 scope="email" 接入时引用）。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_constants_still_available() {
        // 9 个常用正则常量保留导出，供后续 scope->pattern 映射接入时引用。
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
