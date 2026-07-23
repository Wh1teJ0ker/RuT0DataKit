//! 数据校验 pipeline：把 [`Records`] 按 [`RuleSet`] 的 `validators` 逐 cell
//! 跑校验，产出 [`ValidateResult`]。
//!
//! 语义：
//! - 对 `rules.validators` 中每个 [`FieldRule`]，调用
//!   [`crate::rules::build_validator`] 构造校验器，对该列所有 cell 跑
//!   [`Validator::validate`](crate::validators::Validator::validate)。
//! - `valid_matrix[row][col]` 表示该 cell 是否合法；与 `rows` 同形（外层按行、
//!   内层按列）。
//! - 未声明 validator 的列 `valid_matrix[*][col]` 全为 `true`。
//! - `summary.invalid_rows` = 至少有一个非法 cell 的行数。
//! - `summary.by_field` = 每个声明了 validator 的字段的非法 cell 数。
//! - 未知名 validator（`build_validator` 返回 `None`）的字段跳过（不报错，
//!   该列保持全 true）。

use std::collections::HashMap;

use crate::error::CoreError;
use crate::readers::Records;
use crate::rules::{build_validator, RuleSet};
use crate::validators::default_validator_registry;

/// 单字段校验结果汇总。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ValidateSummary {
    /// 数据行总数。
    pub total_rows: usize,
    /// 至少有一个非法 cell 的行数。
    pub invalid_rows: usize,
    /// 每个声明了 validator 的字段的非法 cell 数。
    pub by_field: HashMap<String, usize>,
}

/// 校验 pipeline 产出：原数据 + 逐 cell 合法性矩阵 + 汇总。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidateResult {
    /// 表头（与 `records.headers` 一致）。
    pub headers: Vec<String>,
    /// 原数据行（校验不改数据，仅产出 valid_matrix）。
    pub rows: Vec<Vec<String>>,
    /// 逐 cell 合法性矩阵，`valid_matrix[row][col]` 与 `rows[row][col]` 对应。
    pub valid_matrix: Vec<Vec<bool>>,
    /// 校验汇总。
    pub summary: ValidateSummary,
}

/// 对 `records` 按 `rules.validators` 逐 cell 校验。
///
/// 使用内置默认 validator 注册表（`default_validator_registry`）；YAML 内联
/// `regex` 兜底优先于 `validator` 名。未知名 validator 的字段静默跳过（该列
/// 保持全 true）。
pub fn validate_pipeline(
    records: &Records,
    rules: &RuleSet,
) -> Result<ValidateResult, CoreError> {
    let reg = default_validator_registry();

    // header → col index
    let mut header_idx: HashMap<&str, usize> = HashMap::new();
    for (i, h) in records.headers.iter().enumerate() {
        header_idx.insert(h.as_str(), i);
    }

    let total_rows = records.rows.len();
    let n_cols = records.headers.len();
    let mut valid_matrix: Vec<Vec<bool>> = vec![vec![true; n_cols]; total_rows];
    let mut by_field: HashMap<String, usize> = HashMap::new();

    for rule in &rules.validators {
        // 字段不在 headers 中：跳过（无法定位列）。
        let col_idx = match header_idx.get(rule.field.as_str()) {
            Some(&idx) => idx,
            None => continue,
        };
        let validator = match build_validator(rule, &reg) {
            Some(v) => v,
            None => continue, // 未知名 validator 且无 regex，跳过
        };
        for (row_idx, row) in records.rows.iter().enumerate() {
            let cell = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            let result = validator.validate(cell);
            if !result.valid {
                valid_matrix[row_idx][col_idx] = false;
                *by_field.entry(rule.field.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut invalid_rows: usize = 0;
    for row_valid in &valid_matrix {
        if row_valid.iter().any(|v| !v) {
            invalid_rows += 1;
        }
    }

    Ok(ValidateResult {
        headers: records.headers.clone(),
        rows: records.rows.clone(),
        valid_matrix,
        summary: ValidateSummary {
            total_rows,
            invalid_rows,
            by_field,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::FieldRule;

    #[test]
    fn validate_basic_marks_invalid_cells() {
        // phone validator：合法手机号 11 位 1 开头。
        let records = Records {
            headers: vec!["phone".to_string()],
            rows: vec![
                vec!["13812345678".to_string()],
                vec!["13987654321".to_string()],
                vec!["abc".to_string()],
            ],
        };
        let rules = RuleSet {
            validators: vec![FieldRule {
                field: "phone".into(),
                scope: "phone".into(), tag: "validate".into(),
                params: None,
                message: None,
                description: None,            }],
            maskers: vec![],
        };

        let r = validate_pipeline(&records, &rules).expect("validate");
        assert_eq!(r.summary.total_rows, 3);
        // 前两行合法、第三行非法
        assert!(r.valid_matrix[0][0]);
        assert!(r.valid_matrix[1][0]);
        assert!(!r.valid_matrix[2][0]);
        assert_eq!(r.summary.invalid_rows, 1);
        assert_eq!(*r.summary.by_field.get("phone").unwrap_or(&0), 1);
    }

    #[test]
    fn no_validators_yields_all_true() {
        let records = Records {
            headers: vec!["a".to_string(), "b".to_string()],
            rows: vec![vec!["x".to_string(), "y".to_string()]],
        };
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![],
        };
        let r = validate_pipeline(&records, &rules).expect("validate");
        assert_eq!(r.summary.total_rows, 1);
        assert_eq!(r.summary.invalid_rows, 0);
        assert!(r.summary.by_field.is_empty());
        assert_eq!(r.valid_matrix, vec![vec![true, true]]);
    }

    #[test]
    fn validator_without_regex_unknown_name_skipped() {
        // 未知名 validator 且无 regex → 跳过，该列保持全 true。
        let records = Records {
            headers: vec!["a".to_string()],
            rows: vec![vec!["anything".to_string()]],
        };
        let rules = RuleSet {
            validators: vec![FieldRule {
                field: "a".into(),
                scope: "ghost".into(), tag: "validate".into(),
                params: None,
                message: None,
                description: None,            }],
            maskers: vec![],
        };
        let r = validate_pipeline(&records, &rules).expect("validate");
        assert!(r.valid_matrix[0][0]);
        assert!(r.summary.by_field.is_empty());
        assert_eq!(r.summary.invalid_rows, 0);
    }
}
