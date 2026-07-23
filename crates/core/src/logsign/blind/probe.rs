//! 盲注探针抽取层（v0.5.0 T12-1 拆分自 blind_aggregator.rs）。
//!
//! 负责「SQL 文本 → [`BlindProbe`]」的抽取逻辑：三套自带正则
//! （ascii_binary / equality / length）从已解码 SQL 文本中抽取盲注探针，
//! 并定义 [`ProbeKind`] / [`BlindProbe`] / [`PositionDetail`] / [`AggregatedResult`]
//! 等核心数据结构。
//!
//! 与 [`super::aggregate`] / [`super::reconstruct`] 的边界：
//! - 本模块只做探针抽取 + 正则缓存 + 数据结构定义；
//! - 聚类算法（true_size 众数、位置聚合）在 [`super::aggregate`]；
//! - 数据库结构还原在 [`super::reconstruct`]。

use regex::Regex;

/// 纯文本 SQL 盲注探针特征检测（v0.4.1 T6-4）。
///
/// 与 [`extract_blind_probe`] 共享同一份 `ascii_binary_regex / equality_regex
/// / length_regex`，但不依赖 `response_body_size`——只要任一正则在 `sql`
/// （lowercase）中命中即返回 `true`。供 Tauri `detect_sql_blind_features`
/// 在 PreprocessView 导入后做轻量扫描，命中则自动跳转 SqlParseTool。
///
/// 仅做本地正则匹配，不调用网络（满足 docs/00 §6 「不外发数据」约束）。
pub fn looks_like_blind_probe(sql: &str) -> bool {
    let lower = sql.to_lowercase();
    ascii_binary_regex().is_match(&lower)
        || equality_regex().is_match(&lower)
        || length_regex().is_match(&lower)
}

/// 从一条 SQL 文本提取所有盲注探针（v0.2.4 T5-9 重构）。
///
/// 把 v0.2.2/v0.2.3 写死在 [`super::aggregate::BlindAggregator::collect_from_entries`] 内的
/// 「SQL 文本 → [`BlindProbe`]」逻辑独立成纯函数，使 [`crate::tools::sql_parse`]
/// 能直接接受纯 SQL 输入复用同一份提取逻辑，而 `log_scan` 路径继续调用它
/// 保持完全兼容。
///
/// - `sql`：已解码的完整 SQL 文本（caller 保证已 URL 解码，本函数不再解码）。
/// - `response_body_size`：HTTP 响应 body 字节数；`None` 时返回空 Vec
///   （无 body size 无法判真假，与 v0.2.4 `LogEntry.size = None` 跳过行为一致）。
/// - `source_ip`：来源 IP；`None` 视为空串（tools::sql_parse 路径无 IP 来源时用空）。
///
/// 三类正则（ascii_binary / equality / length）各跑一次，同一条 SQL 可能
/// 同时命中多类（少见），全部返回。`line_no` 固定 0（tools 路径无日志行号；
/// `collect_from_entries` 在调用方覆盖真实 `line_no`）。
pub fn extract_blind_probe(
    sql: &str,
    response_body_size: Option<u64>,
    source_ip: Option<&str>,
) -> Vec<BlindProbe> {
    extract_blind_probe_with_line(sql, response_body_size, source_ip, 0)
}

