//! Tauri commands for RuT0DataKit GUI.
//!
//! v0.1.0 主入口：脱敏（csv/xlsx）+ 导出。pcap/log/校验按钮置灰占位。
//!
//! 状态策略：每个 command 独立重算（v0.1.0 最简），不在 command 间缓存
//! `MaskResult`。若需缓存可后续引入 `tauri::State<Mutex<...>>`。
//!
//! 命令清单（共 20 个）：
//! - `select_file` / `detect_source_type` / `run_mask` / `export_masked_csv`：
//!   v0.1.0 原命令（4 个），保留不破坏。
//! - `load_preview` / `apply_rules` / `export_selected_csv`：T0-11 新增（3 个），
//!   对接 T0-10 的 `mask_pipeline_selected`，给前端提供「导入预览 + 勾选行
//!   应用规则 + 导出」的能力。`rules_json` 为 RuleSet 的 JSON 序列化，
//!   前端编辑后直接传入；`serde_json` 反序列化到 core 的 `RuleSet`（其
//!   `params` 为 `HashMap<String, serde_yml::Value>`，serde_json 能直接
//!   反序列化字符串/数字/布尔等标量到 `serde_yml::Value`）。
//! - `apply_rules_cols` / `run_validate` / `export_records_csv` /
//!   `export_records_xlsx` / `save_ruleset` / `read_ruleset`：T0-16 新增
//!   （6 个），对接列选择脱敏 + 校验 pipeline + CSV/XLSX 导出 + 规则 YAML
//!   存取；`export_records_xlsx` 用 `rust_xlsxwriter` 写 .xlsx。
//! - `preview_mask_rule` / `preview_validate_rule` / `list_mask_op_types` /
//!   `list_validate_op_types`：T0-22 新增（4 个），对接 T0-21 的抽象算子
//!   `MaskOp` / `ValidateOp` + 预置库 + `apply_mask_op` / `apply_validate_op`
//!   公共 API，为前端 RulesView 的「试运行」按钮与下拉源提供后端能力。
//! - `preview_mask_rule_value` / `preview_validate_rule_value`：
//!   T0-23 新增（2 个），直接对用户输入值跑算子，规则管理试运行不再依赖
//!   已导入文件的首行。
//! - `scan_log_file`：T2-5 新增（1 个），v0.2.0 日志扫描 GUI 入口：读 .log
//!   → 跑 core `log_scan::scan_log`（签名引擎 + 弱口令 grep + 敏感扫描）
//!   → 一次 IPC 返回 `{ entries, report }`，前端 LogView 同时拿到原始日志
//!   与 findings/summary，避免再开一条 read_log_file 命令。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

use ruT0_data_kit_core::log::LogReader;
use ruT0_data_kit_core::logsign::SignatureEngine;
use ruT0_data_kit_core::pipeline::{
    detect_type, log_scan::scan_log, mask_pipeline, mask_pipeline_columns,
    mask_pipeline_selected, validate_pipeline, SourceType,
};
use ruT0_data_kit_core::readers::{CsvReader, SourceReader, XlsxReader};
use ruT0_data_kit_core::report::csv_report::{build_csv_mask_report, write_masked_csv};
use ruT0_data_kit_core::rules::{
    apply_mask_op, apply_validate_op, build_validator,
    list_mask_op_types as core_list_mask_op_types,
    list_validate_op_types as core_list_validate_op_types, load_default_mask_ruleset,
    load_ruleset, FieldRule, MaskOp, RuleSet, ValidateOp, ValidatorRegistry,
};
use ruT0_data_kit_core::scan::DefaultSensitiveScan;
use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

/// 把 [`SourceType`] 序列化为前端可读的小写字符串。
fn source_type_name(t: SourceType) -> &'static str {
    match t {
        SourceType::Csv => "csv",
        SourceType::Xlsx => "xlsx",
        SourceType::Log => "log",
        SourceType::Pcap => "pcap",
        SourceType::Unknown => "unknown",
    }
}

