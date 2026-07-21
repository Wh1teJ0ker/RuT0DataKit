//! 数据提取模块：从大段文本中按 PDF spec 提取 phone/bankcard/ip 三类 PII。
//!
//! v0.4.3 新增。复用 DefaultSensitiveScan + 内置 RuleSet（phone/bankcard/ip
//! 三条 validator 规则），不重新实现扫描引擎。scan 算法会：对每条 validator
//! 规则查 extract_pattern 取提取正则 → find_iter 候选 → 用对应 validator 校验
//! → valid==true 才记 finding。因此 bankcard 会过 Luhn，phone 会过 `^1\d{10}$`，
//! ip 会过 IP_REGEX。

use std::path::Path;

use crate::error::CoreError;
use crate::report::Finding;
use crate::rules::{FieldRule, RuleSet};
use crate::scan::{DefaultSensitiveScan, SensitiveScan};

/// 内置提取规则集：phone / bankcard / ip 三条 validator 规则。
///
/// field 与 validator 同名（"phone"/"bankcard"/"ip"），scan 算法用 validator
/// 名查 extract_pattern 取提取正则，用 build_validator 校验候选。
pub fn builtin_extract_ruleset() -> RuleSet {
    RuleSet {
        validators: vec![
            FieldRule {
                field: "phone".into(),
                validator: "phone".into(),
                params: None, regex: None, message: None, description: None, tags: Vec::new(),
            },
            FieldRule {
                field: "bankcard".into(),
                validator: "bankcard".into(),
                params: None, regex: None, message: None, description: None, tags: Vec::new(),
            },
            FieldRule {
                field: "ip".into(),
                validator: "ip".into(),
                params: None, regex: None, message: None, description: None, tags: Vec::new(),
            },
        ],
        maskers: vec![],
    }
}

/// 从文本提取 phone/bankcard/ip 三类 PII。
///
/// 返回的 Vec<Finding> 顺序由 scan 算法决定（按 rule 顺序 + 文本位置），
/// 可能含重复（同一段文本里同一值出现多次会被记多次）—— 去重由调用方
/// （T9-4 Tauri 命令）负责。
pub fn extract_text(content: &str) -> Vec<Finding> {
    let scan = DefaultSensitiveScan::new();
    scan.scan(content, &builtin_extract_ruleset())
}

/// 从 .txt 文件提取 phone/bankcard/ip 三类 PII。
///
/// 读全文 → 调 extract_text。文件不存在或非 UTF-8 时返回 Err。
pub fn extract_file(path: &Path) -> Result<Vec<Finding>, CoreError> {
    let content = std::fs::read_to_string(path)?;
    Ok(extract_text(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_finds_phone_bankcard_ip() {
        let text = "call 15560728076, card 6222023086872493750, ip 163.211.48.156";
        let findings = extract_text(text);
        assert!(findings.iter().any(|f| f.r#type == "phone" && f.value == "15560728076"), "phone not found: {:?}", findings);
        assert!(findings.iter().any(|f| f.r#type == "bankcard" && f.value == "6222023086872493750"), "bankcard not found: {:?}", findings);
        assert!(findings.iter().any(|f| f.r#type == "ip" && f.value == "163.211.48.156"), "ip not found: {:?}", findings);
    }

    #[test]
    fn extract_file_reads_and_scans() {
        let mut tmp = std::env::temp_dir();
        tmp.push("rut0_extract_test.txt");
        std::fs::write(&tmp, "phone 13812345678").unwrap();
        let findings = extract_file(&tmp).expect("extract file");
        assert!(findings.iter().any(|f| f.r#type == "phone" && f.value == "13812345678"));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn extract_file_missing_errors() {
        let res = extract_file(Path::new("/nonexistent/rut0/none.txt"));
        assert!(res.is_err());
    }

    #[test]
    fn builtin_ruleset_has_three_validators() {
        let rs = builtin_extract_ruleset();
        assert_eq!(rs.validators.len(), 3);
    }
}