/// 与 [`extract_blind_probe`] 同语义，但允许调用方传入 `line_no`（log_scan 路径
/// 用真实日志行号）。`collect_from_entries` 内部调它以保持 v0.2.4 行为完全一致。
pub fn extract_blind_probe_with_line(
    sql: &str,
    response_body_size: Option<u64>,
    source_ip: Option<&str>,
    line_no: usize,
) -> Vec<BlindProbe> {
    let size = match response_body_size {
        Some(s) => s,
        None => return Vec::new(),
    };
    let ip = source_ip.unwrap_or("").to_string();
    let lower = sql.to_lowercase();
    let re_ascii = ascii_binary_regex();
    let re_eq = equality_regex();
    let re_len = length_regex();
    let mut probes: Vec<BlindProbe> = Vec::new();

    // AsciiBinary：1=read_target、2=char_position、3=comparator、4=threshold。
    for caps in re_ascii.captures_iter(&lower) {
        let read_target = caps.get(1).map(|m| m.as_str().to_string());
        let char_position = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
        let threshold = caps.get(4).and_then(|m| m.as_str().parse::<u32>().ok());
        if let (Some(rt), Some(pos), Some(thr)) = (read_target, char_position, threshold) {
            probes.push(BlindProbe {
                read_target: rt,
                char_position: pos,
                threshold: thr,
                body_size: size,
                source_ip: ip.clone(),
                line_no,
                probe_kind: ProbeKind::AsciiBinary,
                equality_char: None,
            });
        }
    }

    // Equality：1=read_target、2=char_position、3='c' 单字符、4=char(N) ascii。
    for caps in re_eq.captures_iter(&lower) {
        let read_target = caps.get(1).map(|m| m.as_str().to_string());
        let char_position = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
        let eq_char = if let Some(c) = caps.get(3) {
            c.as_str().chars().next()
        } else if let Some(n) = caps.get(4) {
            n.as_str().parse::<u32>().ok().and_then(char::from_u32)
        } else {
            None
        };
        if let (Some(rt), Some(pos), Some(c)) = (read_target, char_position, eq_char) {
            // threshold 填字符 ascii 值作哨兵（equality 不走二分路径）。
            let thr = c as u32;
            probes.push(BlindProbe {
                read_target: rt,
                char_position: pos,
                threshold: thr,
                body_size: size,
                source_ip: ip.clone(),
                line_no,
                probe_kind: ProbeKind::Equality,
                equality_char: Some(c),
            });
        }
    }

    // Length：1=read_target、2=comparator、3=threshold。
    for caps in re_len.captures_iter(&lower) {
        let read_target = caps.get(1).map(|m| m.as_str().to_string());
        let threshold = caps.get(3).and_then(|m| m.as_str().parse::<u32>().ok());
        if let (Some(rt), Some(thr)) = (read_target, threshold) {
            probes.push(BlindProbe {
                read_target: rt,
                char_position: 0, // 哨兵：length 无位置维度。
                threshold: thr,
                body_size: size,
                source_ip: ip.clone(),
                line_no,
                probe_kind: ProbeKind::Length,
                equality_char: None,
            });
        }
    }

    probes
}

/// 盲注探针类型（v0.2.3）。
///
/// - [`ProbeKind::AsciiBinary`]：`ascii(substr((<rt>),<pos>,1))<cmp><thr>`
///   二分序列（v0.2.2 既有）。
/// - [`ProbeKind::Equality`]：`substr((<rt>),<pos>,1)='c'` 或 `=char(<ascii>)`
///   等值形态（v0.2.3 新增）。
/// - [`ProbeKind::Length`]：`length((<rt>))<cmp><thr>` 长度盲注（v0.2.3 新增）。
///
/// `aggregate` 按 `(read_target, source_ip, ProbeKind)` 三元组分组，避免
/// length 与 ascii_binary 同 read_target 混。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeKind {
    /// `ascii(substr((<rt>),<pos>,1))<cmp><thr>` 二分序列。
    AsciiBinary,
    /// `substr((<rt>),<pos>,1)='c'` 或 `=char(<ascii>)` 等值形态。
    Equality,
    /// `length((<rt>))<cmp><thr>` 长度盲注。
    Length,
}

/// 单条盲注探针。
///
/// 由 [`super::aggregate::BlindAggregator::collect_from_entries`] 从 `LogEntry` 抽出。三类形态：
/// - AsciiBinary：`ascii(substr((<read_target>),<char_position>,1))<cmp><threshold>`。
/// - Equality：`substr((<read_target>),<char_position>,1)='c'` 或 `=char(<ascii>)`，
///   `equality_char` 携带还原字符，`threshold` 填字符 ascii 值（哨兵，不参与
///   二分；equality 不走 threshold 二分路径）。
/// - Length：`length((<read_target>))<cmp><threshold>`，`char_position=0`（哨兵），
///   `equality_char=None`。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BlindProbe {
    /// 完整 substr/length 内层表达式，如 `database()` 或
    /// `select group_concat(table_name) from information_schema.tables where table_schema=database()`。
    pub read_target: String,
    /// 盲注读取第几个字符（1-based）。Length 探针固定 0（哨兵，无位置维度）。
    pub char_position: u32,
    /// ascii 比较阈值。Equality 探针填字符 ascii 值（哨兵）。
    pub threshold: u32,
    /// HTTP 响应 body 字节数（来自 `LogEntry.size`）。
    pub body_size: u64,
    /// 来源 IP（`LogEntry.ip`），用于区分不同注入源。
    pub source_ip: String,
    /// 原始日志行号（回引用）。
    pub line_no: usize,
    /// 探针类型（v0.2.3）。
    pub probe_kind: ProbeKind,
    /// Equality 探针捕获的字符（`'c'` 形态或 `char(N)` 解码）；其他类为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equality_char: Option<char>,
}

