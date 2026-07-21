//! YAML 规则结构：`RuleSet` / `FieldRule` / `MaskRule`。
//!
//! 全部由 serde 从规则 YAML 反序列化得到，字段对齐
//! `docs/02-技术设计文档.md` §2.1。`params` 为弱类型映射，由具体
//! validator / masker 在运行期解释。

use std::collections::HashMap;

use serde::Deserialize;
use serde_yml::Value;

/// 一条字段校验规则。
#[derive(Debug, Clone, Deserialize)]
pub struct FieldRule {
    /// 字段名（CSV/XLSX 列名）。
    pub field: String,
    /// 校验器名，查 `ValidatorRegistry`。当 `regex` 存在时由 loader 改用
    /// `RegexValidator`，本字段仅作 fallback 标识。
    pub validator: String,
    /// 透传给具体校验器的参数。
    #[serde(default)]
    pub params: Option<HashMap<String, Value>>,
    /// YAML 内联正则兜底校验；存在时优先于 `validator` 名。
    #[serde(default)]
    pub regex: Option<String>,
    /// 校验失败时使用的自定义错误消息（覆盖默认）。
    #[serde(default)]
    pub message: Option<String>,
    /// 规则说明，仅文档用途，不影响运行行为。
    #[serde(default)]
    pub description: Option<String>,
    /// 规则标签，供 `RuleSet::by_tag` 分组过滤使用（如 validate / sensitive）。
    ///
    /// 旧 YAML 不含 tags 字段时默认为空 Vec，向后兼容。
    #[serde(default)]
    pub tags: Vec<String>,
}

/// 一条字段脱敏规则。
#[derive(Debug, Clone, Deserialize)]
pub struct MaskRule {
    /// 字段名。
    pub field: String,
    /// 脱敏器名，查 `MaskerRegistry`。
    pub masker: String,
    /// 透传给具体脱敏器的参数。
    #[serde(default)]
    pub params: Option<HashMap<String, Value>>,
    /// 规则说明，仅文档用途，不影响运行行为。
    #[serde(default)]
    pub description: Option<String>,
    /// 规则标签，供 `RuleSet::by_tag_mask` 分组过滤使用（如 mask / sensitive）。
    ///
    /// 旧 YAML 不含 tags 字段时默认为空 Vec，向后兼容。
    #[serde(default)]
    pub tags: Vec<String>,
}

/// 规则集合：从单份 YAML 反序列化得到的所有校验 + 脱敏规则。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuleSet {
    #[serde(default)]
    pub validators: Vec<FieldRule>,
    #[serde(default)]
    pub maskers: Vec<MaskRule>,
}

impl RuleSet {
    /// 返回 `validators` 中 `tags` 包含 `tag` 的校验规则引用。
    ///
    /// 大小写敏感；`tags` 为空时返回空切片。用于按标签（validate / sensitive /
    /// search / sql_parse）筛选规则集合。
    pub fn by_tag(&self, tag: &str) -> Vec<&FieldRule> {
        self.validators
            .iter()
            .filter(|r| r.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// 返回 `maskers` 中 `tags` 包含 `tag` 的脱敏规则引用。
    ///
    /// 语义同 [`RuleSet::by_tag`]，作用于脱敏侧。
    pub fn by_tag_mask(&self, tag: &str) -> Vec<&MaskRule> {
        self.maskers
            .iter()
            .filter(|r| r.tags.iter().any(|t| t == tag))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_ruleset() {
        let yaml = r#"
validators:
  - field: id
    validator: idcard
maskers:
  - field: phone
    masker: phone_mask
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        assert_eq!(rs.validators.len(), 1);
        assert_eq!(rs.validators[0].field, "id");
        assert_eq!(rs.validators[0].validator, "idcard");
        assert!(rs.validators[0].params.is_none());
        assert!(rs.validators[0].regex.is_none());
        assert!(rs.validators[0].message.is_none());
        assert_eq!(rs.maskers.len(), 1);
        assert_eq!(rs.maskers[0].masker, "phone_mask");
    }

    #[test]
    fn parses_full_field_rule() {
        let yaml = r#"
validators:
  - field: phone
    validator: phone
    params:
      prefix_set: ctf
    regex: "^1\\d{10}$"
    message: "invalid phone"
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        let r = &rs.validators[0];
        assert_eq!(r.field, "phone");
        assert_eq!(r.validator, "phone");
        assert_eq!(r.regex.as_deref(), Some("^1\\d{10}$"));
        assert_eq!(r.message.as_deref(), Some("invalid phone"));
        let params = r.params.as_ref().unwrap();
        assert_eq!(params.get("prefix_set").unwrap(), &Value::String("ctf".into()));
    }

    #[test]
    fn parses_empty_ruleset() {
        let rs: RuleSet = serde_yml::from_str("").unwrap();
        assert!(rs.validators.is_empty());
        assert!(rs.maskers.is_empty());
    }

    #[test]
    fn parses_ruleset_with_tags() {
        let yaml = r#"
validators:
  - field: id
    validator: regex
    tags: [validate, sensitive]
maskers:
  - field: id
    masker: template
    tags: [mask, sensitive]
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        assert_eq!(rs.validators[0].tags, vec!["validate", "sensitive"]);
        assert_eq!(rs.maskers[0].tags, vec!["mask", "sensitive"]);
    }

    #[test]
    fn old_yaml_without_tags_defaults_to_empty_vec() {
        // 兼容性：旧 YAML 不含 tags 字段，反序列化成功且 tags 为空 Vec。
        let yaml = r#"
validators:
  - field: id
    validator: idcard
maskers:
  - field: phone
    masker: phone_mask
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        assert!(rs.validators[0].tags.is_empty());
        assert!(rs.maskers[0].tags.is_empty());
    }

    #[test]
    fn by_tag_filters_validators() {
        let yaml = r#"
validators:
  - field: id
    validator: regex
    tags: [validate, sensitive]
  - field: name
    validator: regex
    tags: [validate]
  - field: phone
    validator: regex
    tags: [sensitive]
maskers: []
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        let validate_hits: Vec<_> = rs.by_tag("validate").into_iter().map(|r| r.field.as_str()).collect();
        assert_eq!(validate_hits, ["id", "name"]);
        let sensitive_hits: Vec<_> = rs.by_tag("sensitive").into_iter().map(|r| r.field.as_str()).collect();
        assert_eq!(sensitive_hits, ["id", "phone"]);
        assert!(rs.by_tag("nonexistent").is_empty());
    }

    #[test]
    fn by_tag_mask_filters_maskers() {
        let yaml = r#"
validators: []
maskers:
  - field: id
    masker: template
    tags: [mask, sensitive]
  - field: phone
    masker: template
    tags: [mask]
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        let mask_hits: Vec<_> = rs.by_tag_mask("mask").into_iter().map(|r| r.field.as_str()).collect();
        assert_eq!(mask_hits, ["id", "phone"]);
        let sensitive_hits: Vec<_> = rs.by_tag_mask("sensitive").into_iter().map(|r| r.field.as_str()).collect();
        assert_eq!(sensitive_hits, ["id"]);
        assert!(rs.by_tag_mask("nonexistent").is_empty());
    }
}
