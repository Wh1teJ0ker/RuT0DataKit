//! 签名规则加载器：编译期嵌入内置 YAML + 运行时 `serde_yml` 反序列化。
//!
//! 不引入新依赖：复用 core 已依赖的 `serde_yml`。
//!
//! - [`load_builtin_signatures`]：返回内置 6 条 SQLi 签名规则。
//! - [`load_signatures_from_str`]：从 YAML 字符串解析为 `Vec<SignatureRule>`，
//!   供 T2-5 GUI 编辑后重新装载。

use crate::error::CoreError;
use crate::logsign::SignatureRule;

/// 内置 SQLi 签名 YAML（编译期嵌入，运行时无需找文件）。
pub const BUILTIN_YAML: &str = include_str!("sqli_signatures.yaml");

/// 加载内置 SQLi 签名规则（6 条）。
///
/// YAML 本身在编译期已嵌入，运行时只做反序列化；返回顺序与 YAML 列出顺序一致。
pub fn load_builtin_signatures() -> Vec<SignatureRule> {
    load_signatures_from_str(BUILTIN_YAML).expect("builtin signatures YAML must parse")
}

/// 从 YAML 字符串解析签名规则列表。
///
/// 用于 T2-5 GUI 在线编辑后重新装载。失败返回 [`CoreError::Other`]。
pub fn load_signatures_from_str(yaml: &str) -> Result<Vec<SignatureRule>, CoreError> {
    serde_yml::from_str::<Vec<SignatureRule>>(yaml).map_err(|e| {
        CoreError::Other(format!("parse signatures yaml failed: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_loads_six_rules_all_enabled() {
        let rules = load_builtin_signatures();
        assert_eq!(rules.len(), 6, "builtin SQLi signatures must be 6");
        assert!(rules.iter().all(|r| r.enabled), "all builtin rules enabled");
    }

    #[test]
    fn from_str_round_trips() {
        let yaml = serde_yml::to_string(&load_builtin_signatures()).expect("serialize");
        let again = load_signatures_from_str(&yaml).expect("parse back");
        assert_eq!(again.len(), 6);
    }
}
