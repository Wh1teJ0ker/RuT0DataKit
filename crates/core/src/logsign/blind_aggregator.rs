//! 布尔盲注二分序列聚合还原模块（v0.2.2 T2-12）。
//!
//! 与 [`super::payload_parser`] 互补：payload_parser 只解析「单条 payload 的
//! 语义类别」（盲注/UNION/报错…），本模块在签名命中之后，对一批同源
//! （同 read_target + 同 source_ip）的二分探针按 `char_position` 聚类，
//! 还原出被盲注读取的完整字符串。
//!
//! ## 输入
//!
//! - [`BlindAggregator::collect_from_entries`] 吃 `&[LogEntry]`，对每条 entry
//!   的 `decoded_query`（如 None 则 `decoded_path`）整串跑自带正则，抽取出
//!   `ascii(substr((<read_target>),<pos>,1))<cmp><thr>` 形态的探针。
//! - `body_size` 取 `LogEntry.size`（`Option<u64>`，`None` 跳过该 entry）。
//! - `source_ip` = `LogEntry.ip.clone()`，用于区分不同注入源。
//!
//! ## 真假方向（fixture 验证后写死）
//!
//! 对 `tests/fixtures/samples/log/access.log` line 19-65（`database()` 第 1
//! 字符 'p'=ascii 112）实测：
//!
//! | thr | body_size | 条件 `ascii > thr` |
//! |-----|-----------|---------------------|
//! | 79  | 862       | 112>79 = true       |
//! | 103 | 862       | 112>103 = true      |
//! | 109 | 862       | 112>109 = true      |
//! | 111 | 862       | 112>111 = true      |
//! | 112 | 875       | 112>112 = false     |
//! | 115 | 875       | 112>115 = false     |
//!
//! **条件成立 → body 较小（862）**；条件不成立 → body 较大（875）。
//! 故 **`true_size = min(body_size)`**（条件成立的响应 body 更小）。
//! 单测用合成数据沿用同一语义，不依赖 fixture 频次法。
//!
//! 该方向是 fixture-specific 假设；若未来接入「条件成立→body 更大」的靶机，
//! 需把 `group_true_size` 的取法从 `min` 改为 `max`（见 `aggregate` 注释）。
//!
//! ## 聚合算法
//!
//! 对每个 `(read_target, source_ip)` 分组：
//! 1. `group_true_size = min(所有探针 body_size)`（条件成立响应 body）。
//! 2. 按 `char_position` 子分组。
//! 3. 每个位置：
//!    - 全同 body 且 == group_true_size → `unresolved_all_true`（ascii 大于
//!      所有探针阈值，越界上界未定）。
//!    - 全同 body 且 != group_true_size → `beyond_end`（所有探针 false，
//!      ascii 小于所有阈值，已越出字符串末尾）。
//!    - 混合：true 探针 = `body == group_true_size`（ascii > thr）；
//!      false 探针 = `body != group_true_size`（ascii <= thr）；
//!      `ascii_val = min(false_thresholds)`，自洽性校验
//!      `max(true_thresholds) + 1 == ascii_val`，不符标 `insufficient_probes`。
//! 4. 按 position 升序拼接：resolved 追加字符；unresolved/insufficient 追加
//!    `'?'`；**首个 beyond_end 即停止拼接**（字符串末尾）。
//!
//! ## regex 限制
//!
//! regex crate 无 look-around，`read_target` 内层嵌套括号用
//! `(?:[^()]|\([^()]*\))*` 吃掉单层 `(...)`（如 `database()` /
//! `group_concat(table_name)`）。多层嵌套不支持，是已知简化。

use std::collections::HashMap;

use regex::Regex;

use crate::log::LogEntry;

/// 单条盲注二分探针。
///
/// 由 [`BlindAggregator::collect_from_entries`] 从 `LogEntry` 抽出，对应一条
/// `ascii(substr((<read_target>),<char_position>,1))<cmp><threshold>` 请求。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BlindProbe {
    /// 完整 substr 内层表达式，如 `database()` 或
    /// `select group_concat(table_name) from information_schema.tables where table_schema=database()`。
    pub read_target: String,
    /// 盲注读取第几个字符（1-based）。
    pub char_position: u32,
    /// ascii 比较阈值。
    pub threshold: u32,
    /// HTTP 响应 body 字节数（来自 `LogEntry.size`）。
    pub body_size: u64,
    /// 来源 IP（`LogEntry.ip`），用于区分不同注入源。
    pub source_ip: String,
    /// 原始日志行号（回引用）。
    pub line_no: usize,
}

