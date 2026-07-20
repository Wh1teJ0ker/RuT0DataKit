//! v0.2.0 日志扫描 pipeline：对 `Vec<LogEntry>` 跑签名引擎 + 弱口令 grep +
//! 复用 v0.1.0 的 [`crate::scan::SensitiveScan`]，产出 `kind="log_scan"` 的
//! [`Report`]。
//!
//! 设计要点（见 HANDOFF T2-3）：
//! - 单入口 [`scan_log`]，组合三个检测源：
//!   1. [`crate::logsign::SignatureEngine::scan_log_entry`] → `type="sqli"`
//!      finding，value=rule_id，location=line_no，context=decoded_query 摘要。
//!   2. 弱口令 grep：query 同时含 `username=` 和 `password=` 时，按 `&` 切分
//!      提取并双重 URL 解码后，若 username 或 password 值落在 [`WEAK_KEYWORDS`]
//!      集合内即产 `type="weak_password"` finding。
//!   3. [`crate::scan::SensitiveScan::scan`] 对 decoded query/path 跑敏感字段
//!      扫描，把返回的 Finding.location 补成当前 line_no。
//! - summary 用 `serde_yml::Value::Mapping` 手填，含 total_lines /
//!   sqli_hits / weak_password_hits / sensitive_hits / top_attack_ips（按
//!   IP 聚合命中数排序取前 5，元素 `{ ip, hits }`）。
//! - 不重新实现 URL 解码，复用 [`crate::log::parse_query`]。
//!
//! v0.2.2（T2-13）：`Report.extra` 由 `Value::Null` 改为
//! `Value::Mapping({ "blind_aggregation": [AggregatedResult...] })`，承载
//! T2-12 的盲注二分序列聚合结果。`extra` 为 `serde_yml::Value`，旧前端读
//! `extra` 为 null 不受影响（向后兼容）。

use std::collections::HashMap;

use serde_yml::Value;

use crate::log::{parse_query, LogEntry};
use crate::logsign::{BlindAggregator, SignatureEngine};
use crate::report::{Finding, Report};
use crate::rules::RuleSet;
use crate::scan::{DefaultSensitiveScan, SensitiveScan};

/// 弱口令关键字集合（纯等值比较，不做字典模糊匹配）。
///
/// 来源：HANDOFF T2-3。任一出现在 username 或 password 字段值中即产 finding。
pub const WEAK_KEYWORDS: &[&str] = &[
    "guest", "admin", "root", "test", "user",
    "123456", "123", "1234", "12345678",
    "admin123", "password", "pass", "qwerty",
    "abc123", "111111", "000000",
];

