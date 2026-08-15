//! 规则注册表：`RuleRegistry`。
//!
//! v1.2.1：从 `rules/mod.rs` 拆出，承载 `RuleRegistry` 结构及其 CRUD 方法。
//! `Rule` / `RuleKind` 见 [`crate::processor::rules::rule`]，
//! 内置规则构造器见 [`crate::processor::rules::builtins`]。

use crate::error::CoreError;
use crate::processor::rules::builtins::BuiltinRules;
use crate::processor::rules::rule::Rule;

/// 规则注册表。应用启动时从 DB 加载，内置规则集 seed 到 DB。
#[derive(Debug, Clone)]
pub struct RuleRegistry {
    rules: Vec<Rule>,
}

impl RuleRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 加载内置规则集。
    /// v1.1.0 三条姓名相关规则（脱敏/校验/提取各一条）。
    /// v1.1.3 T49：4 条原独立脱敏规则（身份证 / 手机 / 出生日期 / 银行卡）
    /// 收敛为预设（`idcard_preset()` 等，不再单独 seed）。T54 拆分：原
    /// `general-mask` 一条规则拆为 `simple-mask`（整段脱敏）+ `segment-mask`
    /// （分段脱敏）两条独立规则。`with_defaults()` 共 5 条（3 name +
    /// simple-mask + segment-mask）。旧 `general-mask` 由
    /// `cleanup_deprecated_rules()` 删除。
    /// v1.1.3 T55：新增 3 条提取规则（`phone-extract` / `bankcard-extract` /
    /// `ip-extract`），各持 `params` 函数式校验器。`with_defaults()` 共 8 条。
    /// v1.1.3 T55b：拆分 `ip-extract` 为 `ip4-extract`（IPv4）+ `ip6-extract`
    /// （IPv6）两条独立规则，`with_defaults()` 共 9 条。旧 `ip-extract` 由
    /// `cleanup_deprecated_rules()` 删除。
    /// v1.1.3 T55c：新增 `idcard-extract`（18 位身份证号 + 校验码 + 性别推断），
    /// `with_defaults()` 共 10 条。
    /// v1.1.4 T67：新增 6 条函数式校验规则（`username-validate` / `sex-validate` /
    /// `birth-validate` / `idcard-validate` / `phone-validate` / `address-validate`），
    /// kind=Validate 且带 `params` 走 `validate_extracted` 分发，
    /// `with_defaults()` 共 16 条。
    /// v1.1.4 续轮 T70：新增 `generic-validate`（字符类白名单 + 长度范围），
    /// `with_defaults()` 共 17 条。
    /// v1.1.5 T81：新增 `email-validate`（邮箱地址结构化校验），
    /// `with_defaults()` 共 18 条。
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        for rule in BuiltinRules::all() {
            reg.register(rule.clone());
        }
        reg
    }

    /// 注册一条规则（若 id 已存在则覆盖）。
    pub fn register(&mut self, rule: Rule) {
        if let Some(existing) = self.rules.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule;
        } else {
            self.rules.push(rule);
        }
    }

    /// 列出全部规则（按注册顺序）。
    pub fn list(&self) -> &[Rule] {
        &self.rules
    }

    /// 按 id 查找规则。
    pub fn get(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// 切换规则启用状态。不存在则返回 `Err`。
    pub fn toggle(&mut self, id: &str, enabled: bool) -> Result<(), CoreError> {
        let rule = self
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| CoreError::Processor(format!("rule not found: {id}")))?;
        rule.enabled = enabled;
        Ok(())
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::rules::extract_params::ExtractParams;
    use crate::processor::rules::rule::RuleKind;
    use crate::processor::rules::template::TemplateParams;

    #[test]
    fn with_defaults_loads_ten_rules() {
        let reg = RuleRegistry::with_defaults();
        let rules = reg.list();
        // v1.1.4 续轮 T70：3 条姓名 + simple-mask + segment-mask + 5 条 extract
        // （name-extract + phone/bankcard/ip4/ip6/idcard）+ 7 条 validate
        // （name + username/sex/birth/idcard/phone/address + generic）
        // v1.1.5 T81：+ email-validate = 18 条
        assert_eq!(rules.len(), 18);
        // 脱敏 / 校验 / 提取 三种 kind 都存在
        let kinds: Vec<RuleKind> = rules.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RuleKind::Mask));
        assert!(kinds.contains(&RuleKind::Validate));
        assert!(kinds.contains(&RuleKind::Extract));
        // 3 条 mask 规则（name-mask + simple-mask + segment-mask）
        let mask_count = kinds.iter().filter(|k| **k == RuleKind::Mask).count();
        assert_eq!(mask_count, 3);
        // 6 条 extract 规则（name-extract + phone/bankcard/ip4/ip6/idcard）
        let extract_count = kinds.iter().filter(|k| **k == RuleKind::Extract).count();
        assert_eq!(extract_count, 6);
        // v1.1.5 T81：9 条 validate 规则（name-validate + 6 条 T67 + generic + email）
        let validate_count = kinds.iter().filter(|k| **k == RuleKind::Validate).count();
        assert_eq!(validate_count, 9);
    }

    #[test]
    fn name_validate_rule_pattern_is_chinese_range() {
        let r = BuiltinRules::get("name-validate").unwrap();
        assert_eq!(r.id, "name-validate");
        assert_eq!(r.kind, RuleKind::Validate);
        assert_eq!(r.field.as_deref(), Some("name"));
        assert!(r.pattern.is_some());
        assert!(r.enabled);
        // name 规则无 template / params
        assert!(r.template.is_none());
        assert!(r.params.is_none());
    }

    #[test]
    fn name_mask_rule_keeps_first_char_semantic() {
        let r = BuiltinRules::get("name-mask").unwrap();
        assert_eq!(r.id, "name-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        assert_eq!(r.field.as_deref(), Some("name"));
        // replacement=None → SimpleMasker 默认掩码字符 `*`
        assert!(r.replacement.is_none());
        // 无 template / params → 走 SimpleMasker 旧逻辑
        assert!(r.template.is_none());
        assert!(r.params.is_none());
    }

    #[test]
    fn name_extract_rule_has_chinese_pattern() {
        let r = BuiltinRules::get("name-extract").unwrap();
        assert_eq!(r.id, "name-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert_eq!(r.field.as_deref(), Some("name"));
        assert!(r.pattern.is_some());
        // T55：name-extract 无 params（仅正则提取，不做函数式校验）
        assert!(r.params.is_none());
    }

    #[test]
    fn simple_mask_rule_has_empty_template() {
        let r = BuiltinRules::get("simple-mask").unwrap();
        assert_eq!(r.id, "simple-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        // 持空 Simple 模板 → is_empty()==true
        let tpl = r.template.expect("simple-mask must have template");
        assert!(matches!(tpl, TemplateParams::Simple(_)));
        assert!(tpl.is_empty());
        assert!(r.params.is_none());
    }

    #[test]
    fn segment_mask_rule_has_empty_template() {
        let r = BuiltinRules::get("segment-mask").unwrap();
        assert_eq!(r.id, "segment-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        // 持空 Segment 模板 → is_empty()==true
        let tpl = r.template.expect("segment-mask must have template");
        assert!(matches!(tpl, TemplateParams::Segment(_)));
        assert!(tpl.is_empty());
        assert!(r.params.is_none());
    }

    #[test]
    fn phone_extract_rule_has_phone_prefix_params() {
        let r = BuiltinRules::get("phone-extract").unwrap();
        assert_eq!(r.id, "phone-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert!(r.pattern.is_some());
        // T55：params = PhonePrefix{[]}（空前缀列表 = 不过滤前缀）
        let params = r.params.as_ref().expect("phone-extract must have params");
        match params {
            ExtractParams::PhonePrefix { allowed_prefixes } => {
                assert!(allowed_prefixes.is_empty());
            }
            _ => panic!("expected PhonePrefix, got {params:?}"),
        }
    }

    #[test]
    fn bankcard_extract_rule_has_luhn_params() {
        let r = BuiltinRules::get("bankcard-extract").unwrap();
        assert_eq!(r.id, "bankcard-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert!(r.pattern.is_some());
        // T55：params = Luhn
        let params = r
            .params
            .as_ref()
            .expect("bankcard-extract must have params");
        assert!(matches!(params, ExtractParams::Luhn));
    }

    #[test]
    fn ip4_extract_rule_has_ipv4_params() {
        let r = BuiltinRules::get("ip4-extract").unwrap();
        assert_eq!(r.id, "ip4-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert!(r.pattern.is_some());
        // T55b：params = Ipv4
        let params = r.params.as_ref().expect("ip4-extract must have params");
        assert!(matches!(params, ExtractParams::Ipv4));
    }

    #[test]
    fn ip6_extract_rule_has_ipv6_params() {
        let r = BuiltinRules::get("ip6-extract").unwrap();
        assert_eq!(r.id, "ip6-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert!(r.pattern.is_some());
        // T55b：params = Ipv6
        let params = r.params.as_ref().expect("ip6-extract must have params");
        assert!(matches!(params, ExtractParams::Ipv6));
    }

    #[test]
    fn idcard_extract_rule_has_idcard_params() {
        let r = BuiltinRules::get("idcard-extract").unwrap();
        assert_eq!(r.id, "idcard-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert_eq!(r.pattern.as_deref(), Some(r"\b[1-9]\d{16}[\dXx]\b"));
        // T55c：params = IdCard
        let params = r.params.as_ref().expect("idcard-extract must have params");
        assert!(matches!(params, ExtractParams::IdCard));
    }

    #[test]
    fn generic_validate_rule_default_params() {
        // v1.1.4 续轮 T70：generic-validate 默认参数
        let r = BuiltinRules::get("generic-validate").unwrap();
        assert_eq!(r.id, "generic-validate");
        assert_eq!(r.kind, RuleKind::Validate);
        assert!(r.pattern.is_none());
        assert!(r.template.is_none());
        let params = r
            .params
            .as_ref()
            .expect("generic-validate must have params");
        match params {
            ExtractParams::Generic {
                allow_digits,
                allow_letters,
                allow_special_chars,
                min_len,
                max_len,
            } => {
                assert!(*allow_digits, "default allow_digits = true");
                assert!(*allow_letters, "default allow_letters = true");
                assert!(
                    allow_special_chars.is_empty(),
                    "default allow_special_chars = empty"
                );
                assert_eq!(*min_len, None, "default min_len = None");
                assert_eq!(*max_len, None, "default max_len = None");
            }
            other => panic!("expected Generic, got {other:?}"),
        }
    }

    #[test]
    fn get_by_id_works() {
        let reg = RuleRegistry::with_defaults();
        assert!(reg.get("name-validate").is_some());
        assert!(reg.get("name-mask").is_some());
        assert!(reg.get("name-extract").is_some());
        assert!(reg.get("simple-mask").is_some());
        assert!(reg.get("segment-mask").is_some());
        // T55b：4 条新 extract 规则（phone / bankcard / ip4 / ip6）
        assert!(reg.get("phone-extract").is_some());
        assert!(reg.get("bankcard-extract").is_some());
        assert!(reg.get("ip4-extract").is_some());
        assert!(reg.get("ip6-extract").is_some());
        // T55c：idcard-extract
        assert!(reg.get("idcard-extract").is_some());
        // v1.1.4 T67：6 条新 validate 规则
        assert!(reg.get("username-validate").is_some());
        assert!(reg.get("sex-validate").is_some());
        assert!(reg.get("birth-validate").is_some());
        assert!(reg.get("idcard-validate").is_some());
        assert!(reg.get("phone-validate").is_some());
        assert!(reg.get("address-validate").is_some());
        // v1.1.4 续轮 T70：generic-validate
        assert!(reg.get("generic-validate").is_some());
        // v1.1.5 T81：email-validate
        assert!(reg.get("email-validate").is_some());
        // T55b：旧 ip-extract id 已不存在（拆分后由 cleanup_deprecated_rules 删除）
        assert!(reg.get("ip-extract").is_none());
        // T54：旧 general-mask id 已不存在（由 cleanup_deprecated_rules 删除）
        assert!(reg.get("general-mask").is_none());
        // 旧 id 已不存在
        assert!(reg.get("idcard-mask").is_none());
        assert!(reg.get("phone-mask").is_none());
        assert!(reg.get("birthdate-mask").is_none());
        assert!(reg.get("bankcard-mask").is_none());
        assert!(reg.get("nope").is_none());
    }

    #[test]
    fn toggle_disables_rule() {
        let mut reg = RuleRegistry::with_defaults();
        assert!(reg.get("name-validate").unwrap().enabled);
        reg.toggle("name-validate", false).unwrap();
        assert!(!reg.get("name-validate").unwrap().enabled);
    }

    #[test]
    fn toggle_unknown_returns_err() {
        let mut reg = RuleRegistry::with_defaults();
        assert!(reg.toggle("nope", true).is_err());
    }

    #[test]
    fn register_overwrites_same_id() {
        let mut reg = RuleRegistry::with_defaults();
        let mut r = BuiltinRules::get("name-validate").unwrap();
        r.description = "updated".into();
        reg.register(r);
        assert_eq!(reg.list().len(), 18);
        assert_eq!(reg.get("name-validate").unwrap().description, "updated");
    }

    #[test]
    fn rule_params_field_serde_skip_when_none() {
        // name-mask 无 params → JSON 不输出 params 字段（向后兼容）
        let r = BuiltinRules::get("name-mask").unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("params"));
    }

    #[test]
    fn rule_params_field_serde_present_when_some() {
        // phone-extract 有 params → JSON 输出 params 字段
        let r = BuiltinRules::get("phone-extract").unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("params"));
    }

    #[test]
    fn rule_params_field_deserialize_missing_as_none() {
        // 模拟 v1.1.4 旧 JSON（无 params 字段）→ 反序列化 params=None
        let old_json = r#"{"id":"name-mask","name":"姓名脱敏","kind":"mask","field":"name","pattern":null,"replacement":null,"enabled":true,"description":"保留首尾字符，中间以 * 替换"}"#;
        let r: Rule = serde_json::from_str(old_json).unwrap();
        assert_eq!(r.id, "name-mask");
        assert!(r.params.is_none());
    }

    #[test]
    fn rule_template_field_serde_skip_when_none() {
        // name-mask 无 template → JSON 不输出 template 字段（向后兼容 v1.1.2 前端）
        let r = BuiltinRules::get("name-mask").unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("template"));
    }

    #[test]
    fn rule_template_field_serde_present_when_some() {
        // simple-mask 有 template（空 Simple 模板）→ JSON 输出 template 字段
        let r = BuiltinRules::get("simple-mask").unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("template"));
    }

    #[test]
    fn rule_template_field_deserialize_missing_as_none() {
        // 模拟 v1.1.2 旧 JSON（无 template 字段）→ 反序列化 template=None
        let old_json = r#"{"id":"name-mask","name":"姓名脱敏","kind":"mask","field":"name","pattern":null,"replacement":null,"enabled":true,"description":"保留首尾字符，中间以 * 替换"}"#;
        let r: Rule = serde_json::from_str(old_json).unwrap();
        assert_eq!(r.id, "name-mask");
        assert!(r.template.is_none());
    }
}
