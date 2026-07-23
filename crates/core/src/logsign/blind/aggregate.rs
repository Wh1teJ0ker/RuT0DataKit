//! 盲注探针聚合算法层（v0.5.0 T12-1 拆分自 blind_aggregator.rs）。
//!
//! 负责「一批 [`BlindProbe`] → [`AggregatedResult`]」的聚类还原：按
//! `(read_target, source_ip, ProbeKind)` 三元组分组，对每个位置独立判真假簇，
//! 还原字符串/数值。核心入口 [`BlindAggregator::aggregate`]。
//!
//! 与 [`super::probe`] / [`super::reconstruct`] 的边界：
//! - [`super::probe`] 负责探针抽取 + 数据结构定义；
//! - 本模块做位置聚类、true_size 众数判定、decoded_string 拼接；
//! - [`super::reconstruct`] 做数据库结构还原。

use std::collections::HashMap;

use crate::log::LogEntry;

use super::probe::{
    extract_blind_probe_with_line, AggregatedResult, BlindProbe, PositionDetail, ProbeKind,
};

/// 盲注探针聚合器。
///
/// 用 [`BlindAggregator::collect_from_entries`] 从日志条目抽取探针，
/// 再用 [`BlindAggregator::aggregate`] 还原字符串/数值。
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

    /// 从日志条目列表抽取所有盲注探针（三类正则并行）。
    ///
    /// - 跳过 `method`/`path` 为空的 408 超时行。
    /// - 对 `decoded_query`（None 时 fallback `decoded_path`）整串先
    ///   `.to_lowercase()` 再跑正则，兼容 `ASCII/SUBSTR` 大写形态。
    /// - 三类正则各跑一次：ascii_binary / equality / length。同一条 entry
    ///   可能同时命中多类（少见），全部 push。
    /// - `size` 为 `None` 的 entry 跳过（无 body size 无法判真假）。
    ///
    /// v0.2.4（T5-9）重构：本方法不再内联正则，改对每条 entry 调
    /// [`extract_blind_probe_with_line`]，与 [`crate::tools::sql_parse`] 共享
    /// 同一份「SQL 文本 → BlindProbe」逻辑，保持 v0.2.4 log_scan 路径完全兼容。
    pub fn collect_from_entries(entries: &[LogEntry]) -> Self {
        let mut probes = Vec::new();
        for entry in entries {
            if entry.method.is_empty() || entry.path.is_empty() {
                continue;
            }
            // 优先 decoded_query，None 时用 decoded_path。
            let text = match entry.decoded_query.as_deref() {
                Some(q) => q,
                None => entry.decoded_path.as_str(),
            };
            let mut p = extract_blind_probe_with_line(
                text,
                entry.size,
                Some(entry.ip.as_str()),
                entry.line_no,
            );
            probes.append(&mut p);
        }
        Self { probes }
    }

    /// 从已构造的 [`BlindProbe`] 列表构建聚合器（v0.2.4 T5-9 新增）。
    ///
    /// 供 [`crate::tools::sql_parse`] 直接喂入由 [`super::probe::extract_blind_probe`] 抽取
    /// 的探针，绕开 `LogEntry` 耦合。`log_scan` 路径仍走
    /// [`collect_from_entries`](Self::collect_from_entries)，二者最终都进入
    /// 同一 [`aggregate`](Self::aggregate) 算法。
    pub fn collect_from_probes(probes: Vec<BlindProbe>) -> Self {
        Self { probes }
    }

    /// 聚合所有探针，按 `(read_target, source_ip, ProbeKind)` 分组还原。
    ///
    /// 返回列表每个元素对应一个分组，分组内 `position_details` 按 position
    /// 升序。详见模块级文档「聚合算法」。
    pub fn aggregate(&self) -> Vec<AggregatedResult> {
        // 按 (read_target, source_ip, probe_kind) 三元组分组。
        let mut groups: HashMap<(String, String, ProbeKind), Vec<&BlindProbe>> = HashMap::new();
        for p in &self.probes {
            groups
                .entry((p.read_target.clone(), p.source_ip.clone(), p.probe_kind))
                .or_default()
                .push(p);
        }

        let mut results: Vec<AggregatedResult> = Vec::with_capacity(groups.len());
        for ((read_target, _source_ip, kind), group_probes) in groups {
            let (position_details, decoded_string, resolved_chars, unresolved_chars, beyond_end_positions) =
                match kind {
                    ProbeKind::AsciiBinary => aggregate_ascii_binary_group(&group_probes),
                    ProbeKind::Equality => aggregate_equality_group(&group_probes),
                    ProbeKind::Length => aggregate_length_group(&group_probes),
                };

            // source_ips 去重 + 排序，便于前端展示。
            let source_ips: Vec<String> = group_probes
                .iter()
                .map(|p| p.source_ip.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let probe_count = group_probes.len() as u32;

            // separator_char：从 read_target 解析首个 `0x([0-9a-fA-F]{2})` 字面量。
            let separator_char = super::reconstruct::parse_separator_char(&read_target);

            // kind 字符串：与 ProbeKind serde rename 一致。
            let kind_str = match kind {
                ProbeKind::AsciiBinary => "ascii_binary",
                ProbeKind::Equality => "equality",
                ProbeKind::Length => "length",
            };

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
                kind: Some(kind_str.to_string()),
            });
        }

        // 按 (read_target, source_ip, kind) 升序输出，便于测试稳定。
        results.sort_by(|a, b| {
            a.read_target
                .cmp(&b.read_target)
                .then_with(|| a.source_ips.first().cmp(&b.source_ips.first()))
                .then_with(|| a.kind.cmp(&b.kind))
        });
        results
    }
}

