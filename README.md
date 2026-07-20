# RuT0DataKit

面向数据安全 CTF 场景的快速数据脱敏 / 解析工具集。核心用 Rust 实现，桌面端基于 Tauri v2，
全部处理在本地完成，不上传任何样本或规则。本仓库面向数据安全竞赛与红队场景中的
"敏感数据快速清洗 / 解析" 需求，不依赖 Python 运行时。

> **当前状态：v0.2.4 日志扫描盲注聚合数据库格式还原（T4-1 ~ T4-4 全部 verified_complete，已 release_complete）。** v0.2.4 在 v0.2.3 三轴优化的基础上，把盲注聚合结果从「逐 read_target 卡片」升级为「按数据库格式还原」：
> - **后端结构 + 算法（T4-1）**：`blind_aggregator` 新增 `ReconstructedDatabase` / `ReconstructedTable` / `ReconstructedRow` 三结构 + `BlindAggregator::reconstruct_database` 关联函数，三步算法：Step1 把每个 `AggregatedResult` 的 `read_target` 按 4 类模式分类（`schema`=database() / `table_list`=group_concat(table_name) from information_schema.tables / `column_list`=group_concat(column_name) ... where table_name='X' / `row_data`=group_concat(col1,0xNN,col2,...) from <table>；未匹配进 `unmatched_results`）；Step2 拼装 schema/tables/columns/rows（schema 直填 → table_list 解码表名按 `table_name='T'` 谓词匹配 column_list 全量列 → 按 `from T` 匹配 row_data 解析行：`,` split 行 + `column_separator`（`0xNN` 解码）split 列 → 投影到全量列，未 fetch 列 `None`）；Step3 unmatched 兜底。
> - **pipeline 透传（T4-2）**：`scan_log` 末尾调 `reconstruct_database` 塞 `Report.extra.reconstructed_database`（与 `blind_aggregation` 并列，向后兼容 v0.2.3）。fixture 4 类探针自动还原：schema="person" / 1 表 person_data / 7 列 / ≥2 行 / row[0].id=Some("1")。
> - **GUI 显示（T4-3）**：LogView 段 ③.5 之前新增段 ③.5a「还原数据库视图」Card——antd `Table` 按表渲染（全量列做表头，未 fetch 单元格显示 `-`）+ unmatched 兜底列表 + 空态 Empty；段 ③.5 改名「原始聚合明细」保留既有逐 read_target 卡片作兜底。
> `blind_aggregation` 数组保留不动，新增 `reconstructed_database` 键；新字段 `schema`（Option）/ `column_separator`（Option）均 `#[serde(skip_serializing_if)]`，向后兼容 v0.2.3。版本状态约定见 `docs/04-版本标准.md`。

## 功能

| 版本 | 能力 | 状态 |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX 表格数据脱敏 + 校验 + 导出 + 规则管理（四功能 GUI） | 已发布 v0.1.0 |
| v0.2.0 | 日志文件解析 + SQLi 攻击签名扫描 + 弱口令 / 敏感字段扫描 | 已发布 v0.2.0 |
| v0.2.1 | SQLi payload 语义解析（6 类）+ 字段还原（query/path/UA）+ `+` 解码 | 已发布 v0.2.1 |
| v0.2.2 | 日志扫描盲注二分序列自动聚合还原 flag + GUI 盲注聚合结果卡片 | 已发布 v0.2.2 |
| v0.2.3 | 盲注三轴优化：true_size 众数算法 + equality/length 类型 + GUI kind Tag/separator 高亮/Collapse 位置明细 | 已发布 v0.2.3 |
| v0.2.4 | 盲注聚合数据库格式还原：ReconstructedDatabase 交叉关联 4 类 read_target + GUI antd Table 按表渲染全量列 | 已发布 v0.2.4 |
| v0.3.0（规划） | pcap 流量包敏感数据提取（依赖系统 tshark） | 规划中 |