/// 单个字符位置的聚合详情。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PositionDetail {
    /// 字符位置（1-based）。Length 探针为 0。
    pub position: u32,
    /// 还原出的字符；unresolved/beyond_end/insufficient 为 `None`。
    pub decoded_char: Option<char>,
    /// 还原出的 ascii 值；未还原为 `None`。Length 探针为还原出的长度数值。
    pub ascii_val: Option<u32>,
    /// 该位置条件成立响应的 body size。
    pub true_size: u64,
    /// 该位置的探针数。
    pub probe_count: u32,
    /// 状态：`resolved` / `unresolved_all_true` / `beyond_end` /
    /// `insufficient_probes` / `equality_resolved` / `length_resolved`。
    pub status: String,
}

/// 一个 `(read_target, source_ip, ProbeKind)` 分组聚合后的还原结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AggregatedResult {
    /// 聚合的 read_target。
    pub read_target: String,
    /// 还原出的字符串（按 position 升序拼接，首个 beyond_end 截断；
    /// unresolved/insufficient 位置插 `'?'`；Length 类为十进制数字串）。
    pub decoded_string: String,
    /// 成功还原的字符数。
    pub resolved_chars: u32,
    /// unresolved_all_true / insufficient_probes 的字符数。
    pub unresolved_chars: u32,
    /// beyond_end 位置数（首个 beyond_end 之后的不再计入，但首个本身计入）。
    pub beyond_end_positions: u32,
    /// 该分组总探针数。
    pub probe_count: u32,
    /// 该分组涉及的去重 source_ip 列表。
    pub source_ips: Vec<String>,
    /// 每个位置的详情，按 position 升序。
    pub position_details: Vec<PositionDetail>,
    /// read_target 中首个 `0xNN` 十六进制字面量解码出的分隔符（如有），
    /// 供 GUI 高亮展示。例如 `group_concat(id,0x7e,username,0x7e,idcard)`
    /// 的 read_target 含 `0x7e` → `Some('~')`。无字面量为 `None`（向后兼容）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator_char: Option<char>,
    /// 探针类型字符串（v0.2.3）："ascii_binary" / "equality" / "length"，
    /// 供前端区分展示。与 [`ProbeKind`] serde rename 一致。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// 编译并缓存 AsciiBinary 盲注探针正则。
///
/// 形态：`ascii(substr((<read_target>),<pos>,1))<cmp><thr>`
/// 捕获组：1=read_target、2=char_position、3=comparator、4=threshold。
pub(super) fn ascii_binary_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"ascii\s*\(\s*substr\s*\(\s*\(\s*((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)\s*\)\s*,\s*(\d+)\s*,\s*\d+\s*\)\s*\)\s*(>=?|<=?|=)\s*(\d+)",
        )
        .expect("ascii_binary regex must compile")
    })
}

/// 编译并缓存 Equality 盲注探针正则。
///
/// 形态 A：`substr((<read_target>),<pos>,1)='c'`
/// 形态 B：`substr((<read_target>),<pos>,1)=char(<ascii>)`
/// 捕获组：1=read_target、2=char_position、3=单字符（'c' 形态）、
/// 4=ascii 数字（char(N) 形态，二选一）。
pub(super) fn equality_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"substr\s*\(\s*\(\s*((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)\s*\)\s*,\s*(\d+)\s*,\s*\d+\s*\)\s*=\s*(?:'([^'])'|char\s*\(\s*(\d+)\s*\))",
        )
        .expect("equality regex must compile")
    })
}

/// 编译并缓存 Length 盲注探针正则。
///
/// 形态：`length((<read_target>))<cmp><thr>`
/// 捕获组：1=read_target、2=comparator、3=threshold。
/// comparator 当前被忽略（聚合按 `>` 语义处理，fixture/合成测试均用 `>`）。
pub(super) fn length_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"length\s*\(\s*\(\s*((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)\s*\)\s*\)\s*(>=?|<=?|=)\s*(\d+)",
        )
        .expect("length regex must compile")
    })
}

