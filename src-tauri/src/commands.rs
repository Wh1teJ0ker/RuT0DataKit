//! Tauri commands for RuT0DataKit GUI.
//!
//! v0.1.0 主入口：脱敏（csv/xlsx）+ 导出。pcap/log/校验按钮置灰占位。
//!
//! 状态策略：每个 command 独立重算（v0.1.0 最简），不在 command 间缓存
//! `MaskResult`。若需缓存可后续引入 `tauri::State<Mutex<...>>`。
//!
//! 命令清单（共 21 个）：
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
//! - `list_rule_tags`：T5-4 新增（1 个），返回预置标签 ∪ 当前 ruleset 出现
//!   过的 tag 的并集 `Vec<String>`，供前端 tags Select 下拉源与「按标签过滤」
//!   使用。
//! - `scan_log_file`：T2-5 新增（1 个），v0.2.0 日志扫描 GUI 入口：读 .log
//!   → 跑 core `log_scan::scan_log`（签名引擎 + 弱口令 grep + 敏感扫描）
//!   → 一次 IPC 返回 `{ entries, report }`，前端 LogView 同时拿到原始日志
//!   与 findings/summary，避免再开一条 read_log_file 命令。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

use ruT0_data_kit_core::log::LogReader;
use ruT0_data_kit_core::logsign::SignatureEngine;
use ruT0_data_kit_core::pcap::{self, PcapReader};
use ruT0_data_kit_core::pipeline::{
    detect_type, log_scan::scan_log, mask_pipeline, mask_pipeline_columns,
    mask_pipeline_selected, scan_pcap, validate_pipeline, SourceType,
};
use ruT0_data_kit_core::readers::{CsvReader, SourceReader, XlsxReader};
use ruT0_data_kit_core::report::csv_report::{build_csv_mask_report, write_masked_csv};
use ruT0_data_kit_core::tools::{
    construct_regex as core_construct_regex, explain_regex as core_explain_regex,
    parse_sqls as core_parse_sqls, ConstructedRegex, RegexTokenDesc, SqlParseInput,
    SqlParseResult,
};
use ruT0_data_kit_core::rules::{
    apply_mask_op, apply_validate_op, build_validator,
    list_mask_op_types as core_list_mask_op_types,
    list_tagged_presets, list_validate_op_types as core_list_validate_op_types,
    load_default_mask_ruleset, load_ruleset, FieldRule, MaskOp, RuleSet, ValidateOp,
    ValidatorRegistry,
};
use ruT0_data_kit_core::scan::DefaultSensitiveScan;
use ruT0_data_kit_core::extract;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// 把 [`SourceType`] 序列化为前端可读的小写字符串。
fn source_type_name(t: SourceType) -> &'static str {
    match t {
        SourceType::Csv => "csv",
        SourceType::Xlsx => "xlsx",
        SourceType::Log => "log",
        SourceType::Pcap => "pcap",
        SourceType::Sql => "sql",
        SourceType::Json => "json",
        SourceType::Txt => "txt",
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
        .add_filter("数据文件", &["csv", "xlsx", "sql", "json", "log", "pcap", "pcapng", "txt"])
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
        tags: Vec::new(),
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
        tags: Vec::new(),
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

/// 返回规则可选标签的并集：预置标签（mask / validate / sensitive 等）∪
/// 当前 `rules_json` 中 `maskers` / `validators` 实际出现过的所有 tag。
///
/// 前端 `RuleDrawer` 的 tags Select(mode=multiple) 与 `RulesView` 的「按标签
/// 过滤」Select 共用此下拉源。预置标签来自 core `list_tagged_presets`（对
/// `mask` / `validate` / `sensitive` / `search` / `sql_parse` 等已知标签取
/// 并集），用户在 ruleset 里自定义的 tag 也会被合并进来。
///
/// `rules_json` 为 None / 空字符串 / 反序列化失败时退化为仅返回预置标签，
/// 不抛错（避免 Drawer 加载阶段因临时无效 JSON 阻塞表单）。
#[tauri::command]
pub fn list_rule_tags(rules_json: Option<String>) -> Result<Vec<String>, String> {
    // 预置标签并集：对已知标签集合逐个调 list_tagged_presets，命中即纳入。
    // 这里用静态标签清单而非遍历所有 PresetEntry 的 tags，避免 core 暴露
    // 额外的「列出全部标签」API；新增标签只需在此清单追加。
    let known_tags = [
        "mask",
        "validate",
        "sensitive",
        "search",
        "sql_parse",
    ];
    let mut set: HashSet<String> = HashSet::new();
    for tag in known_tags.iter() {
        if !list_tagged_presets(tag).is_empty() {
            set.insert((*tag).to_string());
        }
    }

    // 合并 ruleset 中实际出现过的 tag。
    if let Some(json_str) = rules_json.as_deref() {
        if !json_str.trim().is_empty() {
            if let Ok(rules) = parse_ruleset_json(json_str) {
                for r in &rules.maskers {
                    for t in &r.tags {
                        set.insert(t.clone());
                    }
                }
                for r in &rules.validators {
                    for t in &r.tags {
                        set.insert(t.clone());
                    }
                }
            }
        }
    }

    let mut tags: Vec<String> = set.into_iter().collect();
    tags.sort();
    Ok(tags)
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

// ─────────────────────────────────────────────────────────────────────
// v0.3.0 pcap 扫描命令
// ─────────────────────────────────────────────────────────────────────

/// 读取 .pcap/.pcapng 文件并跑流量扫描 pipeline（tshark 子进程 + 双重 URL 解码
/// + base64 字段解码 + 敏感扫描），一次返回 `{ entries, report }`。
///
/// - `entries`：core `pcap::PcapReader::read` 解析出的 `Vec<HttpRequest>`，
///   前端用于原始 HTTP 请求表渲染。
/// - `report`：core `pipeline::scan_pcap` 产出的 `Report`，含 findings +
///   summary（total_requests / sensitive_hits / decoded_fragments /
///   top_src_ips）。
///
/// tshark 缺失时返回 `CoreError::DependencyMissing("tshark")`，前端 PcapView
/// 据此 `message.error` 弹提示并禁用按钮。fixture 是 5000 POST × JSON body
/// 单字段 base64，敏感扫描规则集与日志扫描一致（idcard/phone/name 三类）。
#[tauri::command]
pub fn scan_pcap_file(path: String) -> Result<Value, String> {
    let requests = PcapReader::read(Path::new(&path)).map_err(|e| e.to_string())?;
    let scan = DefaultSensitiveScan::new();
    let rules = pcap_sensitive_ruleset();
    let report = scan_pcap(Path::new(&path), &scan, &rules).map_err(|e| e.to_string())?;
    let report_value = serde_json::to_value(&report).map_err(|e| e.to_string())?;
    let entries_value = serde_json::to_value(&requests).map_err(|e| e.to_string())?;
    Ok(json!({
        "entries": entries_value,
        "report": report_value,
    }))
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.0 数据预处理归一化命令
// ─────────────────────────────────────────────────────────────────────

/// 把任意支持格式的文件（csv/xlsx/sql/json/pcap/log）统一读成 `Records`，
/// 前端 PreviewView / 后续 mask/validate/export 复用同一份表格。
///
/// - csv/xlsx/sql/json：按后缀走对应 reader。
/// - pcap：调 `PcapRecordsReader` 适配器（tshark 缺失时返回
///   `DependencyMissing("tshark")`，前端弹提示并禁用按钮）。
/// - log：调 `LogRecordsReader` 适配器。
/// - 不识别的后缀：返回 `Err("unsupported source type")`。
///
/// 返回 `{ headers, rows, source_type, row_count }`，与 `load_preview` 形态
/// 对齐，前端可复用同一渲染逻辑。
#[tauri::command]
pub fn preprocess_file(path: String) -> Result<Value, String> {
    let p = Path::new(&path);
    let t = detect_type(p).map_err(|e| e.to_string())?;
    let records = ruT0_data_kit_core::readers::read_records(p).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": records.headers,
        "rows": records.rows,
        "source_type": source_type_name(t),
        "row_count": records.rows.len(),
    }))
}