v0.1.0 已落地：
- core pipeline：`detect_type` → `SourceReader` → `mask_pipeline` / `mask_pipeline_selected`（行选择，向后兼容）/ `mask_pipeline_columns`（列勾选） → `validate_pipeline`（校验） → `write_masked_csv` / `export_records_csv` / `export_records_xlsx`
- 7 种内置脱敏器 + 1 种通用 `custom` 脱敏器 + 4 种富规则脱敏器（`regex_replace` / `regex_extract` / `delete` / `replace`）
- 7 种内置校验器（v0.2.0 日志扫描复用）
- **抽象算子规则引擎**：`MaskOp`（Template/SplitTemplate/RegexReplace/ConstReplace）+ `ValidateOp`（Regex/Algorithm/RegexWithGuard）+ 预置库（6 脱敏别名 + 7 校验别名）+ `apply_mask_op`/`apply_validate_op` 试运行 API；新增脱敏类型 = 加 variant + 预置库一行映射。
- **规则管理单一列表 + Drawer + 动态试运行**：脱敏+校验合并单一列表，右上角「添加规则」Drawer，每条规则「试运行」由用户独立输入样例值（不依赖任何文件导入），完全脱离数据文件管理。
- **规则/映射双 state 分离**：`state.rules` 是全局规则库（RulesView 管理，独立于文件）；`state.maskOverrides` / `state.validateOverrides` 是会话级临时映射（MaskView/ValidateView 管理，导入文件时清空）。MaskView/ValidateView 选算子只写 override，不污染规则库；「应用」时合并 override + 全局规则组成临时 RuleSet，导出/校验流程不受影响。
- **数据脱敏/校验四段垂直 view**：原始数据 → 表头-规则映射 → 预览 → 操作面板；应用按钮触发预览；导出按钮跳转。
- **数据导出单一 Table 集成**：表头 checkbox + 上下移调序 + 单元格预览，CSV+XLSX 双格式。
- **四功能导航 GUI**（侧边栏 4 active view + 2 disabled；数据脱敏 / 数据校验 / 数据导出 / 规则管理）
  - **按列勾选脱敏**：表头列 checkbox + 「应用规则到勾选列」
  - **数据校验 GUI**：ValidateView 独立 view，逐 cell 合法性矩阵 + 「跳转到数据导出」
  - **数据导出列选 / 列序 / 格式**：CSV + XLSX（src-tauri `rust_xlsxwriter`）
  - **规则详细说明 + 编辑**：`MaskRule` / `FieldRule` 带 `description` 字段；RulesView 编辑（已移除 YAML 保存/加载，仅保留添加/编辑/删除）
  - **跨视图数据不丢**：`App.jsx` 顶层 `useReducer` 内存常驻，切 view 不重置数据
- **`MaskOp::Template` 边界 BUG 修复**：当 `keep_prefix + keep_suffix == 值长度`（中间段长度为 0）时，原实现错误地输出 `原值 + 脱敏星`（如 `李四海*`）；修复后按规格走重叠分支，仅输出 `mask_min_len` 个 `mask_char`（如 `*`）。同步修复旧 `CustomMask::mask`。
- 前端基于 React + Ant Design 5 + @ant-design/icons，纯白主题，无 emoji，下拉使用 antd Select 组件