/// 单个字符位置的聚合详情。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PositionDetail {
    /// 字符位置（1-based）。
    pub position: u32,
    /// 还原出的字符；unresolved/beyond_end/insufficient 为 `None`。
    pub decoded_char: Option<char>,
    /// 还原出的 ascii 值；未还原为 `None`。
    pub ascii_val: Option<u32>,
    /// 该位置条件成立响应的 body size。
    pub true_size: u64,
    /// 该位置的探针数。
    pub probe_count: u32,
    /// 状态：`resolved` / `unresolved_all_true` / `beyond_end` / `insufficient_probes`。
    pub status: String,
}

/// 一个 `(read_target, source_ip)` 分组聚合后的还原结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AggregatedResult {
    /// 聚合的 read_target。
    pub read_target: String,
    /// 还原出的字符串（按 position 升序拼接，首个 beyond_end 截断；
    /// unresolved/insufficient 位置插 `'?'`）。
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
}

/// 盲注二分序列聚合器。
///
/// 用 [`BlindAggregator::collect_from_entries`] 从日志条目抽取探针，
/// 再用 [`BlindAggregator::aggregate`] 还原字符串。
pub struct BlindAggregator {
    probes: Vec<BlindProbe>,
}

impl BlindAggregator {
    /// 空聚合器。
    pub fn new() -> Self {
        Self { probes: Vec::new() }
    }

    /// 已收集的探针切片（只读，供测试与下游检视）。
    pub fn probes(&self) -> &[BlindProbe] {
        &self.probes
    }

    /// 从日志条目列表抽取所有盲注二分探针。
    ///
    /// - 跳过 `method`/`path` 为空的 408 超时行。
    /// - 对 `decoded_query`（None 时 fallback `decoded_path`）整串先
    ///   `.to_lowercase()` 再跑正则，兼容 `ASCII/SUBSTR` 大写形态。
    /// - 同一条 entry 可能命中多处（极少见），全部收集。
    /// - `size` 为 `None` 的 entry 跳过（无 body size 无法判真假）。
    pub fn collect_from_entries(entries: &[LogEntry]) -> Self {
        let re = blind_probe_regex();
        let mut probes = Vec::new();
        for entry in entries {
            if entry.method.is_empty() || entry.path.is_empty() {
                continue;
            }
            let size = match entry.size {
                Some(s) => s,
                None => continue,
            };
            // 优先 decoded_query，None 时用 decoded_path。
            let text = match entry.decoded_query.as_deref() {
                Some(q) => q,
                None => entry.decoded_path.as_str(),
            };
            let lower = text.to_lowercase();
            for caps in re.captures_iter(&lower) {
                let read_target = caps.get(1).map(|m| m.as_str().to_string());
                let char_position = caps
                    .get(2)
                    .and_then(|m| m.as_str().parse::<u32>().ok());
                let threshold = caps
                    .get(4)
                    .and_then(|m| m.as_str().parse::<u32>().ok());
                match (read_target, char_position, threshold) {
                    (Some(rt), Some(pos), Some(thr)) => {
                        // to_lowercase 后 read_target 也是小写；保留小写形态
                        // 作为分组 key（fixture 全小写，不影响）。
                        probes.push(BlindProbe {
                            read_target: rt,
                            char_position: pos,
                            threshold: thr,
                            body_size: size,
                            source_ip: entry.ip.clone(),
                            line_no: entry.line_no,
                        });
                    }
                    _ => continue,
                }
            }
        }
        Self { probes }
    }

