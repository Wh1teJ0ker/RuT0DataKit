//! Masker trait 与内置脱敏器注册。
//!
//! 内置脱敏器：
//! - [`idcard::IdCardMask`] / `idcard_mask`
//! - [`phone::PhoneMask`] / `phone_mask`
//! - [`bankcard::BankCardMask`] / `bankcard_mask`
//! - [`email::EmailMask`] / `email_mask`
//! - [`name::NameMask`] / `name_mask`
//! - [`customer_id::CustomerIdMask`] / `customer_id_mask`
//! - [`custom::CustomMask`] / `custom`
//! - [`regex_replace::RegexReplaceMask`] / `regex_replace`
//! - [`regex_extract::RegexExtractMask`] / `regex_extract`
//! - [`delete::DeleteMask`] / `delete`
//! - [`replace::ReplaceMask`] / `replace`
//!
//! 通过 [`register_builtin_maskers`] 注册到 `MaskerRegistry`，或调用
//! [`default_masker_registry`] 直接得到已注册好内置项的实例。

pub mod bankcard;
pub mod custom;
pub mod customer_id;
pub mod delete;
pub mod email;
pub mod idcard;
pub mod name;
pub mod phone;
pub mod regex_extract;
pub mod regex_replace;
pub mod replace;

use std::collections::HashMap;

use serde_yml::Value;

use crate::rules::registry::MaskerRegistry;

/// 脱敏器 trait。所有内置 / 自定义脱敏器实现该接口。
pub trait Masker: Send + Sync {
    /// 对单个字段值脱敏，返回脱敏后的字符串。
    fn mask(&self, value: &str) -> String;
}

/// 构造一个 `MaskerRegistry`，并注册好全部内置脱敏器。
///
/// 便于测试和后续 pipeline 直接使用。每个 masker 实例均使用默认 params，
/// 业务侧需自定义参数时直接 `XxxMask::new(params)` 构造。
pub fn default_masker_registry() -> MaskerRegistry {
    let mut reg = MaskerRegistry::new();
    register_builtin_maskers(&mut reg);
    reg
}

/// 把全部内置脱敏器注册到给定 `MaskerRegistry`。
///
/// 每个 factory 闭包返回默认 params 实例（`XxxMask::new(HashMap::new())`）。
pub fn register_builtin_maskers(reg: &mut MaskerRegistry) {
    reg.register("idcard_mask", || {
        Box::new(idcard::IdCardMask::new(HashMap::new()))
    });
    reg.register("phone_mask", || {
        Box::new(phone::PhoneMask::new(HashMap::new()))
    });
    reg.register("bankcard_mask", || {
        Box::new(bankcard::BankCardMask::new(HashMap::new()))
    });
    reg.register("email_mask", || {
        Box::new(email::EmailMask::new(HashMap::new()))
    });
    reg.register("name_mask", || {
        Box::new(name::NameMask::new(HashMap::new()))
    });
    reg.register("customer_id_mask", || {
        Box::new(customer_id::CustomerIdMask::new(HashMap::new()))
    });
    reg.register(
        "custom",
        || Box::new(custom::CustomMask::new(HashMap::new())),
    );
    reg.register(
        "regex_replace",
        || Box::new(regex_replace::RegexReplaceMask::new(HashMap::new())),
    );
    reg.register(
        "regex_extract",
        || Box::new(regex_extract::RegexExtractMask::new(HashMap::new())),
    );
    reg.register("delete", || {
        Box::new(delete::DeleteMask::new(HashMap::new()))
    });
    reg.register("replace", || {
        Box::new(replace::ReplaceMask::new(HashMap::new()))
    });
}

// 抑制未使用导入告警：`Value` 在子模块各自使用，这里仅供本模块签名引用占位。
#[allow(dead_code)]
fn _value_use_hint(_v: Value) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_builtin_maskers_registers_all() {
        let reg = default_masker_registry();
        let names = [
            "idcard_mask",
            "phone_mask",
            "bankcard_mask",
            "email_mask",
            "name_mask",
            "customer_id_mask",
            "custom",
            "regex_replace",
            "regex_extract",
            "delete",
            "replace",
        ];
        for name in names {
            assert!(reg.contains(name), "missing {name}");
        }
        // 对每个取出来的实例跑一次基本 mask 断言。
        let idcard = reg.get("idcard_mask").expect("idcard_mask");
        assert_eq!(idcard.mask("110101199001011234"), "110101********1234");
        let phone = reg.get("phone_mask").expect("phone_mask");
        assert_eq!(phone.mask("13812345678"), "138****5678");
        let bank = reg.get("bankcard_mask").expect("bankcard_mask");
        assert_eq!(bank.mask("6225887654321098"), "622588******1098");
        let email = reg.get("email_mask").expect("email_mask");
        // 本地 "zhangsan" 8 字 → 首 + 6 个 * + 末（n-2=6）
        assert_eq!(email.mask("zhangsan@example.com"), "z******n@example.com");
        let name = reg.get("name_mask").expect("name_mask");
        assert_eq!(name.mask("张三"), "张*");
        let cid = reg.get("customer_id_mask").expect("customer_id_mask");
        assert_eq!(cid.mask("12345678"), "1*******");
        let custom = reg.get("custom").expect("custom");
        // 默认 params：全替换为 5 个 *（hello 中间 5 字，max(5,1)=5）
        assert_eq!(custom.mask("abcdef"), "******");
    }

    #[test]
    fn default_registry_unknown_returns_none() {
        let reg = default_masker_registry();
        assert!(reg.get("nope").is_none());
    }
}