/// v0.3.0 pcap 默认敏感扫描规则集：idcard / phone / name 三类内置 validator。
///
/// 与日志扫描口径对齐：fixture 仅含 PII（身份证 / 中文姓名 / 手机号），不跑
/// SQLi 签名（v0.3.0 scope）。返回独立 RuleSet，不污染 mask 视图用的
/// `load_default_mask_ruleset`。
fn pcap_sensitive_ruleset() -> RuleSet {
    use ruT0_data_kit_core::rules::FieldRule;
    RuleSet {
        validators: vec![
            FieldRule {
                field: "id_card".into(),
                validator: "idcard".into(),
                params: None,
                regex: None,
                message: None,
                description: None,
                tags: Vec::new(),
            },
            FieldRule {
                field: "phone".into(),
                validator: "phone".into(),
                params: None,
                regex: None,
                message: None,
                description: None,
                tags: Vec::new(),
            },
            FieldRule {
                field: "name".into(),
                validator: "name".into(),
                params: None,
                regex: None,
                message: None,
                description: None,
                tags: Vec::new(),
            },
        ],
        maskers: vec![],
    }
}

// ─────────────────────────────────────────────────────────────────────
// v0.5.0 正则工具命令
// ─────────────────────────────────────────────────────────────────────

/// 解释正则字符串的每个 token，返回 `{ token, kind, description, position }` 列表。
///
/// 非法正则（`regex::Regex::new` 失败）返回 `Err`，不 panic。
#[tauri::command]
pub fn explain_regex(pattern: String) -> Result<Vec<RegexTokenDesc>, String> {
    core_explain_regex(&pattern).map_err(|e| e.to_string())
}