    /// 聚合所有探针，按 `(read_target, source_ip)` 分组还原字符串。
    ///
    /// 返回列表每个元素对应一个分组，分组内 `position_details` 按 position
    /// 升序。详见模块级文档「聚合算法」。
    pub fn aggregate(&self) -> Vec<AggregatedResult> {
        // 按 (read_target, source_ip) 分组。
        let mut groups: HashMap<(String, String), Vec<&BlindProbe>> = HashMap::new();
        for p in &self.probes {
            groups
                .entry((p.read_target.clone(), p.source_ip.clone()))
                .or_default()
                .push(p);
        }

        let mut results: Vec<AggregatedResult> = Vec::with_capacity(groups.len());
        for ((read_target, _source_ip), group_probes) in groups {
            // group_true_size = min(body_size)（条件成立响应 body 更小，
            // 见模块级文档 fixture 验证表）。
            let group_true_size = group_probes.iter().map(|p| p.body_size).min().unwrap_or(0);

            // 按 char_position 子分组。
            let mut by_pos: HashMap<u32, Vec<&BlindProbe>> = HashMap::new();
            for p in &group_probes {
                by_pos.entry(p.char_position).or_default().push(p);
            }
            let mut positions: Vec<u32> = by_pos.keys().copied().collect();
            positions.sort();

            let mut position_details: Vec<PositionDetail> = Vec::with_capacity(positions.len());
            for pos in &positions {
                let probes = by_pos.get(pos).unwrap();
                let detail = aggregate_position(*pos, probes, group_true_size);
                position_details.push(detail);
            }

            // 拼接 decoded_string：升序，首个 beyond_end 截断；
            // resolved 追加字符，unresolved/insufficient 追加 '?'。
            let mut decoded_string = String::new();
            let mut resolved_chars = 0u32;
            let mut unresolved_chars = 0u32;
            let mut beyond_end_positions = 0u32;
            for d in &position_details {
                match d.status.as_str() {
                    "beyond_end" => {
                        beyond_end_positions += 1;
                        break;
                    }
                    "resolved" => {
                        if let Some(c) = d.decoded_char {
                            decoded_string.push(c);
                            resolved_chars += 1;
                        } else {
                            decoded_string.push('?');
                            unresolved_chars += 1;
                        }
                    }
                    _ => {
                        // unresolved_all_true / insufficient_probes
                        decoded_string.push('?');
                        unresolved_chars += 1;
                    }
                }
            }

            // source_ips 去重 + 排序，便于前端展示。单源场景下与分组 key
            // 的 source_ip 等价（BTreeSet 必非空，因 group_probes 来自分组
            // key，至少含一个 source_ip）。
            let source_ips: Vec<String> = group_probes
                .iter()
                .map(|p| p.source_ip.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            // probe_count = 该分组探针总数。
            let probe_count = group_probes.len() as u32;

            results.push(AggregatedResult {
                read_target,
                decoded_string,
                resolved_chars,
                unresolved_chars,
                beyond_end_positions,
                probe_count,
                source_ips,
                position_details,
            });
        }

        // 按 (read_target, source_ip) 升序输出，便于测试稳定。source_ips
        // 已是单源分组（BTreeSet 至少含分组 key 的 source_ip），取首项即可。
        results.sort_by(|a, b| {
            a.read_target
                .cmp(&b.read_target)
                .then_with(|| a.source_ips.first().cmp(&b.source_ips.first()))
        });
        results
    }
}

impl Default for BlindAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// 聚合单个位置：依据 group_true_size 判定每个探针真假，还原 ascii 值。
fn aggregate_position(
    position: u32,
    probes: &[&BlindProbe],
    group_true_size: u64,
) -> PositionDetail {
    let probe_count = probes.len() as u32;
    let bodies: Vec<u64> = probes.iter().map(|p| p.body_size).collect();
    let pos_min = *bodies.iter().min().unwrap_or(&0);
    let pos_max = *bodies.iter().max().unwrap_or(&0);

    // 全同 body_size：按是否 == group_true_size 区分 all-true vs all-false。
    if pos_min == pos_max {
        if pos_min == group_true_size {
            // 所有探针条件都成立（ascii > 所有 thr），越上界未定。
            return PositionDetail {
                position,
                decoded_char: None,
                ascii_val: None,
                true_size: group_true_size,
                probe_count,
                status: "unresolved_all_true".to_string(),
            };
        } else {
            // 所有探针条件都不成立（ascii <= 所有 thr），且 body 全同 !=
            // true_size → 已越出字符串末尾（ascii 实际为空/0）。
            return PositionDetail {
                position,
                decoded_char: None,
                ascii_val: None,
                true_size: group_true_size,
                probe_count,
                status: "beyond_end".to_string(),
            };
        }
    }

    // 混合：true 探针 = body == group_true_size（ascii > thr）；
    // false 探针 = body != group_true_size（ascii <= thr）。
    let mut true_thresholds: Vec<u32> = Vec::new();
    let mut false_thresholds: Vec<u32> = Vec::new();
    for p in probes {
        if p.body_size == group_true_size {
            true_thresholds.push(p.threshold);
        } else {
            false_thresholds.push(p.threshold);
        }
    }

    if false_thresholds.is_empty() {
        // 全 true（不应到这里，因为 pos_min != pos_max 已排除全同；
        // 但若 group_true_size 不在 bodies 中也会触发，保守归 unresolved）。
        return PositionDetail {
            position,
            decoded_char: None,
            ascii_val: None,
            true_size: group_true_size,
            probe_count,
            status: "unresolved_all_true".to_string(),
        };
    }
    if true_thresholds.is_empty() {
        // 全 false 但 body 不全同（混入了多个非 true_size 值）→ 不可靠。
        return PositionDetail {
            position,
            decoded_char: None,
            ascii_val: None,
            true_size: group_true_size,
            probe_count,
            status: "insufficient_probes".to_string(),
        };
    }

    let max_true = *true_thresholds.iter().max().unwrap();
    let min_false = *false_thresholds.iter().min().unwrap();
    let ascii_val = min_false;
    let self_consistent = max_true + 1 == min_false;
    // 仅在自洽时返回 decoded_char；insufficient_probes 状态下 ascii_val 不可靠。
    let decoded_char = if self_consistent {
        char::from_u32(ascii_val)
    } else {
        None
    };
    let status = if self_consistent {
        "resolved".to_string()
    } else {
        "insufficient_probes".to_string()
    };
    PositionDetail {
        position,
        decoded_char,
        ascii_val: Some(ascii_val),
        true_size: group_true_size,
        probe_count,
        status,
    }
}

/// 编译并缓存盲注二分探针正则。
///
/// 形态：`ascii(substr((<read_target>),<pos>,1))<cmp><thr>`
/// 捕获组：1=read_target、2=char_position、3=comparator、4=threshold。
/// `read_target` 内层单层 `(...)` 用 `(?:[^()]|\([^()]*\))*` 吃掉。
fn blind_probe_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"ascii\s*\(\s*substr\s*\(\s*\(\s*((?:[^()]|\([^()]*\))*)\s*\)\s*,\s*(\d+)\s*,\s*\d+\s*\)\s*\)\s*(>=?|<=?|=)\s*(\d+)",
        )
        .expect("blind_probe regex must compile")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // 408 超时行（method/path 空）不产探针。
        let mut e = mk_entry(1, "1.1.1.1", Some("ascii(substr((database()),1,1))>79"), Some(875));
        e.method.clear();
        e.path.clear();
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert!(agg.probes.is_empty());
    }

    #[test]
    fn regex_skips_entry_without_size() {
        // size=None 跳过。
        let e = mk_entry(1, "1.1.1.1", Some("ascii(substr((database()),1,1))>79"), None);
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert!(agg.probes.is_empty());
    }

    #[test]
    fn regex_skips_non_blind_entry() {
        let e = mk_entry(1, "1.1.1.1", Some("id=1 union select 1,2,3"), Some(100));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert!(agg.probes.is_empty());
    }

    #[test]
    fn regex_falls_back_to_decoded_path() {
        // decoded_query=None 时 fallback decoded_path。
        let mut e = mk_entry(1, "1.1.1.1", None, Some(875));
        e.decoded_path = "ascii(substr((database()),1,1))>79".to_string();
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes.len(), 1);
        assert_eq!(agg.probes[0].read_target, "database()");
        assert_eq!(agg.probes[0].char_position, 1);
        assert_eq!(agg.probes[0].threshold, 79);
    }

    #[test]
    fn regex_matches_uppercase_ascii_substr() {
        let e = mk_entry(1, "1.1.1.1", Some("ASCII(SUBSTR((DATABASE()),1,1))>79"), Some(875));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes.len(), 1);
        assert_eq!(agg.probes[0].read_target, "database()");
    }

    #[test]
    fn regex_supports_comparators() {
        for (cmp, _thr) in [(">=", "79"), ("<", "80"), ("=", "112"), ("<=", "90")] {
            let q = format!("ascii(substr((database()),1,1)){}{}", cmp, _thr);
            let e = mk_entry(1, "1.1.1.1", Some(&q), Some(875));
            let agg = BlindAggregator::collect_from_entries(&[e]);
            assert_eq!(agg.probes.len(), 1, "cmp={} must match", cmp);
        }
    }
}
