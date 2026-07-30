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
/// 原文。返回 `{ detected: bool, samples: Vec<{ sql, body_size, source_ip }> }`。
///
/// **v0.7.4**：移除 samples 上限 50——盲注二分还原必须拿到全量探针，
/// 截断会人为制造 gap 导致 `insufficient_probes`。dedup 后全量传给
/// `parseSqlTool`，IPC 体积在 Tauri JSON 可控范围内。
///
/// **v0.7.1**：cell 先用 [`looks_like_url_encoded`] 检测，命中 `%XX` hex 序列
/// 则调 [`url_decode_twice`] 双重解码再喂 `looks_like_blind_probe`；`samples`
/// 收集解码后形态（与 `parse_sqls` 入口一致，避免下游二次解码）。
///
/// **v0.7.3**：`samples` 改为结构化对象，从同行按 `headers` 定位 `size` 列
/// （HTTP 响应 body 字节数）和 `ip` 列（来源 IP），配对到每个 sample 上。
/// 非 log 源（无 `size`/`ip` 列）→ `body_size=null`、`source_ip=null`，
/// 向后兼容非盲注 payload（走 `parse_payload` 兜底）。按
/// `(sql, body_size, source_ip)` 三元组去重，避免 `query`（encoded）与
/// `decoded_query`（decoded）列重复收集同一条探针。
///
/// 仅本地正则匹配，不调用网络（满足 docs/00 §6 「不外发数据」约束）。
/// PreprocessView「盲注自动提取」按钮调它；`detected=true` 时前端 dispatch
/// `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB("sql")` + `SET_SQL_PARSE_INPUT`
/// 自动跳转 SqlParseTool 并预填命中行（带 `body|sql` 前缀）。
#[tauri::command]
pub fn detect_sql_blind_features(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
) -> Result<Value, String> {
    use ruT0_data_kit_core::logsign::looks_like_blind_probe;
    // v0.7.3：从 headers 定位 size / ip 列下标（按列名，兼容列重命名）。
    let size_idx = headers.iter().position(|h| h == "size");
    let ip_idx = headers.iter().position(|h| h == "ip");
    // 去重 key：(sql, body_size, source_ip) 三元组。
    // v0.7.4：移除 samples 上限 50——盲注二分还原必须拿到全量探针，
    // 丢弃第 50 条之后的探针会导致 gap（如缺 110/111 → insufficient_probes）。
    // dedup + 全量传给 parseSqlTool，IPC 体积在 Tauri JSON 可控范围内。
    let mut seen: Vec<(String, Option<u64>, Option<String>)> = Vec::new();
    let mut samples: Vec<Value> = Vec::new();
    for row in &rows {
        for cell in row {
            if cell.is_empty() {
                continue;
            }
            // v0.7.1：自动识别 URL-encoded cell，命中 `%XX` 则先解码再检测。
            let probe_target = if looks_like_url_encoded(cell) {
                url_decode_twice(cell)
            } else {
                cell.clone()
            };
            if looks_like_blind_probe(&probe_target) {
                // v0.7.3：从同行 size / ip 列配对 body_size + source_ip。
                let body_size = size_idx
                    .and_then(|i| row.get(i))
                    .and_then(|s| {
                        let t = s.trim();
                        if t.is_empty() || t == "-" {
                            None
                        } else {
                            t.parse::<u64>().ok()
                        }
                    });
                let source_ip = ip_idx
                    .and_then(|i| row.get(i))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                // 去重：同一探针 + 同一 body_size + 同一 source_ip 只保留一条。
                let key = (probe_target.clone(), body_size, source_ip.clone());
                if seen.iter().any(|e| e == &key) {
                    continue;
                }
                seen.push(key);
                samples.push(json!({
                    "sql": probe_target,
                    "body_size": body_size,
                    "source_ip": source_ip,
                }));
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
        // v0.7.3：samples 改为结构化对象，断言 sql/body_size/source_ip 三字段。
        let rows = vec![vec!["1'%20or%20ascii(substr((database()),1,1))%3E79%23".to_string()]];
        let v = detect_sql_blind_features(vec![], rows).unwrap();
        assert_eq!(v["detected"], json!(true));
        let samples = v["samples"].as_array().unwrap();
        assert_eq!(samples.len(), 1);
        // samples 收集解码后形态（含字面空格和 >）
        assert_eq!(
            samples[0]["sql"].as_str().unwrap(),
            "1' or ascii(substr((database()),1,1))>79#"
        );
        // 无 size/ip 列 → body_size=null、source_ip=null
        assert_eq!(samples[0]["body_size"], json!(null));
        assert_eq!(samples[0]["source_ip"], json!(null));
    }

    #[test]
    fn detect_sql_blind_features_plain_cell_unchanged() {
        // 已解码纯文本 cell：走原路径，samples sql 与原文一致。
        let rows = vec![vec!["1' or ascii(substr((database()),1,1))>79#".to_string()]];
        let v = detect_sql_blind_features(vec![], rows).unwrap();
        assert_eq!(v["detected"], json!(true));
        let samples = v["samples"].as_array().unwrap();
        assert_eq!(samples[0]["sql"].as_str().unwrap(), "1' or ascii(substr((database()),1,1))>79#");
        assert_eq!(samples[0]["body_size"], json!(null));
        assert_eq!(samples[0]["source_ip"], json!(null));
    }

    #[test]
    fn detect_sql_blind_features_empty_rows() {
        // 空 rows → detected=false，samples=[]。
        let v = detect_sql_blind_features(vec![], vec![]).unwrap();
        assert_eq!(v["detected"], json!(false));
        assert_eq!(v["samples"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn detect_sql_blind_features_carries_body_size_and_source_ip() {
        // v0.7.3：含 size + ip 列的 log 源，sample 应配对 body_size + source_ip。
        let headers = vec![
            "line_no".to_string(),
            "ip".to_string(),
            "timestamp".to_string(),
            "method".to_string(),
            "path".to_string(),
            "query".to_string(),
            "status".to_string(),
            "size".to_string(),
        ];
        // query 列（index 5）含 URL-encoded 探针；size 列（index 7）= 862；ip 列（index 1）= 1.1.1.1
        let rows = vec![vec![
            "1".to_string(),           // line_no
            "1.1.1.1".to_string(),     // ip
            "2024-01-01".to_string(),  // timestamp
            "GET".to_string(),         // method
            "/login".to_string(),      // path
            "username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1".to_string(), // query
            "200".to_string(),         // status
            "862".to_string(),         // size
        ]];
        let v = detect_sql_blind_features(headers, rows).unwrap();
        assert_eq!(v["detected"], json!(true));
        let samples = v["samples"].as_array().unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(
            samples[0]["sql"].as_str().unwrap(),
            "username=1' or ascii(substr((database()),1,1))>79#&password=1"
        );
        assert_eq!(samples[0]["body_size"], json!(862));
        assert_eq!(samples[0]["source_ip"], json!("1.1.1.1"));
    }

    #[test]
    fn detect_sql_blind_features_deduplicates() {
        // v0.7.3：同一行 query（encoded）+ decoded_query（decoded）列都含同一探针，
        // 去重后 samples 仅 1 条。
        let headers = vec![
            "query".to_string(),
            "decoded_query".to_string(),
        ];
        let rows = vec![vec![
            "1'%20or%20ascii(substr((database()),1,1))%3E79%23".to_string(), // query (encoded)
            "1' or ascii(substr((database()),1,1))>79#".to_string(),         // decoded_query (decoded)
        ]];
        let v = detect_sql_blind_features(headers, rows).unwrap();
        let samples = v["samples"].as_array().unwrap();
        // 两条 decode 后均为 "1' or ascii(substr((database()),1,1))>79#"
        // body_size=None（无 size 列）、source_ip=None（无 ip 列）→ 三元组相同 → 去重后 1 条
        assert_eq!(samples.len(), 1);
        assert_eq!(
            samples[0]["sql"].as_str().unwrap(),
            "1' or ascii(substr((database()),1,1))>79#"
        );
    }

    #[test]
    fn detect_sql_blind_features_no_size_header_body_size_none() {
        // v0.7.3：无 size 列时 body_size=null（向后兼容非 log 源）。
        let headers = vec!["query".to_string()];
        let rows = vec![vec!["ascii(substr((database()),1,1))>100".to_string()]];
        let v = detect_sql_blind_features(headers, rows).unwrap();
        let samples = v["samples"].as_array().unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0]["body_size"], json!(null));
        assert_eq!(samples[0]["source_ip"], json!(null));
    }

    #[test]
    fn detect_sql_blind_features_size_dash_parses_none() {
        // v0.7.3：size 列为 CLF `-` 时 body_size=null（LogEntry.size=None → "-"）。
        let headers = vec!["ip".to_string(), "query".to_string(), "size".to_string()];
        let rows = vec![vec![
            "1.1.1.1".to_string(),
            "ascii(substr((database()),1,1))>100".to_string(),
            "-".to_string(),
        ]];
        let v = detect_sql_blind_features(headers, rows).unwrap();
        let samples = v["samples"].as_array().unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0]["body_size"], json!(null));
        assert_eq!(samples[0]["source_ip"], json!("1.1.1.1"));
    }
}
