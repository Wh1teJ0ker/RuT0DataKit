//! 规则管理基础结构。
//!
//! v1.1.0：`Rule` 结构 + `RuleRegistry` + 三条姓名相关内置规则
//! （脱敏 / 校验 / 提取各一条）。规则持久化到 DB（`rules` 表），
//! 启动时若 DB 无规则则 seed 这三条内置规则。
//!
//! `Rule` 经 serde camelCase 序列化与前端对齐：
//! ```json
//! { "id": "name-validate", "name": "姓名校验", "kind": "validate",
//!   "field": "name", "pattern": "^[\\u4e00-\\u9fa5]{2,4}$",
//!   "replacement": null, "enabled": true, "description": "..." }
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
    /// 由 `SimpleMasker` 在「保留首字符 + 其余用掩码字符替换」逻辑中消费。
    pub replacement: Option<String>,
    /// 启用/禁用。
    pub enabled: bool,
    /// 规则说明。
    pub description: String,
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

    /// 加载内置规则集。v1.1.0 三条姓名相关规则（脱敏/校验/提取各一条）。
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Self::name_validate_rule());
        reg.register(Self::name_mask_rule());
        reg.register(Self::name_extract_rule());
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
        }
    }

    /// 姓名脱敏内置规则：≥3 字符保留首尾，中间 `*` 替换；2 字符保留首字符末位 `*`。
    /// `replacement=None` → `SimpleMasker` 默认掩码字符 `*`。
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
    fn with_defaults_loads_three_name_rules() {
        let reg = RuleRegistry::with_defaults();
        let rules = reg.list();
        assert_eq!(rules.len(), 3);
        // 脱敏 / 校验 / 提取 各一条
        let kinds: Vec<RuleKind> = rules.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RuleKind::Mask));
        assert!(kinds.contains(&RuleKind::Validate));
        assert!(kinds.contains(&RuleKind::Extract));
    }

    #[test]
    fn name_validate_rule_pattern_is_chinese_range() {
        let r = RuleRegistry::name_validate_rule();
        assert_eq!(r.id, "name-validate");
        assert_eq!(r.kind, RuleKind::Validate);
        assert_eq!(r.field.as_deref(), Some("name"));
        assert!(r.pattern.is_some());
        assert!(r.enabled);
    }

    #[test]
    fn name_mask_rule_keeps_first_char_semantic() {
        let r = RuleRegistry::name_mask_rule();
        assert_eq!(r.id, "name-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        assert_eq!(r.field.as_deref(), Some("name"));
        // replacement=None → SimpleMasker 默认掩码字符 `*`
        assert!(r.replacement.is_none());
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
    fn get_by_id_works() {
        let reg = RuleRegistry::with_defaults();
        assert!(reg.get("name-validate").is_some());
        assert!(reg.get("name-mask").is_some());
        assert!(reg.get("name-extract").is_some());
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
        assert_eq!(reg.list().len(), 3);
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
