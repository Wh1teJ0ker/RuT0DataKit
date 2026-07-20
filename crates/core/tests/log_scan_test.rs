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
        decoded_path: ruT0_data_kit_core::log::url_decode_twice(path),
        decoded_query: query.map(|q| {
            let pairs = ruT0_data_kit_core::log::parse_query(q);
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
        decoded_ua: ruT0_data_kit_core::log::url_decode_twice("curl/7.88.0"),
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
            decoded_path: String::new(),
            decoded_query: None,
            decoded_ua: String::new(),
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
            decoded_path: String::new(),
            decoded_query: None,
            decoded_ua: String::new(),
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
            decoded_path: String::new(),
            decoded_query: None,
            decoded_ua: String::new(),
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

/// 8. summary 为 serde_yml::Value::Mapping；v0.2.2 起 extra 为
///    `Mapping({ "blind_aggregation": [...] })`（无盲注探针时为空数组）。
#[test]
fn scan_log_report_shape() {
    let e = mk_entry(1, "GET", "/news.php", Some("id=1+union+select+1,2,3"));
    let report = run(&[e]);
    assert!(matches!(report.summary, Value::Mapping(_)));
    // v0.2.2（T2-13）：extra 由 Value::Null 改为 Value::Mapping，
    // 含 blind_aggregation 键（无盲注探针时为空 sequence）。
    let extra = report
        .extra
        .as_mapping()
        .expect("extra must be a mapping after T2-13");
    assert!(
        extra
            .get("blind_aggregation")
            .and_then(|v| v.as_sequence())
            .is_some(),
        "extra must contain blind_aggregation sequence",
    );
    assert_eq!(report.source, "log");
}

/// 9. sqli finding 的 extra 字段含 parsed_payload.summary（盲注二分行：
///    summary 含「盲注二分」）；weak_password / sensitive finding 的 extra 为 None。
#[test]
fn scan_log_finding_extra_carries_parsed_payload() {
    // 盲注二分 payload：parse_query 已把 `+` 预解为空格，`%23` 双重解码为 `#`。
    let blind = mk_entry(
        1,
        "GET",
        "/search.php",
        Some("id=1%2527%20or%20ascii(substr((database())%2C1%2C1))%3E79%23"),
    );
    let report = run(&[blind]);
    let sqli: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.r#type == "sqli")
        .collect();
    assert!(!sqli.is_empty(), "blind binary must hit");
    let s = sqli[0];
    assert!(s.extra.is_some(), "sqli finding extra must be filled");
    let extra = s.extra.as_ref().unwrap();
    let summary = extra
        .as_mapping()
        .and_then(|m| m.get("summary"))
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("extra.summary missing, extra={extra:?}"));
    assert!(
        summary.contains("盲注"),
        "summary must contain 盲注, got {summary}",
    );
}

/// 10. weak_password finding 的 extra 为 None（向后兼容：序列化时不出现 extra）。
#[test]
fn scan_log_weak_password_extra_none() {
    let e = mk_entry(1, "GET", "/", Some("username=guest&password=123456"));
    let report = run(&[e]);
    let wp: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.r#type == "weak_password")
        .collect();
    assert_eq!(wp.len(), 1);
    assert!(wp[0].extra.is_none(), "weak_password extra must be None");
    // 序列化后不含 "extra" key（skip_serializing_if 生效）
    let yaml = serde_yml::to_string(&wp[0]).unwrap();
    assert!(
        !yaml.contains("extra"),
        "serialized weak_password finding must not contain extra, got:\n{yaml}",
    );
}

/// 11. `union all select 1,2,3` 命中 sqli_union（pattern 扩变体后兼容 union all）。
#[test]
fn scan_log_union_all_select_hits() {
    // T2-7 parse_query 已把 `+` 预解为空格：`1+union+all+select+1,2,3` →
    // `1 union all select 1,2,3`，命中 `\bunion\b(?:\s+all)?\s+select\b`。
    let e = mk_entry(1, "GET", "/news.php", Some("id=1+union+all+select+1,2,3"));
    let report = run(&[e]);
    let sqli: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.r#type == "sqli" && f.value == "sqli_union")
        .collect();
    assert!(!sqli.is_empty(), "union all select must hit sqli_union");
}

/// 12. v0.2.2（T2-13）：scan_log 末尾调 BlindAggregator，结果塞进
///     `report.extra.blind_aggregation`。合成几条 database() 第 1 字符 'p'
///     （ascii 112）的盲注二分探针（同源单 IP），断言聚合后 extra 为
///     mapping 且含 `blind_aggregation` 键、数组非空、首项
///     `read_target=="database()"` 且 `decoded_string` 首字符 == `'p'`。
///
/// 合成数据仿照 blind_aggregator.rs 模块级 fixture 验证表：
/// - true_size = 862（条件成立 → body 较小）
/// - false_size = 875
/// - thr∈{79,103,109,111} → body==862（true）
/// - thr∈{112,115} → body==875（false）
/// - min(false_thresholds)=112 == ascii('p')，max(true)+1=112 自洽。
#[test]
fn blind_aggregation_in_report_extra() {
    // 直接构造 LogEntry（不走 mk_entry，因需要自定义 size + decoded_query
    // 含 ascii(substr(...)) 探针）。
    fn blind_entry(line_no: usize, thr: u32, body: u64) -> LogEntry {
        let q = format!("id=1' or ascii(substr((database()),1,1))>{thr}#");
        LogEntry {
            line_no,
            ip: "10.0.0.9".to_string(),
            timestamp: "17/Nov/2023:03:45:42 +0000".to_string(),
            method: "GET".to_string(),
            path: "/news.php".to_string(),
            query: Some(q.clone()),
            status: 200,
            size: Some(body),
            user_agent: "curl".to_string(),
            raw: String::new(),
            decoded_path: String::new(),
            decoded_query: Some(q), // BlindAggregator 直接读 decoded_query
            decoded_ua: String::new(),
        }
    }

    // database() 第 1 字符 'p'=ascii 112 的 7 探针。
    let entries = vec![
        blind_entry(1, 79, 862),   // 112>79 = true  → body 862
        blind_entry(2, 103, 862),  // 112>103 = true → body 862
        blind_entry(3, 109, 862),  // 112>109 = true → body 862
        blind_entry(4, 111, 862),  // 112>111 = true → body 862
        blind_entry(5, 112, 875),  // 112>112 = false → body 875
        blind_entry(6, 115, 875),  // 112>115 = false → body 875
    ];

    let report = run(&entries);

    // extra 为 mapping。
    let extra = report
        .extra
        .as_mapping()
        .expect("report.extra must be a mapping");
    // 含 blind_aggregation 键，值为非空 sequence。
    let blind_agg = extra
        .get("blind_aggregation")
        .and_then(|v| v.as_sequence())
        .expect("blind_aggregation must be a sequence");
    assert!(
        !blind_agg.is_empty(),
        "blind_aggregation must be non-empty for blind entries",
    );

    // 首项 read_target == "database()"。
    let first = blind_agg[0]
        .as_mapping()
        .expect("blind_aggregation item is mapping");
    assert_eq!(
        first.get("read_target").and_then(|v| v.as_str()),
        Some("database()"),
        "first result read_target == database()",
    );

    // decoded_string 首字符 == 'p'。
    let decoded = first
        .get("decoded_string")
        .and_then(|v| v.as_str())
        .expect("decoded_string present");
    assert!(
        decoded.starts_with('p'),
        "decoded_string must start with 'p', got {decoded:?}",
    );
}
