//! 规则管理基础结构。
//!
//! v1.1.0：`Rule` 结构 + `RuleRegistry` + 三条姓名相关内置规则
//! （脱敏 / 校验 / 提取各一条）。规则持久化到 DB（`rules` 表），
//! 启动时若 DB 无规则则 seed 这三条内置规则。
//! v1.1.3：`Rule` 新增 `template: Option<TemplateParams>` 字段（通用模板脱敏参数，
//! JSON 存储到 `rules.template` 列）。T49 子规则化：4 条原独立脱敏规则
//! （身份证 / 手机 / 出生日期 / 银行卡）收敛为**通用脱敏规则** `general-mask`
//! 的 4 个预设（`TemplateParams` 常量），不再单独 seed。`with_defaults()`
//! 共 4 条（3 name + general-mask）。
//!
//! `Rule` 经 serde camelCase 序列化与前端对齐：
//! ```json
//! { "id": "name-validate", "name": "姓名校验", "kind": "validate",
//!   "field": "name", "pattern": "^[\\u4e00-\\u9fa5]{2,4}$",
//!   "replacement": null, "enabled": true, "description": "..." }
//! ```
//! v1.1.3 `general-mask` 规则带**空模板**（`template` 为空 `TemplateParams`，
//! 所有字段 `None`）→ 前端选预设填充参数，不选则不脱敏（透传）。预设示例：
//! ```json
//! { "id": "general-mask", ..., "template": { "keepPrefix": 6, "keepSuffix": 4,
//!   "maskChar": "*", "maskMinLen": 8, "minLen": 18, "maxLen": 18 } }
//! ```

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// 规则类型。决定规则被哪个处理器消费。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleKind {
    /// 脱敏规则（被 `Masker` 消费）
    Mask,
    /// 校验规则（被 `Validator` 消费）
    Validate,
    /// 提取规则（前端测试时用 pattern 做正则提取预览）
    Extract,
}

impl std::fmt::Display for RuleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleKind::Mask => write!(f, "mask"),
            RuleKind::Validate => write!(f, "validate"),
            RuleKind::Extract => write!(f, "extract"),
        }
    }
}

impl RuleKind {
    /// 从 lowercase 字符串解析（DB 列反序列化用）。
    pub fn from_str_lowercase(s: &str) -> Option<Self> {
        match s {
            "mask" => Some(Self::Mask),
            "validate" => Some(Self::Validate),
            "extract" => Some(Self::Extract),
            _ => None,
        }
    }
}

/// 通用模板脱敏参数（v1.1.3 新增）。
///
/// 对齐 v0.8.0 `TemplateOp` 的「通用替换模版」语义：保留前 `keep_prefix`
/// 字符 + 后 `keep_suffix` 字符，中间替换为 `mask_char`（至少
/// `mask_min_len` 个）。`min_len` / `max_len` 为值总字符数的 guard——
/// 不在区间内原样返回。所有字段 `Option`，`None` 取语义默认值（0/0/*/1/None/None）。
///
/// DB 存为 `rules.template TEXT`（JSON 串），`row_to_rule` 读取后
/// `serde_json::from_str` 反序列化。`Rule.template == None` → 走 `SimpleMasker`
/// 旧逻辑（保留首尾各 1），向后兼容 v1.1.0~v1.1.2 的 name-mask 规则。
///
/// T49 子规则化：`general-mask` 规则持有**空模板**（`TemplateParams` 所有字段
/// `None`，即 `TemplateParams::default()`）→ `SimpleMasker` 视为不脱敏（透传，
/// 原样返回）。前端选预设（`idcard_preset()` 等）把参数填充到 `template`，
/// 再经 `mask_column` 的 `template` 临时参数覆盖，或经 `update_rule_template`
/// 持久化到 DB。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TemplateParams {
    /// 保留前缀字符数（默认 0）。
    pub keep_prefix: Option<usize>,
    /// 保留后缀字符数（默认 0）。
    pub keep_suffix: Option<usize>,
    /// 掩码字符（默认 `*`）。
    pub mask_char: Option<char>,
    /// 脱敏段至少插入多少个掩码字符（默认 1）。
    pub mask_min_len: Option<usize>,
    /// 值总字符数下限 guard（默认 None = 不限制）。
    pub min_len: Option<usize>,
    /// 值总字符数上限 guard（默认 None = 不限制）。
    pub max_len: Option<usize>,
}

impl TemplateParams {
    /// 构造一个 keep_prefix/keep_suffix/mask_min_len 都显式给定、其余默认的模板。
    pub fn new(keep_prefix: usize, keep_suffix: usize, mask_min_len: usize) -> Self {
        Self {
            keep_prefix: Some(keep_prefix),
            keep_suffix: Some(keep_suffix),
            mask_char: None,
            mask_min_len: Some(mask_min_len),
            min_len: None,
            max_len: None,
        }
    }

