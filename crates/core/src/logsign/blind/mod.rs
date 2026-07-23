//! 布尔盲注探针聚合还原模块（v0.2.2 T2-12 / v0.2.3 T3-2 / v0.2.4 T4-1，
//! v0.5.0 T12-1 拆分为三层）。
//!
//! 与 [`super::payload_parser`] 互补：payload_parser 只解析「单条 payload 的
//! 语义类别」（盲注/UNION/报错…），本模块在签名命中之后，对一批同源
//! （同 read_target + 同 source_ip + 同 [`probe::ProbeKind`]）的盲注探针按位置
//! 聚类，还原出被盲注读取的完整字符串或数值，并在 v0.2.4 增加数据库结构
//! 还原 [`aggregate::BlindAggregator::reconstruct_database`]：把 4 类标准 read_target
//! （`database()` /
//! `group_concat(table_name) from information_schema.tables` /
//! `group_concat(column_name) from information_schema.columns where table_name='X'` /
//! `group_concat(col1,0xNN,col2,...) from <table>`）交叉关联成结构化的
//! [`reconstruct::ReconstructedDatabase`]（schema → tables → columns → rows）。
//!
//! ## 三层职责（v0.5.0 T12-1 拆分）
//!
//! - [`probe`]：探针抽取层。三套自带正则（ascii_binary / equality / length）
//!   从已解码 SQL 文本抽取 [`probe::BlindProbe`]，并定义 [`probe::ProbeKind`]、
//!   [`probe::PositionDetail`]、[`probe::AggregatedResult`] 等数据结构。
//! - [`aggregate`]：聚合算法层。按 `(read_target, source_ip, ProbeKind)` 三元组
//!   分组，每位置独立判真假簇，跨位置取众数还原字符串/数值；核心入口
//!   [`aggregate::BlindAggregator::aggregate`]。
//! - [`reconstruct`]：数据库结构还原层。4 类标准 read_target 交叉关联成
//!   [`reconstruct::ReconstructedDatabase`]；核心入口
//!   [`aggregate::BlindAggregator::reconstruct_database`]。
//!
//! ## 支持的探针形态（v0.2.3）
//!
//! - [`probe::ProbeKind::AsciiBinary`]：`ascii(substr((<rt>),<pos>,1))<cmp><thr>`
//!   二分序列，按位置聚类还原字符串（v0.2.2 既有）。
//! - [`probe::ProbeKind::Equality`]：`substr((<rt>),<pos>,1)='c'` 或
//!   `substr((<rt>),<pos>,1)=char(<ascii>)` 等值形态，单探针直接得字符
//!   （v0.2.3 新增）。
//! - [`probe::ProbeKind::Length`]：`length((<rt>))<cmp><thr>` 长度盲注，按 threshold
//!   二分还原长度数值，`decoded_string` 输出十进制数字串（如 `"28"`）
//!   （v0.2.3 新增）。
//!
//! 时间盲注聚合 out_of_scope（`LogEntry` 无 `response_time_ms` 字段，
//! fixture 无样本）。
//!
//! ## 输入
//!
//! - [`aggregate::BlindAggregator::collect_from_entries`] 吃 `&[LogEntry]`，对每条 entry
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

pub mod aggregate;
pub mod probe;
pub mod reconstruct;

pub use aggregate::BlindAggregator;
pub use probe::{
    extract_blind_probe, extract_blind_probe_with_line, looks_like_blind_probe, AggregatedResult,
    BlindProbe, PositionDetail, ProbeKind,
};
pub use reconstruct::{ReconstructedDatabase, ReconstructedRow, ReconstructedTable};
