//! T2-12 blind_aggregator 集成测试。
//!
//! 验证 `BlindAggregator::collect_from_entries` 的正则抽取（database() /
//! 嵌套 group_concat read_target）与 `aggregate` 的二分还原（合成 7 探针
//! ascii=112='p'、越界位置、unresolved 全 true）。

mod common;

use ruT0_data_kit_core::log::LogEntry;
use ruT0_data_kit_core::logsign::{AggregatedResult, BlindAggregator};

/// 内联构造 LogEntry（不走 fixture）。
fn mk_entry(line_no: usize, ip: &str, decoded_query: &str, size: Option<u64>) -> LogEntry {
    LogEntry {
        line_no,
        ip: ip.to_string(),
        timestamp: "17/Nov/2023:03:45:42 +0000".to_string(),
        method: "GET".to_string(),
        path: "/".to_string(),
        query: Some(decoded_query.to_string()),
        status: 200,
        size,
        user_agent: "curl".to_string(),
        raw: String::new(),
        decoded_path: String::new(),
        decoded_query: Some(decoded_query.to_string()),
        decoded_ua: String::new(),
    }
}

/// 单条 `ascii(substr((database()),1,1))>79` + size=875 → 抽出 1 个 BlindProbe，
/// read_target=="database()" / char_position==1 / threshold==79 / body_size==875。
#[test]
fn collect_extracts_database_probe() {
    let e = mk_entry(
        2,
        "10.112.16.207",
        "username=1' or ascii(substr((database()),1,1))>79#&password=1",
        Some(875),
    );
    let agg = BlindAggregator::collect_from_entries(&[e]);
    assert_eq!(agg.probes().len(), 1);
    let p = &agg.probes()[0];
    assert_eq!(p.read_target, "database()");
    assert_eq!(p.char_position, 1);
    assert_eq!(p.threshold, 79);
    assert_eq!(p.body_size, 875);
    assert_eq!(p.source_ip, "10.112.16.207");
    assert_eq!(p.line_no, 2);
}

/// 嵌套 read_target `ascii(substr((select group_concat(table_name) from
/// information_schema.tables where table_schema=database()),1,1))>79` 的
/// read_target 捕获组含 `group_concat(table_name)` 整段。
#[test]
fn collect_extracts_nested_read_target() {
    let payload =
        "username=1' or ascii(substr((select group_concat(table_name) from \
         information_schema.tables where table_schema=database()),1,1))>79#&password=1";
    let e = mk_entry(3, "10.112.16.207", payload, Some(862));
    let agg = BlindAggregator::collect_from_entries(&[e]);
    assert_eq!(agg.probes().len(), 1);
    let rt = &agg.probes()[0].read_target;
    assert!(
        rt.contains("group_concat(table_name)"),
        "read_target must contain group_concat(table_name), got: {rt}"
    );
    assert!(
        rt.contains("information_schema.tables"),
        "read_target must contain information_schema.tables, got: {rt}"
    );
    assert_eq!(agg.probes()[0].char_position, 1);
    assert_eq!(agg.probes()[0].threshold, 79);
}

/// 合成 7 探针二分 ascii=112='p'（fixture 真假方向：条件成立→862，否则 875）
/// + 越界位置 8（全同 875），aggregate 得 decoded_string 首字符 == "p"。
#[test]
fn aggregate_decodes_single_char_p() {
    // group_true_size = min(body_size) = 862（条件成立 body）。
    // pos 1: thr 79/103/109/111 → 862 (true, ascii>thr)；
    //         thr 112/115 → 875 (false, ascii<=thr)。
    // min(false_thresholds)=112 → ascii_val=112='p'，max(true)+1=112 自洽。
    let pos1 = vec![
        (79u32, 862u64),
        (103, 862),
        (109, 862),
        (111, 862),
        (112, 875),
        (115, 875),
    ];
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut line = 19usize;
    for (thr, body) in &pos1 {
        let q = format!(
            "username=1' or ascii(substr((database()),1,1))>{}#&password=1",
            thr
        );
        entries.push(mk_entry(line, "10.112.16.207", &q, Some(*body)));
        line += 1;
    }
    // pos 8: 全同 875 → beyond_end。
    for thr in [79u32, 103, 115] {
        let q = format!(
            "username=1' or ascii(substr((database()),8,1))>{}#&password=1",
            thr
        );
        entries.push(mk_entry(line, "10.112.16.207", &q, Some(875)));
        line += 1;
    }

    let agg = BlindAggregator::collect_from_entries(&entries);
    let results = agg.aggregate();
    assert_eq!(results.len(), 1, "expect 1 group, got {:?}", results.len());
    let r: &AggregatedResult = &results[0];
    assert_eq!(r.read_target, "database()");
    assert_eq!(r.probe_count, 9);
    // 首字符应为 'p'。
    assert!(
        r.decoded_string.starts_with('p'),
        "decoded_string must start with 'p', got: {}",
        r.decoded_string
    );
    // 首个 beyond_end 截断：position_details 应到 pos 1 后即 pos 8 beyond_end。
    let pos1_detail = r
        .position_details
        .iter()
        .find(|d| d.position == 1)
        .expect("pos 1 detail");
    assert_eq!(pos1_detail.status, "resolved");
    assert_eq!(pos1_detail.ascii_val, Some(112));
    assert_eq!(pos1_detail.decoded_char, Some('p'));
    assert_eq!(pos1_detail.true_size, 862);
    let pos8_detail = r
        .position_details
        .iter()
        .find(|d| d.position == 8)
        .expect("pos 8 detail");
    assert_eq!(pos8_detail.status, "beyond_end");
    assert_eq!(pos8_detail.decoded_char, None);
    assert!(r.beyond_end_positions >= 1);
}

