//! T2-2 logsign 模块集成测试（6 类签名 × 正例 + 负例 + 引擎加载）。
//!
//! 正例用内联构造的 `LogEntry`（不走 fixture），避免与 T2-6 补造 fixture 循环依赖。
//! 负例用纯正常请求行。
//!
//! T2-8 增补 `payload_parser` 6 类语义解析正反例单测 + fixture 盲注行集成断言。

mod common;

use std::fs;

use ruT0_data_kit_core::log::{LogEntry, LogReader};
use ruT0_data_kit_core::logsign::{
    load_builtin_signatures, load_signatures_from_str, parse_payload, SignatureEngine,
    SignatureRule,
};
/// 内联构造 LogEntry（不走 fixture）。
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

/// 引擎加载测试：内置 6 条规则，且 enabled 全 true。
#[test]
fn engine_loads_six_builtin_rules_all_enabled() {
    let eng = SignatureEngine::default();
    assert_eq!(eng.rule_count(), 6);
    let rules = load_builtin_signatures();
    assert_eq!(rules.len(), 6);
    assert!(rules.iter().all(|r| r.enabled));
}

/// YAML 可经 `load_signatures_from_str` 反序列化回 `Vec<SignatureRule>`。
#[test]
fn yaml_round_trips_through_serde() {
    let rules = load_builtin_signatures();
    let yaml = serde_yml::to_string(&rules).expect("serialize");
    let back: Vec<SignatureRule> = load_signatures_from_str(&yaml).expect("parse back");
    assert_eq!(back.len(), 6);
    assert_eq!(back[0].rule_id, "sqli_blind_binary");
}

// ============ 6 类 × 正例 ============

#[test]
fn sqli_blind_binary_positive() {
    // decoded 后：`1' or ascii(substr((database()),1,1))>79#`
    let q = "id=1%2527%20or%20ascii(substr((database())%2C1%2C1))%3E79%23";
    let e = mk_entry(2, "GET", "/search.php", Some(q));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(hits.iter().any(|h| h.rule_id == "sqli_blind_binary"),
        "blind_binary must hit, hits={:?}", hits);
}

#[test]
fn sqli_union_positive() {
    let q = "id=1+union+select+1,2,3";
    let e = mk_entry(3, "GET", "/news.php", Some(q));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(hits.iter().any(|h| h.rule_id == "sqli_union"),
        "union must hit, hits={:?}", hits);
}

#[test]
fn sqli_error_based_positive() {
    let q = "id=1+and+extractvalue(1,concat(0x7e,version()))";
    let e = mk_entry(4, "GET", "/news.php", Some(q));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(hits.iter().any(|h| h.rule_id == "sqli_error_based"),
        "error_based must hit, hits={:?}", hits);
}

#[test]
fn sqli_time_based_positive() {
    let q = "id=1+and+sleep(5)";
    let e = mk_entry(5, "GET", "/news.php", Some(q));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(hits.iter().any(|h| h.rule_id == "sqli_time_based"),
        "time_based must hit, hits={:?}", hits);
}

#[test]
fn sqli_tautology_positive() {
    // decoded 后：`1 or 1=1`（%20 → 空格；T2-1 只解码 %XX，不解 `+`）。
    let q = "id=1%20or%201=1";
    let e = mk_entry(6, "GET", "/news.php", Some(q));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(hits.iter().any(|h| h.rule_id == "sqli_tautology"),
        "tautology must hit, hits={:?}", hits);
}

#[test]
fn sqli_comment_positive() {
    let q = "id=1'--";
    let e = mk_entry(7, "GET", "/news.php", Some(q));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(hits.iter().any(|h| h.rule_id == "sqli_comment"),
        "comment must hit, hits={:?}", hits);
}

// ============ 6 类 × 负例（正常请求无 false positive） ============

#[test]
fn sqli_blind_binary_negative() {
    // 不含 ascii(substr(...))，不命中盲注二分。
    let e = mk_entry(11, "GET", "/api", Some("x=ascii&y=substr"));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(!hits.iter().any(|h| h.rule_id == "sqli_blind_binary"),
        "no blind_binary fp, hits={:?}", hits);
}

#[test]
fn sqli_union_negative() {
    // 含 select 但无 union，不命中 union。
    let e = mk_entry(12, "GET", "/docs", Some("kind=select-all"));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(!hits.iter().any(|h| h.rule_id == "sqli_union"),
        "no union fp, hits={:?}", hits);
}

#[test]
fn sqli_error_based_negative() {
    // 不含 extractvalue/updatexml/floor(rand(。
    let e = mk_entry(13, "GET", "/p", Some("v=floor&x=extract"));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(!hits.iter().any(|h| h.rule_id == "sqli_error_based"),
        "no error_based fp, hits={:?}", hits);
}

#[test]
fn sqli_time_based_negative() {
    // `sleep` 作为单独单词无 `(`，不命中时间盲注。
    let e = mk_entry(14, "GET", "/p", Some("mode=sleep&max=1"));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(!hits.iter().any(|h| h.rule_id == "sqli_time_based"),
        "no time_based fp, hits={:?}", hits);
}

