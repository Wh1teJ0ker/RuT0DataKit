//! T2-3 log_scan pipeline 集成测试。
//!
//! 覆盖：单条 SQLi 命中、单条弱口令命中、正常请求无命中、混合 summary 计数、
//! top_attack_ips 排序+截断、对 tests/fixtures/samples/log/access.log 实测
//! （SQLi > 1000，弱口令 >= 4），以及非弱口令不误报。

mod common;

use ruT0_data_kit_core::log::LogEntry;
use ruT0_data_kit_core::logsign::SignatureEngine;
use ruT0_data_kit_core::pipeline::scan_log;
use ruT0_data_kit_core::rules::RuleSet;
use ruT0_data_kit_core::scan::DefaultSensitiveScan;
use serde_yml::Value;

/// 内联构造 LogEntry。
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
    }
}

fn empty_ruleset() -> RuleSet {
    RuleSet {
        validators: vec![],
        maskers: vec![],
    }
}

fn run(entries: &[LogEntry]) -> ruT0_data_kit_core::report::Report {
    let eng = SignatureEngine::default();
    let scan = DefaultSensitiveScan::default();
    let rules = empty_ruleset();
    scan_log(entries, &eng, &scan, &rules)
}

/// 1. 单条 union select 命中产 1 个 sqli finding。
#[test]
fn scan_log_basic_sqli_hit() {
    let e = mk_entry(1, "GET", "/news.php", Some("id=1+union+select+1,2,3"));
    let report = run(&[e]);
    assert_eq!(report.kind, "log_scan");
    let sqli: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.r#type == "sqli")
        .collect();
    assert!(!sqli.is_empty(), "must have sqli finding");
    assert_eq!(sqli[0].value, "sqli_union");
    assert_eq!(sqli[0].location.as_deref(), Some("1"));
    assert!(sqli[0].context.is_some(), "context must be filled");
}

/// 2. 弱口令 guest/123 命中产 1 个 weak_password finding。
#[test]
fn scan_log_weak_password_guest() {
    let e = mk_entry(1, "GET", "/", Some("username=guest&password=123"));
    let report = run(&[e]);
    let wp: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.r#type == "weak_password")
        .collect();
    assert_eq!(wp.len(), 1);
    assert_eq!(wp[0].value, "username=guest&password=123");
    assert_eq!(wp[0].location.as_deref(), Some("1"));
}

/// 3. 正常 GET 请求无 finding。
#[test]
fn scan_log_normal_request_no_finding() {
    let e = mk_entry(1, "GET", "/index.html", None);
    let report = run(&[e]);
    assert!(
        report.findings.is_empty(),
        "normal request must not yield findings"
    );
}