/// 弹出文件选择对话框，返回选中文件的绝对路径字符串。
///
/// 用户取消返回 `Ok(None)`。v0.1.0 用单选模式（一次选一个文件）。
#[tauri::command]
pub async fn select_file(app: AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<std::path::PathBuf>>();
    app.dialog()
        .file()
        .add_filter("数据文件", &["csv", "xlsx", "log", "pcap", "pcapng"])
        .pick_file(move |path| {
            let v = path.and_then(|p| p.into_path().ok());
            let _ = tx.send(v);
        });

    let chosen = rx.recv().map_err(|e| e.to_string())?;
    Ok(chosen.map(|p| p.to_string_lossy().into_owned()))
}

/// 调 core 的 `detect_type` 返回数据源类型名（csv/xlsx/log/pcap/unknown）。
#[tauri::command]
pub fn detect_source_type(path: String) -> Result<String, String> {
    let t = detect_type(Path::new(&path)).map_err(|e| e.to_string())?;
    Ok(source_type_name(t).to_string())
}

/// 加载规则：`rules_path` 为 None 时用内置 `default_mask`，Some 时按路径加载。
fn resolve_rules(rules_path: &Option<String>) -> Result<RuleSet, String> {
    match rules_path {
        Some(p) => load_ruleset(p).map_err(|e| e.to_string()),
        None => load_default_mask_ruleset().map_err(|e| e.to_string()),
    }
}

/// 按源类型选择 reader 读取文件，返回 [`ruT0_data_kit_core::readers::Records`]。
fn read_records(path: &str, t: SourceType) -> Result<ruT0_data_kit_core::readers::Records, String> {
    let p = Path::new(path);
    match t {
        SourceType::Csv => CsvReader::new().read(p).map_err(|e| e.to_string()),
        SourceType::Xlsx => XlsxReader::new().read(p).map_err(|e| e.to_string()),
        _ => Err("v0.1.0 仅支持 csv/xlsx".into()),
    }
}

/// 执行脱敏 pipeline 并构造报告。返回 `(MaskResult, Report)` 的 JSON 表达。
///
/// 复用给 `run_mask` / `export_masked_csv`，避免状态共享。
fn run_mask_pipeline(
    input_path: &str,
    rules_path: &Option<String>,
) -> Result<(ruT0_data_kit_core::pipeline::MaskResult, ruT0_data_kit_core::report::Report), String> {
    let t = detect_type(Path::new(input_path)).map_err(|e| e.to_string())?;
    if t != SourceType::Csv && t != SourceType::Xlsx {
        return Err("v0.1.0 仅支持 csv/xlsx".into());
    }
    let rules = resolve_rules(rules_path)?;
    let records = read_records(input_path, t)?;
    let result = mask_pipeline(&records, &rules).map_err(|e| e.to_string())?;
    let report = build_csv_mask_report(input_path, &result);
    Ok((result, report))
}

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

/// 读取 CSV/XLSX 返回预览所需的完整数据。前端自行截断显示行数。
///
/// 与原 v0.1.0「不回传原值」策略不同：此为预览场景，前端只显示当前数据，
/// 不提供「原值表」对比。`rows` 为全量数据行（不含表头）。
#[tauri::command]
pub fn load_preview(path: String) -> Result<Value, String> {
    let t = detect_type(Path::new(&path)).map_err(|e| e.to_string())?;
    if t != SourceType::Csv && t != SourceType::Xlsx {
        return Err("v0.1.0 仅支持 csv/xlsx".into());
    }
    let records = read_records(&path, t)?;
    Ok(json!({
        "headers": records.headers,
        "rows": records.rows,
        "source_type": source_type_name(t),
        "row_count": records.rows.len(),
    }))
}

