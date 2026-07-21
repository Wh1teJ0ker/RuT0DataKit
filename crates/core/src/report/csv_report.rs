//! CSV 脱敏报告产出：
//! - [`write_masked_csv`]：把脱敏后的 [`Records`] 写成 masked.csv。
//! - [`build_csv_mask_report`]：构造 `kind="csv_mask"` 的 [`Report`]。
//!
//! `summary` 填 [`crate::pipeline::MaskSummary`] 序列化结果；`findings` 为空；
//! `extra` 记录 `skipped_fields`。

use std::path::Path;

use crate::error::CoreError;
use crate::pipeline::MaskResult;
use crate::readers::Records;
use crate::report::Report;

/// 把脱敏后的 records 写成 masked.csv。第一行写 headers，其余写数据行。
pub fn write_masked_csv(records: &Records, path: &Path) -> Result<(), CoreError> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(&records.headers)?;
    for row in &records.rows {
        wtr.write_record(row)?;
    }
    wtr.flush()?;
    Ok(())
}

/// 构造 `kind="csv_mask"` 的报告。
pub fn build_csv_mask_report(source: &str, result: &MaskResult) -> Report {
    let summary = serde_yml::to_value(&result.summary)
        .unwrap_or_else(|_| serde_yml::Value::Null);
    // extra 记 skipped_fields
    let mut extra_map = serde_yml::Mapping::new();
    extra_map.insert(
        serde_yml::Value::String("skipped_fields".into()),
        serde_yml::Value::Sequence(
            result
                .skipped_fields
                .iter()
                .map(|s| serde_yml::Value::String(s.clone()))
                .collect(),
        ),
    );
    Report {
        source: source.to_string(),
        kind: "csv_mask".to_string(),
        summary,
        findings: vec![],
        extra: serde_yml::Value::Mapping(extra_map),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{mask_pipeline, MaskSummary};
    use crate::readers::{CsvReader, SourceReader};
    use crate::rules::{MaskRule, RuleSet};
    use serde_yml::Value;
    use std::collections::HashMap;

    /// 构造 phone 列通用模板脱敏参数（等价旧 phone_mask 预置）。
    fn phone_template_params() -> HashMap<String, Value> {
        let mut p = HashMap::new();
        p.insert("keep_prefix".into(), Value::Number(3.into()));
        p.insert("keep_suffix".into(), Value::Number(4.into()));
        p.insert("mask_char".into(), Value::String("*".into()));
        p.insert("mask_min_len".into(), Value::Number(4.into()));
        p.insert("min_len".into(), Value::Number(11.into()));
        p.insert("max_len".into(), Value::Number(11.into()));
        p.insert("cjk".into(), Value::Bool(false));
        p
    }

    #[test]
    fn writes_and_builds_report_for_sample() {
        let csv_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/samples/csv/sample_mask.csv");
        let records = CsvReader::new().read(&csv_path).expect("read");
        let rules = RuleSet {
            validators: vec![],
            maskers: vec![MaskRule {
                field: "phone".into(),
                masker: "template".into(),
                params: Some(phone_template_params()),
            description: None,            tags: Vec::new(),            }],
        };
        let result = mask_pipeline(&records, &rules).expect("mask");

        let out = std::env::temp_dir().join("rut0_masked_out.csv");
        write_masked_csv(&result.masked, &out).expect("write");
        assert!(out.exists());

        // 把 masked.csv 再读回来确认
        let back = CsvReader::new().read(&out).expect("read back");
        assert_eq!(back.headers, records.headers);
        assert_eq!(back.rows[0][3], "138****5678");
        let _ = std::fs::remove_file(&out);

        // 报告
        let report = build_csv_mask_report("sample_mask.csv", &result);
        assert_eq!(report.kind, "csv_mask");
        assert!(report.findings.is_empty());
        // summary 含 total_rows / masked_rows / by_field
        let s: MaskSummary =
            serde_yml::from_value(report.summary.clone()).expect("summary roundtrip");
        assert_eq!(s.total_rows, 2);
        assert_eq!(s.masked_rows, 2);
        assert_eq!(s.by_field.get("phone").copied().unwrap_or(0), 2);
    }
}
