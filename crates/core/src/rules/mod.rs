//! 规则引擎核心：YAML 反序列化的 `RuleSet` / `FieldRule` / `MaskRule` +
//! 校验器 / 脱敏器注册表 + 加载入口 + 抽象算子模型。
//!
//! v0.4.4 重构：删除预置规则集（`default_mask.yaml` / `custom_example.yaml`）
//! 与预置模板元信息（`PRESET_SPECS` / `list_tagged_presets` / `list_*_op_types`）。
//! `FieldRule` / `MaskRule` 字段从 `validator` / `masker` / `regex` / `tags`
//! 重构为 `scope`（数据类型）+ `tag`（单选用途）。规则池初始为空，后续版本
//! 按数据类型（idcard/phone/bankcard/...）逐步接入内置规则与添加入口。
//!
//! 算子实现（`MaskOp` / `ValidateOp` + 9 个正则常量 + `ValidatorRegistry` +
//! `MaskerRegistry`）完整保留，作为后续接入的核心资产。

pub mod builtin;
pub mod loader;
pub mod mask_op;
pub mod patterns;
pub mod presets;
pub mod registry;
pub mod types;
pub mod validate_op;

pub use builtin::{bankcard_extract_rule, builtin_ruleset, ip_extract_rule, phone_extract_rule};
pub use loader::{load_ruleset, load_ruleset_str};
pub use mask_op::{
    apply_mask_op, ConstReplaceOp, MaskOp, MatchMode, RegexReplaceOp, SplitTemplateOp, TemplateOp,
};
pub use registry::{MaskerRegistry, ValidatorRegistry};
pub use types::{FieldRule, MaskRule, RuleSet};
pub use validate_op::{
    apply_validate_op, AlgorithmOp, AlgoKind, GuardKind, RegexOp, RegexWithGuardOp, ValidateOp,
};

/// 根据 [`FieldRule`] 在指定注册表中构造校验器实例。
///
/// v0.4.4：按 `rule.scope`（数据类型）查 `ValidatorRegistry`。`scope` 取代
/// 旧 `rule.validator` 名语义，如 scope="phone" -> 查注册名 "phone" 的
/// `PhoneValidator`。未注册的 scope 返回 `None`，由调用方决定如何报错。
///
/// 注：旧 `rule.regex` 兜底分支已删除（regex 字段已移除）。后续若需自定义
/// 正则校验，可通过 `scope="custom"` + params.pattern 在算子层处理
/// （`ValidateOp::from_rule` 后续接入时实现）。
pub fn build_validator(
    rule: &FieldRule,
    reg: &ValidatorRegistry,
) -> Option<Box<dyn crate::validators::Validator>> {
    // v0.4.4：优先尝试 ValidateOp::from_rule（按 scope 查通用算子，后续接入）；
    // 失败回退到 ValidatorRegistry::get（按 scope 查内置 validator）。
    if let Some(op) = ValidateOp::from_rule(rule) {
        return Some(Box::new(op));
    }
    reg.get(&rule.scope)
}

