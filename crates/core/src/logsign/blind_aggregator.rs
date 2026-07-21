//! 布尔盲注探针聚合还原模块（v0.2.2 T2-12 / v0.2.3 T3-2 / v0.2.4 T4-1）。
//!
//! 与 [`super::payload_parser`] 互补：payload_parser 只解析「单条 payload 的
//! 语义类别」（盲注/UNION/报错…），本模块在签名命中之后，对一批同源
//! （同 read_target + 同 source_ip + 同 [`ProbeKind`]）的盲注探针按位置
//! 聚类，还原出被盲注读取的完整字符串或数值，并在 v0.2.4 增加数据库结构
//! 还原 [`reconstruct_database`]：把 4 类标准 read_target（`database()` /
//! `group_concat(table_name) from information_schema.tables` /
//! `group_concat(column_name) from information_schema.columns where table_name='X'` /
//! `group_concat(col1,0xNN,col2,...) from <table>`）交叉关联成结构化的
//! [`ReconstructedDatabase`]（schema → tables → columns → rows）。
//!
//! ## 支持的探针形态（v0.2.3）
//!
//! - [`ProbeKind::AsciiBinary`]：`ascii(substr((<rt>),<pos>,1))<cmp><thr>`
//!   二分序列，按位置聚类还原字符串（v0.2.2 既有）。
//! - [`ProbeKind::Equality`]：`substr((<rt>),<pos>,1)='c'` 或
//!   `substr((<rt>),<pos>,1)=char(<ascii>)` 等值形态，单探针直接得字符
//!   （v0.2.3 新增）。
//! - [`ProbeKind::Length`]：`length((<rt>))<cmp><thr>` 长度盲注，按 threshold
//!   二分还原长度数值，`decoded_string` 输出十进制数字串（如 `"28"`）
//!   （v0.2.3 新增）。
//!
//! 时间盲注聚合 out_of_scope（`LogEntry` 无 `response_time_ms` 字段，
//! fixture 无样本）。
//!
//! ## 输入
//!
//! - [`BlindAggregator::collect_from_entries`] 吃 `&[LogEntry]`，对每条 entry
//!   的 `decoded_query`（如 None 则 `decoded_path`）整串跑三套自带正则
//!   （ascii_binary / equality / length），抽取出三类探针。
//! - `body_size` 取 `LogEntry.size`（`Option<u64>`，`None` 跳过该 entry）。
//! - `source_ip` = `LogEntry.ip.clone()`，用于区分不同注入源。
//! - 同一条 entry 可能同时命中多类正则（少见），全部收集。
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
//! 按 `(read_target, source_ip, ProbeKind)` 三元组分组（避免 length 与
//! ascii_binary 同 read_target 混）。
//!
//! ### AsciiBinary 分组
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
//! ### Equality 分组
//! - 同 (read_target, source_ip, position) 子组：
//!   - 单簇（全同 body，视为全 true）→ `equality_resolved`，
//!     `decoded_char = equality_char`。
//!   - 多簇 → 取 true 簇（沿用 `mode_per_position_true_size`）的探针
//!     `equality_char`；true 簇内多探针字符一致 → `equality_resolved`；
//!     不一致 → `insufficient_probes`。全 false 簇 → `beyond_end`
//!     （位置越界，攻击者探了但条件不成立）。
//! - decoded_string 按 position 升序拼接 resolved 字符；beyond_end 截断；
//!   unresolved/insufficient 插 `'?'`。
//!
//! ### Length 分组
//! - 所有探针 position=0（无 position 维度，全是「读长度」）。
//! - `group_true_size` 沿用 `mode_per_position_true_size`（单一位置内多探针
//!   → 混合则取低 thr 侧 body，单簇退化 min）。
//! - true 探针（body==group_true_size，`length > thr`）→ length > thr；
//!   false 探针 → length <= thr。
//! - `length_val = min(false_thresholds)`，自洽性校验
//!   `max(true_thresholds) + 1 == length_val`，不符标 `insufficient_probes`。
//! - `decoded_string = length_val.to_string()`（如 `"28"`）。
//! - `position_details` 只有一项：position=0, decoded_char=None,
//!   ascii_val=Some(length_val), status=`length_resolved`。
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
//! equality 正则捕获组 3（`'c'` 单字符）或 4（`char(N)` ascii 数字）；
//! length 正则忽略 comparator（fixture/合成测试均用 `>` 语义）。

use std::collections::HashMap;

use regex::Regex;

use crate::log::LogEntry;