// 编译期占位：本模块抽取函数只吃 `&str`/`Option<u64>`，不直接依赖
// `LogEntry`（entry 字段访问在 [`super::aggregate`] 完成）。doc 引用靠
// rustdoc 跨模块解析，无需 use。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogEntry;

    fn mk_entry(
        line_no: usize,
        ip: &str,
        decoded_query: Option<&str>,
        size: Option<u64>,
    ) -> LogEntry {
        LogEntry {
            line_no,
            ip: ip.to_string(),
            timestamp: "17/Nov/2023:03:45:42 +0000".to_string(),
            method: "GET".to_string(),
            path: "/".to_string(),
            query: decoded_query.map(|s| s.to_string()),
            status: 200,
            size,
            user_agent: "curl".to_string(),
            raw: String::new(),
            decoded_path: String::new(),
            decoded_query: decoded_query.map(|s| s.to_string()),
            decoded_ua: String::new(),
        }
    }

    #[test]
    fn regex_skips_empty_entry() {
        use super::super::aggregate::BlindAggregator;
        let mut e = mk_entry(1, "1.1.1.1", Some("ascii(substr((database()),1,1))>79"), Some(875));
        e.method.clear();
        e.path.clear();
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert!(agg.probes().is_empty());
    }

    #[test]
    fn looks_like_blind_probe_ascii_binary() {
        assert!(looks_like_blind_probe("ascii(substr((database()),1,1))>100"));
    }

    #[test]
    fn looks_like_blind_probe_equality() {
        assert!(looks_like_blind_probe("substr((database()),1,1)='a'"));
    }

    #[test]
    fn looks_like_blind_probe_length() {
        // length 正则要求 `length((<rt>))<cmp><thr>` 双括号形态，与
        // extract_blind_probe 共用同一份 length_regex；单括号 `length(database())>5`
        // 不匹配（read_target 外必须再包一层括号，见 length_regex 注释）。
        assert!(looks_like_blind_probe("length((database()))>5"));
    }

    #[test]
    fn looks_like_blind_probe_negative_normal_sql() {
        assert!(!looks_like_blind_probe("SELECT * FROM users"));
    }

    #[test]
    fn regex_skips_entry_without_size() {
        use super::super::aggregate::BlindAggregator;
        let e = mk_entry(1, "1.1.1.1", Some("ascii(substr((database()),1,1))>79"), None);
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert!(agg.probes().is_empty());
    }

    #[test]
    fn regex_skips_non_blind_entry() {
        use super::super::aggregate::BlindAggregator;
        let e = mk_entry(1, "1.1.1.1", Some("id=1 union select 1,2,3"), Some(100));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert!(agg.probes().is_empty());
    }

    #[test]
    fn regex_falls_back_to_decoded_path() {
        use super::super::aggregate::BlindAggregator;
        let mut e = mk_entry(1, "1.1.1.1", None, Some(875));
        e.decoded_path = "ascii(substr((database()),1,1))>79".to_string();
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes().len(), 1);
        assert_eq!(agg.probes()[0].read_target, "database()");
        assert_eq!(agg.probes()[0].char_position, 1);
        assert_eq!(agg.probes()[0].threshold, 79);
        assert_eq!(agg.probes()[0].probe_kind, ProbeKind::AsciiBinary);
        assert_eq!(agg.probes()[0].equality_char, None);
    }

    #[test]
    fn regex_matches_uppercase_ascii_substr() {
        use super::super::aggregate::BlindAggregator;
        let e = mk_entry(1, "1.1.1.1", Some("ASCII(SUBSTR((DATABASE()),1,1))>79"), Some(875));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes().len(), 1);
        assert_eq!(agg.probes()[0].read_target, "database()");
    }

    #[test]
    fn regex_supports_comparators() {
        use super::super::aggregate::BlindAggregator;
        for (cmp, _thr) in [(">=", "79"), ("<", "80"), ("=", "112"), ("<=", "90")] {
            let q = format!("ascii(substr((database()),1,1)){}{}", cmp, _thr);
            let e = mk_entry(1, "1.1.1.1", Some(&q), Some(875));
            let agg = BlindAggregator::collect_from_entries(&[e]);
            assert_eq!(agg.probes().len(), 1, "cmp={} must match", cmp);
        }
    }
}
