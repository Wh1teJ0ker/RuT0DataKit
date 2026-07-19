//! 规则加载入口。
//!
//! 对外暴露 `load_ruleset(path)` 与 `load_ruleset_str(&str)`。两者都将 YAML
//! 反序列化为 [`RuleSet`]。本模块不直接实例化 validator / masker —— 仅负责
//! 解析结构。按 [`FieldRule::regex`] 做正则兜底、按 validator/masker 名查
//! 注册表构造实例的 dispatch 由调用方（pipeline / scan）完成。
//!
//! 之所以不在 loader 层做 dispatch：注册表实例化需要先由调用方注入
//! `ValidatorRegistry` / `MaskerRegistry`（T0-3/T0-4 才填充内置项），loader
//! 保持无状态，职责单一。

use std::path::Path;

use crate::error::CoreError;
use crate::rules::types::RuleSet;

/// 从 YAML 文件加载 [`RuleSet`]。
///
/// 文件不存在 / IO 失败 → `CoreError::Io`；YAML 语法错误 → `CoreError::Yaml`。
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

/// 加载内置 `default_mask.yaml` 规则集。
///
/// 内容来自 `rules/default_mask.yaml`，通过 `include_str!` 嵌入二进制，
/// 避免运行时定位文件。v0.1.0 默认覆盖 `sample_mask.csv` 全部字段。
pub fn load_default_mask_ruleset() -> Result<RuleSet, CoreError> {
    load_ruleset_str(crate::rules::DEFAULT_MASK_YAML)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_str_minimal() {
        let yaml = r#"
validators:
  - field: id
    validator: regex
    params:
      pattern: "^\\d{17}[0-9Xx]$"
maskers:
  - field: phone
    masker: template
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
        assert_eq!(rs.maskers.len(), 1);
    }

    #[test]
    fn load_str_with_regex_fallback() {
        // rule 携带 regex 字段，loader 仅解析结构，不做 dispatch。
        let yaml = r#"
validators:
  - field: phone
    validator: phone
    regex: "^1\\d{10}$"
    message: "bad phone"
"#;
        let rs = load_ruleset_str(yaml).unwrap();
        let r = &rs.validators[0];
        assert_eq!(r.field, "phone");
        assert!(r.regex.is_some());
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
    fn load_builtin_default_mask_yaml() {
        // v0.1.0 重构后：default_mask.yaml 全部使用通用算子（template /
        // split_template）+ 显式 params，不再有预置别名。
        let rs = crate::rules::load_default_mask_ruleset().expect("default_mask.yaml");
        assert_eq!(rs.maskers.len(), 6);
        let fields: Vec<_> = rs.maskers.iter().map(|m| m.field.as_str()).collect();
        assert_eq!(
            fields,
            ["customer_id", "name", "id_card", "phone", "email", "bank_card"]
        );
        let names: Vec<_> = rs.maskers.iter().map(|m| m.masker.as_str()).collect();
        assert_eq!(
            names,
            [
                "template",
                "template",
                "template",
                "template",
                "split_template",
                "template"
            ]
        );
        // 每条规则都带显式 params（不再依赖预置别名默认值）。
        for m in &rs.maskers {
            assert!(m.params.is_some(), "field {} should carry explicit params", m.field);
        }
    }
}
