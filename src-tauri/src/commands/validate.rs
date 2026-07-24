//! 校验 pipeline 命令（2 命令） + 单条试运行（v0.6.2 新增 trial_validate）。

use std::collections::HashMap;

use ruT0_data_kit_core::pipeline::validate_pipeline;
use ruT0_data_kit_core::readers::Records;
use ruT0_data_kit_core::rules::types::FieldRule;
use ruT0_data_kit_core::rules::{apply_validate_op, ValidateOp};
use serde_json::{json, Value};
use serde_yml;

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

/// 试运行单条校验规则：对一条样例值 `sample_value` 应用 `scope` + `params_json`
/// 构造出的 `ValidateOp`，返回校验结果（不读文件、不落盘）。
///
/// 与 `trial_mask` 对称：供 RulesView「试运行」入口调用。用户选校验模版
/// （当前仅 `regex`，后续可扩展 `algorithm` / `regex_with_guard`）、填参数
/// （pattern / message / empty_message）、输入样例值，立即看到校验结果，
/// 用于在作用于真实数据前验证参数是否正确。
///
/// `params_json` 为前端表单序列化的 JSON 字符串（可为 `{}` 或空串表示无参数）。
/// 返回 `{ ok: bool, valid: bool|null, message: Option<String>, error: Option<String> }`：
/// - `ok=true`：算子构造成功，`valid` 为校验结果，`message` 为失败时的消息
///   （合法时为 None），`error` 为 None。
/// - `ok=false`：算子构造失败（scope 未识别），`valid` 为 null，
///   `error` 为错误原因。
#[tauri::command]
pub fn trial_validate(
    scope: String,
    params_json: Option<String>,
    sample_value: String,
) -> Result<Value, String> {
    // 镜像 trial_mask 的 params_json 解析策略：JSON 标量反序列化成
    // serde_yml::Value，null / 空串视为无参数。
    let params: Option<HashMap<String, serde_yml::Value>> = match params_json.as_deref() {
        None | Some("") | Some("null") => None,
        Some(s) => match serde_json::from_str::<HashMap<String, serde_yml::Value>>(s) {
            Ok(m) if m.is_empty() => None,
            Ok(m) => Some(m),
            Err(e) => {
                return Ok(json!({
                    "ok": false,
                    "valid": null,
                    "message": null,
                    "error": format!("params_json 解析失败: {e}"),
                }));
            }
        },
    };

    // 合成一条 FieldRule，走 ValidateOp::from_rule 构造通用算子。
    let rule = FieldRule {
        field: String::new(),
        scope,
        tag: "validate".to_string(),
        params,
        message: None,
        description: None,
    };

    let op = match ValidateOp::from_rule(&rule) {
        Some(op) => op,
        None => {
            return Ok(json!({
                "ok": false,
                "valid": null,
                "message": null,
                "error": "未识别的校验算子 scope（当前支持 regex）".to_string(),
            }));
        }
    };

    let result = apply_validate_op(&op, &sample_value);
    Ok(json!({
        "ok": true,
        "valid": result.valid,
        "message": if result.valid { Value::Null } else { Value::from(result.message.clone()) },
        "error": null,
    }))
}