v0.2.0 日志处理切片已落地：
- **CLF 日志解析**：`crates/core/src/log/`：`LogReader::read` + `LogEntry`（ip / time / method / path / query / status / user_agent 等字段）+ `parse_query` 双重 URL 解码（`url_decode_twice`，解码 `%XX` 但不解 `+`，pattern 一律按 decoded 形态编写）。fixture `tests/fixtures/samples/log/access.log` 共 1860 行（含 4 条 SQLi 变体样本）。
- **SQLi 攻击签名引擎**：`crates/core/src/logsign/`：`SignatureEngine` + `sqli_signatures.yaml`（6 类内置签名：`sqli_blind_binary` / `sqli_union` / `sqli_error_based` / `sqli_time_based` / `sqli_tautology` / `sqli_comment`）+ `context_anchor` 锚点抑制弱信号误报（如 `or` / `--` / `#` 单独命中不算）。
- **签名引擎装载 / 重载**：`load_builtin_signatures`（内嵌 YAML）+ `load_signatures_from_str`（运行时 YAML 字符串）+ `SignatureEngine::from_rules`（构造时合并 + `enabled` 开关）。
- **日志扫描 pipeline**：`crates/core/src/pipeline/log_scan.rs`：`scan_log` 组合 SignatureEngine + 弱口令 grep（`WEAK_KEYWORDS`：password / passwd / admin / 123456 / root / qwerty 等）+ `DefaultSensitiveScan`（复用 v0.1.0 `SensitiveScan` trait），产出统一 `Report(kind="log_scan")`；`summary` 含 `total_lines` / `sqli_hits` / `weak_password_hits` / `sensitive_hits` / `top_attack_ips`（按命中次数排序，截断到 5 个）。
- **GUI 日志入口激活**：src-tauri `scan_log` 命令；frontend 侧边栏「日志扫描」由 disabled 切 active；pcap 仍置灰（v0.3.0）。
- **可观测性**：日志扫描结果与脱敏共用统一 `Report` / `Finding` serde 结构，前端可序列化展示。

v0.2.1 日志处理切片语义增强已落地：
- **`parse_query` `+` 解码 + decoded 字段**：`parse_query` 按 form-urlencoded 标准 `+`→space 再双重 `%XX` 解码；`LogEntry` 新增 `decoded_path` / `decoded_query` / `decoded_ua` 字段，`parse_line` 一次性填充，供签名引擎与 GUI 直接消费已解码文本（v0.2.0 口径只解 `%XX` 不解 `+`，v0.2.1 起改为标准 form-urlencoded）。
- **SQLi payload 语义解析**：`crates/core/src/logsign/payload_parser.rs`：`parse_payload(decoded_text) -> Option<ParsedPayload>` 对命中后的 decoded payload 做 6 类结构化语义解析（blind_boolean / union / error / time / tautology / comment），提取 read_target / char_position / compared_ascii / comparator / union_columns / sleep_seconds + 人类可读 `summary`。
- **`SignatureHit.parsed_payload`**：`scan_log_entry` 产 hit 时对命中文本段调 `parse_payload` 填充，未解析到为 `None`。
- **`Finding.extra` 透传**：`Finding` 新增 `extra: Option<serde_yml::Value>`（`skip_serializing_if = "Option::is_none"`，向后兼容）；`scan_log` pipeline 把 sqli hit 的 `parsed_payload` 序列化透传进 `Finding.extra`，供前端展示「解析结果」/「读取目标」列；weak_password / sensitive / csv_mask 场景 `extra=None`。
- **6 类签名 pattern 扩变体**：`sqli_union` 兼容 `union all select`；`sqli_error_based` 兼容 `exp(~...)` / `floor(rand(0)*2)` 报错注入变体。
- **GUI LogView decoded 列 + 解析结果/读取目标列**：原始日志表新增 `decoded_path` / `decoded_query` / `decoded_ua` 列；findings 表新增「解析结果」（`ParsedPayload.summary`）+「读取目标」（`read_target`）列。

