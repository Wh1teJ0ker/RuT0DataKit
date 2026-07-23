//! 按 scope 集中定义的提取/校验正则表（v0.5.0 起）。
//!
//! v0.5.0 重构：把原本散布在 `scan::mod.rs::extract_pattern`（宽松召回）
//! 与 `validators/*.rs`（严格校验）的双正则源收敛到本表。所有业务模块
//! 统一引用 `patterns::PHONE.validate` / `patterns::extract_pattern("phone")`
//! 等，消除硬编码散布。行为零回归：正则字符串与原散布版本逐字符一致。
//!
//! - `extract`：宽松召回候选（带 `\b` 边界，用于 scan 阶段 find_iter）
//! - `validate`：严格校验（`^...$` 锚定，用于 validator 阶段过滤误报）
//!
//! 注：`idcard` 无独立正则校验器（IdCardValidator 走地址码 + 出生日期 +
//! GB11643 校验码算法，非正则），`IDCARD.validate` 仅给一个格式锚定串
//! 供需要快速 is_match 的场景使用，不替代 IdCardValidator。

/// 单个 scope 的成对正则。
pub struct ScopePatterns {
    /// 宽松召回正则（带 `\b` 边界）。
    pub extract: &'static str,
    /// 严格校验正则（`^...$` 锚定）。
    pub validate: &'static str,
}

/// 身份证号：18 位，末位 X/x 大小写等价。
pub const IDCARD: ScopePatterns = ScopePatterns {
    extract: r"\b\d{17}[\dXx]\b",
    validate: r"^\d{17}[\dXx]$",
};
/// 手机号：11 位数字，首位 1。
pub const PHONE: ScopePatterns = ScopePatterns {
    extract: r"\b\d{11}\b",
    validate: r"^1\d{10}$",
};
/// 银行卡号：13..19 位数字（再过 Luhn 校验）。
pub const BANKCARD: ScopePatterns = ScopePatterns {
    extract: r"\b\d{13,19}\b",
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
    extract: r"\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b",
    validate: r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$",
};
/// 用户名：仅字母数字。
pub const USERNAME: ScopePatterns = ScopePatterns {
    extract: r"[A-Za-z0-9]{3,}",
    validate: r"^[A-Za-z0-9]+$",
};
/// 中文姓名：全 CJK。
pub const NAME: ScopePatterns = ScopePatterns {
    extract: r"[\x{4e00}-\x{9fa5}]{2,}",
    validate: r"^[\x{4e00}-\x{9fa5}]+$",
};

/// 按 `scope`（数据类型）返回宽松召回正则（带 `\b` 边界）。
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
}