/// 越界位置（全同 body_size 且 != group_true_size）→ status=="beyond_end"。
#[test]
fn aggregate_beyond_end_position() {
    // pos 1: 一条 862（true），pos 2: 全同 875（false）。
    // group_true_size = min(862, 875, 875) = 862。
    // pos 2 全同 875 != 862 → beyond_end。
    let entries = vec![
        mk_entry(
            1,
            "1.1.1.1",
            "ascii(substr((database()),1,1))>79",
            Some(862),
        ),
        mk_entry(
            2,
            "1.1.1.1",
            "ascii(substr((database()),2,1))>79",
            Some(875),
        ),
        mk_entry(
            3,
            "1.1.1.1",
            "ascii(substr((database()),2,1))>103",
            Some(875),
        ),
    ];
    let agg = BlindAggregator::collect_from_entries(&entries);
    let results = agg.aggregate();
    let r = &results[0];
    let pos2 = r
        .position_details
        .iter()
        .find(|d| d.position == 2)
        .expect("pos 2 detail");
    assert_eq!(pos2.status, "beyond_end");
    assert_eq!(pos2.decoded_char, None);
    assert_eq!(pos2.true_size, 862);
}

/// 全 true 位置（无 false 探针，全同 body == group_true_size）→
/// status=="unresolved_all_true" + decoded_char==None。
#[test]
fn aggregate_unresolved_all_true() {
    // pos 1: 全 862（条件都成立），且 group_true_size=862 → unresolved_all_true。
    let entries = vec![
        mk_entry(
            1,
            "1.1.1.1",
            "ascii(substr((database()),1,1))>79",
            Some(862),
        ),
        mk_entry(
            2,
            "1.1.1.1",
            "ascii(substr((database()),1,1))>103",
            Some(862),
        ),
        mk_entry(
            3,
            "1.1.1.1",
            "ascii(substr((database()),1,1))>115",
            Some(862),
        ),
    ];
    let agg = BlindAggregator::collect_from_entries(&entries);
    let results = agg.aggregate();
    let r = &results[0];
    let pos1 = r
        .position_details
        .iter()
        .find(|d| d.position == 1)
        .expect("pos 1 detail");
    assert_eq!(pos1.status, "unresolved_all_true");
    assert_eq!(pos1.decoded_char, None);
    assert_eq!(pos1.ascii_val, None);
    assert_eq!(pos1.true_size, 862);
    // decoded_string 该位插 '?'。
    assert!(r.decoded_string.contains('?'));
    assert_eq!(r.unresolved_chars, 1);
}

/// 不自洽的混合位置 → insufficient_probes。
#[test]
fn aggregate_insufficient_probes_when_not_self_consistent() {
    // pos 1: thr=79→862(true), thr=100→875(false)，但 max(true)+1=80 != min(false)=100。
    let entries = vec![
        mk_entry(
            1,
            "1.1.1.1",
            "ascii(substr((database()),1,1))>79",
            Some(862),
        ),
        mk_entry(
            2,
            "1.1.1.1",
            "ascii(substr((database()),1,1))>100",
            Some(875),
        ),
    ];
    let agg = BlindAggregator::collect_from_entries(&entries);
    let results = agg.aggregate();
    let r = &results[0];
    let pos1 = r
        .position_details
        .iter()
        .find(|d| d.position == 1)
        .expect("pos 1 detail");
    assert_eq!(pos1.status, "insufficient_probes");
    assert_eq!(pos1.decoded_char, None);
}