/// 对日志 entries 跑签名引擎 + 弱口令 grep + 敏感字段扫描，产出
/// `kind="log_scan"` 的 [`Report`]。
///
/// - `sig_engine` / `scan` / `rules` 由调用方注入，避免在 pipeline 内部
///   重复构造（GUI 多次扫描场景友好）。
/// - `source` 从 entries 第一行的 `raw` 推断太脆弱，统一填 `"log"`。
/// - v0.2.2（T2-13）：末尾调 [`BlindAggregator`] 还原盲注二分字符串，
///   塞进 `Report.extra.blind_aggregation`。
pub fn scan_log(
    entries: &[LogEntry],
    sig_engine: &SignatureEngine,
    scan: &DefaultSensitiveScan,
    rules: &RuleSet,
) -> Report {
    let mut findings: Vec<Finding> = Vec::new();
    // IP → 命中条数（一条 entry 命中 N 次记 N 次，按 finding 数而非 entry 数）。
    let mut ip_hits: HashMap<String, u64> = HashMap::new();
    let mut sqli_hits: u64 = 0;
    let mut weak_password_hits: u64 = 0;
    let mut sensitive_hits: u64 = 0;

    for entry in entries {
        let line_no_str = entry.line_no.to_string();

        // ---- a. 签名引擎 ----
        let sig_hits = sig_engine.scan_log_entry(entry);
        let decoded_query_summary = decoded_query_summary(entry);
        for hit in sig_hits {
            findings.push(Finding {
                r#type: "sqli".to_string(),
                value: hit.rule_id.clone(),
                location: Some(line_no_str.clone()),
                valid: None,
                context: Some(decoded_query_summary.clone()),
                extra: hit
                    .parsed_payload
                    .as_ref()
                    .and_then(|p| serde_yml::to_value(p).ok()),
            });
            *ip_hits.entry(entry.ip.clone()).or_insert(0) += 1;
            sqli_hits += 1;
        }

        // ---- b. 弱口令 grep ----
        if let Some(f) = scan_weak_password(entry) {
            findings.push(Finding {
                r#type: "weak_password".to_string(),
                value: f,
                location: Some(line_no_str.clone()),
                valid: None,
                context: None,
                extra: None,
            });
            *ip_hits.entry(entry.ip.clone()).or_insert(0) += 1;
            weak_password_hits += 1;
        }

        // ---- c. 敏感字段扫描（对 decoded query + path 拼接文本） ----
        let scan_text = build_scan_text(entry);
        let sens = scan.scan(&scan_text, rules);
        for mut f in sens {
            // 原 SensitiveScan 不填 location，补成当前 line_no。
            f.location = Some(line_no_str.clone());
            // SensitiveScan 不产 parsed_payload，extra 显式置 None 保持向后兼容。
            f.extra = None;
            findings.push(f);
            *ip_hits.entry(entry.ip.clone()).or_insert(0) += 1;
            sensitive_hits += 1;
        }
    }

    // ---- top_attack_ips：按命中数倒序，取前 5 ----
    let mut ip_vec: Vec<(String, u64)> = ip_hits.into_iter().collect();
    ip_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let top_attack_ips: Vec<Value> = ip_vec
        .into_iter()
        .take(5)
        .map(|(ip, hits)| {
            let mut m = serde_yml::Mapping::new();
            m.insert(
                Value::String("ip".into()),
                Value::String(ip),
            );
            m.insert(
                Value::String("hits".into()),
                Value::Number(hits.into()),
            );
            Value::Mapping(m)
        })
        .collect();

    let mut summary = serde_yml::Mapping::new();
    summary.insert(
        Value::String("total_lines".into()),
        Value::Number((entries.len() as u64).into()),
    );
    summary.insert(
        Value::String("sqli_hits".into()),
        Value::Number(sqli_hits.into()),
    );
    summary.insert(
        Value::String("weak_password_hits".into()),
        Value::Number(weak_password_hits.into()),
    );
    summary.insert(
        Value::String("sensitive_hits".into()),
        Value::Number(sensitive_hits.into()),
    );
    summary.insert(
        Value::String("top_attack_ips".into()),
        Value::Sequence(top_attack_ips),
    );

    // v0.2.2（T2-13）：盲注二分序列聚合还原。直接对全量 entries 跑
    // BlindAggregator（同源/同 read_target 自动分组），结果塞进
    // Report.extra.blind_aggregation。无盲注探针时返回空 Vec，
    // extra 仍为含空数组的 Mapping（前端可统一按 sequence 读）。
    //
    // v0.2.4（T4-2）：在 blind_aggregation 之外新增 reconstructed_database 键，
    // 把 4 类标准 read_target（database() / group_concat(table_name) /
    // group_concat(column_name) / group_concat(col,0xNN,...)）交叉关联成
    // 结构化 ReconstructedDatabase（schema → tables → columns → rows），
    // 供前端按数据库格式展示。blind_aggregation 数组保留不动（向后兼容）。
    let aggregator = BlindAggregator::collect_from_entries(entries);
    let blind_results = aggregator.aggregate();
    let reconstructed = BlindAggregator::reconstruct_database(&blind_results);
    let mut extra_map = serde_yml::Mapping::new();
    extra_map.insert(
        Value::String("blind_aggregation".into()),
        serde_yml::to_value(&blind_results).unwrap_or(Value::Null),
    );
    extra_map.insert(
        Value::String("reconstructed_database".into()),
        serde_yml::to_value(&reconstructed).unwrap_or(Value::Null),
    );

    Report {
        source: "log".to_string(),
        kind: "log_scan".to_string(),
        summary: Value::Mapping(summary),
        findings,
        extra: Value::Mapping(extra_map),
    }
}