/// 把前端编辑的 `rules_json`（RuleSet 的 JSON 序列化）反序列化到 core 的
/// [`RuleSet`]。
///
/// core 的 `MaskRule.params` 为 `Option<HashMap<String, serde_yml::Value>>`。
/// 实测 serde_json 能直接把 JSON 标量反序列化成 `serde_yml::Value`
/// （Number/String/Bool/Sequence/Mapping 互通），无需中间转换结构。
fn parse_ruleset_json(rules_json: &str) -> Result<RuleSet, String> {
    serde_json::from_str::<RuleSet>(rules_json).map_err(|e| format!("rules_json 解析失败: {e}"))
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
    if t != SourceType::Csv && t != SourceType::Xlsx {
        return Err("v0.1.0 仅支持 csv/xlsx".into());
    }
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
    if t != SourceType::Csv && t != SourceType::Xlsx {
        return Err("v0.1.0 仅支持 csv/xlsx".into());
    }
    let records = read_records(&input_path, t)?;
    let selected: HashSet<usize> = selected_row_indices.into_iter().collect();
    let result = mask_pipeline_selected(&records, &rules, &selected).map_err(|e| e.to_string())?;
    write_masked_csv(&result.masked, Path::new(&out_path)).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────────────
// T0-16 新增命令：列脱敏 + 校验 + 导出 + 规则集落盘
// ─────────────────────────────────────────────────────────────────────

/// `read_records` 的无 `SourceType` 入参版本：自动 detect。仅支持 csv/xlsx，
/// 其他类型报错。
fn read_records_auto(input_path: &str) -> Result<ruT0_data_kit_core::readers::Records, String> {
    let t = detect_type(Path::new(input_path)).map_err(|e| e.to_string())?;
    if t != SourceType::Csv && t != SourceType::Xlsx {
        return Err("v0.1.0 仅支持 csv/xlsx".into());
    }
    read_records(input_path, t)
}

/// 按 `column_order` 顺序选出在 `selected_columns` 中的列，返回投影后的
/// `(headers, rows)`。
///
/// - `column_order` 中不在原 `headers` 的列名忽略。
/// - `selected_columns` 控制哪些列被保留；`column_order` 控制顺序。
/// - `column_order` 为空时退化为「按 selected_columns 顺序，再按 headers 顺序」。
fn project_columns(
    records: &ruT0_data_kit_core::readers::Records,
    selected_columns: &[String],
    column_order: &[String],
) -> (Vec<String>, Vec<Vec<String>>) {
    let selected: HashSet<&str> = selected_columns.iter().map(|s| s.as_str()).collect();
    let header_idx: std::collections::HashMap<&str, usize> = records
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.as_str(), i))
        .collect();

    // 决定输出列序：优先 column_order，column_order 为空时按 selected_columns 顺序。
    let order: Vec<&str> = if !column_order.is_empty() {
        column_order
            .iter()
            .map(|s| s.as_str())
            .filter(|name| selected.contains(name) && header_idx.contains_key(*name))
            .collect()
    } else {
        selected_columns
            .iter()
            .map(|s| s.as_str())
            .filter(|name| header_idx.contains_key(*name))
            .collect()
    };

    let out_headers: Vec<String> = order
        .iter()
        .map(|name| records.headers[header_idx[*name]].clone())
        .collect();
    let out_indices: Vec<usize> = order.iter().map(|name| header_idx[*name]).collect();
    let out_rows: Vec<Vec<String>> = records
        .rows
        .iter()
        .map(|row| out_indices.iter().map(|&i| row.get(i).cloned().unwrap_or_default()).collect())
        .collect();
    (out_headers, out_rows)
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

/// 应用列脱敏 → 按 `column_order` 投影列 → 可选按行索引过滤 → 写 CSV。
///
/// `selected_row_indices = None` 表示导出全部行；`Some(vec)` 只导出指定行号
/// （0-based，越界忽略）。投影后行号按原 records 顺序。
#[tauri::command]
pub fn export_records_csv(
    input_path: String,
    rules_json: String,
    selected_columns: Vec<String>,
    column_order: Vec<String>,
    selected_row_indices: Option<Vec<usize>>,
    out_path: String,
) -> Result<(), String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = read_records_auto(&input_path)?;
    let selected: HashSet<String> = selected_columns.iter().cloned().collect();
    let result = mask_pipeline_columns(&records, &rules, &selected).map_err(|e| e.to_string())?;
    let (headers, mut rows) = project_columns(&result.masked, &selected_columns, &column_order);
    if let Some(indices) = selected_row_indices {
        let idx_set: HashSet<usize> = indices.into_iter().collect();
        rows = rows
            .into_iter()
            .enumerate()
            .filter(|(i, _)| idx_set.contains(i))
            .map(|(_, r)| r)
            .collect();
    }
    write_csv(&headers, &rows, &out_path)
}

