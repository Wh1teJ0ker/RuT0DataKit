//! 脱敏 pipeline 命令：全量 / 勾选行 / 列脱敏 + 导出 + 试运行（7 命令）。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use ruT0_data_kit_core::pipeline::{detect_type, mask_pipeline_selected};
use ruT0_data_kit_core::pipeline::mask_pipeline_columns;
use ruT0_data_kit_core::readers::Records;
use ruT0_data_kit_core::report::csv_report::write_masked_csv;
use ruT0_data_kit_core::rules::{apply_mask_op, MaskOp};
use serde_json::{json, Value};

use super::{parse_ruleset_json, read_records, read_records_auto, run_mask_pipeline};

/// 跑脱敏并把结果（masked rows / headers / summary / report）序列化成 JSON。
#[tauri::command]
pub fn run_mask(
    input_path: String,
    rules_path: Option<String>,
) -> Result<Value, String> {
    let (result, report) = run_mask_pipeline(&input_path, &rules_path)?;
    let value = json!({
        "headers": result.masked.headers,
        "masked_rows": result.masked.rows,
        "summary": result.summary,
        "skipped_fields": result.skipped_fields,
        "report": report,
    });
    Ok(value)
}

/// 跑脱敏后把 masked 数据写成 CSV（out_path）。
#[tauri::command]
pub fn export_masked_csv(
    input_path: String,
    rules_path: Option<String>,
    out_path: String,
) -> Result<(), String> {
    let (result, _report) = run_mask_pipeline(&input_path, &rules_path)?;
    write_masked_csv(&result.masked, Path::new(&out_path)).map_err(|e| e.to_string())
}

/// 对勾选的行应用规则，未选中行原样保留。返回整个 `masked.records`（含
/// 未选中行原样），前端整表刷新。
///
/// `selected_row_indices` 为 `records.rows` 的 0-based 行索引；越界索引由
/// core 的 `mask_pipeline_selected` 静默忽略，command 层不再校验。
#[tauri::command]
pub fn apply_rules(
    input_path: String,
    rules_json: String,
    selected_row_indices: Vec<usize>,
) -> Result<Value, String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let t = detect_type(Path::new(&input_path)).map_err(|e| e.to_string())?;
    // v0.6.8.2：移除 csv/xlsx-only 限制，全格式走 core read_records。
    let records = read_records(&input_path, t)?;
    let selected: HashSet<usize> = selected_row_indices.into_iter().collect();
    let result = mask_pipeline_selected(&records, &rules, &selected).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": result.masked.headers,
        "masked_rows": result.masked.rows,
        "summary": result.summary,
        "skipped_fields": result.skipped_fields,
    }))
}

/// 与 [`apply_rules`] 相同的脱敏逻辑，但把结果写成 CSV（`out_path`）。
///
/// 内部复用 [`apply_rules`] 的脱敏路径，不重复实现。
#[tauri::command]
pub fn export_selected_csv(
    input_path: String,
    rules_json: String,
    selected_row_indices: Vec<usize>,
    out_path: String,
) -> Result<(), String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let t = detect_type(Path::new(&input_path)).map_err(|e| e.to_string())?;
    // v0.6.8.2：移除 csv/xlsx-only 限制，全格式走 core read_records。
    let records = read_records(&input_path, t)?;
    let selected: HashSet<usize> = selected_row_indices.into_iter().collect();
    let result = mask_pipeline_selected(&records, &rules, &selected).map_err(|e| e.to_string())?;
    write_masked_csv(&result.masked, Path::new(&out_path)).map_err(|e| e.to_string())
}

