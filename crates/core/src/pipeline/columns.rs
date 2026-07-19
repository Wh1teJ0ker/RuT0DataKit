//! 按列勾选脱敏 pipeline：与 [`crate::pipeline::mask::mask_pipeline`] 同样的
//! 脱敏语义，但只对 `selected_columns` 列出的字段应用 masker。
//!
//! 语义：
//! - `selected_columns` 是「要应用脱敏的列名集合」。
//! - 只对同时满足「在 `selected_columns` 中」且「在 `rules.maskers[].field`
//!   中」的字段应用 masker。
//! - 未在 `selected_columns` 中的规则列：跳过 masker，原样输出。
//! - `selected_columns` 中未声明规则的列：原样输出。
//! - 返回与 [`crate::pipeline::mask::mask_pipeline`] 相同的 [`MaskResult`]；
//!   `summary.by_field` 只统计实际被 masker 调用的字段。

use std::collections::HashMap;
use std::collections::HashSet;

use crate::error::CoreError;
use crate::pipeline::mask::MaskResult;
use crate::readers::Records;
use crate::rules::{build_masker, RuleSet};

/// 对 `records` 按 `rules.maskers` 脱敏，但只对 `selected_columns` 中列出的
/// 字段应用 masker。
///
/// 表头不脱敏；`selected_columns` 中未声明规则的列原样输出；规则列不在
/// `selected_columns` 中的原样输出；未知名 masker 静默跳过。
///
/// 与 [`crate::pipeline::mask::mask_pipeline`] 一样，`summary.by_field` 只
/// 统计被 masker 调用的字段，`masked_rows` 为至少有一个字段被脱敏的行数。
pub fn mask_pipeline_columns(
    records: &Records,
    rules: &RuleSet,
    selected_columns: &HashSet<String>,
) -> Result<MaskResult, CoreError> {
    // header → col index
    let mut header_idx: HashMap<&str, usize> = HashMap::new();
    for (i, h) in records.headers.iter().enumerate() {
        header_idx.insert(h.as_str(), i);
    }

    // 只对 selected_columns ∩ rules.maskers[].field 交集列构造 plan。
    let mut skipped_fields: Vec<String> = Vec::new();
    let mut plan: Vec<(usize, String, Box<dyn crate::maskers::Masker>)> = Vec::new();
    for rule in &rules.maskers {
        // 规则列未被勾选 → 跳过（不计入 by_field、不算 skipped_fields）。
        if !selected_columns.contains(&rule.field) {
            continue;
        }
        let masker = match build_masker(rule) {
            Some(m) => m,
            None => continue, // 未知名脱敏器，跳过
        };
        match header_idx.get(rule.field.as_str()) {
            Some(&idx) => plan.push((idx, rule.field.clone(), masker)),
            None => skipped_fields.push(rule.field.clone()),
        }
    }

    let total_rows = records.rows.len();
    let mut by_field: HashMap<String, usize> = HashMap::new();
    let mut masked_rows: usize = 0;
    let mut new_rows: Vec<Vec<String>> = Vec::with_capacity(total_rows);

    for row in &records.rows {
        let mut new_row: Vec<String> = row.clone();
        let mut row_touched = false;
        for (idx, field, masker) in &plan {
            let original = std::mem::take(&mut new_row[*idx]);
            let masked_val = masker.mask(&original);
            new_row[*idx] = masked_val;
            *by_field.entry(field.clone()).or_insert(0) += 1;
            row_touched = true;
        }
        if row_touched {
            masked_rows += 1;
        }
        new_rows.push(new_row);
    }

    Ok(MaskResult {
        masked: Records {
            headers: records.headers.clone(),
            rows: new_rows,
        },
        summary: crate::pipeline::mask::MaskSummary {
            total_rows,
            masked_rows,
            by_field,
        },
        skipped_fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::MaskRule;
    use serde_yml::Value;
    use std::collections::HashMap;

    /// 构造 string/number/bool 参数 HashMap 辅助。
    fn p(items: &[(&str, Value)]) -> HashMap<String, Value> {
        items
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    /// phone 模板 params（11 位 guard）。
    fn phone_params() -> HashMap<String, Value> {
        p(&[
            ("keep_prefix", Value::Number(3.into())),
            ("keep_suffix", Value::Number(4.into())),
            ("mask_char", Value::String("*".into())),
            ("mask_min_len", Value::Number(4.into())),
            ("min_len", Value::Number(11.into())),
            ("max_len", Value::Number(11.into())),
            ("cjk", Value::Bool(false)),
        ])
    }

    /// name 模板 params（cjk 分支）。
    fn name_params() -> HashMap<String, Value> {
        p(&[
            ("keep_prefix", Value::Number(1.into())),
            ("keep_suffix", Value::Number(1.into())),
            ("mask_char", Value::String("*".into())),
            ("mask_min_len", Value::Number(1.into())),
            ("cjk", Value::Bool(true)),
        ])
    }

    fn sample_records() -> Records {
        Records {
            headers: vec!["phone".to_string(), "name".to_string()],
            rows: vec![
                vec!["13812345678".to_string(), "张三".to_string()],
                vec!["13987654321".to_string(), "李四海".to_string()],
            ],
        }
    }

    fn sample_rules() -> RuleSet {
        RuleSet {
            validators: vec![],
            maskers: vec![
                MaskRule {
                    field: "phone".into(),
                    masker: "template".into(),
                    params: Some(phone_params()),
                    description: None,
                },
                MaskRule {
                    field: "name".into(),
                    masker: "template".into(),
                    params: Some(name_params()),
                    description: None,
                },
            ],
        }
    }

    #[test]
    fn only_selected_column_is_masked() {
        let records = sample_records();
        let rules = sample_rules();
        let mut selected: HashSet<String> = HashSet::new();
        selected.insert("phone".to_string());

        let r = mask_pipeline_columns(&records, &rules, &selected).expect("mask");
        // phone 被脱敏
        assert_eq!(r.masked.rows[0][0], "138****5678");
        assert_eq!(r.masked.rows[1][0], "139****4321");
        // name 未勾选 → 原样
        assert_eq!(r.masked.rows[0][1], "张三");
        assert_eq!(r.masked.rows[1][1], "李四海");
        assert_eq!(r.summary.masked_rows, 2);
        // by_field 只含 phone
        assert!(r.summary.by_field.contains_key("phone"));
        assert!(!r.summary.by_field.contains_key("name"));
        assert!(r.skipped_fields.is_empty());
    }

    #[test]
    fn empty_selected_set_is_passthrough() {
        let records = sample_records();
        let rules = sample_rules();
        let selected: HashSet<String> = HashSet::new();

        let r = mask_pipeline_columns(&records, &rules, &selected).expect("mask");
        assert_eq!(r.summary.masked_rows, 0);
        assert!(r.summary.by_field.is_empty());
        assert_eq!(r.masked.rows[0][0], "13812345678");
        assert_eq!(r.masked.rows[0][1], "张三");
        assert_eq!(r.masked.rows[1][0], "13987654321");
        assert_eq!(r.masked.rows[1][1], "李四海");
    }

    #[test]
    fn unknown_selected_column_is_ignored() {
        let records = sample_records();
        let rules = sample_rules();
        let mut selected: HashSet<String> = HashSet::new();
        selected.insert("ghost_column".to_string());

        let r = mask_pipeline_columns(&records, &rules, &selected).expect("mask");
        assert_eq!(r.summary.masked_rows, 0);
        assert!(r.summary.by_field.is_empty());
        // 所有列原样
        assert_eq!(r.masked.rows[0][0], "13812345678");
        assert_eq!(r.masked.rows[1][1], "李四海");
    }
}
