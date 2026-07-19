//! T2-1 log 模块单元/集成测试。
//!
//! 覆盖 HANDOFF acceptance_criteria 要求的 6 个用例：
//! - parse_basic_clf
//! - parse_nginx_combined
//! - parse_query_double_decode
//! - parse_malformed_returns_err
//! - parse_url_encoded_ip
//! - access_log_fixture_all_lines

mod common;

use ruT0_data_kit_core::log::{parse_query, url_decode_twice, LogEntry, LogReader};
use ruT0_data_kit_core::error::CoreError;
use std::path::PathBuf;

fn access_log_path() -> PathBuf {
    common::fixtures_dir().join("log/access.log")
}

/// 1. 基本 CLF 格式（无 referer、无 UA → 用 "-" 占位的 CLF 行）能解析。
#[test]
fn parse_basic_clf() {
    // 经典 CLF 行：`- -` ident/daemon，request `"GET / HTTP/1.1"`，
    // size 900，referer "-"，UA "-"。
    let line = r#"127.0.0.1 - - [10/Oct/2023:13:55:36 +0000] "GET / HTTP/1.1" 200 1043 "-" "curl/7.88.0""#;
    let r = LogReader::new().expect("reader");
    let e = r.parse_line(line, 1).expect("parse basic clf");
    assert_eq!(e.line_no, 1);
    assert_eq!(e.ip, "127.0.0.1");
    assert_eq!(e.timestamp, "10/Oct/2023:13:55:36 +0000");
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/");
    assert!(e.query.is_none());
    assert_eq!(e.status, 200);
    assert_eq!(e.size, Some(1043));
    assert_eq!(e.user_agent, "curl/7.88.0");
    assert_eq!(e.raw, line);
}

/// 2. Nginx Combined 格式（带 query + 完整 UA）能解析。
#[test]
fn parse_nginx_combined() {
    let line = r#"10.112.16.160 - - [17/Nov/2023:03:44:24 +0000] "GET /?username=guest&password=123456 HTTP/1.1" 200 874 "-" "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36""#;
    let r = LogReader::new().expect("reader");
    let e = r.parse_line(line, 7).expect("parse nginx combined");
    assert_eq!(e.line_no, 7);
    assert_eq!(e.ip, "10.112.16.160");
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/");
    assert_eq!(e.query.as_deref(), Some("username=guest&password=123456"));
    assert_eq!(e.status, 200);
    assert_eq!(e.size, Some(874));
    assert!(e.user_agent.starts_with("Mozilla/5.0 (Macintosh"));
}

/// 3. parse_query 双重 URL 解码：`%2527` → `%27` → `'`，`%20` → 空格。
#[test]
fn parse_query_double_decode() {
    // %2527 → 第一次 %25→% 得 %27 → 第二次 %27→'  → 单引号
    // %20 → 第一次空格，第二次空格（已是普通字符）
    let q = "k=%2527&name=hello%20world&empty";
    let pairs = parse_query(q);
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0], ("k".to_string(), "'".to_string()));
    assert_eq!(pairs[1], ("name".to_string(), "hello world".to_string()));
    assert_eq!(pairs[2], ("empty".to_string(), "".to_string()));

    // 直接 %27 → ' 单层双重解码也 OK
    assert_eq!(url_decode_twice("%27"), "'");
    assert_eq!(url_decode_twice("%20"), " ");
    // 已解码字符串再解一次不破坏
    assert_eq!(url_decode_twice("plain text"), "plain text");
    // 非法 % （% 后非 hex）原样保留
    assert_eq!(url_decode_twice("a%b"), "a%b");
    assert_eq!(url_decode_twice("%zz"), "%zz");
    // %25 → % → 第二次再遇 % 后跟非 hex（zz）→ 原样保留
    assert_eq!(url_decode_twice("%25zz"), "%zz");
    // %2527 → %27 → '（双重解码关键场景）
    assert_eq!(url_decode_twice("%2527"), "'");
}

/// 4. 格式不匹配返回 Err(CoreError::InvalidInput)。
#[test]
fn parse_malformed_returns_err() {
    let r = LogReader::new().expect("reader");
    let bad = "this is not a log line";
    let res = r.parse_line(bad, 1);
    match res {
        Err(CoreError::InvalidInput(_)) => {}
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

/// 5. URL 编码 IP / 路径中的百分号场景：日志里出现 `%xx` 形式的 path/query
///    应能正确解析，query 内 key/value 双重解码。
#[test]
fn parse_url_encoded_ip() {
    // path 含 %2e（点），query 含 username=ad%27min（攻击 payload 形式）
    let line = r#"10.0.0.1 - - [10/Oct/2023:13:55:36 +0000] "GET /admin%2ephp?id=1%27%20OR%201=1 HTTP/1.1" 500 123 "-" "sqlmap/1.6""#;
    let r = LogReader::new().expect("reader");
    let e = r.parse_line(line, 3).expect("parse url-encoded");
    // path 保留原始（不解码），由下游决定
    assert_eq!(e.path, "/admin%2ephp");
    assert_eq!(e.query.as_deref(), Some("id=1%27%20OR%201=1"));
    assert_eq!(e.status, 500);
    assert_eq!(e.size, Some(123));
    // query 拆分后双重解码
    let q = e.query.as_deref().unwrap_or("");
    let pairs = parse_query(q);
    assert_eq!(pairs[0], ("id".to_string(), "1' OR 1=1".to_string()));
}

/// 6. tests/fixtures/samples/log/access.log 全 1860 行 100% 解析成功。
#[test]
fn access_log_fixture_all_lines() {
    let path = access_log_path();
    let r = LogReader::new().expect("reader");
    let entries: Vec<LogEntry> = r.read(&path).expect("read access.log");
    // 行数与文件行数一致（无空行）
    let file_lines = std::fs::read_to_string(&path).unwrap().lines().count();
    assert_eq!(entries.len(), file_lines, "all lines parsed");
    assert_eq!(entries.len(), 1860, "fixture has 1860 lines");
    // line_no 连续递增
    for (i, e) in entries.iter().enumerate() {
        assert_eq!(e.line_no, i + 1, "line_no should be sequential");
        assert!(!e.ip.is_empty(), "ip not empty at line {}", e.line_no);
        // 第 18 行 request line 为 "-"（408 连接异常），method/path 留空属正常
        if e.line_no != 18 {
            assert!(!e.method.is_empty(), "method not empty at line {}", e.line_no);
            assert!(!e.path.is_empty(), "path not empty at line {}", e.line_no);
        }
        assert!(e.status >= 100 && e.status < 600, "status sane at line {}", e.line_no);
    }
    // 抽几行 query 验证弱口令 payload 形态：username=guest&password=123456
    let line2 = &entries[1]; // 第 2 行即 guest/123456
    let q = line2.query.as_deref().unwrap_or("");
    let pairs = parse_query(q);
    let map: std::collections::HashMap<&str, &str> =
        pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    assert_eq!(map.get("username"), Some(&"guest"));
    assert_eq!(map.get("password"), Some(&"123456"));
}