/// 与 [`export_records_csv`] 同签名，但写 XLSX（rust_xlsxwriter）。
#[tauri::command]
pub fn export_records_xlsx(
    input_path: String,
    rules_json: String,
    selected_columns: Vec<String>,
    column_order: Vec<String>,
    selected_row_indices: Option<Vec<usize>>,
    out_path: String,
) -> Result<(), String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = read_records_auto(&input_path)?;
    let selected: HashSet<String> = selected_columns.iter().cloned().collect();
    let result = mask_pipeline_columns(&records, &rules, &selected).map_err(|e| e.to_string())?;
    let (headers, mut rows) = project_columns(&result.masked, &selected_columns, &column_order);
    if let Some(indices) = selected_row_indices {
        let idx_set: HashSet<usize> = indices.into_iter().collect();
        rows = rows
            .into_iter()
            .enumerate()
            .filter(|(i, _)| idx_set.contains(i))
            .map(|(_, r)| r)
            .collect();
    }
    write_xlsx(&headers, &rows, &out_path)
}

/// 把 `rules_json`（RuleSet 的 JSON 序列化）反序列化校验后用 `serde_yml`
/// 序列化为 YAML 写盘。`RuleSet` 本身只 derive `Deserialize`，这里先解析
/// 成 `serde_json::Value`（既能校验 JSON 合法性，又能直接给 serde_yml 序列化）。
#[tauri::command]
pub fn save_ruleset(rules_json: String, out_path: String) -> Result<(), String> {
    // 先反序列化为 RuleSet 校验结构合法性（field/validator/masker 字段齐全）。
    let _rules = parse_ruleset_json(&rules_json)?;
    // 再把原始 JSON 解析为 Value，交给 serde_yml 输出 YAML。这样无需 RuleSet
    // 实现 Serialize（core 端 RuleSet 只 derive Deserialize，T0-16 不动 core）。
    let value: Value = serde_json::from_str(&rules_json)
        .map_err(|e| format!("rules_json 解析失败: {e}"))?;
    let yaml = serde_yml::to_string(&value).map_err(|e| format!("YAML 序列化失败: {e}"))?;
    std::fs::write(&out_path, yaml).map_err(|e| format!("写入失败: {e}"))
}

/// 读取 YAML 规则集文件内容为字符串返回。前端用 js-yaml 解析后 dispatch SET_RULES。
///
/// 只做读盘，不做结构校验；校验交给前端的 SET_RULES reducer 与再次 save_ruleset
/// 时的 parse_ruleset_json。这样允许用户加载部分可解析的 YAML（例如含注释）。
#[tauri::command]
pub fn read_ruleset(path: String) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("读取失败: {e}"))
}

// ─────────────────────────────────────────────────────────────────────
// T0-22 新增命令：规则试运行 + 算子类型清单
// ─────────────────────────────────────────────────────────────────────

/// 读取首行指定字段的值作为试运行输入；空数据时返回空串。
///
/// 复用给 `preview_mask_rule` / `preview_validate_rule`，避免重复实现「找列索引
/// + 取首行」逻辑。找不到 `field` 时返回 `Err("field {field} not found in headers")`。
fn preview_read_field(input_path: &str, field: &str) -> Result<String, String> {
    let records = read_records_auto(input_path)?;
    let col_idx = records
        .headers
        .iter()
        .position(|h| h == field)
        .ok_or_else(|| format!("field {field} not found in headers"))?;
    let input = records
        .rows
        .first()
        .and_then(|r| r.get(col_idx))
        .cloned()
        .unwrap_or_default();
    Ok(input)
}

