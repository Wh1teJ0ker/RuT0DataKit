//! 校验 pipeline 命令（2 命令）。

use ruT0_data_kit_core::pipeline::validate_pipeline;
use ruT0_data_kit_core::readers::Records;
use serde_json::{json, Value};

use super::{parse_ruleset_json, read_records_auto};

/// 跑校验 pipeline：对 `rules.validators` 逐 cell 校验，产出 valid_matrix。
#[tauri::command]
pub fn run_validate(input_path: String, rules_json: String) -> Result<Value, String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = read_records_auto(&input_path)?;
    let result = validate_pipeline(&records, &rules).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": result.headers,
        "rows": result.rows,
        "valid_matrix": result.valid_matrix,
        "summary": result.summary,
    }))
}

/// 跑校验 pipeline：对 `rules.validators` 逐 cell 校验，产出 valid_matrix（v0.4.0 T5-7）。
///
/// 输入 `headers` / `rows` 即 PreprocessView 归一化产物；返回结构同
/// [`run_validate`]。
#[tauri::command]
pub fn run_validate_records(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    rules_json: String,
) -> Result<Value, String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = Records { headers, rows };
    let result = validate_pipeline(&records, &rules).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": result.headers,
        "rows": result.rows,
        "valid_matrix": result.valid_matrix,
        "summary": result.summary,
    }))
}
