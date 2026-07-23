//! YAML 规则结构：`RuleSet` / `FieldRule` / `MaskRule`。
//!
//! v0.4.4 重构：删除 `validator` / `masker` / `regex` / `tags: Vec<String>`
//! 字段，改为 `scope`（数据类型，如 idcard/phone/bankcard）+ `tag`（单选
//! 用途："extract" | "mask" | "validate"）。`params` 为弱类型映射，由具体
//! 算子在运行期解释。`scope` 同时作为 ValidatorRegistry 查键 + scan
//! extract_pattern 查键，统一了原 validator/masker 名语义。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_yml::Value;

/// 一条字段校验/提取规则。
///
/// v0.4.4：`scope` 取代旧 `validator` 字段，作为 `ValidatorRegistry::get`
/// 的查键 + `scan::extract_pattern` 的查键。`tag` 单选用途（"extract" /
/// "validate"），不再用 `tags: Vec<String>` 多选。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldRule {
    /// 字段名（CSV/XLSX 列名）。
    pub field: String,
    /// 数据类型（scope）：idcard / phone / bankcard / email / ip / mac /
    /// username / name / custom。作为 ValidatorRegistry 查键 + scan
    /// extract_pattern 查键。
    pub scope: String,
    /// 单选用途标签："extract"（数据提取）| "validate"（数据校验）。
    pub tag: String,
    /// 透传给具体算子的参数。
    #[serde(default)]
    pub params: Option<HashMap<String, Value>>,
    /// 校验失败时使用的自定义错误消息（覆盖默认）。
    #[serde(default)]
    pub message: Option<String>,
    /// 规则说明，仅文档用途，不影响运行行为。
    #[serde(default)]
    pub description: Option<String>,
}

/// 一条字段脱敏规则。
///
/// v0.4.4：`scope` 取代旧 `masker` 字段，作为算子构造的查键。`tag` 单选
/// 用途固定为 "mask"。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaskRule {
    /// 字段名。
    pub field: String,
    /// 数据类型（scope）：idcard / phone / bankcard / email / ip / mac /
    /// username / name / custom。作为 MaskOp 算子构造查键。
    pub scope: String,
    /// 单选用途标签：固定为 "mask"（数据脱敏）。
    pub tag: String,
    /// 透传给具体脱敏算子的参数。
    #[serde(default)]
    pub params: Option<HashMap<String, Value>>,
    /// 校验失败时使用的自定义错误消息（覆盖默认）。
    #[serde(default)]
    pub message: Option<String>,
    /// 规则说明，仅文档用途，不影响运行行为。
    #[serde(default)]
    pub description: Option<String>,
}

/// 规则集合：从单份 YAML 反序列化得到的所有校验 + 脱敏规则。
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RuleSet {
    #[serde(default)]
    pub validators: Vec<FieldRule>,
    #[serde(default)]
    pub maskers: Vec<MaskRule>,
}

impl RuleSet {
    /// 返回 `validators` 中 `tag` 等于给定值的校验规则引用。
    ///
    /// v0.4.4：`tag` 单值匹配（不再 `tags.contains`）。用于按用途
    /// （"extract" / "validate"）筛选规则集合。
    pub fn by_tag(&self, tag: &str) -> Vec<&FieldRule> {
        self.validators
            .iter()
            .filter(|r| r.tag == tag)
            .collect()
    }

    /// 返回 `maskers` 中 `tag` 等于给定值的脱敏规则引用。
    ///
    /// 语义同 [`RuleSet::by_tag`]，作用于脱敏侧。
    pub fn by_tag_mask(&self, tag: &str) -> Vec<&MaskRule> {
        self.maskers
            .iter()
            .filter(|r| r.tag == tag)
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
    scope: idcard
    tag: validate
maskers:
  - field: phone
    scope: phone
    tag: mask
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        assert_eq!(rs.validators.len(), 1);
        assert_eq!(rs.validators[0].field, "id");
        assert_eq!(rs.validators[0].scope, "idcard");
        assert_eq!(rs.validators[0].tag, "validate");
        assert!(rs.validators[0].params.is_none());
        assert!(rs.validators[0].message.is_none());
        assert_eq!(rs.maskers.len(), 1);
        assert_eq!(rs.maskers[0].scope, "phone");
        assert_eq!(rs.maskers[0].tag, "mask");
    }

    #[test]
    fn parses_full_field_rule() {
        let yaml = r#"
validators:
  - field: phone
    scope: phone
    tag: validate
    params:
      prefix_set: ctf
    message: "invalid phone"
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        let r = &rs.validators[0];
        assert_eq!(r.field, "phone");
        assert_eq!(r.scope, "phone");
        assert_eq!(r.tag, "validate");
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
    fn parses_ruleset_with_tag() {
        // v0.4.4：tag 是单值字符串，不再用 tags 数组。
        let yaml = r#"
validators:
  - field: id
    scope: idcard
    tag: validate
maskers:
  - field: id
    scope: idcard
    tag: mask
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        assert_eq!(rs.validators[0].tag, "validate");
        assert_eq!(rs.maskers[0].tag, "mask");
    }

    #[test]
    fn old_yaml_without_tag_fails_deserialize() {
        // v0.4.4：tag 是必填字段（无默认值），旧 YAML 不含 tag 会反序列化失败。
        // 这是预期的破坏性变更，因规则池初始为空且无导入入口，无实际影响。
        let yaml = r#"
validators:
  - field: id
    scope: idcard
"#;
        let res: Result<RuleSet, _> = serde_yml::from_str(yaml);
        assert!(res.is_err(), "tag 缺失应反序列化失败");
    }

    #[test]
    fn by_tag_filters_validators() {
        let yaml = r#"
validators:
  - field: id
    scope: idcard
    tag: validate
  - field: phone
    scope: phone
    tag: extract
  - field: email
    scope: email
    tag: validate
maskers: []
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        let validate_hits: Vec<_> = rs.by_tag("validate").into_iter().map(|r| r.field.as_str()).collect();
        assert_eq!(validate_hits, ["id", "email"]);
        let extract_hits: Vec<_> = rs.by_tag("extract").into_iter().map(|r| r.field.as_str()).collect();
        assert_eq!(extract_hits, ["phone"]);
        assert!(rs.by_tag("nonexistent").is_empty());
    }

    #[test]
    fn by_tag_mask_filters_maskers() {
        let yaml = r#"
validators: []
maskers:
  - field: id
    scope: idcard
    tag: mask
  - field: phone
    scope: phone
    tag: mask
"#;
        let rs: RuleSet = serde_yml::from_str(yaml).unwrap();
        let mask_hits: Vec<_> = rs.by_tag_mask("mask").into_iter().map(|r| r.field.as_str()).collect();
        assert_eq!(mask_hits, ["id", "phone"]);
        assert!(rs.by_tag_mask("validate").is_empty());
        assert!(rs.by_tag_mask("nonexistent").is_empty());
    }
}