/// v0.4.1 T6-5：从自然语言描述构造正则。
///
/// 纯本地规则化推断（位数 / 字符集 / 锚定前缀 / 邮箱 / URL / 身份证），
/// 不调用网络。无法识别线索时返回 `Err`。
#[tauri::command]
pub fn regex_construct(statement: String) -> Result<ConstructedRegex, String> {
    core_construct_regex(&statement).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────────────
// v0.5.0 T5-9 SQL 解析工具命令
// ─────────────────────────────────────────────────────────────────────

/// 对一批纯 SQL 文本做盲注探针提取 + 聚类 + 数据库还原（v0.5.0 T5-9）。
///
/// 调 core 的 `tools::parse_sqls`：接受 `Vec<SqlParseInput>`（每项含 sql +
/// 可选 response_body_size + 可选 source_ip），返回 `SqlParseResult`
/// `{ probes, aggregated, reconstructed, parsed_payloads }`。
///
/// 与 `scan_log_file` 的区别：本命令不读日志文件、不依赖 `LogEntry`，前端
/// 直接粘贴 SQL 文本列表即可还原数据库结构（T5-10 GUI 调用）。
#[tauri::command]
pub fn parse_sql_tool(inputs: Vec<SqlParseInput>) -> Result<SqlParseResult, String> {
    Ok(core_parse_sqls(inputs))
}

// ─────────────────────────────────────────────────────────────────────
// v0.6.0 T5-8 新增命令：JSON 导出
// ─────────────────────────────────────────────────────────────────────

/// 与 [`export_records_csv`] 同签名同流程，但写 JSON（v0.6.0 T5-8）。
///
/// 流程：read_records_auto → mask_pipeline_columns → project_columns →
/// 可选行过滤 → 把每行按 `column_order` 映射为 `HashMap<String, String>`
/// （headers 做 key），用 `serde_json` 序列化为 JSON 数组写盘。每行扁平对象，
/// value 全为字符串，不做嵌套（v0.6.0 scope）。
#[tauri::command]
pub fn export_records_json(
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
    write_json(&headers, &rows, &out_path)
}

/// 把 `headers` + `rows` 序列化为 JSON 数组写盘。每行一个对象，key=headers[i]，
/// value=rows[r][i]（字符串）。空表头/空行也合法（输出 `[]` 或 `[{}]`）。
fn write_json(headers: &[String], rows: &[Vec<String>], out_path: &str) -> Result<(), String> {
    let arr: Vec<HashMap<&str, &str>> = rows
        .iter()
        .map(|row| {
            headers
                .iter()
                .enumerate()
                .map(|(i, h)| (h.as_str(), row.get(i).map(|s| s.as_str()).unwrap_or("")))
                .collect()
        })
        .collect();
    let json_str = serde_json::to_string(&arr).map_err(|e| format!("JSON 序列化失败: {e}"))?;
    std::fs::write(out_path, json_str).map_err(|e| format!("写入失败: {e}"))
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.0 T5-7 records-based 脱敏 / 校验命令
// ─────────────────────────────────────────────────────────────────────
// 与 `apply_rules_cols` / `run_validate` 区别：入参不再走文件路径，而是直接
// 接收 PreprocessView 产出的 `Records`（headers + rows），避免脱敏/校验视图
// 再读盘；前端 MaskView/ValidateView 切到 state.records 作为数据源后调用本命令。
// 返回结构与文件路径版一致，便于前端复用渲染逻辑。

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
    let records = ruT0_data_kit_core::readers::Records { headers, rows };
    let selected: HashSet<String> = selected_columns.iter().cloned().collect();
    let result = mask_pipeline_columns(&records, &rules, &selected).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": result.masked.headers,
        "masked_rows": result.masked.rows,
        "summary": result.summary,
        "skipped_fields": result.skipped_fields,
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
    let records = ruT0_data_kit_core::readers::Records { headers, rows };
    let result = validate_pipeline(&records, &rules).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": result.headers,
        "rows": result.rows,
        "valid_matrix": result.valid_matrix,
        "summary": result.summary,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 写一份临时 CSV 测试数据并返回路径。调用方负责清理。
    fn write_temp_csv(headers: &[&str], rows: &[Vec<&str>]) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "rut0_t5_8_{}.csv",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = std::fs::File::create(&dir).unwrap();
        let mut wtr = csv::Writer::from_writer(file);
        wtr.write_record(headers).unwrap();
        for r in rows {
            wtr.write_record(r).unwrap();
        }
        wtr.flush().unwrap();
        dir
    }

    fn empty_ruleset() -> String {
        serde_json::json!({ "maskers": [], "validators": [] }).to_string()
    }

    /// 验证 JSON 行数 = 原始行数，key = headers 子集（按勾选列过滤）。
    #[test]
    fn export_records_json_filters_columns_and_preserves_rows() {
        let csv_path =
            write_temp_csv(&["id", "name", "phone"], &[vec!["1", "alice", "138"], vec!["2", "bob", "139"]]);
        let out = csv_path.with_extension("json");
        let selected = vec!["id".to_string(), "phone".to_string()];
        let order = vec!["phone".to_string(), "id".to_string()];
        export_records_json(
            csv_path.to_string_lossy().into_owned(),
            empty_ruleset(),
            selected,
            order,
            None,
            out.to_string_lossy().into_owned(),
        )
        .unwrap();

        let body = std::fs::read_to_string(&out).unwrap();
        let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed.len(), 2, "行数应等于原始行数");
        // key 应是 selected_columns 子集（不含 name）
        for row in &parsed {
            assert!(row.contains_key("id"));
            assert!(row.contains_key("phone"));
            assert!(!row.contains_key("name"));
        }
        // column_order 控制顺序：phone 在 id 之前（HashMap 无序但 values 应正确）
        assert_eq!(parsed[0].get("id").unwrap(), "1");
        assert_eq!(parsed[0].get("phone").unwrap(), "138");
        assert_eq!(parsed[1].get("id").unwrap(), "2");
        assert_eq!(parsed[1].get("phone").unwrap(), "139");

        let _ = std::fs::remove_file(&csv_path);
        let _ = std::fs::remove_file(&out);
    }

    /// 验证 selected_row_indices 只保留指定行。
    #[test]
    fn export_records_json_filters_rows_by_indices() {
        let csv_path = write_temp_csv(
            &["id", "name"],
            &[vec!["1", "a"], vec!["2", "b"], vec!["3", "c"]],
        );
        let out = csv_path.with_extension("json");
        export_records_json(
            csv_path.to_string_lossy().into_owned(),
            empty_ruleset(),
            vec!["id".to_string(), "name".to_string()],
            vec![],
            Some(vec![0, 2]),
            out.to_string_lossy().into_owned(),
        )
        .unwrap();

        let body = std::fs::read_to_string(&out).unwrap();
        let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed.len(), 2, "只保留索引 0/2 两行");
        assert_eq!(parsed[0].get("id").unwrap(), "1");
        assert_eq!(parsed[1].get("id").unwrap(), "3");

        let _ = std::fs::remove_file(&csv_path);
        let _ = std::fs::remove_file(&out);
    }
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.0 T5-5 搜索命令
// ─────────────────────────────────────────────────────────────────────