/// 从一条 SQL 文本提取所有盲注探针（v0.2.4 T5-9 重构）。
///
/// 把 v0.2.2/v0.2.3 写死在 [`BlindAggregator::collect_from_entries`] 内的
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
/// 由 [`BlindAggregator::collect_from_entries`] 从 `LogEntry` 抽出。三类形态：
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
    /// 供 [`crate::tools::sql_parse`] 直接喂入由 [`extract_blind_probe`] 抽取
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
            let separator_char = parse_separator_char(&read_target);

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

// ===== v0.2.4 数据库结构还原（reconstruct_database） =====

/// 重建出的数据库视图（v0.2.4 T4-1）。
///
/// 由 [`BlindAggregator::reconstruct_database`] 对一批 [`AggregatedResult`]
/// 做 4 类 read_target 交叉关联得到：schema（`database()` 还原值）+ 一组
/// [`ReconstructedTable`]（每张表含全量 columns + 已还原行）。未匹配 4 类
/// 模式的 AggregatedResult 原样回填到 [`ReconstructedDatabase::unmatched_results`]，
/// 供前端兜底展示。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReconstructedDatabase {
    /// `database()` 还原值（数据库名）；无 database() 探针时为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// 重建出的表列表。
    pub tables: Vec<ReconstructedTable>,
    /// 未匹配 4 类标准模式的 AggregatedResult（原样回传前端兜底）。
    pub unmatched_results: Vec<AggregatedResult>,
}

/// 重建出的一张表（v0.2.4 T4-1）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReconstructedTable {
    /// 表名，来自 `information_schema.tables` 或行数据查询的 `from <table>`。
    pub name: String,
    /// 全量列名列表，来自 `information_schema.columns` 还原（无列清单时用
    /// `row_data_columns` 兜底）。
    pub columns: Vec<String>,
    /// 重建出的行；每行 `cells` 与 `columns` 等长，未 fetch 列为 `None`。
    pub rows: Vec<ReconstructedRow>,
    /// 行数据查询（`group_concat(col1,0xNN,col2,...)`）实际 fetch 的列名
    /// 列表，用于 GUI 区分「已还原列」与「未 fetch 列」。
    pub row_data_columns: Vec<String>,
    /// 行数据查询的首个 `0xNN` 字面量解码分隔符（如 `Some('~')`）；无
    /// `0xNN` 字面量时为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_separator: Option<char>,
    /// 该表汇集的探针总数（来自 column_list + row_data 两类聚合的
    /// `probe_count` 之和，参考用）。
    pub source_probe_count: u32,
}

/// 重建出的一行（v0.2.4 T4-1）。
///
/// `cells` 与所属 [`ReconstructedTable::columns`] 等长，未 fetch 列为 `None`。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReconstructedRow {
    /// 单元格值列表，与 `columns` 等长，未 fetch 列为 `None`。
    pub cells: Vec<Option<String>>,
}

