//! 脱敏 pipeline：把 [`Records`] 按 [`RuleSet`] 的 `maskers` 逐字段脱敏，
//! 产出 [`MaskResult`]。
//!
//! 规则：
//! - 对每条 `MaskRule` 调 [`crate::rules::build_masker`] 构造具体脱敏器。
//!   `rule.masker` 未知名时跳过该条规则（不计入 by_field）。
//! - `rule.field` 不在 `records.headers` 中时跳过（记入 `skipped_fields`）。
//! - 表头行不脱敏。
//! - 字段不在 maskers 列表中的原样输出。
//! - 短于阈值的输入由具体 masker 自行处理，pipeline 不做长度校验，不 panic。
//! - `summary.by_field` 记录每个字段被脱敏的 cell 数（含原值与脱敏后相同的情况，
//!   只要 masker 被调用即计数）。`masked_rows` 为至少有一个字段被脱敏的行数。

use std::collections::HashMap;
use std::collections::HashSet;

use crate::error::CoreError;
use crate::readers::Records;
use crate::rules::{build_masker, RuleSet};

/// 单字段脱敏结果汇总。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MaskSummary {
    /// 数据行总数。
    pub total_rows: usize,
    /// 至少有一个字段被脱敏的行数。
    pub masked_rows: usize,
    /// 每个字段被脱敏的 cell 数。
    pub by_field: HashMap<String, usize>,
}

/// 脱敏 pipeline 产出：脱敏后的 [`Records`] + 汇总 + 跳过字段列表。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MaskResult {
    /// 脱敏后的数据（表头不变，rows 已替换）。
    pub masked: Records,
    /// 脱敏汇总。
    pub summary: MaskSummary,
    /// 规则中存在但 records 里找不到对应 header 的字段名。
    pub skipped_fields: Vec<String>,
}

/// 对 `records` 按 `rules.maskers` 做脱敏。
///
/// 表头不脱敏；字段不在 maskers 列表中的原样输出；短于阈值的输入由具体
/// masker 处理，pipeline 不 panic。
pub fn mask_pipeline(
    records: &Records,
    rules: &RuleSet,
) -> Result<MaskResult, CoreError> {
    // header → col index
    let mut header_idx: HashMap<&str, usize> = HashMap::new();
    for (i, h) in records.headers.iter().enumerate() {
        header_idx.insert(h.as_str(), i);
    }

    // 构造 (col_idx, field, masker) 列表，跳过未知名 masker 与不存在的字段。
    let mut skipped_fields: Vec<String> = Vec::new();
    let mut plan: Vec<(usize, String, Box<dyn crate::maskers::Masker>)> = Vec::new();
    for rule in &rules.maskers {
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
        summary: MaskSummary {
            total_rows,
            masked_rows,
            by_field,
        },
        skipped_fields,
    })
}

