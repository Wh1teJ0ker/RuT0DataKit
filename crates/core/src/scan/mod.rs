//! 敏感扫描共享底座：SensitiveScan trait + DefaultSensitiveScan。
//!
//! v0.1.0 落地 trait + 最小实现，供 v0.1.1 日志扫描 / v0.1.2 pcap 扫描扩展。
//! v0.1.0 GUI 不调用，但 trait/类型/最小实现必须可编译可测。

use crate::report::Finding;
use crate::rules::{build_validator, RuleSet, ValidatorRegistry};
use crate::validators::default_validator_registry;

/// 敏感扫描 trait。
pub trait SensitiveScan {
    /// 对文本用 `rules.validators` 识别敏感项，返回 Finding 列表。
    fn scan(&self, text: &str, rules: &RuleSet) -> Vec<Finding>;
}

/// 默认实现：对 RuleSet.validators 中每个 FieldRule：
/// - regex 存在 → 用 RegexValidator 识别
/// - 否则用注册表中的内置 validator 识别
/// 命中（valid==true）则产 Finding { type, value, location:None, valid:Some(bool), context:None }
///
/// 注意：扫描是"在自由文本里找敏感片段"，而 validator 是"整段校验单个字段值"。
/// v0.1.0 最小实现采取的简化策略：
///   - 对每条 FieldRule，在 text 上按 regex 找候选子串；若 rule 无 regex，则用
///     validator 名对应的内置 regex（本任务为每个内置 validator 提供一个候选提取正则）
///     提取候选，再对每个候选跑 build_validator 得到的 validator.validate()，
///     valid==true 才记为 finding。
///   - 提取正则表（validator 名 → 提取 pattern）：
///     idcard: `\b\d{17}[\dXx]\b`
///     phone: `\b\d{11}\b`
///     bankcard: `\b\d{13,19}\b`
///     email: `[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`
///     mac: `\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b`
///     username: `[A-Za-z0-9]{3,}`
///     name: `[\u4e00-\u9fa5]{2,}`  // Rust 正则用 [\x{4e00}-\x{9fa5}]
///   - rule.validator 不在表中且无 regex → 跳过该 rule
///
/// 这是"最小实现"：v0.1.1 日志扫描会替换为更精确的分词/上下文策略，
/// 但 trait 签名与 Finding schema 不变。
pub struct DefaultSensitiveScan {
    reg: ValidatorRegistry,
}

impl DefaultSensitiveScan {
    pub fn new() -> Self {
        Self {
            reg: default_validator_registry(),
        }
    }

    /// 用自定义注册表构造（供 v0.1.1+ 注入扩展 validator）。
    pub fn with_registry(reg: ValidatorRegistry) -> Self {
        Self { reg }
    }

    fn extract_pattern(validator_name: &str) -> Option<&'static str> {
        Some(match validator_name {
            "idcard" => r"\b\d{17}[\dXx]\b",
            "phone" => r"\b\d{11}\b",
            "bankcard" => r"\b\d{13,19}\b",
            "email" => r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
            "mac" => r"\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b",
            "username" => r"[A-Za-z0-9]{3,}",
            "name" => r"[\x{4e00}-\x{9fa5}]{2,}",
            _ => return None,
        })
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
            // 确定提取 pattern：rule.regex 优先，否则查内置表
            let pattern = if let Some(re) = rule.regex.as_ref() {
                Some(re.as_str())
            } else {
                Self::extract_pattern(&rule.validator)
            };
            let Some(pattern) = pattern else { continue };
            let Ok(re) = regex::Regex::new(pattern) else { continue };

            // rule.regex 存在时，提取 pattern == rule.regex，候选必匹配，
            // 直接记为 finding(valid=true)；否则用注册表 validator 校验候选，
            // valid==true 才记。
            for m in re.find_iter(text) {
                let candidate = m.as_str();
                if rule.regex.is_some() {
                    findings.push(Finding {
                        r#type: rule.field.clone(),
                        value: candidate.to_string(),
                        location: None,
                        valid: Some(true),
                        context: None,
                        extra: None,
                    });
                } else if let Some(v) = build_validator(rule, &self.reg) {
                    let r = v.validate(candidate);
                    if r.valid {
                        findings.push(Finding {
                            r#type: rule.validator.clone(),
                            value: candidate.to_string(),
                            location: None,
                            valid: Some(true),
                            context: None,
                            extra: None,
                        });
                    }
                }
                // 无可用 validator → 跳过该候选（不标 valid）
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{FieldRule, RuleSet};

    #[test]
    fn scan_finds_idcard_in_text() {
        let rules = RuleSet {
            validators: vec![FieldRule {
                field: "id_card".into(),
                validator: "idcard".into(),
                params: None,
                regex: None,
                message: None,
                description: None,
            }],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        // 合法 idcard
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
            validators: vec![FieldRule {
                field: "id_card".into(),
                validator: "idcard".into(),
                params: None,
                regex: None,
                message: None,
                description: None,
            }],
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
    fn scan_regex_rule_yields_finding() {
        let rules = RuleSet {
            validators: vec![FieldRule {
                field: "token".into(),
                validator: "custom_token".into(),
                params: None,
                regex: Some(r"TKN-[A-Z0-9]{6}".into()),
                message: None,
                description: None,
            }],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        let text = "tokens: TKN-AB12CD and TKN-ZZ99ZZ";
        let findings = scan.scan(text, &rules);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].r#type, "token");
        assert_eq!(findings[0].valid, Some(true));
    }

    #[test]
    fn scan_multiple_validator_types() {
        let rules = RuleSet {
            validators: vec![
                FieldRule {
                    field: "id".into(),
                    validator: "idcard".into(),
                    params: None,
                    regex: None,
                    message: None,
                    description: None,
                },
                FieldRule {
                    field: "p".into(),
                    validator: "phone".into(),
                    params: None,
                    regex: None,
                    message: None,
                    description: None,
                },
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
    fn scan_unknown_validator_no_regex_skipped() {
        let rules = RuleSet {
            validators: vec![FieldRule {
                field: "x".into(),
                validator: "ghost".into(),
                params: None,
                regex: None,
                message: None,
                description: None,
            }],
            maskers: vec![],
        };
        let scan = DefaultSensitiveScan::new();
        assert!(scan.scan("anything", &rules).is_empty());
    }
}