/// 对预处理后的 `Records`（前端传入 `headers` + `rows`）跑搜索，返回命中列表。
///
/// `query_json` 是 core `SearchQuery` 的 JSON 序列化，形如：
/// - `{"kind":"keyword","terms":["foo"],"mode":"or"}`
/// - `{"kind":"regex","pattern":"@example\\.com$"}`
/// - `{"kind":"exact_field","field":"name","value":"Alice"}`
///
/// 与 `preprocess_file` 配合：前端先调 `preprocess_file` 拿到 `{headers, rows}`，
/// 再把同样的数据 + 查询条件传到这里。不在 Tauri State 缓存索引——每次现建，
/// v0.4.0 大文件（10w×10）性能基线 < 5s 满足交互式需求。
///
/// 返回 `SearchResult { hits: Vec<SearchHit> }`，每条 hit 含 row/col/field/
/// value/snippet（snippet ±20 字符上下文，UTF-8 友好）。非法正则返回
/// `Err`，不 panic（core 端 `CoreError::InvalidInput` → 字符串）。
#[tauri::command]
pub fn search_records(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    query_json: String,
) -> Result<ruT0_data_kit_core::search::SearchResult, String> {
    let query: ruT0_data_kit_core::search::SearchQuery =
        serde_json::from_str(&query_json).map_err(|e| format!("query_json 解析失败: {e}"))?;
    let records = ruT0_data_kit_core::readers::Records { headers, rows };
    ruT0_data_kit_core::search::search_records(&records, &query).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.1 T6-4 SQL 盲注探针特征检测
// ─────────────────────────────────────────────────────────────────────

/// 扫描预处理后的 `Records` 找 SQL 盲注探针特征行（v0.4.1 T6-4）。
///
/// 遍历 `rows` 所有 cell 调 core `looks_like_blind_probe`，命中则收集该 cell
/// 原文。返回 `{ detected: bool, samples: Vec<String> }`，`samples` 上限 50
/// 避免过大。`headers` 暂未用于过滤（保留参数以与 records 结构对齐）。
///
/// 仅本地正则匹配，不调用网络（满足 docs/00 §6 「不外发数据」约束）。
/// PreprocessView `handleImport` 成功后调它；`detected=true` 时前端 dispatch
/// `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB("sql")` + `SET_SQL_PARSE_INPUT`
/// 自动跳转 SqlParseTool 并预填命中行。
#[tauri::command]
pub fn detect_sql_blind_features(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
) -> Result<Value, String> {
    use ruT0_data_kit_core::logsign::blind_aggregator::looks_like_blind_probe;
    let _ = &headers; // 暂未用于过滤，保留参数以与 records 结构对齐
    let mut samples: Vec<String> = Vec::new();
    for row in &rows {
        for cell in row {
            if !cell.is_empty() && looks_like_blind_probe(cell) && samples.len() < 50 {
                samples.push(cell.clone());
            }
        }
    }
    Ok(json!({
        "detected": !samples.is_empty(),
        "samples": samples,
    }))
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.2 T7-2 设置模块：tshark 多平台自动探测 + 路径配置持久化
// ─────────────────────────────────────────────────────────────────────

/// settings.json 在 `app_config_dir` 下的相对文件名。
const SETTINGS_FILE_NAME: &str = "settings.json";

/// 解析 `app_config_dir/settings.json` 的绝对路径。
///
/// 失败返回字符串错误（前端按 message.error 弹出）。目录不存在不在此处创建——
/// 读取时返回 None 即可，写入时由 `save_tshark_path` 调 `create_dir_all`。
fn settings_file_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法定位配置目录: {e}"))?;
    Ok(config_dir.join(SETTINGS_FILE_NAME))
}

/// 自动探测本机可用的 tshark（v0.4.2 T7-2）。
///
/// 调 core `pcap::detect_tshark`：按优先级探测覆盖路径 → PATH `tshark` →
/// 各平台候选绝对路径，跑 `<path> --version` 退出 0 即视为可用。返回：
/// - 命中：`{ "path": "...", "version": "TShark (Wireshark) 4.x.x" }`
/// - 未命中：`{ "path": null, "version": null }`
///
/// 仅本机子进程探测，不调用网络（满足 docs/00 §6「不外发数据」）。
#[tauri::command]
pub fn detect_tshark() -> Result<Value, String> {
    match pcap::detect_tshark() {
        Some(info) => Ok(json!({
            "path": info.path,
            "version": info.version,
        })),
        None => Ok(json!({ "path": null, "version": null })),
    }
}

/// 从 `app_config_dir/settings.json` 读取用户保存的 tshark 覆盖路径。
///
/// 文件不存在 / 字段缺失返回 `Ok(None)`，不视为错误。读到非空路径时同步调
/// `pcap::set_tshark_path` 注入运行时，让后续 `PcapReader::read` 立即生效。
///
/// 启动时由 SettingsView `useEffect` 调用一次，把持久化配置灌入内存。
#[tauri::command]
pub fn load_tshark_path(app: AppHandle) -> Result<Option<String>, String> {
    let path = settings_file_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 settings.json 失败: {e}"))?;
    let v: Value = serde_json::from_str(&content)
        .map_err(|e| format!("settings.json 解析失败: {e}"))?;
    let saved = v
        .get("tshark_path")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());
    // 注入运行时（即使为 None 也调一次，确保与文件语义对齐）。
    pcap::set_tshark_path(saved.clone());
    Ok(saved)
}

