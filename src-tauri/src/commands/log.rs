//! 日志扫描命令（2 命令）。

use std::path::Path;

use ruT0_data_kit_core::log::LogReader;
use ruT0_data_kit_core::logsign::SignatureEngine;
use ruT0_data_kit_core::pipeline::log_scan::scan_log;
use ruT0_data_kit_core::rules::RuleSet;
use ruT0_data_kit_core::scan::DefaultSensitiveScan;
use serde_json::{json, Value};

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
/// （1856 行），一次 IPC 体积可控。敏感扫描使用空规则集
/// （`RuleSet::default()`，v0.4.4 规则池初始为空），其 validators 为空时
/// sensitive findings 为空，这是可接受的（HANDOFF 提到 sensitive 可空）。
/// 后续版本接入规则池后，可改为读取用户 ruleset。
#[tauri::command]
pub fn scan_log_file(path: String) -> Result<Value, String> {
    let reader = LogReader::new().map_err(|e| e.to_string())?;
    let entries = reader.read(Path::new(&path)).map_err(|e| e.to_string())?;
    let engine = SignatureEngine::new().map_err(|e| e.to_string())?;
    let scan = DefaultSensitiveScan::new();
    let rules = RuleSet::default();
    let report = scan_log(&entries, &engine, &scan, &rules);
    let report_value = serde_json::to_value(&report).map_err(|e| e.to_string())?;
    let entries_value = serde_json::to_value(&entries).map_err(|e| e.to_string())?;
    Ok(json!({
        "entries": entries_value,
        "report": report_value,
    }))
}

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
    use ruT0_data_kit_core::logsign::looks_like_blind_probe;
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