v0.2.2 日志扫描盲注二分序列聚合还原已落地：
- **blind_aggregator 模块**：`crates/core/src/logsign/blind_aggregator.rs`：`BlindAggregator::collect_from_entries` 跑自带正则抽 `ascii(substr((<read_target>),N,1))<cmp><thr>` 形态二分探针；按 `(read_target, source_ip)` 分组 + `char_position` 子分组；基于 `LogEntry.size`（HTTP 响应 body 字节数）自动聚类真假方向（`true_size = min(body_size)`，fixture-specific 假设：条件成立响应 body 更小）；首个 `beyond_end` 即停止拼接，还原出完整 flag 字符串（如 `database()`=`"person"` / `table_name`=`"person_data"` / `column_name` 含 `id,username,password`）。
- **BlindProbe / PositionDetail / AggregatedResult 结构**：`read_target` / `char_position` / `threshold` / `body_size` / `source_ip` / `line_no` / `decoded_char` / `ascii_val` / `true_size` / `probe_count` / `status`（`resolved` / `unresolved_all_true` / `beyond_end` / `insufficient_probes`）/ `decoded_string` / `resolved_chars` / `unresolved_chars` / `beyond_end_positions` / `position_details`。
- **自带嵌套正则**：regex crate 无 look-around，`read_target` 内层嵌套括号用 `(?:[^()]|\([^()]*\))*` 吃单层 `(...)`（如 `database()` / `group_concat(table_name)`）；多层嵌套不支持（已知简化）。
- **Report.extra.blind_aggregation 透传**：`scan_log` 末尾把 `Vec<AggregatedResult>` 序列化进 `Report.extra` 的 `Value::Mapping({blind_aggregation: [...]})`；无盲注探针时 extra 仍为含空数组的 Mapping；向后兼容（v0.2.1 `Finding.extra: Option<Value>` 既有行为不破）。
- **GUI LogView 盲注聚合结果卡片**：findings 表下方新增段 ③.5，读 `Report.extra.blind_aggregation` 渲染 antd inner Card + `Typography.Text` copyable 还原串（一键复制 flag）+ 探针数 + 来源 IP + 位置明细 Tooltip + 空态 `Empty`。

v0.2.3 日志扫描盲注三轴优化已落地：
- **算法优化（T3-1）**：`true_size` 判定由 v0.2.2 的 `min(body_size)`（fixture-specific 假设，反向场景误判）改为 `mode_per_position_true_size`——按 `char_position` 分组取每位置真假簇 `body_size` 的 min/max → 跨位置众数（并列取较小者）→ 无混合位置退化 `min(body_size)`；修复 v0.2.2 R1 第 4 read_target（`group_concat(id,0x7e,username,0x7e,idcard)`，false 频次 737 > true 669）全局频次法误判为 875 → 全 `?` 的回归，新算法每位置 true 簇 = 862 → 众数 862 正确解出 `1~zhangsan~...~lisi~...`。自带正则从单层 `(?:[^()]|\([^()]*\))*` 扩到两层 `((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)` 覆盖 `where table_schema=database()`。`AggregatedResult` 新增 `separator_char: Option<char>`（`#[serde(skip_serializing_if)]`）从 `read_target` 首个 `0xNN` 字面量解码（`0x7e`→`~`，`0xff+` 返回 `None`）。
- **盲注类型扩展（T3-2）**：`BlindProbe` 新增 `probe_kind: ProbeKind` 枚举（`AsciiBinary` / `Equality` / `Length`）+ `equality_char: Option<char>`；新增 2 条正则：equality（`substr((...),N,1)=('x'|char(NN))` 直接解出单字符）+ length（`length((...))(>=?|<=?|=)(\d+)` 解出字符串长度，取最小 `=` 或最大 `<` 阈值）。聚合分组键由 `(read_target, source_ip)` 升级为 `(read_target, source_ip, ProbeKind)` 三元组，避免同一 read_target 的 ascii_binary 与 length 探针串扰。`AggregatedResult` 新增 `kind: Option<String>`（序列化为 `ascii_binary` / `equality` / `length`，`#[serde(skip_serializing_if)]`）。`PositionDetail.status` 新增 `equality_resolved` / `length_resolved`。
- **GUI 显示优化（T3-3）**：LogView 段 ③.5 inner Card：extra 加 kind Tag 着色（ascii_binary=red / equality=orange / length=blue；`r.kind` 缺失时前端按 `position_details[0].status` 兜底推断）+ 已解 N/M Tag；还原结果由 v0.2.2 的 `Text copyable` 升级为 `separator_char` 存在时 `Text strong` + volcano Tag 包裹分隔符高亮分段 + 独立「复制」按钮（`navigator.clipboard.writeText`），否则 `Text strong copyable`；位置明细由 v0.2.2 的 Tooltip + JSON.stringify 升级为 antd `Collapse`（ghost / size=small / 默认折叠）+ 内嵌 `Table`（列 位置 / 字符 / ASCII / 探针数 / 状态 Tag 6 色着色：resolved=green / unresolved_all_true=orange / beyond_end=default / insufficient_probes=red / equality_resolved=blue / length_resolved=purple）。