/// 根据 [`MaskRule`] 构造一个具体脱敏器实例。
///
/// v0.4.4：按 `rule.scope`（数据类型）查 `MaskOp::from_rule` 构造通用算子。
/// `scope` 取代旧 `rule.masker` 名语义。未知名返回 `None`。
///
/// 与 [`build_validator`] 不同，脱敏器构造**不查 `MaskerRegistry`**：注册表里
/// 存的是默认 params 的工厂闭包，而 `MaskRule.params`（如 `keep_prefix` /
/// `keep_suffix`）必须透传到具体 masker 的 `new(params)`。
pub fn build_masker(rule: &MaskRule) -> Option<Box<dyn crate::maskers::Masker>> {
    MaskOp::from_rule(&rule.scope, rule.params.as_ref())
        .map(|op| Box::new(op) as Box<dyn crate::maskers::Masker>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yml::Value;
    use std::collections::HashMap;

    #[test]
    fn build_validator_falls_back_to_registry() {
        // v0.4.4：scope="always_ok" 在 ValidateOp::from_rule 中无映射，
        // 回退到 ValidatorRegistry::get("always_ok") 命中自定义注册项。
        struct AlwaysOk;
        impl crate::validators::Validator for AlwaysOk {
            fn validate(&self, _value: &str) -> crate::validators::ValidationResult {
                crate::validators::ValidationResult::ok()
            }
        }
        let mut reg = ValidatorRegistry::new();
        reg.register("always_ok", || Box::new(AlwaysOk));
        let rule = FieldRule {
            field: "x".into(),
            scope: "always_ok".into(),
            tag: "validate".into(),
            params: None,
            message: None,
            description: None,
        };
        let v = build_validator(&rule, &reg).expect("registry lookup should produce validator");
        assert!(v.validate("anything").valid);
    }

    #[test]
    fn build_validator_missing_scope_returns_none() {
        // 未注册的 scope，ValidateOp 无映射 + registry 无注册 -> None。
        let reg = ValidatorRegistry::new();
        let rule = FieldRule {
            field: "x".into(),
            scope: "ghost".into(),
            tag: "validate".into(),
            params: None,
            message: None,
            description: None,
        };
        assert!(build_validator(&rule, &reg).is_none());
    }

    #[test]
    fn build_validator_phone_scope_uses_registry() {
        // v0.4.4：scope="phone" 在 ValidateOp::from_rule 中暂无映射（后续接入），
        // 回退到 ValidatorRegistry::get("phone") 命中内置 PhoneValidator。
        let reg = crate::validators::default_validator_registry();
        let rule = FieldRule {
            field: "phone".into(),
            scope: "phone".into(),
            tag: "validate".into(),
            params: None,
            message: None,
            description: None,
        };
        let v = build_validator(&rule, &reg).expect("phone scope should hit registry");
        assert!(v.validate("13812345678").valid);
        assert!(!v.validate("abc").valid);
    }

    #[test]
    fn build_masker_template_with_params() {
        // scope="template" 在 MaskOp::from_rule 中映射到 Template 算子。
        // 带 params（keep_prefix:6/keep_suffix:4/mask_char:*/mask_min_len:8/
        // min_len:18/max_len:18/cjk:false）-> 对 "110101199001011234" 得
        // "110101********1234"。
        let mut params = HashMap::new();
        params.insert("keep_prefix".to_string(), Value::Number(6.into()));
        params.insert("keep_suffix".to_string(), Value::Number(4.into()));
        params.insert("mask_char".to_string(), Value::String("*".into()));
        params.insert("mask_min_len".to_string(), Value::Number(8.into()));
        params.insert("min_len".to_string(), Value::Number(18.into()));
        params.insert("max_len".to_string(), Value::Number(18.into()));
        params.insert("cjk".to_string(), Value::Bool(false));
        let rule = MaskRule {
            field: "x".into(),
            scope: "template".into(),
            tag: "mask".into(),
            params: Some(params),
            message: None,
            description: None,
        };
        let m = build_masker(&rule).expect("template masker should be built");
        assert_eq!(m.mask("110101199001011234"), "110101********1234");
        // 非 18 位原样返回（guard）
        assert_eq!(m.mask("11010119900101"), "11010119900101");
    }

    #[test]
    fn build_masker_unknown_scope_returns_none() {
        // v0.4.4：未知名（含旧预置别名 idcard_mask/phone_mask/custom 等）一律 None。
        for name in ["ghost", "idcard_mask", "phone_mask", "custom", "delete", "replace", "regex_extract"] {
            let rule = MaskRule {
                field: "x".into(),
                scope: name.into(),
                tag: "mask".into(),
                params: None,
                message: None,
                description: None,
            };
            assert!(
                build_masker(&rule).is_none(),
                "{name} should NOT be built (preset aliases removed)"
            );
        }
    }

    #[test]
    fn build_masker_all_generic_ops_return_some() {
        // 4 个通用脱敏算子名均能被 build_masker 构造出实例。
        for name in ["template", "split_template", "regex_replace", "const_replace"] {
            let rule = MaskRule {
                field: "x".into(),
                scope: name.into(),
                tag: "mask".into(),
                params: None,
                message: None,
                description: None,
            };
            assert!(
                build_masker(&rule).is_some(),
                "{name} should be built by build_masker"
            );
        }
    }
}
