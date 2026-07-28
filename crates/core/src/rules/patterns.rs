//! 按 scope 集中定义的提取/校验正则表（v0.5.0 起）。
//!
//! v0.5.0 重构：把原本散布在 `scan::mod.rs::extract_pattern`（宽松召回）
//! 与 `validators/*.rs`（严格校验）的双正则源收敛到本表。所有业务模块
//! 统一引用 `patterns::PHONE.validate` / `patterns::extract_pattern("phone")`
//! 等，消除硬编码散布。行为零回归：正则字符串与原散布版本逐字符一致。
//!
//! - `extract`：宽松召回候选（带 `(?-u)\b` ASCII 边界，避免中文旁 `\b`
//!   失效；v0.6.7 修复——regex crate 默认 Unicode-aware 把中文当 word char，
//!   导致 `联系13812345678打电话` 中无 `\b` 边界而召回不到候选）
//! - `validate`：严格校验（`^...$` 锚定，用于 validator 阶段过滤误报）
//!
//! 注：`idcard` 无独立正则校验器（IdCardValidator 走地址码 + 出生日期 +
//! GB11643 校验码算法，非正则），`IDCARD.validate` 仅给一个格式锚定串
//! 供需要快速 is_match 的场景使用，不替代 IdCardValidator。

/// 单个 scope 的成对正则。
pub struct ScopePatterns {
    /// 宽松召回正则（带 `(?-u)\b` ASCII 边界；见文件头注释）。
    pub extract: &'static str,
    /// 严格校验正则（`^...$` 锚定）。
    pub validate: &'static str,
}

/// 身份证号：18 位，末位 X/x 大小写等价。
pub const IDCARD: ScopePatterns = ScopePatterns {
    extract: r"(?-u)\b\d{17}[\dXx]\b",
    validate: r"^\d{17}[\dXx]$",
};
/// 手机号：11 位数字，首位 1。
pub const PHONE: ScopePatterns = ScopePatterns {
    extract: r"(?-u)\b\d{11}\b",
    validate: r"^1\d{10}$",
};
/// 银行卡号：13..19 位数字（再过 Luhn 校验）。
pub const BANKCARD: ScopePatterns = ScopePatterns {
    extract: r"(?-u)\b\d{13,19}\b",
    validate: r"^\d{13,19}$",
};
/// 邮箱。
pub const EMAIL: ScopePatterns = ScopePatterns {
    extract: r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
    validate: r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$",
};
/// IPv4：四段 0-255，拒绝前导零。
pub const IP: ScopePatterns = ScopePatterns {
    extract: r"\b(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)){3}\b",
    validate: r"^((25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\.){3}(25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)$",
};
/// MAC 地址：`XX:XX:XX:XX:XX:XX` 或 `XX-XX-XX-XX-XX-XX`，大小写不敏感。
pub const MAC: ScopePatterns = ScopePatterns {
    extract: r"(?-u)\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b",
    validate: r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$",
};
/// 用户名：仅字母数字。
pub const USERNAME: ScopePatterns = ScopePatterns {
    extract: r"[A-Za-z0-9]{3,}",
    validate: r"^[A-Za-z0-9]+$",
};
/// 中文姓名：全 CJK。v0.6.7 提取正则限长 2-4 字，降低「联系电话」「身份证号」
/// 等长中文段误报率；校验正则不限长（仍 `^...$` 全中文即可）。
pub const NAME: ScopePatterns = ScopePatterns {
    extract: r"[\x{4e00}-\x{9fa5}]{2,4}",
    validate: r"^[\x{4e00}-\x{9fa5}]+$",
};

/// 按 `scope`（数据类型）返回宽松召回正则（带 `(?-u)\b` ASCII 边界）。
///
/// `scope` 取代旧 validator 名语义：scan 阶段按 `rule.scope` 查本表得到
/// 提取正则，find_iter 出候选子串后再交给 validator 严格校验。
/// 未注册的 scope 返回 `None`，由调用方决定跳过或报错。
pub fn extract_pattern(scope: &str) -> Option<&'static str> {
    Some(match scope {
        "idcard" => IDCARD.extract,
        "phone" => PHONE.extract,
        "bankcard" => BANKCARD.extract,
        "email" => EMAIL.extract,
        "ip" => IP.extract,
        "mac" => MAC.extract,
        "username" => USERNAME.extract,
        "name" => NAME.extract,
        _ => return None,
    })
}

