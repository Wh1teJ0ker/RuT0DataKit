//! Validator trait 与内置校验器注册。
//!
//! 提供 `Validator` trait / `ValidationResult` / `RegexValidator`（YAML `regex`
//! 兜底），以及 8 个内置业务校验器：idcard / phone / bankcard / email / ip /
//! mac / username / name。`register_builtin_validators` 把它们注册进
//! `ValidatorRegistry`，`default_validator_registry` 返回一个已注册全部内置校验器
//! 的注册表。
//!
//! v0.6.8（修订）：`pinfo_phone` scope 不再有独立校验器文件（原
//! `validators/pinfo_phone.rs` 已删除），改由 `build_validator` 在 `pinfo_phone`
//! scope 时直接构造 `PhoneValidator::new(params)`，与 `phone` scope 统一。
//! 默认行为从 52 虚假号段改为 1 开头正常号码（用户反馈修正）。

pub mod bankcard;
pub mod email;
pub mod idcard;
pub mod ip;
pub mod mac;
pub mod name;
pub mod phone;
pub mod username;

use std::collections::HashMap;

use regex::Regex;

use crate::rules::registry::ValidatorRegistry;

/// 校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    pub valid: bool,
    pub message: Option<String>,
}

impl ValidationResult {
    /// 校验通过。
    pub fn ok() -> Self {
        Self {
            valid: true,
            message: None,
        }
    }

    /// 校验失败，附带固定 message。
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            valid: false,
            message: Some(message.into()),
        }
    }
}

/// 校验器 trait。所有内置 / 自定义校验器实现该接口。
pub trait Validator: Send + Sync {
    /// 校验单个字段值。
    fn validate(&self, value: &str) -> ValidationResult;
}

/// YAML `regex:` 兜底校验器：按正则整段 match。
///
/// 用于 v0.1.x 的自定义校验，不依赖内置 validator 名。
pub struct RegexValidator {
    re: Regex,
    message: Option<String>,
}

impl RegexValidator {
    pub fn new(re: Regex, message: Option<String>) -> Self {
        Self { re, message }
    }
}

impl Validator for RegexValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        if self.re.is_match(value) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail(
                self.message
                    .clone()
                    .unwrap_or_else(|| "regex does not match".to_string()),
            )
        }
    }
}

/// 注册内置校验器到给定注册表。
///
/// 注册：idcard / phone / bankcard / email / ip / mac / username / name，
/// 均以默认参数构造。具体参数注入（如 phone 的 `prefixes`、mac 的 `prefix`）
/// 由调用方直接构造对应 validator 实现（T0-5 pipeline / build_validator）。
pub fn register_builtin_validators(reg: &mut ValidatorRegistry) {
    reg.register("idcard", || {
        Box::new(idcard::IdCardValidator::new(HashMap::new()))
    });
    reg.register("phone", || {
        Box::new(phone::PhoneValidator::new(HashMap::new()))
    });
    reg.register("bankcard", || {
        Box::new(bankcard::BankCardValidator::new(HashMap::new()))
    });
    reg.register("email", || {
        Box::new(email::EmailValidator::new(HashMap::new()))
    });
    reg.register("ip", || {
        Box::new(ip::IpValidator::new(HashMap::new()))
    });
    reg.register("mac", || Box::new(mac::MacValidator::new(HashMap::new())));
    reg.register("username", || {
        Box::new(username::UsernameValidator::new(HashMap::new()))
    });
    reg.register("name", || {
        Box::new(name::NameValidator::new(HashMap::new()))
    });
}

/// 返回已注册全部内置校验器的注册表。
pub fn default_validator_registry() -> ValidatorRegistry {
    let mut reg = ValidatorRegistry::new();
    register_builtin_validators(&mut reg);
    reg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_builtin_validators_registers_all() {
        let reg = default_validator_registry();
        for name in [
            "idcard",
            "phone",
            "bankcard",
            "email",
            "ip",
            "mac",
            "username",
            "name",
        ] {
            assert!(reg.contains(name), "missing builtin validator: {name}");
        }
    }
}