    /// 链式设置 mask_char。
    pub fn with_mask_char(mut self, c: char) -> Self {
        self.mask_char = Some(c);
        self
    }

    /// 链式设置 min_len / max_len guard（长度上下限，含端点）。
    pub fn with_len_range(mut self, min: usize, max: usize) -> Self {
        self.min_len = Some(min);
        self.max_len = Some(max);
        self
    }

    /// 是否为空模板（所有字段 `None`）。
    ///
    /// T49：`general-mask` 规则默认持空模板 → `SimpleMasker` 视为不脱敏（透传）。
    /// 前端选预设后填充字段 → `is_empty()` 返回 `false` → 走模板脱敏逻辑。
    pub fn is_empty(&self) -> bool {
        self.keep_prefix.is_none()
            && self.keep_suffix.is_none()
            && self.mask_char.is_none()
            && self.mask_min_len.is_none()
            && self.min_len.is_none()
            && self.max_len.is_none()
    }
}

// ---- T49 通用脱敏预设（general-mask 的子规则）----
//
// 4 条预设对应原 v1.1.3 独立规则（idcard/phone/birthdate/bankcard）的模板参数，
// 现作为 `general-mask` 的子规则供前端选择。前端选预设 → 填充 6 个可编辑参数框
// → 用户可继续修改 → 执行脱敏时把模板透传给 `mask_column`。
// 预设只是 `TemplateParams` 常量构造器，不再单独 seed 到 DB。

/// 身份证号预设：保留前 6 位地区码 + 后 4 位校验码，中间 8 位用 `*` 替换。
/// `min_len=max_len=18` guard → 非 18 位原样返回。
pub fn idcard_preset() -> TemplateParams {
    TemplateParams::new(6, 4, 8).with_len_range(18, 18)
}

/// 手机号预设：保留前 3 位 + 后 4 位，中间 4 位用 `*` 替换。
/// `min_len=max_len=11` guard → 非 11 位原样返回。
pub fn phone_preset() -> TemplateParams {
    TemplateParams::new(3, 4, 4).with_len_range(11, 11)
}

/// 出生日期预设：保留年份和月份（前 8 字符 `YYYY-MM-`），日期 2 位用 `*` 替换。
/// `min_len=max_len=10` guard → 非 10 位原样返回。例：`1990-01-15` → `1990-01-**`。
pub fn birthdate_preset() -> TemplateParams {
    TemplateParams::new(8, 0, 2).with_len_range(10, 10)
}

/// 银行卡号预设：保留前 4 位 + 后 4 位，中间位数用 `*` 替换。
/// 不设长度 guard（银行卡号长度 15~19 位不等）。例：`6222021234567890123` → `6222***********0123`。
pub fn bankcard_preset() -> TemplateParams {
    TemplateParams::new(4, 4, 1)
}

