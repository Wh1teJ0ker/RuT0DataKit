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
//! ## 真假方向（每位置独立判定 + 跨位置众数）
//!
//! 盲注二分探针在「条件成立」与「不成立」两种响应下，HTTP body 字节数
//! 不同（靶机通常在条件成立时走不同分支导致响应长度变化）。
//!
//! **不**用「全位置 body_size 众数」判 true_size：fixture 第 4 个 read_target
//! `select group_concat(id,0x7e,username,0x7e,idcard) from person_data` 上
//! false 探针频次（737）可高于 true 探针频次（669），全位置众数会误判到
//! false 簇（875），使第 4 RT 退化为全 `?`。
//!
//! 实际算法：对每个「混合位置」（同 char_position 内 body_size 不全同，
//! 即同时有 true/false 探针），独立判该位置的 true 簇 = **出现在最低
//! threshold 一侧的 body_size**（thr 越低越可能 `ascii > thr` 成立 → true）。
//! 这对 fixture（true body=862=min）每位置取 862，对反向场景
//! （true body=900=max）每位置取 900，方向自动适配，无需写死 min/max。
//! 再跨混合位置对 true 簇标识取众数（并列取较小者，偏向 fixture 方向）。
//!
//! 单簇位置（beyond_end / unresolved_all_true）不参与 true_size 选举，
//! 避免 all-false 位置投票压倒真正的 true 簇（HANDOFF risks 指出的退化场景）。
//! 无混合位置时退化到 `min(body_size)`（v0.2.2 行为，不破坏 fixture）。
//!
//! - fixture（`access.log`，4 个 read_target）：每混合位置 true 簇 = 862，
//!   众数 → 862（与 v0.2.2 `min(body_size)` 等价，向后兼容）。
//! - 反向场景（true body=900 / false body=850）：每混合位置 true 簇 = 900
//!   （低 thr 侧），众数 → 900，无需改算法。
//!
//! 并列频次（两簇等频）取较小者，偏向 fixture 真假方向（true 通常 body 更
//! 小）。无混合位置时退化到 `min(body_size)`（v0.2.2 行为，不破坏 fixture），
//! 仍能区分 all-true / all-false（见聚合算法）。
//!
//! ## 聚合算法
//!
//! 对每个 `(read_target, source_ip)` 分组：
//! 1. `group_true_size = mode_per_position_true_size`（每混合位置判 true 簇，
//!    跨位置取众数，并列取较小者偏向）。
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
//! ## 分隔符高亮（0xNN 字面量）
//!
//! read_target 若含 `0xNN` 十六进制字面量（如 fixture
//! `group_concat(id,0x7e,username,0x7e,idcard)` 的 0x7e=`~`），
//! `aggregate()` 末尾解析首个字面量为 `AggregatedResult.separator_char`
//! （`Option<char>`，无字面量为 `None`，向后兼容）。decoded_string 本身
//! 不变；该字段仅供 GUI（T3-3）高亮分隔符。
//!
//! ## regex 限制
//!
//! regex crate 无 look-around，`read_target` 内层嵌套括号用
//! `(?:[^()]|\((?:[^()]|\([^()]*\))*\))*` 吃掉最多 2 层嵌套（如
//! `database()` / `group_concat(table_name)` / `where
//! table_schema=database()`）。3 层及以上嵌套不支持，是已知简化。

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
    /// read_target 中首个 `0xNN` 十六进制字面量解码出的分隔符（如有），
    /// 供 GUI 高亮展示。例如 `group_concat(id,0x7e,username,0x7e,idcard)`
    /// 的 read_target 含 `0x7e` → `Some('~')`。无字面量为 `None`（向后兼容）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator_char: Option<char>,
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
            // group_true_size = 每混合位置独立判 true 簇，跨位置取众数
            // （并列取较小者偏向 fixture 方向）。
            // 双簇自动判定：每个「混合位置」（同位置内 body_size 不全同，即
            // 既有 true 又有 false 探针）独立判该位置 true 簇 = 出现在最低
            // threshold 一侧的 body_size（thr 越低越可能 ascii>thr 成立）。
            // 这对 fixture（true body=min）每位置取 min，对反向场景（true
            // body=max）每位置取 max，方向自动适配，无需写死 min/max。
            // 再跨混合位置对 true 簇标识取众数。单簇位置（beyond_end /
            // unresolved_all_true）不参与，避免 all-false 位置投票压倒
            // true 簇。无混合位置时退化到 `min(body_size)`（v0.2.2 行为）。
            let group_true_size = mode_per_position_true_size(&group_probes);

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

            // separator_char：从 read_target 解析首个 `0x([0-9a-fA-F]{2})` 字面量，
            // 解码为对应 ASCII 字符（如 0x7e → '~'），供 GUI 高亮。无字面量 None。
            let separator_char = parse_separator_char(&read_target);

            results.push(AggregatedResult {
                read_target,
                decoded_string,
                resolved_chars,
                unresolved_chars,
                beyond_end_positions,
                probe_count,
                source_ips,
                position_details,
                separator_char,
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

/// 每混合位置独立判 true 簇，跨位置对 true 簇标识取众数作为 true_size。
///
/// 算法：
/// 1. 对每个「混合位置」（同 char_position 内 body_size 不全同，即既有 true
///    又有 false 探针，能直接区分真假簇），独立判该位置的 true 簇 = 出现在
///    最低 threshold 一侧的 body_size（thr 越低越可能 `ascii > thr` 成立 →
///    true）。这对 fixture（true body=min）每位置取 min，对反向场景
///    （true body=max）每位置取 max，方向自动适配，无需写死 min/max。
/// 2. 收集所有混合位置的 true 簇标识，取众数（并列取较小者，偏向 fixture
///    真假方向 true 通常 body 更小）作为 `group_true_size`。
/// 3. 单簇位置（beyond_end / unresolved_all_true）不参与 true_size 选举，
///    避免 all-false 位置投票压倒真正的 true 簇（HANDOFF risks 指出的退化
///    场景）。
/// 4. 无混合位置时退化到 `min(body_size)`（v0.2.2 行为，不破坏 fixture）。
///    探针为空时返回 0。
///
/// 为什么不取「全位置 body_size 众数」：fixture 第 4 RT 上 false 探针频次
/// （737）可高于 true（669），全位置众数会误判到 false 簇（875），使第 4
/// RT 退化为全 `?`。每位置独立判 true 簇避开此问题（每位置 true 簇 = 862，
/// 众数 → 862）。
fn mode_per_position_true_size(probes: &[&BlindProbe]) -> u64 {
    // 按 char_position 聚合。
    let mut by_pos: HashMap<u32, Vec<&BlindProbe>> = HashMap::new();
    for p in probes {
        by_pos.entry(p.char_position).or_default().push(p);
    }
    // 对每个混合位置（body_size 不全同），独立判该位置 true 簇 = 出现在
    // 最低 threshold 一侧的 body_size。
    let mut freq: HashMap<u64, u32> = HashMap::new();
    for pos_probes in by_pos.values() {
        let bmin = pos_probes.iter().map(|p| p.body_size).min().unwrap_or(0);
        let bmax = pos_probes.iter().map(|p| p.body_size).max().unwrap_or(0);
        if bmin == bmax {
            // 单簇位置：不参与 true_size 选举。
            continue;
        }
        // 该位置 true 簇 = 出现在最低 threshold 一侧的 body_size。
        // thr 越低越可能 ascii > thr 成立（探针为真）→ 那个 body 即 true 簇。
        let min_thr = pos_probes.iter().map(|p| p.threshold).min().expect("non-empty");
        // 取最低 threshold 对应的 body_size（若有多个同 thr 取众数再任取）。
        let true_body_at_pos = pos_probes
            .iter()
            .filter(|p| p.threshold == min_thr)
            .map(|p| p.body_size)
            .next()
            .expect("non-empty");
        *freq.entry(true_body_at_pos).or_insert(0) += 1;
    }
    if freq.is_empty() {
        // 无混合位置 → 退化到 min(body_size)（v0.2.2 行为）。
        return probes.iter().map(|p| p.body_size).min().unwrap_or(0);
    }
    // 取频次最高者；并列时取 body_size 较小者。
    freq.into_iter()
        .max_by_key(|(body, count)| (*count, std::cmp::Reverse(*body)))
        .map(|(body, _)| body)
        .unwrap_or(0)
}

/// 从 read_target 解析首个 `0x([0-9a-fA-F]{2})` 十六进制字面量分隔符。
///
/// 用于 fixture `group_concat(id,0x7e,username,0x7e,idcard)` 中 0x7e → '~'
/// 的展示层高亮。仅返回字面量解码后的首个 ASCII 字符；无字面量返回 None。
/// 字面量值 > 127（非 ASCII）返回 None。
fn parse_separator_char(read_target: &str) -> Option<char> {
    let re = hex_literal_regex();
    let caps = re.captures(read_target)?;
    let hex = caps.get(1)?.as_str();
    let val = u32::from_str_radix(hex, 16).ok()?;
    if val > 0x7f {
        return None;
    }
    char::from_u32(val)
}

fn hex_literal_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"0x([0-9a-fA-F]{2})").expect("hex_literal regex must compile")
    })
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
/// `read_target` 内层嵌套 `(...)` 用
/// `(?:[^()]|\((?:[^()]|\([^()]*\))*\))*` 吃掉最多 2 层嵌套
/// （覆盖 fixture `where table_schema=database()` 中 database() 包在
/// 最外层 substr 内层、其本身又有空括号的双层场景）。regex crate 无
/// look-around，2 层足够 fixture；更深嵌套是已知简化。
fn blind_probe_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"ascii\s*\(\s*substr\s*\(\s*\(\s*((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)\s*\)\s*,\s*(\d+)\s*,\s*\d+\s*\)\s*\)\s*(>=?|<=?|=)\s*(\d+)",
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

    // ---- v0.2.3 双簇自动判定 / 多层嵌套正则 / 0xNN 分隔符 ----

    /// 双簇自动判定：合成「条件成立→body 更大」场景（与 fixture 相反方向）。
    ///
    /// true body=900（条件成立响应更大），false body=850。pos 1 ascii=112='p'：
    /// - thr 79/103/109/111 → 900 (true, ascii>thr)
    /// - thr 112/115 → 850 (false, ascii<=thr)
    /// 众数判定：900 出现 4 次 > 850 出现 2 次 → group_true_size=900，
    /// 与 v0.2.2 min(body_size)=850 不同，但还原结果仍为 'p'。
    #[test]
    fn true_size_auto_detects_larger_body() {
        let pos1 = vec![
            (79u32, 900u64),
            (103, 900),
            (109, 900),
            (111, 900),
            (112, 850),
            (115, 850),
        ];
        let mut entries: Vec<LogEntry> = Vec::new();
        let mut line = 1usize;
        for (thr, body) in &pos1 {
            let q = format!(
                "ascii(substr((database()),1,1))>{}",
                thr
            );
            entries.push(mk_entry(line, "1.1.1.1", Some(&q), Some(*body)));
            line += 1;
        }
        // pos 8: 全同 850 → beyond_end（== max false，全 false）。
        for thr in [79u32, 103, 115] {
            let q = format!(
                "ascii(substr((database()),8,1))>{}",
                thr
            );
            entries.push(mk_entry(line, "1.1.1.1", Some(&q), Some(850)));
            line += 1;
        }

        let agg = BlindAggregator::collect_from_entries(&entries);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        let r = &results[0];
        // 双簇自动判定：group_true_size == 900（条件成立 body 更大场景）。
        let pos1_detail = r
            .position_details
            .iter()
            .find(|d| d.position == 1)
            .expect("pos 1 detail");
        assert_eq!(pos1_detail.status, "resolved");
        assert_eq!(pos1_detail.ascii_val, Some(112));
        assert_eq!(pos1_detail.decoded_char, Some('p'));
        assert_eq!(pos1_detail.true_size, 900, "auto-detect larger body");
        assert!(
            r.decoded_string.starts_with('p'),
            "decoded_string starts with p, got {}",
            r.decoded_string
        );
    }

    /// 多层嵌套正则：read_target 含 `where table_schema=database()`，2 层
    /// 嵌套括号（外层 substr(...) 内的 (...) 中含 database()），应被完整捕获。
    #[test]
    fn regex_captures_double_nested_parens() {
        let payload = "1' or ascii(substr((select group_concat(table_name) from \
         information_schema.tables where table_schema=database()),1,1))>79#";
        let e = mk_entry(1, "1.1.1.1", Some(payload), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes.len(), 1, "double-nested read_target must match");
        let rt = &agg.probes[0].read_target;
        assert!(
            rt.contains("group_concat(table_name)"),
            "must contain group_concat(table_name), got: {rt}"
        );
        assert!(
            rt.contains("information_schema.tables"),
            "must contain information_schema.tables, got: {rt}"
        );
        assert!(
            rt.contains("where table_schema=database()"),
            "must contain where table_schema=database(), got: {rt}"
        );
        assert_eq!(agg.probes[0].char_position, 1);
        assert_eq!(agg.probes[0].threshold, 79);
    }

    /// 0xNN 字面量分隔符：read_target 含 `0x7e` → separator_char==Some('~')；
    /// 含 `0x41` → Some('A')。
    #[test]
    fn separator_char_extracted_from_hex_literal() {
        // 0x7e → '~'
        let payload_7e = "1' or ascii(substr((group_concat(id,0x7e,username,0x7e,idcard) from person_data),1,1))>79#";
        let e = mk_entry(1, "1.1.1.1", Some(payload_7e), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].separator_char, Some('~'),
            "0x7e must decode to '~', got {:?}",
            results[0].separator_char
        );

        // 0x41 → 'A'
        let payload_41 = "1' or ascii(substr((group_concat(id,0x41,username) from person_data),1,1))>79#";
        let e = mk_entry(2, "1.1.1.1", Some(payload_41), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].separator_char, Some('A'),
            "0x41 must decode to 'A', got {:?}",
            results[0].separator_char
        );

        // 无字面量 → None
        let payload_none = "1' or ascii(substr((database()),1,1))>79#";
        let e = mk_entry(3, "1.1.1.1", Some(payload_none), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].separator_char, None,
            "no 0xNN literal → separator_char None"
        );
    }

    /// mode_per_position_true_size 单测：每混合位置独立判 true 簇（出现在
    /// 最低 threshold 一侧的 body_size），跨位置对 true 簇标识取众数；
    /// 并列频次取较小者；无混合位置退化到 min(body_size)。
    #[test]
    fn mode_body_size_picks_smaller_on_tie() {
        // 混合位置 pos 1：thr=1 body=875、thr=2 body=862。
        // min thr=1 → 该位置 true 簇 = 875。无并列，返回 875。
        let mk_probe = |pos: u32, thr: u32, body: u64| BlindProbe {
            read_target: "x".to_string(),
            char_position: pos,
            threshold: thr,
            body_size: body,
            source_ip: "1.1.1.1".to_string(),
            line_no: thr as usize,
        };
        let p1 = mk_probe(1, 1, 875);
        let p2 = mk_probe(1, 2, 862);
        let probes: Vec<&BlindProbe> = vec![&p1, &p2];
        assert_eq!(mode_per_position_true_size(&probes), 875);

        // 反向场景：pos 1 中 thr=1 body=900、thr=99 body=850。
        // min thr=1 → true 簇 = 900。
        let p3 = mk_probe(1, 1, 900);
        let p4 = mk_probe(1, 99, 850);
        let probes2: Vec<&BlindProbe> = vec![&p3, &p4];
        assert_eq!(mode_per_position_true_size(&probes2), 900);

        // 两个混合位置（pos 1 与 pos 2），各自 true 簇 = 862 → 众数 862。
        let p_a = mk_probe(1, 1, 862);
        let p_b = mk_probe(1, 99, 875);
        let p_c = mk_probe(2, 1, 862);
        let p_d = mk_probe(2, 99, 875);
        let probes3: Vec<&BlindProbe> = vec![&p_a, &p_b, &p_c, &p_d];
        assert_eq!(mode_per_position_true_size(&probes3), 862);

        // 无混合位置（pos 1 全 850，pos 2 全 900）→ 退化到 min=850。
        let p_a = mk_probe(1, 1, 850);
        let p_b = mk_probe(1, 2, 850);
        let p_c = mk_probe(2, 1, 900);
        let p_d = mk_probe(2, 2, 900);
        let no_mixed: Vec<&BlindProbe> = vec![&p_a, &p_b, &p_c, &p_d];
        assert_eq!(
            mode_per_position_true_size(&no_mixed),
            850,
            "no mixed positions → fallback to min(body_size)"
        );
    }

    /// parse_separator_char 单测：0xff（>0x7f 非 ASCII）→ None。
    #[test]
    fn parse_separator_char_rejects_non_ascii() {
        assert_eq!(parse_separator_char("group_concat(id,0x7e,x)"), Some('~'));
        assert_eq!(parse_separator_char("no literal here"), None);
        assert_eq!(
            parse_separator_char("group_concat(id,0xff,x)"),
            None,
            "0xff > 0x7f non-ASCII → None"
        );
    }
}