v0.2.4 日志扫描盲注聚合数据库格式还原已落地：
- **后端结构 + 算法（T4-1）**：`blind_aggregator.rs` 新增 `ReconstructedDatabase { schema, tables, unmatched_results }` / `ReconstructedTable { name, columns, rows, row_data_columns, column_separator, source_probe_count }` / `ReconstructedRow { cells: Vec<Option<String>> }` 三结构 + `BlindAggregator::reconstruct_database(&[AggregatedResult]) -> ReconstructedDatabase` 关联函数。三步算法：Step1 分类（4 类 read_target 模式：`schema`=database() / `table_list`=group_concat(table_name) from information_schema.tables / `column_list`=group_concat(column_name) ... where table_name='X' / `row_data`=group_concat(col1,0xNN,col2,...) from <table>；未匹配进 `unmatched_results`）；Step2 拼装（schema 直填 → table_list 解码表名按 `table_name='T'` 谓词匹配 column_list 全量列（无谓词按索引对齐，无列清单兜底 row_data_columns）→ 按 `from T` 匹配 row_data 解析行：`,` split 行 + `column_separator`（`0xNN` 解码）split 列 → 投影到全量列，未 fetch 列 `None`）；Step3 unmatched 原样塞回兜底。8 条单测覆盖全还原 / unmatched / 缺 column_list 兜底 / 空 results / 无分隔符单列 / 3 正则单元测试。
- **pipeline 透传（T4-2）**：`scan_log` 末尾调 `reconstruct_database` 塞 `Report.extra.reconstructed_database` 键（与 `blind_aggregation` 并列，向后兼容 v0.2.3）。fixture 4 类探针自动还原：schema="person" / 1 表 person_data / 7 列 (id/username/password/sex/birth/idcard/phone) / ≥2 行 / row[0].id=Some("1") / row[0].username=contains zhangsan/lisi / row_data_columns=[id,username,idcard] / column_separator=Some('~')。e2e `log_scan_full` 新增五段断言。
- **GUI 显示（T4-3）**：LogView 段 ③.5 之前新增段 ③.5a「还原数据库视图」Card——顶部 Descriptions（schema `<Text code>` + 表数 + unmatched 数）；每张 `ReconstructedTable` 一个 inner Card + antd `Table`（全量列做表头，未 fetch 单元格 `render: v => v ?? <Text type="secondary">-</Text>` 显示 `-`）+ extra（`column_separator` volcano Tag + `row_data_columns` Tooltip「已 fetch N/M 列」+ 探针 Tag）；unmatched 非空时下方 `Divider` + 简版列表（read_target + decoded_string copyable）；空态 `Empty`（「无法还原为数据库结构」）。段 ③.5 改名「原始聚合明细」保留既有逐 read_target 卡片作兜底（kind Tag / separator 高亮 / Collapse+Table 位置明细全部不动），数据源 `blindAggregation` 不变。
- **向后兼容**：`blind_aggregation` 数组保留不动；新增 `reconstructed_database` 键 + 新结构字段 `schema`（Option）/ `column_separator`（Option）均 `#[serde(skip_serializing_if)]`。
- **已知简化**：行切分依赖 group_concat 默认分隔符 `,`（fixture idcard 数字无 `,`）；`group_concat SEPARATOR 'X'` 子句解析、三层+嵌套正则、UNION 报错聚合 / 时间盲注聚合留 v0.3.0+。