/// 按 `scope`（数据类型）返回严格校验正则（`^...$` 锚定）。
///
/// 供 `validators/*.rs` 引用，替代各自硬编码的正则字面量。未注册的
/// scope 返回 `None`。
pub fn validate_pattern(scope: &str) -> Option<&'static str> {
    Some(match scope {
        "idcard" => IDCARD.validate,
        "phone" => PHONE.validate,
        "bankcard" => BANKCARD.validate,
        "email" => EMAIL.validate,
        "ip" => IP.validate,
        "mac" => MAC.validate,
        "username" => USERNAME.validate,
        "name" => NAME.validate,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_and_validate_lookup_roundtrip() {
        for scope in [
            "idcard", "phone", "bankcard", "email", "ip", "mac", "username", "name",
        ] {
            assert!(extract_pattern(scope).is_some(), "{scope} extract");
            assert!(validate_pattern(scope).is_some(), "{scope} validate");
        }
    }

    #[test]
    fn unknown_scope_returns_none() {
        assert!(extract_pattern("ghost").is_none());
        assert!(validate_pattern("ghost").is_none());
    }

    #[test]
    fn phone_validate_is_first_digit_one() {
        // 逐字符保留自 validators/phone.rs：首位 1，共 11 位。
        assert_eq!(PHONE.validate, r"^1\d{10}$");
    }

    #[test]
    fn ip_validate_matches_presets() {
        // 与 presets::IP_REGEX 逐字符一致（行为零回归）。
        assert_eq!(
            IP.validate,
            crate::rules::presets::IP_REGEX
        );
    }

    // v0.6.7 回归测试：`(?-u)\b` ASCII 边界修复中文旁 `\b` 失效问题。
    // 旧版本 PHONE.extract = r"\b\d{11}\b" 在 Unicode-aware 模式下，
    // 把中文字符当 word char → "联系13812345678打电话" 中无 `\b` 边界
    // → find_iter 召回不到候选 → 提取失败（核心 bug）。
    #[test]
    fn phone_extract_matches_in_chinese_context() {
        use regex::Regex;
        let re = Regex::new(PHONE.extract).expect("valid regex");
        let text = "联系13812345678打电话";
        let m = re.find(text).expect("phone should be extracted");
        assert_eq!(m.as_str(), "13812345678");
    }

    #[test]
    fn idcard_extract_matches_in_chinese_context() {
        use regex::Regex;
        let re = Regex::new(IDCARD.extract).expect("valid regex");
        let text = "身份证286071197501111126登记";
        let m = re.find(text).expect("idcard should be extracted");
        assert_eq!(m.as_str(), "286071197501111126");
    }

    #[test]
    fn bankcard_extract_matches_in_chinese_context() {
        use regex::Regex;
        let re = Regex::new(BANKCARD.extract).expect("valid regex");
        // Luhn 合法的 16 位卡号
        let text = "卡号6210987632101234结束";
        let m = re.find(text).expect("bankcard should be extracted");
        assert_eq!(m.as_str(), "6210987632101234");
    }

    #[test]
    fn mac_extract_matches_in_chinese_context() {
        use regex::Regex;
        let re = Regex::new(MAC.extract).expect("valid regex");
        let text = "设备地址AA:BB:CC:DD:EE:FF结束";
        let m = re.find(text).expect("mac should be extracted");
        assert_eq!(m.as_str(), "AA:BB:CC:DD:EE:FF");
    }

    #[test]
    fn name_extract_caps_at_four_chars() {
        use regex::Regex;
        let re = Regex::new(NAME.extract).expect("valid regex");
        // 四字姓名整段召回
        assert_eq!(re.find("王二麻子张三").map(|m| m.as_str()), Some("王二麻子"));
        // 五字连续段只召回前 4 字（贪婪，但被 {2,4} 截断）
        assert_eq!(re.find("欧阳娜娜子").map(|m| m.as_str()), Some("欧阳娜娜"));
    }
}