/// 弱口令检测：query 同时含 `username=` 和 `password=` 时，按 `&` 切分
/// 提取并双重 URL 解码后，若 username 或 password 值落在 [`WEAK_KEYWORDS`]
/// 集合内，返回 `Some("username=X&password=Y")`。
fn scan_weak_password(entry: &LogEntry) -> Option<String> {
    let q = entry.query.as_deref()?;
    let lower = q.to_ascii_lowercase();
    if !lower.contains("username=") || !lower.contains("password=") {
        return None;
    }
    let pairs = parse_query(q);
    let mut username: Option<String> = None;
    let mut password: Option<String> = None;
    for (k, v) in &pairs {
        let kl = k.to_ascii_lowercase();
        if kl == "username" && username.is_none() {
            username = Some(v.clone());
        } else if kl == "password" && password.is_none() {
            password = Some(v.clone());
        }
    }
    let (Some(u), Some(p)) = (username, password) else {
        return None;
    };
    let hit = WEAK_KEYWORDS
        .iter()
        .any(|kw| kw == &u || kw == &p);
    if hit {
        Some(format!("username={u}&password={p}"))
    } else {
        None
    }
}

/// 构造敏感字段扫描文本：`path` + decoded query 的所有 value（与
/// [`SignatureEngine`] 的匹配目标保持一致，便于复用同一解码口径）。
fn build_scan_text(entry: &LogEntry) -> String {
    let mut text = entry.path.clone();
    if let Some(q) = entry.query.as_deref() {
        for (_k, v) in parse_query(q) {
            if !v.is_empty() {
                text.push(' ');
                text.push_str(&v);
            }
        }
    }
    text
}

