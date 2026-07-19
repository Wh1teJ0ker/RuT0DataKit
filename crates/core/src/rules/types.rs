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
}

/// 规则集合：从单份 YAML 反序列化得到的所有校验 + 脱敏规则。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuleSet {
    #[serde(default)]
    pub validators: Vec<FieldRule>,
    #[serde(default)]
    pub maskers: Vec<MaskRule>,
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
}