## 安装

### 构建依赖

- **Rust toolchain**（stable，推荐 1.75+）：通过 rustup 安装。
- **Tauri v2 CLI**：`cargo install tauri-cli --version "^2"` 或 `cargo tauri` 子命令（用于跑 `cargo tauri dev` / `cargo tauri build`）。
- **tshark**：仅 **v0.3.0**（pcap 解析）需要。v0.1.0 与 v0.2.0 **不需要** tshark。
  macOS：`brew install wireshark`；Debian/Ubuntu：`apt install tshark`。

### 构建源码

```sh
git clone <repo-url> RuT0DataKit
cd RuT0DataKit
cargo build --workspace
```

## 快速开始

### 跑核心 pipeline（测试）

```sh
cargo build --workspace
# core 单元 + 集成测试
cargo test --workspace
```

### 启动 GUI

> 前端基于 React + Vite + Ant Design，首次运行需先安装 npm 依赖（仅一次，联网拉取 react/antd/@ant-design/icons/vite）。

```sh
cd frontend && npm install && cd ..
cargo tauri dev
```

打包发布版（产出 `src-tauri/target/release/ruT0-data-kit`，dist 已内嵌）：

```sh
cd frontend && npm install && npm run build && cd ..
cargo build --manifest-path src-tauri/Cargo.toml --release
```

默认规则文件位于 `rules/default_mask.yaml`，可直接引用或拷贝改写；GUI 内可在"自定义规则"下加载本地 YAML。

## 规则编写

规则文件为 YAML，包含 `validators` 与 `maskers` 两条列表。每条规则指定目标 `field`、
脱敏器 / 校验器名，以及可选 `params`（仅 `custom` masker 与 `regex` validator 需要参数）。

参考示例：

- `rules/default_mask.yaml`：v0.1.0 内置脱敏规则，覆盖 `sample_mask.csv` 全部字段。
- `rules/custom_example.yaml`：演示 `custom` masker 的通用参数。

### 内置脱敏器（maskers）

| name | 规则 | 备注 |
| --- | --- | --- |
| `idcard_mask` | 前 6 + 后 4，中间 8 位 `*` | 18 位身份证；短于阈值原样返回 |
| `phone_mask` | 前 3 + 后 4，中间 4 位 `*` | 11 位手机号 |
| `bankcard_mask` | 前 6 + 后 4，中间 `*` | 银行卡号 |
| `email_mask` | 本地首末保留，中间 `*` × (n-2)；本地 1 字原样、2 字 `z*` | 域名不动 |
| `name_mask` | 2 字 `X*`；≥3 字 `X*…*Y` | 中文姓名 |
| `customer_id_mask` | 首字符保留，其余 `*` | |
| `custom` | `keep_prefix` + `keep_suffix` + `mask_char` + `mask_min_len` | 通用可配 |
| `regex_replace` | `pattern` 正则 + `replacement`（支持 `$1`/`$2` 捕获组） | 富规则；非法/缺失 pattern 原值返回 |
| `regex_extract` | `pattern` 正则提取首个匹配子串 | 富规则；无匹配原值返回 |
| `delete` | 对任意输入返回 `""` | 富规则；无参数 |
| `replace` | 对任意输入返回 `with` 字符串 | 富规则；缺 `with` 退化为空串 |

> v0.1.0 起，全部内置脱敏器收敛到 `MaskOp` 抽象算子（4 通用 variant + 6 预置别名）；预置别名清单见 `list_mask_op_types()`（src-tauri 命令）或 `docs/02-技术设计文档.md` §2.3。

