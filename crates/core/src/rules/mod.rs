//! 规则引擎核心：YAML 反序列化的 `RuleSet` / `FieldRule` / `MaskRule` +
//! 校验器 / 脱敏器注册表 + 加载入口 + 抽象算子模型（T0-21）。
//!
//! 详见 `types.rs` / `registry.rs` / `loader.rs` / `operator.rs` / `presets.rs`。
//! 具体业务 validator / masker 由 T0-3 / T0-4 填充；T0-21 把 11 个 masker
//! 收敛为 4 个通用算子（`MaskOp`）、8 个 validator 收敛为 3 个（`ValidateOp`），
//! `build_masker` / `build_validator` 内部走 `Op::from_rule`，签名与行为保持
//! 向后兼容。
//!
//! v0.1.0 重构：删除所有预置别名（用户要求「只做规则模版」），`presets`
//! 只保留通用算子元信息（`list_*_op_types`）与常用正则常量。

pub mod loader;
pub mod operator;
pub mod presets;
pub mod registry;
pub mod types;

pub use loader::{load_default_mask_ruleset, load_ruleset, load_ruleset_str};
pub use operator::{
    apply_mask_op, apply_validate_op, AlgorithmOp, AlgoKind, ConstReplaceOp, GuardKind, MaskOp,
    MatchMode, RegexOp, RegexReplaceOp, RegexWithGuardOp, SplitTemplateOp, TemplateOp, ValidateOp,
};
pub use presets::{list_mask_op_types, list_validate_op_types};
pub use registry::{MaskerRegistry, ValidatorRegistry};
pub use types::{FieldRule, MaskRule, RuleSet};

/// 内置 `default_mask.yaml` 内容（嵌入二进制，避免运行时找文件）。
pub const DEFAULT_MASK_YAML: &str = include_str!("../../../../rules/default_mask.yaml");
/// 内置 `custom_example.yaml` 内容。
pub const CUSTOM_EXAMPLE_YAML: &str = include_str!("../../../../rules/custom_example.yaml");

/// 根据 [`FieldRule`] 在指定注册表中构造校验器实例。
///
/// 自定义兜底：当 `rule.regex` 存在时构造 [`RegexValidator`]，忽略
/// `rule.validator` 名；否则按 `rule.validator` 名查注册表。未注册时返回
/// `None`，由调用方决定如何报错。
///
/// T0-21 起：内部优先尝试 `ValidateOp::from_rule`（覆盖 regex 兜底 + 7 个预置
/// 别名），失败时回退到 `ValidatorRegistry::get`（保持自定义注册项可用）。
pub fn build_validator(
    rule: &FieldRule,
    reg: &ValidatorRegistry,
) -> Option<Box<dyn crate::validators::Validator>> {
    // T0-21：优先走通用算子模型（覆盖 7 个预置别名）。
    if rule.regex.is_none() {
        if let Some(op) = ValidateOp::from_rule(rule) {
            return Some(Box::new(op));
        }
    } else {
        // regex 兜底：保留旧行为——pattern 非法时返回 None（与旧
        // `Regex::new(pattern).ok()?` 一致），合法时构造 RegexValidator。
        use crate::validators::RegexValidator;
        if let Some(pattern) = rule.regex.as_ref() {
            let re = regex::Regex::new(pattern).ok()?;
            return Some(Box::new(RegexValidator::new(re, rule.message.clone())));
        }
    }
    // 回退：自定义注册项（非预置别名且无 regex）。
    reg.get(&rule.validator)
}

/// 根据 [`MaskRule`] 构造一个具体脱敏器实例。
///
/// 与 [`build_validator`] 不同，脱敏器构造**不查 `MaskerRegistry`**：注册表里
/// 存的是默认 params 的工厂闭包，而 `MaskRule.params`（如 `custom` 的
/// `keep_prefix` / `keep_suffix`）必须透传到具体 masker 的 `new(params)`。
///
/// T0-21 起：内部走 `MaskOp::from_rule` 构造通用算子，`MaskOp` 自身 impl
/// [`crate::maskers::Masker`]，直接装箱返回。未知名返回 `None`。
pub fn build_masker(rule: &MaskRule) -> Option<Box<dyn crate::maskers::Masker>> {
    MaskOp::from_rule(&rule.masker, rule.params.as_ref())
        .map(|op| Box::new(op) as Box<dyn crate::maskers::Masker>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yml::Value;
    use std::collections::HashMap;

    #[test]
    fn build_validator_regex_fallback_overrides_name() {
        // 注册表里没有 phone；但因 regex 存在，应返回 RegexValidator。
        let reg = ValidatorRegistry::new();
        let rule = FieldRule {
            field: "phone".into(),
            validator: "phone".into(),
            params: None,
            regex: Some(r"^1\d{10}$".into()),
            message: Some("bad phone".into()),
            description: None,
        };
        let v = build_validator(&rule, &reg).expect("regex fallback should produce validator");
        assert!(v.validate("13800138000").valid);
        assert!(!v.validate("abc").valid);
    }

    #[test]
    fn build_validator_falls_back_to_registry() {
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
            validator: "always_ok".into(),
            params: None,
            regex: None,
            message: None,
            description: None,
        };
        let v = build_validator(&rule, &reg).expect("registry lookup should produce validator");
        assert!(v.validate("anything").valid);
    }

    #[test]
    fn build_validator_missing_name_no_regex_returns_none() {
        let reg = ValidatorRegistry::new();
        let rule = FieldRule {
            field: "x".into(),
            validator: "ghost".into(),
            params: None,
            regex: None,
            message: None,
            description: None,
        };
        assert!(build_validator(&rule, &reg).is_none());
    }

    #[test]
    fn build_validator_invalid_regex_returns_none() {
        let reg = ValidatorRegistry::new();
        let rule = FieldRule {
            field: "x".into(),
            validator: "x".into(),
            params: None,
            regex: Some("(".into()),
            message: None,
            description: None,
        };
        assert!(build_validator(&rule, &reg).is_none());
    }

    #[test]
    fn build_masker_template_with_params() {
        // 带 params 的 template MaskRule（keep_prefix:6/keep_suffix:4/
        // mask_char:*/mask_min_len:8/min_len:18/max_len:18/cjk:false）
        // → 对 "110101199001011234" 得 "110101********1234"（等价旧 idcard_mask）。
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
            masker: "template".into(),
            params: Some(params),
            description: None,
        };
        let m = build_masker(&rule).expect("template masker should be built");
        assert_eq!(m.mask("110101199001011234"), "110101********1234");
        // 非 18 位原样返回（guard）
        assert_eq!(m.mask("11010119900101"), "11010119900101");
    }

    #[test]
    fn build_masker_unknown_returns_none() {
        // v0.1.0 重构后：未知名（含旧预置别名）一律返回 None。
        for name in ["ghost", "idcard_mask", "phone_mask", "custom", "delete", "replace", "regex_extract"] {
            let rule = MaskRule {
                field: "x".into(),
                masker: name.into(),
                params: None,
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
                masker: name.into(),
                params: None,
                description: None,
            };
            assert!(
                build_masker(&rule).is_some(),
                "{name} should be built by build_masker"
            );
        }
    }
}