impl BlindAggregator {
    /// 从已聚合的 [`AggregatedResult`] 列表交叉关联，重建结构化数据库
    /// （v0.2.4 T4-1）。
    ///
    /// 算法：
    /// 1. **分类**：对每个 AggregatedResult，按 `read_target` 模式分类：
    ///    - `schema`：`read_target == "database()"`
    ///    - `table_list`：含 `group_concat(table_name)` + `information_schema.tables`
    ///    - `column_list`：含 `group_concat(column_name)` + `information_schema.columns`，
    ///      并从 `table_name='X'` 谓词抽归属表名。
    ///    - `row_data`：含 `group_concat(` 且非上述两类，从 `from <table>` 抽
    ///      表名、从 `group_concat(<cols>)` 抽列清单 + `0xNN` 分隔符。
    ///    - 其他 → `unmatched_results`。
    /// 2. **拼装**：schema 直接填；对每张表（来自 table_list 或 row_data 的
    ///    表名）关联 column_list（按 `table_name='X'` 谓词匹配，无谓词按索引
    ///    对齐）+ row_data；row 按 `,` split 行、按 `column_separator` split
    ///    列，映射到 `columns`，未 fetch 列填 `None`。
    /// 3. **unmatched**：未匹配的 AggregatedResult 原样回传。
    ///
    /// **已知简化**（Risks）：
    /// - 行切分依赖 group_concat 默认行分隔符 `,`；若某列值含 `,` 会切错
    ///   （fixture idcard 为数字无 `,`）。SEPARATOR 子句解析 out_of_scope。
    /// - column_list 谓词缺失时按索引对齐（fixture 单表无影响）。
    pub fn reconstruct_database(results: &[AggregatedResult]) -> ReconstructedDatabase {
        let mut schema: Option<String> = None;
        // (table_name, column_list)
        let mut table_lists: Vec<(String, AggregatedResult)> = Vec::new();
        // (table_name predicate Option, column_list decoded)
        let mut column_lists: Vec<(Option<String>, AggregatedResult)> = Vec::new();
        // (table_name, row_data AggregatedResult)
        let mut row_data: Vec<(String, AggregatedResult)> = Vec::new();
        let mut unmatched: Vec<AggregatedResult> = Vec::new();

        for r in results {
            let rt = r.read_target.trim().to_lowercase();
            if rt == "database()" {
                schema = Some(r.decoded_string.clone());
                continue;
            }
            if rt.contains("group_concat(table_name)") && rt.contains("information_schema.tables") {
                // 表名清单：decoded_string 按 `,` split。优先取首个非空表名
                // 作为关联键（fixture 单表 "person_data"）。
                for t in r.decoded_string.split(',') {
                    let t = t.trim();
                    if !t.is_empty() {
                        table_lists.push((t.to_string(), r.clone()));
                    }
                }
                continue;
            }
            if rt.contains("group_concat(column_name)") && rt.contains("information_schema.columns") {
                // 解析 table_name='X' 谓词。
                let predicate = parse_table_name_predicate(&r.read_target);
                column_lists.push((predicate, r.clone()));
                continue;
            }
            if rt.contains("group_concat(") {
                // 行数据查询：解析 from <table> + group_concat(...) 列清单 + 0xNN 分隔符。
                if let Some(table_name) = parse_from_table(&r.read_target) {
                    row_data.push((table_name, r.clone()));
                } else {
                    unmatched.push(r.clone());
                }
                continue;
            }
            unmatched.push(r.clone());
        }

        // 拼装表：以 table_list 的表名为准，row_data 中无 table_list 的也补上。
        let mut table_names: Vec<String> = table_lists.iter().map(|(t, _)| t.clone()).collect();
        for (t, _) in &row_data {
            if !table_names.iter().any(|n| n == t) {
                table_names.push(t.clone());
            }
        }

        let mut tables: Vec<ReconstructedTable> = Vec::with_capacity(table_names.len());
        for tname in &table_names {
            // 关联 column_list：优先按谓词匹配；无谓词按索引对齐。
            let col_list = column_lists
                .iter()
                .find_map(|(pred, r)| {
                    if pred.as_deref() == Some(tname.as_str()) {
                        Some(r.clone())
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    column_lists
                        .iter()
                        .find(|(pred, _)| pred.is_none())
                        .map(|(_, r)| r.clone())
                });

            // 关联 row_data：按 from <table> 匹配。
            let row_r = row_data.iter().find_map(|(t, r)| {
                if t == tname {
                    Some(r.clone())
                } else {
                    None
                }
            });

            // 全量列：优先 column_list 还原；无则用 row_data_columns 兜底。
            let (columns, row_data_columns, column_separator, rows, source_probe_count) =
                build_table_columns_and_rows(&col_list, &row_r);

            tables.push(ReconstructedTable {
                name: tname.clone(),
                columns,
                rows,
                row_data_columns,
                column_separator,
                source_probe_count,
            });
        }

        // 未匹配 column_list / row_data（无 table_list 关联）补进 unmatched。
        // column_lists 中已被表关联消费的不重复计入；此处简化为不二次追加
        // （unmatched 已在分类阶段收集完非标准 RT）。

        ReconstructedDatabase {
            schema,
            tables,
            unmatched_results: unmatched,
        }
    }
}

/// 解析 `table_name='X'` 谓词，返回表名 `X`（小写 read_target 中保持原值）。
fn parse_table_name_predicate(read_target: &str) -> Option<String> {
    let re = table_name_predicate_regex();
    let caps = re.captures(read_target)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

/// 解析 `from <table>` 子句，返回表名（标识符）。
fn parse_from_table(read_target: &str) -> Option<String> {
    let re = from_table_regex();
    let caps = re.captures(read_target)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

/// 从行数据查询的 read_target 解析列清单与分隔符。
///
/// 返回 (row_data_columns, column_separator)。列清单取自
/// `group_concat(<cols>)` 的捕获组 1，按 `,` split 后过滤 `0xNN` 字面量
/// token（字面量作为分隔符，不作为列）；首个 `0xNN` 字面量解码为
/// `column_separator`。
fn parse_row_data_columns(read_target: &str) -> (Vec<String>, Option<char>) {
    let re = group_concat_args_regex();
    let args = re
        .captures(read_target)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_default();
    let mut cols: Vec<String> = Vec::new();
    let mut separator: Option<char> = None;
    for tok in args.split(',') {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        // 0xNN 字面量 → 分隔符（取首个）。
        if let Some(c) = parse_separator_char(tok) {
            if separator.is_none() {
                separator = Some(c);
            }
            continue;
        }
        cols.push(tok.to_string());
    }
    (cols, separator)
}

/// 为一张表构建全量列、行数据、分隔符、探针总数。
///
/// 返回 (columns, row_data_columns, column_separator, rows, source_probe_count)。
fn build_table_columns_and_rows(
    col_list: &Option<AggregatedResult>,
    row_r: &Option<AggregatedResult>,
) -> (Vec<String>, Vec<String>, Option<char>, Vec<ReconstructedRow>, u32) {
    // 全量列：优先 column_list 的 decoded_string（按 `,` split）；无则用
    // row_data_columns 兜底。
    let (row_data_columns, column_separator) = match row_r {
        Some(r) => parse_row_data_columns(&r.read_target),
        None => (Vec::new(), None),
    };

    let columns: Vec<String> = match col_list {
        Some(r) => r
            .decoded_string
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => row_data_columns.clone(),
    };

    // 行解析：decoded_string 按 `,` split 行（group_concat 默认行分隔符），
    // 每行按 column_separator split 列。
    let mut rows: Vec<ReconstructedRow> = Vec::new();
    let mut source_probe_count: u32 = 0;
    if let Some(r) = row_r {
        source_probe_count = source_probe_count.saturating_add(r.probe_count);
        if !r.decoded_string.is_empty() && !row_data_columns.is_empty() {
            for row_str in r.decoded_string.split(',') {
                let row_str = row_str.trim();
                if row_str.is_empty() {
                    continue;
                }
                let values: Vec<String> = match column_separator {
                    Some(sep) => row_str.split(sep).map(|s| s.to_string()).collect(),
                    None => vec![row_str.to_string()],
                };
                // 建 row_data_columns → value 映射。
                let mut map: HashMap<String, String> = HashMap::new();
                for (i, col) in row_data_columns.iter().enumerate() {
                    if let Some(v) = values.get(i) {
                        map.insert(col.clone(), v.clone());
                    }
                }
                // 按 columns 取值，未 fetch 列 = None。
                let cells: Vec<Option<String>> = columns
                    .iter()
                    .map(|c| map.get(c).cloned())
                    .collect();
                rows.push(ReconstructedRow { cells });
            }
        }
    }
    if let Some(r) = col_list {
        source_probe_count = source_probe_count.saturating_add(r.probe_count);
    }

    (columns, row_data_columns, column_separator, rows, source_probe_count)
}

fn table_name_predicate_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"table_name\s*=\s*'([^']+)'")
            .expect("table_name_predicate regex must compile")
    })
}

