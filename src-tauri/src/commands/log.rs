//! 日志扫描命令（2 命令）。

use std::path::Path;

use ruT0_data_kit_core::log::LogReader;
use ruT0_data_kit_core::log::{looks_like_url_encoded, url_decode_twice};
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
/// **v0.7.1**：cell 先用 [`looks_like_url_encoded`] 检测，命中 `%XX` hex 序列
/// 则调 [`url_decode_twice`] 双重解码再喂 `looks_like_blind_probe`；`samples`
/// 收集解码后形态（与 `parse_sqls` 入口一致，避免下游二次解码）。
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
            if cell.is_empty() || samples.len() >= 50 {
                continue;
            }
            // v0.7.1：自动识别 URL-encoded cell，命中 `%XX` 则先解码再检测。
            let probe_target = if looks_like_url_encoded(cell) {
                url_decode_twice(cell)
            } else {
                cell.clone()
            };
            if looks_like_blind_probe(&probe_target) {
                // samples 收集解码后形态：下游 SqlParseTool 直接用 samples 填
                // textarea 再调 parse_sqls，解码后形态过 looks_like_url_encoded
                // 返回 false，走原路径，避免二次解码。
                samples.push(probe_target);
            }
        }
    }
    Ok(json!({
        "detected": !samples.is_empty(),
        "samples": samples,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_sql_blind_features_decodes_url_encoded_cell() {
        // v0.7.1：URL-encoded cell 命中 ascii_binary 探针特征，解码后 detected=true。
        let rows = vec![vec!["1'%20or%20ascii(substr((database()),1,1))%3E79%23".to_string()]];
        let v = detect_sql_blind_features(vec![], rows).unwrap();
        assert_eq!(v["detected"], json!(true));
        let samples = v["samples"].as_array().unwrap();
        assert_eq!(samples.len(), 1);
        // samples 收集解码后形态（含字面空格和 >）
        assert_eq!(
            samples[0].as_str().unwrap(),
            "1' or ascii(substr((database()),1,1))>79#"
        );
    }

    #[test]
    fn detect_sql_blind_features_plain_cell_unchanged() {
        // 已解码纯文本 cell：走原路径，samples 与原文一致。
        let rows = vec![vec!["1' or ascii(substr((database()),1,1))>79#".to_string()]];
        let v = detect_sql_blind_features(vec![], rows).unwrap();
        assert_eq!(v["detected"], json!(true));
        let samples = v["samples"].as_array().unwrap();
        assert_eq!(samples[0].as_str().unwrap(), "1' or ascii(substr((database()),1,1))>79#");
    }

    #[test]
    fn detect_sql_blind_features_empty_rows() {
        // 空 rows → detected=false，samples=[]。
        let v = detect_sql_blind_features(vec![], vec![]).unwrap();
        assert_eq!(v["detected"], json!(false));
        assert_eq!(v["samples"].as_array().unwrap().len(), 0);
    }
}