/// 规则定义。前后端经 serde camelCase 序列化统一。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// 规则 ID（唯一标识，如 `"name-validate"`）。
    pub id: String,
    /// 规则显示名（如 `"姓名校验"`）。
    pub name: String,
    /// 规则类型。
    pub kind: RuleKind,
    /// 目标列名（如 `"name"`）。`None` 表示不限定列。
    pub field: Option<String>,
    /// 正则模式串（校验/提取用，脱敏可选）。`None` 表示不使用正则。
    pub pattern: Option<String>,
    /// 脱敏掩码字符（取首个字符；`None`/空 → 默认 `*`）。
    /// 由 `SimpleMasker` 在「保留首字符 + 其余用掩码字符替换」逻辑中消费，
    /// 也可临时覆盖模板的 `mask_char`（优先级：replacement > template.mask_char > 默认 `*`）。
    pub replacement: Option<String>,
    /// 启用/禁用。
    pub enabled: bool,
    /// 规则说明。
    pub description: String,
    /// 通用模板脱敏参数（v1.1.3 新增）。`None` → 走 `SimpleMasker` 旧逻辑
    /// （保留首尾各 1），向后兼容 v1.1.0~v1.1.2 的 name-mask 规则。
    /// T49：`general-mask` 规则持空模板（`TemplateParams::default()`，
    /// 所有字段 `None`）→ `SimpleMasker` 视为不脱敏（透传）。前端选预设后
    /// 填充字段 → 走模板脱敏。DB 存为 `template TEXT`（JSON 串）。
    /// `#[serde(default, skip_serializing_if = "Option::is_none")]` 保证旧 Rule JSON
    /// （无 template 字段）反序列化时 `template=None`，且序列化时不输出空字段，
    /// 前端拿到的 name-mask 规则 JSON 形态不变。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateParams>,
}

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
    /// 收敛为**通用脱敏规则** `general-mask` 的 4 个预设（`idcard_preset()` 等，
    /// 不再单独 seed）。`with_defaults()` 共 4 条（3 name + general-mask）。
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Self::name_validate_rule());
        reg.register(Self::name_mask_rule());
        reg.register(Self::name_extract_rule());
        // v1.1.3 T49 通用脱敏规则（持空模板，前端选预设填充参数）
        reg.register(Self::general_mask_rule());
        reg
    }

    /// 姓名校验内置规则：2-4 位中文字符。
    pub fn name_validate_rule() -> Rule {
        Rule {
            id: "name-validate".into(),
            name: "姓名校验".into(),
            kind: RuleKind::Validate,
            field: Some("name".into()),
            pattern: Some(r"^[\u4e00-\u9fa5]{2,4}$".into()),
            replacement: None,
            enabled: true,
            description: "校验姓名为 2-4 位中文字符".into(),
            template: None,
        }
    }

    /// 姓名脱敏内置规则：≥3 字符保留首尾，中间 `*` 替换；2 字符保留首字符末位 `*`。
    /// `replacement=None` → `SimpleMasker` 默认掩码字符 `*`。
    /// 无 `template` → 走 `SimpleMasker` 旧逻辑（保留首尾各 1）。
    pub fn name_mask_rule() -> Rule {
        Rule {
            id: "name-mask".into(),
            name: "姓名脱敏".into(),
            kind: RuleKind::Mask,
            field: Some("name".into()),
            pattern: None,
            replacement: None,
            enabled: true,
            description: "保留首尾字符，中间以 * 替换".into(),
            template: None,
        }
    }

    /// 姓名提取内置规则：2-4 位连续中文字符。
    /// 前端测试时用此 pattern 做正则提取预览。
    pub fn name_extract_rule() -> Rule {
        Rule {
            id: "name-extract".into(),
            name: "姓名提取".into(),
            kind: RuleKind::Extract,
            field: Some("name".into()),
            pattern: Some(r"[\u4e00-\u9fa5]{2,4}".into()),
            replacement: None,
            enabled: true,
            description: "提取姓名候选：2-4 位连续中文字符".into(),
            template: None,
        }
    }

    /// 通用脱敏内置规则（v1.1.3 T49）：持**空模板**（`TemplateParams::default()`，
    /// 所有字段 `None`）→ `SimpleMasker` 视为不脱敏（透传，原样返回）。
    ///
    /// 前端选预设（身份证 / 手机 / 出生日期 / 银行卡 / 自定义）→ 填充 6 个可编辑
    /// 参数框 → 执行脱敏时把模板透传给 `mask_column(template=...)` 临时覆盖，
    /// 或经 `update_rule_template` 持久化到 DB。不选预设 → 空模板 → 不脱敏。
    ///
    /// 取代 v1.1.3 原先的 4 条独立规则（`idcard-mask`/`phone-mask`/
    /// `birthdate-mask`/`bankcard-mask`），改为 `general-mask` 的 4 个预设
    /// （`idcard_preset()` 等）。
    pub fn general_mask_rule() -> Rule {
        Rule {
            id: "general-mask".into(),
            name: "通用脱敏".into(),
            kind: RuleKind::Mask,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "通用模板脱敏：选预设（身份证/手机/出生日期/银行卡）或自定义参数".into(),
            template: Some(TemplateParams::default()),
        }
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

    #[test]
    fn with_defaults_loads_four_rules() {
        let reg = RuleRegistry::with_defaults();
        let rules = reg.list();
        // v1.1.3 T49：3 条姓名 + 1 条通用脱敏 = 4 条
        assert_eq!(rules.len(), 4);
        // 脱敏 / 校验 / 提取 三种 kind 都存在
        let kinds: Vec<RuleKind> = rules.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RuleKind::Mask));
        assert!(kinds.contains(&RuleKind::Validate));
        assert!(kinds.contains(&RuleKind::Extract));
        // 2 条 mask 规则（name-mask + general-mask）
        let mask_count = kinds.iter().filter(|k| **k == RuleKind::Mask).count();
        assert_eq!(mask_count, 2);
    }

    #[test]
    fn name_validate_rule_pattern_is_chinese_range() {
        let r = RuleRegistry::name_validate_rule();
        assert_eq!(r.id, "name-validate");
        assert_eq!(r.kind, RuleKind::Validate);
        assert_eq!(r.field.as_deref(), Some("name"));
        assert!(r.pattern.is_some());
        assert!(r.enabled);
        // name 规则无 template
        assert!(r.template.is_none());
    }

    #[test]
    fn name_mask_rule_keeps_first_char_semantic() {
        let r = RuleRegistry::name_mask_rule();
        assert_eq!(r.id, "name-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        assert_eq!(r.field.as_deref(), Some("name"));
        // replacement=None → SimpleMasker 默认掩码字符 `*`
        assert!(r.replacement.is_none());
        // 无 template → 走 SimpleMasker 旧逻辑
        assert!(r.template.is_none());
    }

    #[test]
    fn name_extract_rule_has_chinese_pattern() {
        let r = RuleRegistry::name_extract_rule();
        assert_eq!(r.id, "name-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert_eq!(r.field.as_deref(), Some("name"));
        assert!(r.pattern.is_some());
    }

    #[test]
    fn general_mask_rule_has_empty_template() {
        let r = RuleRegistry::general_mask_rule();
        assert_eq!(r.id, "general-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        // 持空模板（TemplateParams::default()）→ is_empty()==true
        let tpl = r.template.expect("general-mask must have template");
        assert!(tpl.is_empty());
    }

    #[test]
    fn idcard_preset_params() {
        let tpl = idcard_preset();
        assert_eq!(tpl.keep_prefix, Some(6));
        assert_eq!(tpl.keep_suffix, Some(4));
        assert_eq!(tpl.mask_min_len, Some(8));
        assert_eq!(tpl.min_len, Some(18));
        assert_eq!(tpl.max_len, Some(18));
        assert!(!tpl.is_empty());
    }

    #[test]
    fn phone_preset_params() {
        let tpl = phone_preset();
        assert_eq!(tpl.keep_prefix, Some(3));
        assert_eq!(tpl.keep_suffix, Some(4));
        assert_eq!(tpl.mask_min_len, Some(4));
        assert_eq!(tpl.min_len, Some(11));
        assert_eq!(tpl.max_len, Some(11));
    }

    #[test]
    fn birthdate_preset_params() {
        let tpl = birthdate_preset();
        assert_eq!(tpl.keep_prefix, Some(8));
        assert_eq!(tpl.keep_suffix, Some(0));
        assert_eq!(tpl.mask_min_len, Some(2));
        assert_eq!(tpl.min_len, Some(10));
        assert_eq!(tpl.max_len, Some(10));
    }

    #[test]
    fn bankcard_preset_params() {
        let tpl = bankcard_preset();
        assert_eq!(tpl.keep_prefix, Some(4));
        assert_eq!(tpl.keep_suffix, Some(4));
        assert_eq!(tpl.mask_min_len, Some(1));
        // 银行卡不设长度 guard（15~19 位不等）
        assert_eq!(tpl.min_len, None);
        assert_eq!(tpl.max_len, None);
    }

    #[test]
    fn template_params_serde_roundtrip() {
        let tpl = idcard_preset();
        let json = serde_json::to_string(&tpl).unwrap();
        // camelCase 序列化
        assert!(json.contains("keepPrefix"));
        assert!(json.contains("keepSuffix"));
        assert!(json.contains("maskMinLen"));
        assert!(json.contains("minLen"));
        assert!(json.contains("maxLen"));
        let back: TemplateParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tpl);
    }

    #[test]
    fn rule_template_field_serde_skip_when_none() {
        // name-mask 无 template → JSON 不输出 template 字段（向后兼容 v1.1.2 前端）
        let r = RuleRegistry::name_mask_rule();
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("template"));
    }

    #[test]
    fn rule_template_field_serde_present_when_some() {
        // general-mask 有 template（空模板）→ JSON 输出 template 字段
        let r = RuleRegistry::general_mask_rule();
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

    #[test]
    fn get_by_id_works() {
        let reg = RuleRegistry::with_defaults();
        assert!(reg.get("name-validate").is_some());
        assert!(reg.get("name-mask").is_some());
        assert!(reg.get("name-extract").is_some());
        assert!(reg.get("general-mask").is_some());
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
        let mut r = RuleRegistry::name_validate_rule();
        r.description = "updated".into();
        reg.register(r);
        assert_eq!(reg.list().len(), 4);
        assert_eq!(reg.get("name-validate").unwrap().description, "updated");
    }

    #[test]
    fn rulekind_serializes_lowercase() {
        let json = serde_json::to_string(&RuleKind::Validate).unwrap();
        assert_eq!(json, "\"validate\"");
        let m: RuleKind = serde_json::from_str("\"mask\"").unwrap();
        assert_eq!(m, RuleKind::Mask);
    }

    #[test]
    fn rulekind_from_str_lowercase_round_trip() {
        for k in [RuleKind::Mask, RuleKind::Validate, RuleKind::Extract] {
            let s = k.to_string();
            assert_eq!(RuleKind::from_str_lowercase(&s), Some(k));
        }
        assert_eq!(RuleKind::from_str_lowercase("nope"), None);
    }
}