/// 试运行单条脱敏规则：读首行 `field` 列值 → 用 `MaskOp::from_rule` 构造算子 →
/// `apply_mask_op` → 返回 `{ field, input, output }`。
#[tauri::command]
pub fn preview_mask_rule(
    input_path: String,
    field: String,
    masker: String,
    params: Option<HashMap<String, serde_yml::Value>>,
) -> Result<Value, String> {
    let input = preview_read_field(&input_path, &field)?;
    let op = MaskOp::from_rule(&masker, params.as_ref())
        .ok_or_else(|| format!("unknown masker: {masker}"))?;
    let output = apply_mask_op(&op, &input);
    Ok(json!({
        "field": field,
        "input": input,
        "output": output,
    }))
}

/// 试运行单条校验规则：读首行 `field` 列值 → 构造临时 `FieldRule` → 优先
/// `ValidateOp::from_rule`，失败时回退到 `ValidatorRegistry` + `build_validator`
/// （保留自定义注册项兜底）→ 返回 `{ field, input, valid, message }`。
#[tauri::command]
pub fn preview_validate_rule(
    input_path: String,
    field: String,
    validator: String,
    params: Option<HashMap<String, serde_yml::Value>>,
    regex: Option<String>,
    message: Option<String>,
) -> Result<Value, String> {
    let input = preview_read_field(&input_path, &field)?;
    let rule = FieldRule {
        field: field.clone(),
        validator: validator.clone(),
        params: params.clone(),
        regex: regex.clone(),
        message: message.clone(),
        description: None,
    };
    let result = if let Some(op) = ValidateOp::from_rule(&rule) {
        apply_validate_op(&op, &input)
    } else {
        let reg = ValidatorRegistry::new();
        let v = build_validator(&rule, &reg)
            .ok_or_else(|| format!("unknown validator: {validator}"))?;
        v.validate(&input)
    };
    Ok(json!({
        "field": field,
        "input": input,
        "valid": result.valid,
        "message": result.message,
    }))
}

/// 试运行单条脱敏规则（直接接受输入值，不依赖任何文件）：
/// 用 `MaskOp::from_rule` 构造算子 → `apply_mask_op` → 返回
/// `{ input, output }`。
///
/// 规则管理是独立系统，试运行不再读取已导入数据文件的首行；
/// 调用方（前端 Drawer / 规则卡片）传入用户手动输入的样例值即可。
#[tauri::command]
pub fn preview_mask_rule_value(
    input: String,
    masker: String,
    params: Option<HashMap<String, serde_yml::Value>>,
) -> Result<Value, String> {
    let op = MaskOp::from_rule(&masker, params.as_ref())
        .ok_or_else(|| format!("unknown masker: {masker}"))?;
    let output = apply_mask_op(&op, &input);
    Ok(json!({
        "input": input,
        "output": output,
    }))
}

/// 试运行单条校验规则（直接接受输入值，不依赖任何文件）：
/// 构造临时 `FieldRule` → 优先 `ValidateOp::from_rule`，失败时回退到
/// `ValidatorRegistry` + `build_validator`（保留自定义注册项兜底）→
/// 返回 `{ input, valid, message }`。
///
/// 规则管理是独立系统，试运行不再读取已导入数据文件的首行；
/// 调用方传入用户手动输入的样例值即可。
#[tauri::command]
pub fn preview_validate_rule_value(
    input: String,
    validator: String,
    params: Option<HashMap<String, serde_yml::Value>>,
    regex: Option<String>,
    message: Option<String>,
) -> Result<Value, String> {
    let rule = FieldRule {
        field: String::new(),
        validator: validator.clone(),
        params: params.clone(),
        regex: regex.clone(),
        message: message.clone(),
        description: None,
    };
    let result = if let Some(op) = ValidateOp::from_rule(&rule) {
        apply_validate_op(&op, &input)
    } else {
        let reg = ValidatorRegistry::new();
        let v = build_validator(&rule, &reg)
            .ok_or_else(|| format!("unknown validator: {validator}"))?;
        v.validate(&input)
    };
    Ok(json!({
        "input": input,
        "valid": result.valid,
        "message": result.message,
    }))
}