/// 与 [`mask_pipeline`] 相同的脱敏语义，但只对 `selected` 中列出的数据行应用
/// 规则；未选中行原样复制到 `masked.rows`。
///
/// 约定：
/// - `selected` 为 `records.rows` 的 0-based 行索引；越界索引静默忽略，不 panic。
/// - 表头不脱敏（`records.headers` 原样保留）。
/// - `summary.masked_rows` 只统计被脱敏的选中行数；未选中行不计入。
/// - `summary.total_rows` 仍为全部数据行数。
/// - `summary.by_field` 只统计选中行中被 masker 调用的 cell 数。
pub fn mask_pipeline_selected(
    records: &Records,
    rules: &RuleSet,
    selected: &HashSet<usize>,
) -> Result<MaskResult, CoreError> {
    // header → col index
    let mut header_idx: HashMap<&str, usize> = HashMap::new();
    for (i, h) in records.headers.iter().enumerate() {
        header_idx.insert(h.as_str(), i);
    }

    let mut skipped_fields: Vec<String> = Vec::new();
    let mut plan: Vec<(usize, String, Box<dyn crate::maskers::Masker>)> = Vec::new();
    for rule in &rules.maskers {
        let masker = match build_masker(rule) {
            Some(m) => m,
            None => continue,
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

    for (row_idx, row) in records.rows.iter().enumerate() {
        // 未选中的行原样复制，不应用任何脱敏。
        if !selected.contains(&row_idx) {
            new_rows.push(row.clone());
            continue;
        }
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
        summary: MaskSummary {
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
    use crate::readers::CsvReader;
    use crate::readers::SourceReader;
    use crate::rules::MaskRule;
    use serde_yml::Value;
    use std::collections::HashMap;

    /// 构造一个 string 参数 HashMap 辅助。
    fn p(items: &[(&str, Value)]) -> HashMap<String, Value> {
        items
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    /// phone 模板 params（11 位 guard，keep 3+4）。
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

    /// email 切分模板 params（@ 切分 + 内层 cjk 分支）。
    fn email_params() -> HashMap<String, Value> {
        p(&[
            ("separator", Value::String("@".into())),
            ("segment_index", Value::Number(0.into())),
            ("keep_prefix", Value::Number(1.into())),
            ("keep_suffix", Value::Number(1.into())),
            ("mask_char", Value::String("*".into())),
            ("mask_min_len", Value::Number(1.into())),
            ("cjk", Value::Bool(true)),
        ])
    }

    /// idcard 模板 params（18 位 guard，keep 6+4）。
    fn idcard_params() -> HashMap<String, Value> {
        p(&[
            ("keep_prefix", Value::Number(6.into())),
            ("keep_suffix", Value::Number(4.into())),
            ("mask_char", Value::String("*".into())),
            ("mask_min_len", Value::Number(8.into())),
            ("min_len", Value::Number(18.into())),
            ("max_len", Value::Number(18.into())),
            ("cjk", Value::Bool(false)),
        ])
    }

    /// customer_id 模板 params（≥2 字 guard，keep 1+0）。
    fn customer_id_params() -> HashMap<String, Value> {
        p(&[
            ("keep_prefix", Value::Number(1.into())),
            ("keep_suffix", Value::Number(0.into())),
            ("mask_char", Value::String("*".into())),
            ("mask_min_len", Value::Number(4.into())),
            ("min_len", Value::Number(2.into())),
            ("cjk", Value::Bool(false)),
        ])
    }

    /// bankcard 模板 params（16~19 位 guard，keep 6+4）。
    fn bankcard_params() -> HashMap<String, Value> {
        p(&[
            ("keep_prefix", Value::Number(6.into())),
            ("keep_suffix", Value::Number(4.into())),
            ("mask_char", Value::String("*".into())),
            ("mask_min_len", Value::Number(4.into())),
            ("min_len", Value::Number(16.into())),
            ("max_len", Value::Number(19.into())),
            ("cjk", Value::Bool(false)),
        ])
    }

    /// 内联 6 条 MaskRule（对齐 sample_mask.csv 表头，使用通用 template 算子）。
    fn sample_ruleset() -> RuleSet {
        RuleSet {
            validators: vec![],
            maskers: vec![
                MaskRule {
                    field: "customer_id".into(),
                    masker: "template".into(),
                    params: Some(customer_id_params()),
                    description: None,                    tags: Vec::new(),                },
                MaskRule {
                    field: "name".into(),
                    masker: "template".into(),
                    params: Some(name_params()),
                    description: None,                    tags: Vec::new(),                },
                MaskRule {
                    field: "id_card".into(),
                    masker: "template".into(),
                    params: Some(idcard_params()),
                    description: None,                    tags: Vec::new(),                },
                MaskRule {
                    field: "phone".into(),
                    masker: "template".into(),
                    params: Some(phone_params()),
                    description: None,                    tags: Vec::new(),                },
                MaskRule {
                    field: "email".into(),
                    masker: "split_template".into(),
                    params: Some(email_params()),
                    description: None,                    tags: Vec::new(),                },
                MaskRule {
                    field: "bank_card".into(),
                    masker: "template".into(),
                    params: Some(bankcard_params()),
                    description: None,                    tags: Vec::new(),                },
            ],
        }
    }

    fn fixtures_csv() -> std::path::PathBuf {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../tests/fixtures/samples/csv/sample_mask.csv");
        p
    }

    #[test]
    fn end_to_end_sample_mask_csv() {
        let csv_path = fixtures_csv();
        let records = CsvReader::new().read(&csv_path).expect("read csv");
        let rules = sample_ruleset();
        let result = mask_pipeline(&records, &rules).expect("mask");

        assert_eq!(result.masked.headers, records.headers);
        assert_eq!(result.masked.rows.len(), 2);
        assert_eq!(result.summary.total_rows, 2);
        assert_eq!(result.summary.masked_rows, 2);
        // 每个字段都被脱敏 2 次。
        for f in ["customer_id", "name", "id_card", "phone", "email", "bank_card"] {
            assert_eq!(*result.summary.by_field.get(f).unwrap_or(&0), 2, "field {f}");
        }
        assert!(result.skipped_fields.is_empty());

        // 逐字段对照 acceptance_criteria。
        // 第 0 行
        assert_eq!(result.masked.rows[0][0], "1*******");
        assert_eq!(result.masked.rows[0][1], "张*");
        assert_eq!(result.masked.rows[0][2], "110101********1234");
        assert_eq!(result.masked.rows[0][3], "138****5678");
        assert_eq!(result.masked.rows[0][4], "z******n@example.com");
        assert_eq!(result.masked.rows[0][5], "622588******1098");
        // 第 1 行
        assert_eq!(result.masked.rows[1][0], "8*******");
        assert_eq!(result.masked.rows[1][1], "李*海");
        assert_eq!(result.masked.rows[1][2], "110101********1234");
        assert_eq!(result.masked.rows[1][3], "139****4321");
        // 注：email 数据为 "lisi@x.cn"，本地 4 字 "lisi"，
        // 按 T0-3 已验证的 EmailMask 规格（首 + *(n-2) + 末）→ "l**i@x.cn"。
        // HANDOFF acceptance_criteria 写的 "l*@x.com" 与之不符（疑似笔误：
        // 既把域名 .cn 写成 .com，又按 2 字本地规则期望）。masker 行为以
        // T0-3 verified_complete 为准，本任务不改 masker。
        assert_eq!(result.masked.rows[1][4], "l**i@x.cn");
        assert_eq!(result.masked.rows[1][5], "622588******2233");
    }

    #[test]
    fn header_not_masked() {
        let records = Records {
            headers: vec!["phone".to_string()],
            rows: vec![vec!["13812345678".to_string()]],
        };
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![MaskRule {
                field: "phone".into(),
                masker: "template".into(),
                params: Some(phone_params()),
                description: None,                tags: Vec::new(),            }],
        };
        let r = mask_pipeline(&records, &rules).unwrap();
        assert_eq!(r.masked.headers, vec!["phone".to_string()]);
        assert_eq!(r.masked.rows[0][0], "138****5678");
    }

    #[test]
    fn field_not_in_headers_is_skipped() {
        let records = Records {
            headers: vec!["a".to_string()],
            rows: vec![vec!["x".to_string()]],
        };
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![MaskRule {
                field: "ghost".into(),
                masker: "template".into(),
                params: Some(phone_params()),
                description: None,                tags: Vec::new(),            }],
        };
        let r = mask_pipeline(&records, &rules).unwrap();
        assert_eq!(r.skipped_fields, vec!["ghost".to_string()]);
        assert_eq!(r.summary.masked_rows, 0);
        assert!(r.summary.by_field.is_empty());
        // 非 maskers 列表的字段原样输出
        assert_eq!(r.masked.rows[0][0], "x");
    }

    #[test]
    fn unknown_masker_skipped_silently() {
        let records = Records {
            headers: vec!["a".to_string()],
            rows: vec![vec!["x".to_string()]],
        };
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![MaskRule {
                field: "a".into(),
                masker: "no_such_mask".into(),
                params: None,
            description: None,            tags: Vec::new(),            }],
        };
        let r = mask_pipeline(&records, &rules).unwrap();
        assert!(r.skipped_fields.is_empty()); // 未知名 masker 不算 skipped_fields
        assert_eq!(r.summary.masked_rows, 0);
        assert_eq!(r.masked.rows[0][0], "x"); // 原样
    }

    #[test]
    fn short_input_does_not_panic() {
        // 1 字 name / 1 字 email / 1 字 id_card 等
        let records = Records {
            headers: vec!["name".to_string(), "email".to_string()],
            rows: vec![vec!["张".to_string(), "a@b.com".to_string()]],
        };
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![
                MaskRule {
                    field: "name".into(),
                    masker: "template".into(),
                    params: Some(name_params()),
                    description: None,                    tags: Vec::new(),                },
                MaskRule {
                    field: "email".into(),
                    masker: "split_template".into(),
                    params: Some(email_params()),
                    description: None,                    tags: Vec::new(),                },
            ],
        };
        let r = mask_pipeline(&records, &rules).unwrap();
        // name 1 字原样
        assert_eq!(r.masked.rows[0][0], "张");
        // email "a" 本地 1 字原样
        assert_eq!(r.masked.rows[0][1], "a@b.com");
    }

    #[test]
    fn mask_pipeline_selected_applies_only_to_selected_rows() {
        use std::collections::HashSet;

        // 3 行数据，选中第 0、2 行应用 phone 模板。
        let records = Records {
            headers: vec!["phone".to_string()],
            rows: vec![
                vec!["13812345678".to_string()],
                vec!["13912345678".to_string()],
                vec!["13712345678".to_string()],
            ],
        };
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![MaskRule {
                field: "phone".into(),
                masker: "template".into(),
                params: Some(phone_params()),
                description: None,                tags: Vec::new(),            }],
        };
        let mut selected: HashSet<usize> = HashSet::new();
        selected.insert(0);
        selected.insert(2);

        let r = mask_pipeline_selected(&records, &rules, &selected).expect("mask");
        assert_eq!(r.summary.total_rows, 3);
        assert_eq!(r.summary.masked_rows, 2);
        // 选中行被脱敏
        assert_eq!(r.masked.rows[0][0], "138****5678");
        // 未选中行原样
        assert_eq!(r.masked.rows[1][0], "13912345678");
        // 选中行被脱敏
        assert_eq!(r.masked.rows[2][0], "137****5678");
        // 表头不变
        assert_eq!(r.masked.headers, vec!["phone".to_string()]);
        // by_field 只统计选中行的 cell
        assert_eq!(*r.summary.by_field.get("phone").unwrap_or(&0), 2);
    }

    #[test]
    fn mask_pipeline_selected_ignores_out_of_range_indices() {
        use std::collections::HashSet;

        // selected 含越界索引（99）不应 panic，应被忽略。
        let records = Records {
            headers: vec!["phone".to_string()],
            rows: vec![vec!["13812345678".to_string()]],
        };
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![MaskRule {
                field: "phone".into(),
                masker: "template".into(),
                params: Some(phone_params()),
                description: None,                tags: Vec::new(),            }],
        };
        let mut selected: HashSet<usize> = HashSet::new();
        selected.insert(99);

        let r = mask_pipeline_selected(&records, &rules, &selected).expect("mask");
        // 越界索引被忽略，无行被脱敏
        assert_eq!(r.summary.masked_rows, 0);
        assert_eq!(r.masked.rows[0][0], "13812345678");
    }

    #[test]
    fn mask_pipeline_selected_empty_selected_passthrough() {
        use std::collections::HashSet;

        let records = Records {
            headers: vec!["phone".to_string()],
            rows: vec![vec!["13812345678".to_string()]],
        };
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![MaskRule {
                field: "phone".into(),
                masker: "template".into(),
                params: Some(phone_params()),
                description: None,                tags: Vec::new(),            }],
        };
        let selected: HashSet<usize> = HashSet::new();

        let r = mask_pipeline_selected(&records, &rules, &selected).expect("mask");
        assert_eq!(r.summary.masked_rows, 0);
        assert_eq!(r.masked.rows[0][0], "13812345678");
        assert!(r.summary.by_field.is_empty());
    }
}
