//! 数据提取模块：从大段文本中按规则提取 PII。
//!
//! v0.4.3 初版：复用 DefaultSensitiveScan + 内置 RuleSet（phone/bankcard/ip
//! 三条 validator 规则），scan 算法会：对每条 validator 规则查 extract_pattern
//! 取提取正则 -> find_iter 候选 -> 用对应 validator 校验 -> valid==true 才记
//! finding。
//!
//! v0.4.4 重构：删除 `builtin_extract_ruleset`（内置规则集已清空，规则池
//! 初始为空）。`extract_text` / `extract_file` 改为薄包装调
//! `extract_text_with_rules(content, &RuleSet::default())`（空池返回空 Vec）。
//! `extract_text_with_rules` / `extract_file_with_rules` 接口完整保留，
//! 供前端从「规则管理」池勾选规则后传入任意 RuleSet 提取。
//!
//! v0.4.5：接入第一条内置规则——手机号提取（scope="phone", tag="extract"）。
//! `extract_text` / `extract_file` 默认改用 `builtin_ruleset()`，这样无需用户
//! 配置即可提取手机号。后续版本逐步接入更多内置规则（idcard/bankcard/...）。

use std::path::Path;

use crate::error::CoreError;
use crate::report::Finding;
use crate::rules::{builtin_ruleset, RuleSet};
use crate::scan::{DefaultSensitiveScan, SensitiveScan};

/// 从文本提取 PII（内置规则入口）。
///
/// v0.4.5：等价于 `extract_text_with_rules(content, &builtin_ruleset())`。
/// 内置规则集当前含手机号提取（scope="phone"），后续版本逐步追加。
pub fn extract_text(content: &str) -> Vec<Finding> {
    extract_text_with_rules(content, &builtin_ruleset())
}

/// 用指定 RuleSet 从文本提取 PII。
///
/// 底层调 `DefaultSensitiveScan::scan(content, rules)`，scan 算法对每条
/// validator 规则按 `rule.scope` 查 extract_pattern 取提取正则 ->
/// find_iter 候选 -> 用对应 validator 校验 -> valid==true 才记 finding。
/// 因此传入任意 RuleSet（如前端从「规则管理」池勾选的 idcard/email/mac
/// 等规则）都能提取。
pub fn extract_text_with_rules(content: &str, rules: &RuleSet) -> Vec<Finding> {
    let scan = DefaultSensitiveScan::new();
    scan.scan(content, rules)
}

/// 从 .txt 文件提取 PII（内置规则入口）。
///
/// 读全文 -> 调 extract_text。文件不存在或非 UTF-8 时返回 Err。
///
/// v0.4.5：等价于 `extract_file_with_rules(path, &builtin_ruleset())`。
pub fn extract_file(path: &Path) -> Result<Vec<Finding>, CoreError> {
    extract_file_with_rules(path, &builtin_ruleset())
}

/// 用指定 RuleSet 从 .txt 文件提取 PII。
///
/// 读全文 -> 调 extract_text_with_rules。文件不存在或非 UTF-8 时返回 Err。
pub fn extract_file_with_rules(path: &Path, rules: &RuleSet) -> Result<Vec<Finding>, CoreError> {
    let content = std::fs::read_to_string(path)?;
    Ok(extract_text_with_rules(&content, rules))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{FieldRule, RuleSet};

    fn rule(field: &str, scope: &str, tag: &str) -> FieldRule {
        FieldRule {
            field: field.into(),
            scope: scope.into(),
            tag: tag.into(),
            params: None,
            message: None,
            description: None,
        }
    }

    #[test]
    fn extract_text_with_empty_ruleset_returns_empty() {
        // 空规则集 -> 返回空 Vec（extract_text 默认用 builtin_ruleset，
        // 要测空池行为需显式传 RuleSet::default()）。
        let findings = extract_text_with_rules("call 13812345678", &RuleSet::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn extract_text_default_uses_builtin_phone() {
        // v0.4.5：extract_text 默认用 builtin_ruleset（含 phone extract），
        // 能提取手机号（首位 1 + 11 位）。
        let findings = extract_text("call 13812345678 end");
        assert!(findings.iter().any(|f| f.r#type == "phone" && f.value == "13812345678"));
    }

    #[test]
    fn extract_text_with_rules_finds_phone_bankcard_ip() {
        // v0.4.4：用自定义 RuleSet（phone/bankcard/ip 三条 extract 规则）提取。
        // 合成测试号（非 tips/ fixture 真实 PII）；bankcard 用能过 Luhn 的合成号。
        let rules = RuleSet {
            validators: vec![
                rule("phone", "phone", "extract"),
                rule("bankcard", "bankcard", "extract"),
                rule("ip", "ip", "extract"),
            ],
            maskers: vec![],
        };
        let text = "call 13812345678, card 62258800000000002, ip 192.168.1.1";
        let findings = extract_text_with_rules(text, &rules);
        assert!(findings.iter().any(|f| f.r#type == "phone" && f.value == "13812345678"), "phone not found: {:?}", findings);
        assert!(findings.iter().any(|f| f.r#type == "bankcard" && f.value == "62258800000000002"), "bankcard not found: {:?}", findings);
        assert!(findings.iter().any(|f| f.r#type == "ip" && f.value == "192.168.1.1"), "ip not found: {:?}", findings);
    }

    #[test]
    fn extract_file_with_rules_reads_and_scans() {
        // v0.4.4：extract_file_with_rules 读取 tmp 文件 + 用自定义 RuleSet 扫描。
        let rules = RuleSet {
            validators: vec![rule("phone", "phone", "extract")],
            maskers: vec![],
        };
        let tmp = std::env::temp_dir().join("rut0_extract_test_v044.txt");
        std::fs::write(&tmp, "contact 13812345678 end").unwrap();
        let findings = extract_file_with_rules(&tmp, &rules).unwrap();
        assert!(findings.iter().any(|f| f.r#type == "phone" && f.value == "13812345678"));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn extract_file_missing_errors() {
        let res = extract_file(Path::new("/nonexistent/rut0_extract_missing.txt"));
        assert!(res.is_err());
    }

    #[test]
    fn extract_file_with_rules_missing_errors() {
        let rules = RuleSet::default();
        let res = extract_file_with_rules(Path::new("/nonexistent/rut0_extract_missing.txt"), &rules);
        assert!(res.is_err());
    }
}