fn from_table_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"from\s+([a-zA-Z_][a-zA-Z0-9_]*)")
            .expect("from_table regex must compile")
    })
}

fn group_concat_args_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"group_concat\s*\(([^)]*)\)")
            .expect("group_concat_args regex must compile")
    })
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

/// 编译并缓存 AsciiBinary 盲注探针正则。
///
/// 形态：`ascii(substr((<read_target>),<pos>,1))<cmp><thr>`
/// 捕获组：1=read_target、2=char_position、3=comparator、4=threshold。
fn ascii_binary_regex() -> &'static Regex {
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
fn equality_regex() -> &'static Regex {
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
fn length_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"length\s*\(\s*\(\s*((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)\s*\)\s*\)\s*(>=?|<=?|=)\s*(\d+)",
        )
        .expect("length regex must compile")
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
        let mut e = mk_entry(1, "1.1.1.1", Some("ascii(substr((database()),1,1))>79"), Some(875));
        e.method.clear();
        e.path.clear();
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert!(agg.probes.is_empty());
    }

    #[test]
    fn regex_skips_entry_without_size() {
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
        let mut e = mk_entry(1, "1.1.1.1", None, Some(875));
        e.decoded_path = "ascii(substr((database()),1,1))>79".to_string();
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes.len(), 1);
        assert_eq!(agg.probes[0].read_target, "database()");
        assert_eq!(agg.probes[0].char_position, 1);
        assert_eq!(agg.probes[0].threshold, 79);
        assert_eq!(agg.probes[0].probe_kind, ProbeKind::AsciiBinary);
        assert_eq!(agg.probes[0].equality_char, None);
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
        assert_eq!(agg.probes.len(), 1, "double-nested read_target must match");
        let rt = &agg.probes[0].read_target;
        assert!(rt.contains("group_concat(table_name)"));
        assert!(rt.contains("information_schema.tables"));
        assert!(rt.contains("where table_schema=database()"));
        assert_eq!(agg.probes[0].char_position, 1);
        assert_eq!(agg.probes[0].threshold, 79);
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
        assert_eq!(parse_separator_char("group_concat(id,0x7e,x)"), Some('~'));
        assert_eq!(parse_separator_char("no literal here"), None);
        assert_eq!(parse_separator_char("group_concat(id,0xff,x)"), None);
    }

    // ---- v0.2.3 equality / length 探针 ----

    #[test]
    fn equality_probe_resolves_single_char() {
        // substr((database()),1,1)='p'，body==true_size（单簇 → 全 true）。
        let e = mk_entry(1, "1.1.1.1", Some("substr((database()),1,1)='p'"), Some(862));
        let agg = BlindAggregator::collect_from_entries(&[e]);
        assert_eq!(agg.probes.len(), 1);
        assert_eq!(agg.probes[0].probe_kind, ProbeKind::Equality);
        assert_eq!(agg.probes[0].equality_char, Some('p'));
        assert_eq!(agg.probes[0].char_position, 1);
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
        assert_eq!(agg.probes.len(), 1);
        assert_eq!(agg.probes[0].probe_kind, ProbeKind::Equality);
        assert_eq!(agg.probes[0].equality_char, Some('p'));
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
        assert_eq!(agg.probes.len(), 5);
        assert!(agg.probes.iter().all(|p| p.probe_kind == ProbeKind::Length));
        assert!(agg.probes.iter().all(|p| p.char_position == 0));
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

    // ---- v0.2.4 reconstruct_database 数据库结构还原 ----

    fn mk_result(read_target: &str, decoded_string: &str, probe_count: u32) -> AggregatedResult {
        AggregatedResult {
            read_target: read_target.to_string(),
            decoded_string: decoded_string.to_string(),
            resolved_chars: decoded_string.chars().count() as u32,
            unresolved_chars: 0,
            beyond_end_positions: 0,
            probe_count,
            source_ips: vec!["1.1.1.1".to_string()],
            position_details: Vec::new(),
            separator_char: parse_separator_char(read_target),
            kind: Some("ascii_binary".to_string()),
        }
    }

    #[test]
    fn reconstruct_full_fixture_database_view() {
        // 合成 4 个 AggregatedResult，模拟 fixture access.log 的盲注聚合结果。
        let results = vec![
            mk_result("database()", "person", 100),
            mk_result(
                "select group_concat(table_name) from information_schema.tables where table_schema=database()",
                "person_data",
                200,
            ),
            mk_result(
                "select group_concat(column_name) from information_schema.columns where table_name='person_data'",
                "id,username,password,sex,birth,idcard,phone",
                300,
            ),
            mk_result(
                "select group_concat(id,0x7e,username,0x7e,idcard) from person_data",
                "1~zhangsan~IDCARD1,2~lisi~IDCARD2",
                400,
            ),
        ];
        let db = BlindAggregator::reconstruct_database(&results);
        assert_eq!(db.schema.as_deref(), Some("person"));
        assert_eq!(db.tables.len(), 1, "expect 1 table");
        assert!(db.unmatched_results.is_empty(), "no unmatched");

        let table = &db.tables[0];
        assert_eq!(table.name, "person_data");
        assert_eq!(
            table.columns,
            vec!["id", "username", "password", "sex", "birth", "idcard", "phone"],
            "full columns from column_list",
        );
        assert_eq!(
            table.row_data_columns,
            vec!["id", "username", "idcard"],
            "row_data_columns parsed from group_concat args",
        );
        assert_eq!(table.column_separator, Some('~'));
        assert_eq!(table.rows.len(), 2, "2 rows");
        // row 0: 1~zhangsan~IDCARD1
        assert_eq!(table.rows[0].cells.len(), 7);
        assert_eq!(table.rows[0].cells[0].as_deref(), Some("1"));
        assert_eq!(table.rows[0].cells[1].as_deref(), Some("zhangsan"));
        assert_eq!(table.rows[0].cells[2], None, "password not fetched");
        assert_eq!(table.rows[0].cells[3], None, "sex not fetched");
        assert_eq!(table.rows[0].cells[4], None, "birth not fetched");
        assert_eq!(table.rows[0].cells[5].as_deref(), Some("IDCARD1"));
        assert_eq!(table.rows[0].cells[6], None, "phone not fetched");
        // row 1: 2~lisi~IDCARD2
        assert_eq!(table.rows[1].cells[0].as_deref(), Some("2"));
        assert_eq!(table.rows[1].cells[1].as_deref(), Some("lisi"));
        assert_eq!(table.rows[1].cells[5].as_deref(), Some("IDCARD2"));
        assert_eq!(table.source_probe_count, 300 + 400);
    }

    #[test]
    fn reconstruct_unmatched_results_collected() {
        // 非标准 RT → unmatched_results。
        let results = vec![
            mk_result("database()", "person", 100),
            mk_result("some weird payload", "xyz", 50),
        ];
        let db = BlindAggregator::reconstruct_database(&results);
        assert_eq!(db.schema.as_deref(), Some("person"));
        assert_eq!(db.tables.len(), 0, "no tables");
        assert_eq!(db.unmatched_results.len(), 1, "1 unmatched");
        assert_eq!(db.unmatched_results[0].read_target, "some weird payload");
    }

    #[test]
    fn reconstruct_partial_no_column_list_fallback() {
        // 只有 database() + row_data（无 table_name/column_name）→ columns 用
        // row_data_columns 兜底。
        let results = vec![
            mk_result("database()", "person", 100),
            mk_result(
                "select group_concat(id,0x7e,username,0x7e,idcard) from person_data",
                "1~zhangsan~IDCARD1",
                400,
            ),
        ];
        let db = BlindAggregator::reconstruct_database(&results);
        assert_eq!(db.schema.as_deref(), Some("person"));
        assert_eq!(db.tables.len(), 1);
        let table = &db.tables[0];
        assert_eq!(table.name, "person_data");
        // 无 column_list → columns 用 row_data_columns 兜底。
        assert_eq!(table.columns, vec!["id", "username", "idcard"]);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells[0].as_deref(), Some("1"));
        assert_eq!(table.rows[0].cells[1].as_deref(), Some("zhangsan"));
        assert_eq!(table.rows[0].cells[2].as_deref(), Some("IDCARD1"));
    }

    #[test]
    fn reconstruct_empty_results() {
        let db = BlindAggregator::reconstruct_database(&[]);
        assert!(db.schema.is_none());
        assert!(db.tables.is_empty());
        assert!(db.unmatched_results.is_empty());
    }

    #[test]
    fn reconstruct_no_separator_single_column_row() {
        // 行数据查询无 0xNN 分隔符，单列 group_concat(username) → 每行整体为单元格。
        let results = vec![mk_result(
            "select group_concat(username) from person_data",
            "zhangsan,lisi",
            50,
        )];
        let db = BlindAggregator::reconstruct_database(&results);
        assert_eq!(db.tables.len(), 1);
        let table = &db.tables[0];
        assert_eq!(table.name, "person_data");
        assert_eq!(table.columns, vec!["username"], "columns fallback to row_data_columns");
        assert_eq!(table.column_separator, None);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[0].as_deref(), Some("zhangsan"));
        assert_eq!(table.rows[1].cells[0].as_deref(), Some("lisi"));
    }

    #[test]
    fn parse_table_name_predicate_extracts_name() {
        assert_eq!(
            parse_table_name_predicate(
                "select group_concat(column_name) from information_schema.columns where table_name='person_data'"
            ),
            Some("person_data".to_string()),
        );
        assert_eq!(
            parse_table_name_predicate("select group_concat(column_name) from information_schema.columns"),
            None,
        );
    }

    #[test]
    fn parse_from_table_extracts_name() {
        assert_eq!(
            parse_from_table("select group_concat(id,0x7e,username,0x7e,idcard) from person_data"),
            Some("person_data".to_string()),
        );
        assert_eq!(
            parse_from_table("select group_concat(table_name) from information_schema.tables where table_schema=database()"),
            Some("information_schema".to_string()),
        );
    }

    #[test]
    fn parse_row_data_columns_filters_hex_literals() {
        let (cols, sep) = parse_row_data_columns(
            "select group_concat(id,0x7e,username,0x7e,idcard) from person_data",
        );
        assert_eq!(cols, vec!["id", "username", "idcard"]);
        assert_eq!(sep, Some('~'));
    }
}