/// 返回脱敏算子类型清单（通用算子 + 预置别名），供前端下拉源使用。
#[tauri::command]
pub fn list_mask_op_types() -> Result<Value, String> {
    let items: Vec<Value> = core_list_mask_op_types()
        .into_iter()
        .map(|(n, l)| json!({ "name": n, "label": l }))
        .collect();
    Ok(Value::Array(items))
}

/// 返回校验算子类型清单（通用算子 + 预置别名），供前端下拉源使用。
#[tauri::command]
pub fn list_validate_op_types() -> Result<Value, String> {
    let items: Vec<Value> = core_list_validate_op_types()
        .into_iter()
        .map(|(n, l)| json!({ "name": n, "label": l }))
        .collect();
    Ok(Value::Array(items))
}

/// 用 csv crate 写 UTF-8 CSV（表头 + 数据行）。
fn write_csv(headers: &[String], rows: &[Vec<String>], out_path: &str) -> Result<(), String> {
    let file = File::create(out_path).map_err(|e| format!("创建文件失败: {e}"))?;
    let mut wtr = csv::Writer::from_writer(file);
    wtr.write_record(headers).map_err(|e| e.to_string())?;
    for row in rows {
        wtr.write_record(row).map_err(|e| e.to_string())?;
    }
    wtr.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// 用 rust_xlsxwriter 写 XLSX（表头第 0 行，数据从第 1 行起）。
fn write_xlsx(headers: &[String], rows: &[Vec<String>], out_path: &str) -> Result<(), String> {
    use rust_xlsxwriter::{Format, Workbook};
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let header_fmt = Format::new().set_bold();
    for (c, h) in headers.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, h, &header_fmt)
            .map_err(|e| e.to_string())?;
    }
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            ws.write_string((r + 1) as u32, c as u16, cell)
                .map_err(|e| e.to_string())?;
        }
    }
    wb.save(out_path).map_err(|e| e.to_string())?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// T2-5 新增命令：日志扫描（v0.2.0）
// ─────────────────────────────────────────────────────────────────────

/// 读取 .log 文件并跑日志扫描 pipeline（签名引擎 + 弱口令 grep + 敏感扫描），
/// 一次返回 `{ entries, report }`。
///
/// - `entries`：core `LogReader::read` 解析出的 `Vec<LogEntry>`，前端用于
///   原始日志表渲染。
/// - `report`：core `log_scan::scan_log` 产出的 `Report`，含 findings +
///   summary（total_lines / sqli_hits / weak_password_hits /
///   sensitive_hits / top_attack_ips）。
///
/// 单命令返回两份数据避免再开 `read_log_file`：v0.2.0 fixture 数据量小
/// （1856 行），一次 IPC 体积可控。敏感扫描复用 `load_default_mask_ruleset`
/// 加载的 RuleSet（其 validators 为空时 sensitive findings 为空，这是可接受的，
/// HANDOFF 提到 sensitive 可空）。
#[tauri::command]
pub fn scan_log_file(path: String) -> Result<Value, String> {
    let reader = LogReader::new().map_err(|e| e.to_string())?;
    let entries = reader.read(Path::new(&path)).map_err(|e| e.to_string())?;
    let engine = SignatureEngine::new().map_err(|e| e.to_string())?;
    let scan = DefaultSensitiveScan::new();
    let rules = load_default_mask_ruleset().unwrap_or_default();
    let report = scan_log(&entries, &engine, &scan, &rules);
    let report_value = serde_json::to_value(&report).map_err(|e| e.to_string())?;
    let entries_value = serde_json::to_value(&entries).map_err(|e| e.to_string())?;
    Ok(json!({
        "entries": entries_value,
        "report": report_value,
    }))
}