/// 把 tshark 覆盖路径保存到 `app_config_dir/settings.json`（v0.4.2 T7-2）。
///
/// `path` 为 `Some(s)` 时写入 `{ "tshark_path": s }`；为 `None` 时写入
/// `{ "tshark_path": null }`（语义：清除自定义路径，回退到 PATH）。同时调
/// `pcap::set_tshark_path` 注入运行时，立即生效。
///
/// 目录不存在时 `create_dir_all` 兜底创建。文件 IO 失败返回字符串错误。
#[tauri::command]
pub fn save_tshark_path(app: AppHandle, path: Option<String>) -> Result<(), String> {
    let file_path = settings_file_path(&app)?;
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    let v = json!({ "tshark_path": path });
    let s = serde_json::to_string_pretty(&v)
        .map_err(|e| format!("settings.json 序列化失败: {e}"))?;
    std::fs::write(&file_path, s).map_err(|e| format!("写入 settings.json 失败: {e}"))?;
    // 注入运行时，立即生效。
    pcap::set_tshark_path(path);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.3 T9-4 新增命令：数据提取（phone/bankcard/ip）
// ─────────────────────────────────────────────────────────────────────

/// 提取结果条目（前端友好结构）。
#[derive(serde::Serialize, serde::Deserialize)]
struct ExtractItem {
    #[serde(rename = "type")]
    r#type: String,
    value: String,
}

/// 从文本提取 phone/bankcard/ip 三类 PII，去重后返回 findings + counts。
#[tauri::command]
pub fn extract_text(content: String) -> Result<Value, String> {
    let raw = extract::extract_text(&content);
    Ok(package_extract_result(raw))
}

/// 从 .txt 文件提取 phone/bankcard/ip 三类 PII，去重后返回 findings + counts。
#[tauri::command]
pub fn extract_file(path: String) -> Result<Value, String> {
    let raw = extract::extract_file(Path::new(&path)).map_err(|e| e.to_string())?;
    Ok(package_extract_result(raw))
}

/// 把 findings 按指定格式（txt/csv/json）写到 out_path。
///
/// txt 格式：每行 `type_value`（小写 type + 下划线，匹配 PDF spec）。
/// csv 格式：type,value 两列（复用 write_csv）。
/// json 格式：结构化数组（复用 write_json）。
#[tauri::command]
pub fn export_extract(
    findings_json: String,
    format: String,
    out_path: String,
) -> Result<(), String> {
    let items: Vec<ExtractItem> = serde_json::from_str(&findings_json)
        .map_err(|e| format!("findings_json 解析失败: {e}"))?;
    match format.as_str() {
        "txt" => {
            let mut buf = String::new();
            for it in &items {
                buf.push_str(&format!("{}_{}\n", it.r#type, it.value));
            }
            // 去掉末尾多余换行
            if buf.ends_with('\n') { buf.pop(); }
            std::fs::write(&out_path, buf).map_err(|e| format!("写入失败: {e}"))
        }
        "csv" => {
            let headers = vec!["type".to_string(), "value".to_string()];
            let rows: Vec<Vec<String>> = items.iter()
                .map(|it| vec![it.r#type.clone(), it.value.clone()])
                .collect();
            write_csv(&headers, &rows, &out_path)
        }
        "json" => {
            let headers = vec!["type".to_string(), "value".to_string()];
            let rows: Vec<Vec<String>> = items.iter()
                .map(|it| vec![it.r#type.clone(), it.value.clone()])
                .collect();
            write_json(&headers, &rows, &out_path)
        }
        _ => Err(format!("unsupported format: {format}")),
    }
}

/// 把 core Finding 列表去重 + 打包成 { findings, counts } JSON。
fn package_extract_result(raw: Vec<ruT0_data_kit_core::report::Finding>) -> Value {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut items: Vec<ExtractItem> = Vec::new();
    let mut phone = 0i64;
    let mut bankcard = 0i64;
    let mut ip = 0i64;
    for f in raw {
        let key = (f.r#type.clone(), f.value.clone());
        if seen.insert(key.clone()) {
            match f.r#type.as_str() {
                "phone" => phone += 1,
                "bankcard" => bankcard += 1,
                "ip" => ip += 1,
                _ => {}
            }
            items.push(ExtractItem {
                r#type: f.r#type,
                value: f.value,
            });
        }
    }
    json!({
        "findings": items,
        "counts": {
            "phone": phone,
            "bankcard": bankcard,
            "ip": ip,
        }
    })
}