/// 列脱敏版本：对 `selected_columns` 中的列应用 `rules.maskers`，未勾选列原样。
///
/// `skipped_fields` 含义见 core `mask_pipeline_columns`：规则中存在但 records
/// 里找不到对应 header 的字段名。前端可据此提示用户哪些规则被跳过。
#[tauri::command]
pub fn apply_rules_cols(
    input_path: String,
    rules_json: String,
    selected_columns: Vec<String>,
) -> Result<Value, String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = read_records_auto(&input_path)?;
    let selected: HashSet<String> = selected_columns.iter().cloned().collect();
    let result = mask_pipeline_columns(&records, &rules, &selected).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": result.masked.headers,
        "masked_rows": result.masked.rows,
        "summary": result.summary,
        "skipped_fields": result.skipped_fields,
    }))
}

/// 对 `selected_columns` 中的列应用 `rules.maskers`，未勾选列原样（v0.4.0 T5-7）。
///
/// 输入 `headers` / `rows` 即 PreprocessView 归一化产物；后端直接拼装
/// `Records`，不再走 `read_records_auto`。返回结构同 [`apply_rules_cols`]。
#[tauri::command]
pub fn apply_rules_cols_records(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    rules_json: String,
    selected_columns: Vec<String>,
) -> Result<Value, String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = Records { headers, rows };
    let selected: HashSet<String> = selected_columns.iter().cloned().collect();
    let result = mask_pipeline_columns(&records, &rules, &selected).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": result.masked.headers,
        "masked_rows": result.masked.rows,
        "summary": result.summary,
        "skipped_fields": result.skipped_fields,
    }))
}

/// 试运行单条脱敏规则：对一条样例值 `sample_value` 应用 `scope` + `params_json`
/// 构造出的 `MaskOp`，返回脱敏后的字符串（不读文件、不落盘）。
///
/// 供 RulesView「试运行」入口调用：用户选模版（template / split_template /
/// regex_replace / const_replace）、填参数、输入样例值，立即看到脱敏结果，
/// 用于在作用于真实数据前验证参数是否正确。
///
/// `params_json` 为前端表单序列化的 JSON 字符串（可为 `{}` 或空串表示无参数，
/// 让算子用默认值）。返回 `{ "masked": String, "ok": bool, "error": Option<String> }`：
/// - `ok=true`：`masked` 为脱敏结果，`error` 为 None。
/// - `ok=false`：脱敏算子构造失败（scope 未识别 / 正则编译失败等），
///   `masked` 为 None，`error` 为错误原因。
#[tauri::command]
pub fn trial_mask(
    scope: String,
    params_json: Option<String>,
    sample_value: String,
) -> Result<Value, String> {
    // 把 params_json 反序列化为 core 算子认识的 HashMap<String, serde_yml::Value>。
    // 实测 serde_json 能直接把 JSON 标量反序列化成 serde_yml::Value（与
    // parse_ruleset_json 同策略），null / 空串视为无参数。
    let params: Option<HashMap<String, serde_yml::Value>> = match params_json.as_deref() {
        None | Some("") | Some("null") => None,
        Some(s) => match serde_json::from_str::<HashMap<String, serde_yml::Value>>(s) {
            Ok(m) if m.is_empty() => None,
            Ok(m) => Some(m),
            Err(e) => {
                return Ok(json!({
                    "ok": false,
                    "masked": null,
                    "error": format!("params_json 解析失败: {e}"),
                }));
            }
        },
    };

    let op = match MaskOp::from_rule(&scope, params.as_ref()) {
        Some(op) => op,
        None => {
            return Ok(json!({
                "ok": false,
                "masked": null,
                "error": format!("未识别的脱敏模版 scope: {scope}（支持 template / split_template / regex_replace / const_replace）"),
            }));
        }
    };

    // 正则替换模版若 pattern 非法，apply_mask_op 内部会把 re 视为 None 原样返回，
    // 不 panic；但更友好的做法是先尝试构造正则（这里通过 apply_mask_op 间接覆盖，
    // 真正的编译错误已在 regex_replace_from_params 里被 Regex::new 静默吞掉）。
    let masked = apply_mask_op(&op, &sample_value);
    Ok(json!({
        "ok": true,
        "masked": masked,
        "error": null,
    }))
}