/// 5. sqli_tautology_negative 注释更新（T2-7 已支持 `+` 解码）。
#[test]
fn sqli_tautology_negative() {
    // `or 1` 不成 `or 数字=数字`，不命中恒真。
    let e = mk_entry(15, "GET", "/p", Some("x=or%201"));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(!hits.iter().any(|h| h.rule_id == "sqli_tautology"),
        "no tautology fp, hits={:?}", hits);
}

#[test]
fn sqli_comment_negative() {
    // 不含引号上下文，注释类不命中。
    let e = mk_entry(16, "GET", "/page.html", Some("v=hello#world"));
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(!hits.iter().any(|h| h.rule_id == "sqli_comment"),
        "no comment fp, hits={:?}", hits);
}

// ============ 边界用例 ============

#[test]
fn scan_skips_408_timeout_line() {
    // 408 超时行：method/path 为空，0 命中。
    let e = mk_entry(18, "", "", None);
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(hits.is_empty(), "408 line must produce 0 hit");
}

#[test]
fn scan_normal_home_request_no_fp() {
    let e = mk_entry(1, "GET", "/", None);
    let hits = SignatureEngine::default().scan_log_entry(&e);
    assert!(hits.is_empty(), "normal GET / must produce 0 hit");
}

// ============ T2-8 payload_parser 6 类语义解析 ============

#[test]
fn payload_blind_boolean_positive() {
    let r = parse_payload("1' or ascii(substr((database()),1,1))>79#").unwrap();
    assert_eq!(r.attack_type, "blind_boolean");
    assert_eq!(r.technique, "binary_search");
    assert_eq!(r.read_target.as_deref(), Some("database()"));
    assert_eq!(r.char_position, Some(1));
    assert_eq!(r.comparator.as_deref(), Some(">"));
    assert_eq!(r.compared_ascii, Some(79));
}

#[test]
fn payload_blind_boolean_negative() {
    assert!(parse_payload("normal query").is_none());
}

#[test]
fn payload_union_positive() {
    let r = parse_payload("1 union select 1,2,3").unwrap();
    assert_eq!(r.attack_type, "union");
    assert_eq!(r.union_columns, Some(3));
}

#[test]
fn payload_union_all_positive() {
    let r = parse_payload("1 union all select 1,2,3,4").unwrap();
    assert_eq!(r.attack_type, "union");
    assert_eq!(r.union_columns, Some(4));
}

#[test]
fn payload_union_negative() {
    assert!(parse_payload("select 1").is_none());
}

#[test]
fn payload_error_positive() {
    let r = parse_payload("1 and extractvalue(1,concat(0x7e,user()))").unwrap();
    assert_eq!(r.attack_type, "error");
    assert_eq!(r.read_target.as_deref(), Some("user()"));
}

#[test]
fn payload_error_negative() {
    assert!(parse_payload("concat(1,2)").is_none());
}

#[test]
fn payload_time_positive() {
    let r = parse_payload("1 and sleep(5)").unwrap();
    assert_eq!(r.attack_type, "time");
    assert_eq!(r.sleep_seconds, Some(5));
}

#[test]
fn payload_time_negative() {
    assert!(parse_payload("sleep mode").is_none());
}

#[test]
fn payload_tautology_positive() {
    let r = parse_payload("1 or 1=1").unwrap();
    assert_eq!(r.attack_type, "tautology");
    assert!(r.summary.contains("恒真"));
}

#[test]
fn payload_tautology_negative() {
    assert!(parse_payload("1=1").is_none());
}

#[test]
fn payload_comment_positive() {
    let r = parse_payload("1'--").unwrap();
    assert_eq!(r.attack_type, "comment");
    assert!(r.summary.contains("注释"));
}

#[test]
fn payload_comment_negative() {
    assert!(parse_payload("normal").is_none());
}

#[test]
fn payload_none_on_normal_text() {
    assert!(parse_payload("normal text without sqli").is_none());
}

/// SignatureHit.parsed_payload 在 fixture 盲注行（access.log L19）命中时为
/// `Some(_)`，read_target=Some("database()")。
#[test]
fn scan_log_entry_fills_parsed_payload_on_blind_fixture_line() {
    let path = common::fixtures_dir().join("log/access.log");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read access.log: {e:?}"));
    let reader = LogReader::new().expect("reader");
    let entries = reader
        .read(&path)
        .unwrap_or_else(|e| panic!("parse access.log: {e:?}"));
    // 第 19 行（1-based）即首个盲注二分行。
    let blind_line = entries
        .iter()
        .find(|e| e.line_no == 19)
        .unwrap_or_else(|| panic!("missing line 19; entries={}", entries.len()));
    let _ = raw;
    let eng = SignatureEngine::default();
    let hits = eng.scan_log_entry(blind_line);
    let blind_hit = hits
        .iter()
        .find(|h| h.rule_id == "sqli_blind_binary")
        .expect("blind_binary must hit on line 19");
    let parsed = blind_hit
        .parsed_payload
        .as_ref()
        .expect("parsed_payload must be Some on blind fixture line");
    assert_eq!(parsed.attack_type, "blind_boolean");
    assert_eq!(parsed.read_target.as_deref(), Some("database()"));
}
