//! 敏感扫描共享底座：SensitiveScan trait + DefaultSensitiveScan。
//!
//! v0.4.4 重构：scan 算法统一走 `rule.scope` -> `extract_pattern` 查表 ->
//! find_iter 候选 -> `build_validator` 校验 -> valid==true 才记 finding。
//! 删除旧 `rule.regex` 分支（regex 字段已移除）。`Finding.type` 统一填
//! `rule.scope`（不再有 field/validator 二义）。

use crate::report::Finding;
use crate::rules::{build_validator, RuleSet, ValidatorRegistry};
use crate::validators::default_validator_registry;

/// 敏感扫描 trait。
pub trait SensitiveScan {
    /// 对文本用 `rules.validators` 识别敏感项，返回 Finding 列表。
    fn scan(&self, text: &str, rules: &RuleSet) -> Vec<Finding>;
}

/// 默认实现：对 RuleSet.validators 中每个 FieldRule：
/// - 按 `rule.scope`（数据类型）查内置提取正则表 `extract_pattern`
/// - find_iter 候选子串
/// - 对每个候选跑 `build_validator(rule, &reg)` 得到的 validator.validate()
/// - valid==true 才记 Finding { type: scope, value, valid: Some(true), ... }
///
/// 提取正则表（scope 名 -> 提取 pattern）：
/// - idcard: `\b\d{17}[\dXx]\b`
/// - phone: `\b\d{11}\b`
/// - bankcard: `\b\d{13,19}\b`
/// - email: `[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`
/// - ip: `\b(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)(?:\.(?:...)){3}\b`
/// - mac: `\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b`
/// - username: `[A-Za-z0-9]{3,}`
/// - name: `[\x{4e00}-\x{9fa5}]{2,}`
///
/// `rule.scope` 不在表中 -> 跳过该 rule。
pub struct DefaultSensitiveScan {
    reg: ValidatorRegistry,
}

impl DefaultSensitiveScan {
    pub fn new() -> Self {
        Self {
            reg: default_validator_registry(),
        }
    }

    /// 用自定义注册表构造（供注入扩展 validator）。
    pub fn with_registry(reg: ValidatorRegistry) -> Self {
        Self { reg }
    }

    /// 按 `scope`（数据类型）返回提取正则。scope 取代旧 validator 名。
    ///
    /// v0.5.0：函数体改为查 `rules::patterns::extract_pattern` 表，
    /// 不再在本文件硬编码正则字面量。
    fn extract_pattern(scope: &str) -> Option<&'static str> {
        crate::rules::patterns::extract_pattern(scope)
    }
}

impl Default for DefaultSensitiveScan {
    fn default() -> Self {
        Self::new()
    }
}

impl SensitiveScan for DefaultSensitiveScan {
    fn scan(&self, text: &str, rules: &RuleSet) -> Vec<Finding> {
        let mut findings = Vec::new();
        for rule in &rules.validators {
            // v0.4.4：统一走 scope -> extract_pattern -> validator 校验路径。
            // 旧 rule.regex 分支已删除（regex 字段已移除）。
            let Some(pattern) = Self::extract_pattern(&rule.scope) else { continue };
            let Ok(re) = regex::Regex::new(pattern) else { continue };

            // 对每个候选子串跑 build_validator 校验，valid==true 才记 finding。
            for m in re.find_iter(text) {
                let candidate = m.as_str();
                if let Some(v) = build_validator(rule, &self.reg) {
                    let r = v.validate(candidate);
                    if r.valid {
                        findings.push(Finding {
                            r#type: rule.scope.clone(),
                            value: candidate.to_string(),
                            location: None,
                            valid: Some(true),
                            context: None,
                            extra: None,
                        });
                    }
                }
                // 无可用 validator -> 跳过该候选（不标 valid）
            }
        }
        findings
    }
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
    fn scan_finds_idcard_in_text() {
        let rules = RuleSet {
            validators: vec![rule("id_card", "idcard", "extract")],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        // 合成测试号（非 tips/ fixture 真实 PII）
        let text = "用户 286071197501111126 于 2024-01-01 注册";
        let findings = scan.scan(text, &rules);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].r#type, "idcard");
        assert_eq!(findings[0].value, "286071197501111126");
        assert_eq!(findings[0].valid, Some(true));
    }

    #[test]
    fn scan_skips_invalid_idcard() {
        // 含一个 18 位但校验位非法的串，不应产 finding
        let rules = RuleSet {
            validators: vec![rule("id_card", "idcard", "extract")],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        // 801615200409127668 是已知非法（末位应为 9）
        let text = "bad 801615200409127668 here";
        let findings = scan.scan(text, &rules);
        assert!(
            findings.is_empty(),
            "invalid idcard should not yield finding"
        );
    }

    #[test]
    fn scan_multiple_scopes() {
        let rules = RuleSet {
            validators: vec![
                rule("id", "idcard", "extract"),
                rule("p", "phone", "extract"),
            ],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        let text = "id 286071197501111126 phone 13812345678";
        let findings = scan.scan(text, &rules);
        assert_eq!(findings.len(), 2);
        let types: Vec<_> = findings.iter().map(|f| f.r#type.as_str()).collect();
        assert!(types.contains(&"idcard"));
        assert!(types.contains(&"phone"));
    }

    #[test]
    fn scan_unknown_scope_skipped() {
        // v0.4.4：未注册的 scope -> extract_pattern 返回 None -> 跳过。
        let rules = RuleSet {
            validators: vec![rule("x", "ghost", "extract")],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        assert!(scan.scan("anything", &rules).is_empty());
    }

    #[test]
    fn scan_finds_ip_in_text() {
        let rules = RuleSet {
            validators: vec![rule("ip", "ip", "extract")],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        let text = "contact 192.168.1.1 for info";
        let findings = scan.scan(text, &rules);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].r#type, "ip");
        assert_eq!(findings[0].value, "192.168.1.1");
        assert_eq!(findings[0].valid, Some(true));
    }

    #[test]
    fn scan_finds_bankcard_passing_luhn() {
        // 合成测试号 62258800000000002 通过 Luhn 校验
        let rules = RuleSet {
            validators: vec![rule("card", "bankcard", "extract")],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        let text = "card 62258800000000002 end";
        let findings = scan.scan(text, &rules);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].r#type, "bankcard");
        assert_eq!(findings[0].value, "62258800000000002");
    }
}
