//! 规则加载入口。
//!
//! 对外暴露 `load_ruleset(path)` 与 `load_ruleset_str(&str)`。两者都将 YAML
//! 反序列化为 [`RuleSet`]。本模块不直接实例化 validator / masker -- 仅负责
//! 解析结构。按 `rule.scope` 做正则兜底、按 scope 名查注册表构造实例的
//! dispatch 由调用方（pipeline / scan）完成。
//!
//! v0.4.4 重构：删除 `load_default_mask_ruleset` + `DEFAULT_MASK_YAML`
//! include_str!（内置规则集已清空，规则池初始为空，后续版本逐步接入）。

use std::path::Path;

use crate::error::CoreError;
use crate::rules::types::RuleSet;

/// 从 YAML 文件加载 [`RuleSet`]。
///
/// 文件不存在 / IO 失败 -> `CoreError::Io`；YAML 语法错误 -> `CoreError::Yaml`。
pub fn load_ruleset(path: impl AsRef<Path>) -> Result<RuleSet, CoreError> {
    let text = std::fs::read_to_string(path.as_ref())?;
    load_ruleset_str(&text)
}

/// 从 YAML 字符串加载 [`RuleSet`]。
pub fn load_ruleset_str(text: &str) -> Result<RuleSet, CoreError> {
    let rs: RuleSet = serde_yml::from_str(text)
        .map_err(|e| CoreError::Yaml(e.to_string()))?;
    Ok(rs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_str_minimal() {
        // v0.4.4：FieldRule/MaskRule 字段为 field/scope/tag/params/message/description。
        let yaml = r#"
validators:
  - field: id
    scope: idcard
    tag: validate
    params:
      pattern: "^\\d{17}[0-9Xx]$"
maskers:
  - field: phone
    scope: phone
    tag: mask
    params:
      keep_prefix: 3
      keep_suffix: 4
      mask_char: "*"
      mask_min_len: 4
      min_len: 11
      max_len: 11
      cjk: false
"#;
        let rs = load_ruleset_str(yaml).unwrap();
        assert_eq!(rs.validators.len(), 1);
        assert_eq!(rs.validators[0].scope, "idcard");
        assert_eq!(rs.validators[0].tag, "validate");
        assert_eq!(rs.maskers.len(), 1);
        assert_eq!(rs.maskers[0].scope, "phone");
        assert_eq!(rs.maskers[0].tag, "mask");
    }

    #[test]
    fn load_str_with_message() {
        // v0.4.4：rule 携带 message 字段，loader 仅解析结构，不做 dispatch。
        let yaml = r#"
validators:
  - field: phone
    scope: phone
    tag: validate
    message: "bad phone"
"#;
        let rs = load_ruleset_str(yaml).unwrap();
        let r = &rs.validators[0];
        assert_eq!(r.field, "phone");
        assert_eq!(r.scope, "phone");
        assert_eq!(r.message.as_deref(), Some("bad phone"));
    }

    #[test]
    fn load_nonexistent_file_returns_err() {
        let res = load_ruleset("/nonexistent/RuT0DataKit/missing_rules.yml");
        assert!(res.is_err());
        match res {
            Err(CoreError::Io(_)) => {}
            other => panic!("expected Io error, got {other:?}"),
        }
    }

    #[test]
    fn load_malformed_yaml_returns_err() {
        let res = load_ruleset_str("validators: [unclosed");
        assert!(res.is_err());
        match res {
            Err(CoreError::Yaml(_)) => {}
            other => panic!("expected Yaml error, got {other:?}"),
        }
    }

    #[test]
    fn load_empty_ruleset() {
        // v0.4.4：空 YAML -> 空 RuleSet（规则池初始为空）。
        let rs = load_ruleset_str("").unwrap();
        assert!(rs.validators.is_empty());
        assert!(rs.maskers.is_empty());
    }
}