### 内置校验器（validators）

| name | 用途 |
| --- | --- |
| `idcard` | 身份证号格式校验 |
| `phone` | 手机号格式校验 |
| `bankcard` | 银行卡号格式校验 |
| `email` | 邮箱格式校验 |
| `mac` | MAC 地址格式校验（v0.2.0 日志扫描用） |
| `username` | 用户名格式校验（v0.2.0 日志扫描用） |
| `name` | 姓名格式校验 |

> v0.1.0 起，全部内置校验器收敛到 `ValidateOp` 抽象算子（3 通用 variant + 7 预置别名）；预置别名清单见 `list_validate_op_types()`（src-tauri 命令）或 `docs/02-技术设计文档.md` §2.4。

v0.1.0 GUI 已通过「数据校验」view 暴露校验入口：选字段 + validator → 运行校验 → 非法单元格高亮 → 可跳转「数据导出」导出合法/非法子集；validator 同时供 v0.2.0 日志扫描 pipeline 复用。

### custom masker 参数示例

```yaml
maskers:
  - field: customer_id
    masker: custom
    params:
      keep_prefix: 2
      keep_suffix: 2
      mask_char: "#"
      mask_min_len: 4
```

应用结果（对 `12345678`）：`12####78`。

## 开发指南

### 仓库布局

```
RuT0DataKit/
├── crates/
│   └── core/          # ruT0-data-kit-core：pipeline / readers / rules / maskers / validators / report
├── src-tauri/         # Tauri v2 后端（GUI 入口）
├── frontend/          # Tauri 前端
├── rules/             # default_mask.yaml / custom_example.yaml
├── tests/             # fixtures（样本文件）
└── docs/              # 需求 / 技术设计 / 任务清单 / 版本标准
```

core 的集成测试位于 `crates/core/tests/e2e.rs`，用 `CARGO_MANIFEST_DIR` 回退两级定位
`tests/fixtures/samples` 与 `rules`。

### 常用命令

```sh
# 构建
cargo build --workspace
# 全量测试
cargo test --workspace
# 仅 E2E 集成测试
cargo test --test e2e
# 启动 GUI
cargo tauri dev
```

## 版本路线

| 版本 | 目标能力 | 当前状态 |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX 脱敏 + 校验 + 导出 + 规则管理（四功能 GUI）+ Tauri GUI | 已发布 v0.1.0 |
| v0.2.0 | 日志文件解析 + SQLi 攻击签名扫描 + 弱口令 / 敏感字段扫描 | 已发布 v0.2.0 |
| v0.2.1 | SQLi payload 语义解析（6 类）+ 字段还原（query/path/UA）+ `+` 解码 | 已发布 v0.2.1 |
| v0.2.2 | 日志扫描盲注二分序列自动聚合还原 flag + GUI 盲注聚合结果卡片 | 已发布 v0.2.2 |
| v0.2.3 | 盲注三轴优化：true_size 众数算法 + equality/length 类型 + GUI kind Tag/separator 高亮/Collapse 位置明细 | 已发布 v0.2.3 |
| v0.2.4 | 盲注聚合数据库格式还原：ReconstructedDatabase 交叉关联 4 类 read_target + GUI antd Table 按表渲染全量列 | 已发布 v0.2.4 |
| v0.3.0 | pcap 流量包敏感数据提取（依赖 tshark） | 规划中 |

版本判定标准见 `docs/04-版本标准.md`。

## 安全与隐私

- 全部数据在本地处理，不调用任何远程接口；样本与规则不上传。
- 不引入 Python 运行时依赖；核心与扩展均为 Rust 原生。
- 输出文件路径由用户在 GUI 中显式指定，工具不自行外发。
- pcap 解析（v0.3.0）依赖系统 `tshark`，仍在本机执行。

## 许可证

待定（license 待确认）。