/// 4. 混合 entries：summary 的 total_lines / sqli_hits / weak_password_hits 正确。
#[test]
fn scan_log_summary_counts() {
    let entries = vec![
        mk_entry(1, "GET", "/news.php", Some("id=1+union+select+1,2,3")),
        mk_entry(2, "GET", "/", Some("username=guest&password=123456")),
        mk_entry(3, "GET", "/", Some("username=alice&password=Str0ngP@ss")),
        mk_entry(4, "GET", "/index.html", None),
    ];
    let report = run(&entries);
    let m = report.summary.as_mapping().expect("summary is mapping");
    assert_eq!(m.get("total_lines").and_then(|v| v.as_u64()), Some(4));
    assert!(m.get("sqli_hits").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
    assert_eq!(
        m.get("weak_password_hits").and_then(|v| v.as_u64()),
        Some(1),
        "only one weak_password finding"
    );
}

/// 5. top_attack_ips 排序 + 取前 5：6 个 IP 各命中不同次数，截断到 5 且有序。
#[test]
fn scan_log_top_attack_ips() {
    let mut entries = Vec::new();
    let mut line = 1usize;
    // ip1 命中 3 次
    for _ in 0..3 {
        entries.push(LogEntry {
            line_no: line,
            ip: "10.0.0.1".into(),
            timestamp: String::new(),
            method: "GET".into(),
            path: "/news.php".into(),
            query: Some("id=1+union+select+1,2,3".into()),
            status: 200,
            size: None,
            user_agent: String::new(),
            raw: String::new(),
        });
        line += 1;
    }
    // ip2 命中 2 次
    for _ in 0..2 {
        entries.push(LogEntry {
            line_no: line,
            ip: "10.0.0.2".into(),
            timestamp: String::new(),
            method: "GET".into(),
            path: "/news.php".into(),
            query: Some("id=1+union+select+1,2,3".into()),
            status: 200,
            size: None,
            user_agent: String::new(),
            raw: String::new(),
        });
        line += 1;
    }
    // ip3..ip6 各命中 1 次（共 4 个 IP）
    for idx in 0..4 {
        entries.push(LogEntry {
            line_no: line,
            ip: format!("10.0.0.{}", 3 + idx),
            timestamp: String::new(),
            method: "GET".into(),
            path: "/news.php".into(),
            query: Some("id=1+union+select+1,2,3".into()),
            status: 200,
            size: None,
            user_agent: String::new(),
            raw: String::new(),
        });
        line += 1;
    }
    let report = run(&entries);
    let m = report.summary.as_mapping().unwrap();
    let top = m
        .get("top_attack_ips")
        .and_then(|v| v.as_sequence())
        .expect("top_attack_ips is sequence");
    assert_eq!(top.len(), 5, "must be truncated to top 5");
    let first = top[0].as_mapping().unwrap();
    assert_eq!(first.get("ip").and_then(|v| v.as_str()), Some("10.0.0.1"));
    assert_eq!(first.get("hits").and_then(|v| v.as_u64()), Some(3));
    let second = top[1].as_mapping().unwrap();
    assert_eq!(second.get("ip").and_then(|v| v.as_str()), Some("10.0.0.2"));
    assert_eq!(second.get("hits").and_then(|v| v.as_u64()), Some(2));
}

/// 6. 对 tests/fixtures/samples/log/access.log 实测：
///    SQLi 命中 > 1000，弱口令命中 >= 4（guest/123456、guest/123、guest/admin、lisi/admin）。
#[test]
fn scan_log_fixture_access_log() {
    let path = common::fixtures_dir().join("log/access.log");
    let reader = ruT0_data_kit_core::log::LogReader::default();
    let entries: Vec<LogEntry> = reader
        .read(&path)
        .unwrap_or_else(|e| panic!("read access.log: {e:?}"));
    assert!(!entries.is_empty(), "fixture must have entries");
    let report = run(&entries);
    let m = report.summary.as_mapping().unwrap();
    let sqli = m
        .get("sqli_hits")
        .and_then(|v| v.as_u64())
        .expect("sqli_hits present");
    assert!(sqli > 1000, "SQLi hits > 1000, got {sqli}");
    let wp = m
        .get("weak_password_hits")
        .and_then(|v| v.as_u64())
        .expect("weak_password_hits present");
    assert!(wp >= 4, "weak_password hits >= 4, got {wp}");
    // top_attack_ips 至少含一项且结构正确
    let top = m
        .get("top_attack_ips")
        .and_then(|v| v.as_sequence())
        .unwrap();
    assert!(!top.is_empty());
    let first = top[0].as_mapping().unwrap();
    assert!(first.get("ip").is_some());
    assert!(first.get("hits").is_some());
}

/// 7. 非弱口令（username/password 均非弱关键字）不误报。
#[test]
fn scan_log_non_weak_password_no_finding() {
    let e = mk_entry(1, "GET", "/", Some("username=alice&password=Str0ngP@ss"));
    let report = run(&[e]);
    let wp: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.r#type == "weak_password")
        .collect();
    assert!(wp.is_empty(), "non-weak creds must not yield finding");
}

/// 8. summary 为 serde_yml::Value::Mapping，extra 为 Null。
#[test]
fn scan_log_report_shape() {
    let e = mk_entry(1, "GET", "/news.php", Some("id=1+union+select+1,2,3"));
    let report = run(&[e]);
    assert!(matches!(report.summary, Value::Mapping(_)));
    assert!(matches!(report.extra, Value::Null));
    assert_eq!(report.source, "log");
}