/// decoded query 摘要：取首个非空 value 截断到 80 字符，作为 finding.context
/// 回引线索；无 query 时回退到 path。
fn decoded_query_summary(entry: &LogEntry) -> String {
    if let Some(q) = entry.query.as_deref() {
        let pairs = parse_query(q);
        for (_k, v) in &pairs {
            if !v.is_empty() {
                return truncate(v, 80);
            }
        }
    }
    truncate(&entry.path, 80)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogEntry;
    use crate::logsign::SignatureEngine;
    use crate::rules::RuleSet;
    use crate::scan::DefaultSensitiveScan;

    fn mk_entry(line_no: usize, method: &str, path: &str, query: Option<&str>) -> LogEntry {
        LogEntry {
            line_no,
            ip: "127.0.0.1".to_string(),
            timestamp: "10/Oct/2023:13:55:36 +0000".to_string(),
            method: method.to_string(),
            path: path.to_string(),
            query: query.map(|s| s.to_string()),
            status: 200,
            size: Some(100),
            user_agent: "curl/7.88.0".to_string(),
            raw: format!("{method} {path} {query:?}"),
            decoded_path: crate::log::url_decode_twice(path),
            decoded_query: query.map(|q| {
                let pairs = crate::log::parse_query(q);
                pairs
                    .iter()
                    .map(|(k, v)| {
                        if v.is_empty() {
                            k.clone()
                        } else {
                            format!("{k}={v}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("&")
            }),
            decoded_ua: crate::log::url_decode_twice("curl/7.88.0"),
        }
    }

    fn empty_ruleset() -> RuleSet {
        RuleSet {
            validators: vec![],
            maskers: vec![],
        }
    }

    #[test]
    fn basic_sqli_hit_yields_finding() {
        let e = mk_entry(1, "GET", "/news.php", Some("id=1+union+select+1,2,3"));
        let eng = SignatureEngine::default();
        let scan = DefaultSensitiveScan::default();
        let rules = empty_ruleset();
        let report = scan_log(&[e], &eng, &scan, &rules);
        assert_eq!(report.kind, "log_scan");
        let sqli: Vec<_> = report.findings.iter().filter(|f| f.r#type == "sqli").collect();
        assert!(!sqli.is_empty(), "must have sqli finding");
        assert_eq!(sqli[0].location.as_deref(), Some("1"));
        assert_eq!(sqli[0].value, "sqli_union");
    }

    #[test]
    fn weak_password_guest_yields_finding() {
        let e = mk_entry(1, "GET", "/", Some("username=guest&password=123456"));
        let eng = SignatureEngine::default();
        let scan = DefaultSensitiveScan::default();
        let rules = empty_ruleset();
        let report = scan_log(&[e], &eng, &scan, &rules);
        let wp: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.r#type == "weak_password")
            .collect();
        assert_eq!(wp.len(), 1);
        assert_eq!(wp[0].value, "username=guest&password=123456");
        assert_eq!(wp[0].location.as_deref(), Some("1"));
    }

    #[test]
    fn normal_request_no_finding() {
        let e = mk_entry(1, "GET", "/", None);
        let eng = SignatureEngine::default();
        let scan = DefaultSensitiveScan::default();
        let rules = empty_ruleset();
        let report = scan_log(&[e], &eng, &scan, &rules);
        assert!(report.findings.is_empty(), "no findings for normal request");
    }

    #[test]
    fn summary_counts_mixed() {
        let e1 = mk_entry(1, "GET", "/news.php", Some("id=1+union+select+1,2,3"));
        let e2 = mk_entry(2, "GET", "/", Some("username=guest&password=123"));
        let e3 = mk_entry(3, "GET", "/", None);
        let entries = vec![e1, e2, e3];
        let eng = SignatureEngine::default();
        let scan = DefaultSensitiveScan::default();
        let rules = empty_ruleset();
        let report = scan_log(&entries, &eng, &scan, &rules);
        let m = report.summary.as_mapping().expect("summary is mapping");
        assert_eq!(
            m.get("total_lines").and_then(|v| v.as_u64()),
            Some(3),
            "total_lines"
        );
        let sqli = m.get("sqli_hits").and_then(|v| v.as_u64()).unwrap_or(0);
        let wp = m.get("weak_password_hits").and_then(|v| v.as_u64()).unwrap_or(0);
        assert!(sqli >= 1, "sqli_hits >=1, got {sqli}");
        assert_eq!(wp, 1, "weak_password_hits");
    }

    #[test]
    fn top_attack_ips_sorted_top5() {
        // 构造 6 个不同 IP 各命中 N 次，验证排序 + 截断到 5。
        let mut entries = Vec::new();
        // ip1 命中 3 次（最多）
        for i in 1..=3 {
            entries.push(LogEntry {
                line_no: i,
                ip: "10.0.0.1".into(),
                timestamp: String::new(),
                method: "GET".into(),
                path: "/news.php".into(),
                query: Some("id=1+union+select+1,2,3".into()),
                status: 200,
                size: None,
                user_agent: String::new(),
                raw: String::new(),
                decoded_path: String::new(),
                decoded_query: None,
                decoded_ua: String::new(),
            });
        }
        // ip2 命中 2 次
        for i in 4..=5 {
            entries.push(LogEntry {
                line_no: i,
                ip: "10.0.0.2".into(),
                timestamp: String::new(),
                method: "GET".into(),
                path: "/news.php".into(),
                query: Some("id=1+union+select+1,2,3".into()),
                status: 200,
                size: None,
                user_agent: String::new(),
                raw: String::new(),
                decoded_path: String::new(),
                decoded_query: None,
                decoded_ua: String::new(),
            });
        }
        // ip3..ip6 各命中 1 次
        for idx in 0..4 {
            entries.push(LogEntry {
                line_no: 6 + idx,
                ip: format!("10.0.0.{}", 3 + idx),
                timestamp: String::new(),
                method: "GET".into(),
                path: "/news.php".into(),
                query: Some("id=1+union+select+1,2,3".into()),
                status: 200,
                size: None,
                user_agent: String::new(),
                raw: String::new(),
                decoded_path: String::new(),
                decoded_query: None,
                decoded_ua: String::new(),
            });
        }
        let eng = SignatureEngine::default();
        let scan = DefaultSensitiveScan::default();
        let rules = empty_ruleset();
        let report = scan_log(&entries, &eng, &scan, &rules);
        let m = report.summary.as_mapping().unwrap();
        let top = m.get("top_attack_ips").and_then(|v| v.as_sequence()).unwrap();
        assert_eq!(top.len(), 5, "top_attack_ips truncated to 5");
        // 第一应是 10.0.0.1，hits=3
        let first = top[0].as_mapping().unwrap();
        assert_eq!(first.get("ip").and_then(|v| v.as_str()), Some("10.0.0.1"));
        assert_eq!(first.get("hits").and_then(|v| v.as_u64()), Some(3));
        // 第二应是 10.0.0.2，hits=2
        let second = top[1].as_mapping().unwrap();
        assert_eq!(second.get("ip").and_then(|v| v.as_str()), Some("10.0.0.2"));
        assert_eq!(second.get("hits").and_then(|v| v.as_u64()), Some(2));
    }

    #[test]
    fn weak_password_not_triggered_for_non_weak() {
        // username/password 均非弱关键字，不应产 finding。
        let e = mk_entry(1, "GET", "/", Some("username=alice&password=Str0ngP@ss"));
        let eng = SignatureEngine::default();
        let scan = DefaultSensitiveScan::default();
        let rules = empty_ruleset();
        let report = scan_log(&[e], &eng, &scan, &rules);
        let wp: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.r#type == "weak_password")
            .collect();
        assert!(wp.is_empty(), "non-weak creds must not yield finding");
    }
}