impl Default for BlindAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// AsciiBinary 分组聚合：二分序列还原字符串。
///
/// 返回 (position_details, decoded_string, resolved_chars, unresolved_chars,
/// beyond_end_positions)。
fn aggregate_ascii_binary_group(group_probes: &[&BlindProbe]) -> (Vec<PositionDetail>, String, u32, u32, u32) {
    let group_true_size = mode_per_position_true_size(group_probes);

    // 按 char_position 子分组。
    let mut by_pos: HashMap<u32, Vec<&BlindProbe>> = HashMap::new();
    for p in group_probes {
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

    let (decoded_string, resolved_chars, unresolved_chars, beyond_end_positions) =
        build_decoded_string(&position_details);
    (position_details, decoded_string, resolved_chars, unresolved_chars, beyond_end_positions)
}

/// Equality 分组聚合：等值探针直接得字符。
fn aggregate_equality_group(group_probes: &[&BlindProbe]) -> (Vec<PositionDetail>, String, u32, u32, u32) {
    let group_true_size = mode_per_position_true_size(group_probes);

    let mut by_pos: HashMap<u32, Vec<&BlindProbe>> = HashMap::new();
    for p in group_probes {
        by_pos.entry(p.char_position).or_default().push(p);
    }
    let mut positions: Vec<u32> = by_pos.keys().copied().collect();
    positions.sort();

    let mut position_details: Vec<PositionDetail> = Vec::with_capacity(positions.len());
    for pos in &positions {
        let probes = by_pos.get(pos).unwrap();
        let detail = aggregate_equality_position(*pos, probes, group_true_size);
        position_details.push(detail);
    }

    let (decoded_string, resolved_chars, unresolved_chars, beyond_end_positions) =
        build_decoded_string(&position_details);
    (position_details, decoded_string, resolved_chars, unresolved_chars, beyond_end_positions)
}

/// Length 分组聚合：按 threshold 二分还原长度数值。
fn aggregate_length_group(group_probes: &[&BlindProbe]) -> (Vec<PositionDetail>, String, u32, u32, u32) {
    let group_true_size = mode_per_position_true_size(group_probes);
    let probe_count = group_probes.len() as u32;

    let bodies: Vec<u64> = group_probes.iter().map(|p| p.body_size).collect();
    let pos_min = *bodies.iter().min().unwrap_or(&0);
    let pos_max = *bodies.iter().max().unwrap_or(&0);

    // 全同 body：单簇。== group_true_size → 全 true（length 大于所有 thr，
    // 越上界未定）；!= → 全 false（length 小于等于所有 thr，仅得下界 0）。
    if pos_min == pos_max {
        let (status, ascii_val) = if pos_min == group_true_size {
            ("unresolved_all_true".to_string(), None)
        } else {
            // 全 false：length <= min(thr)；下界为 0（无法精确还原）。
            ("insufficient_probes".to_string(), None)
        };
        let detail = PositionDetail {
            position: 0,
            decoded_char: None,
            ascii_val,
            true_size: group_true_size,
            probe_count,
            status,
        };
        let decoded_string = "?".to_string();
        return (
            vec![detail],
            decoded_string,
            0,
            1,
            0,
        );
    }

    // 混合：true 探针 = body == group_true_size（length > thr）；
    // false 探针 = body != group_true_size（length <= thr）。
    let mut true_thresholds: Vec<u32> = Vec::new();
    let mut false_thresholds: Vec<u32> = Vec::new();
    for p in group_probes {
        if p.body_size == group_true_size {
            true_thresholds.push(p.threshold);
        } else {
            false_thresholds.push(p.threshold);
        }
    }

    if false_thresholds.is_empty() {
        let detail = PositionDetail {
            position: 0,
            decoded_char: None,
            ascii_val: None,
            true_size: group_true_size,
            probe_count,
            status: "unresolved_all_true".to_string(),
        };
        return (vec![detail], "?".to_string(), 0, 1, 0);
    }
    if true_thresholds.is_empty() {
        // 全 false 但 body 不全同 → 不可靠。
        let detail = PositionDetail {
            position: 0,
            decoded_char: None,
            ascii_val: None,
            true_size: group_true_size,
            probe_count,
            status: "insufficient_probes".to_string(),
        };
        return (vec![detail], "?".to_string(), 0, 1, 0);
    }

    let max_true = *true_thresholds.iter().max().unwrap();
    let min_false = *false_thresholds.iter().min().unwrap();
    let length_val = min_false;
    let self_consistent = max_true + 1 == min_false;
    let (status, decoded_char) = if self_consistent {
        ("length_resolved".to_string(), None)
    } else {
        ("insufficient_probes".to_string(), None)
    };
    let ascii_val = if self_consistent { Some(length_val) } else { None };
    let detail = PositionDetail {
        position: 0,
        decoded_char,
        ascii_val,
        true_size: group_true_size,
        probe_count,
        status,
    };
    let decoded_string = if self_consistent {
        length_val.to_string()
    } else {
        "?".to_string()
    };
    let (resolved_chars, unresolved_chars) = if self_consistent {
        (1, 0)
    } else {
        (0, 1)
    };
    (vec![detail], decoded_string, resolved_chars, unresolved_chars, 0)
}

/// 从 position_details 构建 decoded_string：升序，resolved/equality_resolved
/// 追加字符，length_resolved 不应出现在多位置场景（length 只单位置），
/// unresolved/insufficient 追加 `'?'`，首个 beyond_end 截断。
fn build_decoded_string(position_details: &[PositionDetail]) -> (String, u32, u32, u32) {
    let mut decoded_string = String::new();
    let mut resolved_chars = 0u32;
    let mut unresolved_chars = 0u32;
    let mut beyond_end_positions = 0u32;
    for d in position_details {
        match d.status.as_str() {
            "beyond_end" => {
                beyond_end_positions += 1;
                break;
            }
            "resolved" | "equality_resolved" => {
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
    (decoded_string, resolved_chars, unresolved_chars, beyond_end_positions)
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
/// **Equality/Length 限制**：本函数沿用「最低 thr 侧 = true」启发，对
/// AsciiBinary `>` 形态正确；Equality 多探针混合场景下，threshold 已被填
/// 为字符 ascii 值，最低 thr 侧未必是 true 簇（equality 无 thr 序语义）。
/// 单探针 equality 退化到 `min(body_size)` = 该探针 body → 视为 true →
/// `equality_resolved`，与 HANDOFF 单探针测试一致。多探针 equality 混合
/// 场景 fixture 无样本，是已知简化。
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
        let min_thr = pos_probes.iter().map(|p| p.threshold).min().expect("non-empty");
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

/// 聚合单个 AsciiBinary 位置：依据 group_true_size 判定每个探针真假，还原 ascii 值。
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
            return PositionDetail {
                position,
                decoded_char: None,
                ascii_val: None,
                true_size: group_true_size,
                probe_count,
                status: "unresolved_all_true".to_string(),
            };
        } else {
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

/// 聚合单个 Equality 位置：依据 group_true_size 判定探针真假，取 true 簇字符。
fn aggregate_equality_position(
    position: u32,
    probes: &[&BlindProbe],
    group_true_size: u64,
) -> PositionDetail {
    let probe_count = probes.len() as u32;
    let bodies: Vec<u64> = probes.iter().map(|p| p.body_size).collect();
    let pos_min = *bodies.iter().min().unwrap_or(&0);
    let pos_max = *bodies.iter().max().unwrap_or(&0);

    // 单簇：全同 body → 视为全 true（HANDOFF：单探针/单簇直接得字符）。
    if pos_min == pos_max {
        if pos_min == group_true_size {
            // 取首个探针的 equality_char（单簇内字符应一致；不一致则 insufficient）。
            let chars: Vec<Option<char>> = probes.iter().map(|p| p.equality_char).collect();
            let consistent = chars.iter().all(|c| *c == chars[0]);
            let (decoded_char, status) = if consistent {
                (chars[0], "equality_resolved".to_string())
            } else {
                (None, "insufficient_probes".to_string())
            };
            return PositionDetail {
                position,
                decoded_char,
                ascii_val: None,
                true_size: group_true_size,
                probe_count,
                status,
            };
        } else {
            // 全同 body 且 != group_true_size → beyond_end（位置越界）。
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

    // 混合：true 探针 = body == group_true_size；取 true 簇 equality_char。
    let mut true_chars: Vec<Option<char>> = Vec::new();
    for p in probes {
        if p.body_size == group_true_size {
            true_chars.push(p.equality_char);
        }
    }
    if true_chars.is_empty() {
        return PositionDetail {
            position,
            decoded_char: None,
            ascii_val: None,
            true_size: group_true_size,
            probe_count,
            status: "beyond_end".to_string(),
        };
    }
    let consistent = true_chars.iter().all(|c| *c == true_chars[0]);
    let (decoded_char, status) = if consistent {
        (true_chars[0], "equality_resolved".to_string())
    } else {
        (None, "insufficient_probes".to_string())
    };
    PositionDetail {
        position,
        decoded_char,
        ascii_val: None,
        true_size: group_true_size,
        probe_count,
        status,
    }
}

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

    // ---- v0.2.3 双簇自动判定 / 多层嵌套正则 / 0xNN 分隔符 ----

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
            let q = format!("ascii(substr((database()),1,1))>{}", thr);
            entries.push(mk_entry(line, "1.1.1.1", Some(&q), Some(*body)));
            line += 1;
        }
        for thr in [79u32, 103, 115] {
            let q = format!("ascii(substr((database()),8,1))>{}", thr);
            entries.push(mk_entry(line, "1.1.1.1", Some(&q), Some(850)));
            line += 1;
        }

        let agg = BlindAggregator::collect_from_entries(&entries);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        let r = &results[0];
        let pos1_detail = r
            .position_details
            .iter()
            .find(|d| d.position == 1)
            .expect("pos 1 detail");
        assert_eq!(pos1_detail.status, "resolved");
        assert_eq!(pos1_detail.ascii_val, Some(112));
        assert_eq!(pos1_detail.decoded_char, Some('p'));
        assert_eq!(pos1_detail.true_size, 900, "auto-detect larger body");
        assert!(r.decoded_string.starts_with('p'));
    }

    #[test]
    fn regex_captures_double_nested_parens() {
        let payload = "1' or ascii(substr((select group_concat(table_name) from \
         information_schema.tables where table_schema=database()),1,1))>79#";
        let e = mk_entry(1, "1.1.1.1", Some(payload), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes().len(), 1, "double-nested read_target must match");
        let rt = &agg.probes()[0].read_target;
        assert!(rt.contains("group_concat(table_name)"));
        assert!(rt.contains("information_schema.tables"));
        assert!(rt.contains("where table_schema=database()"));
        assert_eq!(agg.probes()[0].char_position, 1);
        assert_eq!(agg.probes()[0].threshold, 79);
    }

    #[test]
    fn separator_char_extracted_from_hex_literal() {
        let payload_7e = "1' or ascii(substr((group_concat(id,0x7e,username,0x7e,idcard) from person_data),1,1))>79#";
        let e = mk_entry(1, "1.1.1.1", Some(payload_7e), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].separator_char, Some('~'));

        let payload_41 = "1' or ascii(substr((group_concat(id,0x41,username) from person_data),1,1))>79#";
        let e = mk_entry(2, "1.1.1.1", Some(payload_41), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].separator_char, Some('A'));

        let payload_none = "1' or ascii(substr((database()),1,1))>79#";
        let e = mk_entry(3, "1.1.1.1", Some(payload_none), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].separator_char, None);
    }

    #[test]
    fn mode_body_size_picks_smaller_on_tie() {
        let mk_probe = |pos: u32, thr: u32, body: u64| BlindProbe {
            read_target: "x".to_string(),
            char_position: pos,
            threshold: thr,
            body_size: body,
            source_ip: "1.1.1.1".to_string(),
            line_no: thr as usize,
            probe_kind: ProbeKind::AsciiBinary,
            equality_char: None,
        };
        let p1 = mk_probe(1, 1, 875);
        let p2 = mk_probe(1, 2, 862);
        let probes: Vec<&BlindProbe> = vec![&p1, &p2];
        assert_eq!(mode_per_position_true_size(&probes), 875);

        let p3 = mk_probe(1, 1, 900);
        let p4 = mk_probe(1, 99, 850);
        let probes2: Vec<&BlindProbe> = vec![&p3, &p4];
        assert_eq!(mode_per_position_true_size(&probes2), 900);

        let p_a = mk_probe(1, 1, 862);
        let p_b = mk_probe(1, 99, 875);
        let p_c = mk_probe(2, 1, 862);
        let p_d = mk_probe(2, 99, 875);
        let probes3: Vec<&BlindProbe> = vec![&p_a, &p_b, &p_c, &p_d];
        assert_eq!(mode_per_position_true_size(&probes3), 862);

        let p_a = mk_probe(1, 1, 850);
        let p_b = mk_probe(1, 2, 850);
        let p_c = mk_probe(2, 1, 900);
        let p_d = mk_probe(2, 2, 900);
        let no_mixed: Vec<&BlindProbe> = vec![&p_a, &p_b, &p_c, &p_d];
        assert_eq!(mode_per_position_true_size(&no_mixed), 850);
    }

    #[test]
    fn parse_separator_char_rejects_non_ascii() {
        assert_eq!(super::super::reconstruct::parse_separator_char("group_concat(id,0x7e,x)"), Some('~'));
        assert_eq!(super::super::reconstruct::parse_separator_char("no literal here"), None);
        assert_eq!(super::super::reconstruct::parse_separator_char("group_concat(id,0xff,x)"), None);
    }

    // ---- v0.2.3 equality / length 探针 ----

    #[test]
    fn equality_probe_resolves_single_char() {
        // substr((database()),1,1)='p'，body==true_size（单簇 → 全 true）。
        let e = mk_entry(1, "1.1.1.1", Some("substr((database()),1,1)='p'"), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes().len(), 1);
        assert_eq!(agg.probes()[0].probe_kind, ProbeKind::Equality);
        assert_eq!(agg.probes()[0].equality_char, Some('p'));
        assert_eq!(agg.probes()[0].char_position, 1);
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.kind.as_deref(), Some("equality"));
        assert_eq!(r.decoded_string, "p");
        let pos1 = r.position_details.iter().find(|d| d.position == 1).expect("pos1");
        assert_eq!(pos1.status, "equality_resolved");
        assert_eq!(pos1.decoded_char, Some('p'));
    }

    #[test]
    fn equality_probe_char_form() {
        // substr((database()),1,1)=char(112) → 'p'。
        let e = mk_entry(1, "1.1.1.1", Some("substr((database()),1,1)=char(112)"), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes().len(), 1);
        assert_eq!(agg.probes()[0].probe_kind, ProbeKind::Equality);
        assert_eq!(agg.probes()[0].equality_char, Some('p'));
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.decoded_string, "p");
    }

    #[test]
    fn length_probe_resolves_numeric() {
        // length((database()))>N，length=6。true body=862，false body=875。
        // thr=5→862(true); thr=10/7/6/8→875(false)。min(false)=6=max(true)+1。
        let cases = [(5u32, 862u64), (10, 875), (7, 875), (6, 875), (8, 875)];
        let mut entries: Vec<LogEntry> = Vec::new();
        for (i, (thr, body)) in cases.iter().enumerate() {
            let q = format!("length((database()))>{}", thr);
            entries.push(mk_entry(i + 1, "1.1.1.1", Some(&q), Some(*body)));
        }
        let agg = BlindAggregator::collect_from_entries(&entries);
        assert_eq!(agg.probes().len(), 5);
        assert!(agg.probes().iter().all(|p| p.probe_kind == ProbeKind::Length));
        assert!(agg.probes().iter().all(|p| p.char_position == 0));
        let results = agg.aggregate();
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.kind.as_deref(), Some("length"));
        assert_eq!(r.decoded_string, "6");
        assert_eq!(r.position_details.len(), 1);
        let d = &r.position_details[0];
        assert_eq!(d.position, 0);
        assert_eq!(d.status, "length_resolved");
        assert_eq!(d.ascii_val, Some(6));
        assert_eq!(d.decoded_char, None);
    }

    #[test]
    fn mixed_kinds_no_cross_contamination() {
        // 同 read_target 同时有 ascii_binary 与 length 探针 → 两个独立 AggregatedResult。
        let mut entries: Vec<LogEntry> = Vec::new();
        // ascii_binary: pos 1, ascii=112='p'。
        let ascii_cases = [(79u32, 862u64), (103, 862), (111, 862), (112, 875)];
        for (i, (thr, body)) in ascii_cases.iter().enumerate() {
            let q = format!("ascii(substr((database()),1,1))>{}", thr);
            entries.push(mk_entry(i + 1, "1.1.1.1", Some(&q), Some(*body)));
        }
        // length: length=6。
        let len_cases = [(5u32, 862u64), (6, 875), (8, 875)];
        for (i, (thr, body)) in len_cases.iter().enumerate() {
            let q = format!("length((database()))>{}", thr);
            entries.push(mk_entry(100 + i, "1.1.1.1", Some(&q), Some(*body)));
        }
        let agg = BlindAggregator::collect_from_entries(&entries);
        let results = agg.aggregate();
        // 两个分组（同 read_target=database()，同 source_ip，但 probe_kind 不同）。
        assert_eq!(results.len(), 2, "expect 2 groups (ascii_binary + length)");
        let ascii_r = results
            .iter()
            .find(|r| r.kind.as_deref() == Some("ascii_binary"))
            .expect("ascii_binary group");
        let len_r = results
            .iter()
            .find(|r| r.kind.as_deref() == Some("length"))
            .expect("length group");
        assert!(
            ascii_r.decoded_string.starts_with('p'),
            "ascii group decodes 'p', got {}",
            ascii_r.decoded_string
        );
        assert_eq!(len_r.decoded_string, "6");
    }
}
