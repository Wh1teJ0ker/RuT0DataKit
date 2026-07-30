# RuT0DataKit

面向数据安全 CTF 场景的快速数据脱敏 / 解析工具集。核心用 Rust 实现，桌面端基于 Tauri v2，
全部处理在本地完成，不上传任何样本或规则。本仓库面向数据安全竞赛与红队场景中的
"敏感数据快速清洗 / 解析" 需求，不依赖 Python 运行时。

> **当前状态：v0.8.0 已发布（T28-1 ~ T28-5 verified_complete，release_complete）。** v0.8.0 是收尾发布 minor 版本：
> - **v0.7.4 两处盲注 BUG 修复合并走门禁**：(a) `crates/core/src/tools/sql_parse.rs::SqlParseInput` 加 `#[serde(rename_all = "camelCase")]` root cause 修复——Tauri v2 `#[command]` 仅对顶层参数 camelCase→snake_case，嵌套 struct 字段走原生 serde，无 rename_all 则前端 camelCase `responseBodySize`/`sourceIp` 被 serde 静默丢 → `response_body_size: None` → `probe.rs None => Vec::new()` → 0 探针 → schema 空。v0.7.2 Rust 测试用 struct 字面量绕开 serde 故全绿但 GUI 从未跑通。独立 serde 验证（`/tmp/serde_test_proj`）复现：NoRename → None（静默丢）；WithRename → Some。(b) `src-tauri/src/commands/log.rs::detect_sql_blind_features` 移除 `samples.len() >= 50` 截断——长 log 上制造 gap → `insufficient_probes`，dedup 后全量传给 `parseSqlTool` IPC 体积可控。
> - **手机号自定义前缀功能 v0.6.8 修订已交付**（commit `666b496`，本版本仅索引不改代码）：用户原话「数据提取的phone可以支持自定义前缀三位，如果没有就默认1的正常号码…默认的52是不正确的」——确认 `phone`/`pinfo_phone` 已支持 `prefixes` 参数自定义前 1-3 位号段，缺省默认 1 开头正常号码，**无 "52" 拼留**（"52" 只在历史注释里标记废弃）。入口：RulesView phone/pinfo_phone 的 prefixes TextArea（placeholder `"138,159,734"`）→ `SET_EXTRACT_OVERRIDE`/`SET_VALIDATE_OVERRIDE`。
> - **版本号 0.7.4 → 0.8.0**：4 处 manifest 同步 + Cargo.lock ×2。非破坏性：serde 属性不改字段名/类型/签名只让前端 camelCase 正确反序列化；50-cap 移除只增不减；手机前缀零代码改动。版本状态约定见 `docs/04-版本标准.md`。
> - 详见 `docs/versions/0.8.0/更新日志.md` 与 `docs/qa/versions/0.8.0/QA-审计报告.md`。

## 功能

| 版本 | 能力 | 状态 |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX 表格数据脱敏 + 校验 + 导出 + 规则管理（四功能 GUI） | 已发布 v0.1.0 |
| v0.2.0 | 日志文件解析 + SQLi 攻击签名扫描 + 弱口令 / 敏感字段扫描 | 已发布 v0.2.0 |
| v0.2.1 | SQLi payload 语义解析（6 类）+ 字段还原（query/path/UA）+ `+` 解码 | 已发布 v0.2.1 |
| v0.2.2 | 日志扫描盲注二分序列自动聚合还原 flag + GUI 盲注聚合结果卡片 | 已发布 v0.2.2 |
| v0.2.3 | 盲注三轴优化：true_size 众数算法 + equality/length 类型 + GUI kind Tag/separator 高亮/Collapse 位置明细 | 已发布 v0.2.3 |
| v0.2.4 | 盲注聚合数据库格式还原：ReconstructedDatabase 交叉关联 4 类 read_target + GUI antd Table 按表渲染全量列 | 已发布 v0.2.4 |
| v0.3.0 | pcap 流量包敏感数据提取（tshark 子进程 + HTTP 字段提取 + 双重 URL 解码 + 自动 base64 字段解码 + 敏感扫描 + PcapView 四段 GUI） | 已发布 v0.3.0 |
| v0.4.0 | 7 界面架构性完整重构（统一预处理 6 类源 + 多标签规则引擎 + 统一搜索 SearchQuery 枚举 + Tools SQL 解析/正则解析） | 已发布 v0.4.0 |
| v0.4.1 | 5 项缺陷修复：数据流打通 + 移除各界面 FileToolbar + ToolsView 下拉栏 + SQL 盲注特征自动跳转 + RegexTool 语句→构造正则 + 搜索子串匹配修正 | 已发布 v0.4.1 |
| v0.4.2 | 设置模块首期：Sidebar 底部「设置」入口 + tshark 多平台自动检测 + 路径配置持久化 + SettingsView UI（+ 3 项 patch 修复：文件导入对话框补 json/sql 扩展名 / 侧边栏与主页面分离滚动 / 脱敏校验下拉改用用户规则，版本号不变） | 已发布 v0.4.2 |
| v0.4.3 | txt 兼容（PreprocessView 支持 .txt 导入）+ 数据提取独立模块（ExtractView：文件/文本输入 → phone/bankcard/ip 提取 → txt/csv/json 导出，匹配 PDF spec type_value 格式）+ 规则引擎去绝对化（PhoneValidator 删 CTF/real 白名单 → `^1\d{10}$`；新增 IpValidator） | 已发布 v0.4.3 |
| v0.5.0 | 架构性质升级：blind_aggregator/commands/operator 三大 God 文件拆分 + rules/patterns.rs 正则集中化 + 前端 state 领域切片 + ColumnRuleMapper 公共组件（依据代码质量审计报告 P0+P1+P2，零行为回归） | 已发布 v0.5.0 |
| v0.7.0 | 删除 Tools/正则解析子工具（前端 RegexTool/RegexConstructTab + Tauri explain_regex/regex_construct + core regex_explain/regex_construct/regex_template 全移除）+ PreprocessView 表头新增「列级 SQL 解析」按钮（取该列全部非空行 → `parseSqlTool` → 跳转 Tools/Sql 复用现有 UI） | 已发布 v0.7.0 |
| v0.8.0 | 收尾发布 minor：v0.7.4 两处盲注 BUG 修复（`SqlParseInput` 加 `#[serde(rename_all = "camelCase")]` root cause — Tauri v2 嵌套 struct 字段走原生 serde，前端 camelCase 字段被静默丢 → 0 探针 → schema 空；`detect_sql_blind_features` 移除 50 截断 — 长 log 制造 gap → insufficient_probes）+ 手机号自定义前缀 v0.6.8 修订已交付索引（`phone`/`pinfo_phone` 支持 `prefixes` 参数自定义前 1-3 位号段，缺省默认 1 开头，无 "52" 拼留，本版本仅索引不改代码） | 已发布 v0.8.0 |

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

v0.3.0 流量包处理切片已落地：
- **tshark 子进程 + HttpRequest 提取（T3-1）**：`crates/core/src/pcap/reader.rs::PcapReader::read(path)` 调系统 `tshark`（`-Y "http.request" -T fields -e frame.number -e ip.src -e ip.dst -e http.request.method -e http.host -e http.request.uri -e http.file_data -e http.user_agent -E separator=\t -E occurrence=f`）→ TSV 按行 split → 每行按 `\t` split 8 字段 → `HttpRequest { frame_no, src_ip, dst_ip, method, host, uri, body: Option<String>, user_agent }`；`http.file_data` hex 串经手写 `hex_to_bytes`（不引入 `hex` crate，empty/奇数长度/非法字符三边界处理）→ `String::from_utf8_lossy`。tshark 缺失：`Command::new("tshark").arg("--version")` 探测失败 → `CoreError::DependencyMissing("tshark")`，不 panic。fixture `tests/fixtures/samples/pcap/data.pcapng`（9.4MB / 60290 帧 / 5000 HTTP POST）解出 ≥5000 HttpRequest。
- **解码（T3-2）**：`crates/core/src/pcap/decoder.rs` 提供 4 公开函数：`decode_url_twice`（独立实现，不依赖 log 模块，避免循环依赖）；`try_decode_base64_field`（手写 `b64_decode`，不引入 `base64` crate；要求 `length≥4` + `length%4==0` + charset ⊂ `[A-Za-z0-9+/=]` + `printable_ratio≥0.8` 三重防误伤；UTF-8 only，GBK 留 v0.3.1+）；`extract_decoded_fields`（三降级：① `serde_json::from_str` 解析 Object 每字段值递归 base64 → ② form-urlencoded `&`/`=` split → ③ 纯文本整体当 `_body` 字段）；`reassemble_base64`（规则化兜底，fixture 每 POST body 单独 base64 不需要，保留接口供 CTF 分块场景）。`crates/core/Cargo.toml` 新增 `serde_json = "1"` 依赖。fixture 第一条 POST body 解出 7 字段全对：username=chenyong / name=付里夏旋 / sex=女 / birth=20010905 / idcard=506051200109055743 / phone=74733385248 / address=黑龙江省哈尔滨市通河县三站镇377号159室。
- **扫描 + pipeline + e2e（T3-3）**：`crates/core/src/pcap/scanner.rs::PcapScanner::scan(path, scan, rules)` 复用 `DefaultSensitiveScan`，对每个 HttpRequest 拼装 `scan_text = decode_url_twice(uri)` + body 经 `extract_decoded_fields` 解出的非空字段值（累加 `decoded_fragments`）→ `scan.scan(&scan_text, rules)` → 每个 Finding 填 `location=Some("frame:{frame_no}")` + `context=Some("{method} {host} -> {src_ip}")` + `extra=None` → 累加 `sensitive_hits` + `ip_counts: HashMap`；产出 `Report { source, kind: "pcap_scan", summary: { total_requests, sensitive_hits, decoded_fragments, top_src_ips（按 hits 倒序取前 5，每项 {ip, hits}） }, findings, extra: Value::Null }`。`pipeline/pcap_scan.rs::scan_pcap` 薄包装透传。e2e `pcap_scan_full`（`#[ignore]`，本机 tshark 在场手动跑）断言 ≥4000 sensitive hits + kind=="pcap_scan" + 含 idcard 类型全绿。
- **命令 + GUI（T3-4）**：Tauri 新增 `scan_pcap_file(path) -> Result<Value, String>` 命令（一次 IPC 返回 `{ entries: Vec<HttpRequest>, report: Report }`，避免二次调用）；`main.rs` `generate_handler!` 注册（共 21 命令）；`pcap_sensitive_ruleset()` 独立 helper 返回 idcard/phone/name validators RuleSet（不污染 mask view 的 `load_default_mask_ruleset`，其 validators 空）。前端 `PcapView.jsx` 四段布局（镜像 LogView）：① 导入 `.pcap/.pcapng`（`select_file` → `detect_source_type === "pcap"` 校验 → `scanPcapFile`）；② 原始 HTTP 表（pageSize 50，body slice 200 预览）；③ 扫描按钮 + summary Descriptions + top_src_ips volcano Tag；④ Findings 表（type Tag 按 `TAG_COLOR_BY_TYPE` 着色：idcard=purple / phone=blue / bankcard=magenta / email=cyan / mac=geekblue / username=gold / name=green）。state 加 `pcapEntries` / `pcapReport` / `pcapLoading`；`Sidebar.jsx` 删除 pcap `disabled: true`，6 项全 active。tshark 缺失：`msg.includes("tshark") || msg.includes("DependencyMissing")` → `message.error("流量分析需要系统 tshark，请先安装 Wireshark CLI (brew install wireshark)")`。0 emoji / 0 原生 select 保持。
- **仅敏感扫描，不做 SQLi 签名**（fixture 无 SQLi）；`kind="pcap_scan"` 新增，向后兼容 v0.2.4（旧 `csv_mask` / `log_scan` kind 不破）。

v0.4.0 架构性完整重构已落地：
- **统一预处理入口（界面 1）**：`read_records(path)` 在 core 层按 `detect_type(path)` 分派到 `CsvReader` / `XlsxReader` / `SqlReader`（按 `;` 切分语句、跳过字符串内分号，产出 `sql_text` + `statement_type` 两列）/ `JsonReader`（数组对象 union keys 作 headers、单值数组归 `value` 列、标量 to_string）/ `PcapRecordsReader`（tshark 缺失返回 `DependencyMissing("tshark")`）/ `LogRecordsReader`（同 v0.2.0 行解析口径产出统一列）。前端 `PreprocessView` 调 `read_records` 命令后写入 `state.records`，并暴露「搜索 / 数据脱敏 / 数据校验 / 数据导出」4 个跳转按钮。
- **多标签规则引擎（界面 2）**：`FieldRule` 与 `MaskRule` 增加 `tags: Vec<String>`（默认空 Vec 向后兼容），`RuleSet::by_tag(tag)` / `by_tag_mask(tag)` 按标签过滤；一条规则可同时挂多个标签（如 `[mask, sensitive]` / `[validate, sensitive]`），让同一规则在脱敏 / 校验 / 搜索 / SQL 解析多个视图复用；`RulesView` Drawer 多标签编辑。
- **统一搜索（界面 3）**：`search_records(records, query)` 统一入口，`SearchQuery` 枚举 `Keyword { terms, mode: And|Or }` / `Regex { pattern }` / `ExactField { field, value }`；keyword 走 `SearchIndex::build` 倒排索引 + `search_keyword`，regex 线性扫（非法 pattern 返回 `InvalidInput`），exact_field 按列名定位 + 精确匹配。`SearchView` 三选 Radio + 命中表（`<mark>` 高亮，无 `dangerouslySetInnerHTML`），「跳转脱敏 / 跳转导出」把命中行号去重排序写入 `state.filteredRowIndices`。
- **数据脱敏/校验/导出（界面 4-6）**：四段垂直布局，MaskView/ValidateView 消费 `state.filteredRowIndices` 实现「仅搜索命中行」过滤；ExportView 单一 antd Table + 表头 Checkbox 勾选导出列 + 上下移调序 + 单元格预览 + 源数据 Select（脱敏后 / 校验后 / 原始）+ 行过滤 + 格式 Select（CSV / XLSX / JSON）。
- **Tools 页面（界面 7）**：Sidebar SubMenu 下拉两个子工具——(a) **SQL 解析**：`parse_sqls(inputs) -> SqlParseResult { probes, aggregated, reconstructed, parsed_payloads }`，`extract_blind_probe(sql, response_body_size, source_ip)` 抽 `BlindProbe`（AsciiBinary / Equality / Length 三类），`reconstruct_database` 关联 4 类 read_target 自动还原数据库 schema/tables；非盲注 payload 走 `parse_payload` 兜底。(b) **加密/解密**：通用 EncryptTool。**v0.7.0：原「正则解析」子工具已删除**（详见 v0.7.0 更新日志）。
- **PreprocessView 列级 SQL 解析跳转（v0.7.0 T24-2）**：预览表表头 ✏ 改名按钮旁新增 `ConsoleSqlOutlined` 按钮，点击触发 `handleColumnSqlParse(columnName)`——取该列所有非空 cell 值作为 `SqlParseInput[]` → `parseSqlTool(inputs)` → 写入 `SET_SQL_PARSE_INPUT` + `SET_SQL_PARSE_RESULT` → 跳转 Tools/Sql 复用现有 `SqlParseTool.jsx` UI（镜像 Sidebar.jsx:60-62 跳转模式）；用户可在 Tools/Sql 继续编辑 textarea 重跑。空列 warning 不跳转，失败 message.error 不清空已有结果。仅本地处理，不外发数据。
- **跨视图 state 不丢**：`App.jsx` 顶层 `useReducer` 内存常驻，`SET_VIEW` 只切 view 不重置数据；搜索命中行号、脱敏结果、校验结果、SQL 解析结果均跨视图保留。
- **不外发数据**：全本地处理；规则与样本不上传（保留 v0.1.0 §6 安全约束）。

v0.4.1 5 项缺陷修复已落地：
- **T6-1 数据流打通**：`ExportView` 从读 `state.filePath`（SET_FILE 通道，v0.4.0 实现漏写）迁移到读 `state.records`（SET_RECORDS 通道，与 `PreprocessView.handleImport` 唯一写入端对齐）；`computeExportArgs` fallback 到 records 作为后端 inputPath；新增 e2e `preprocess_to_search_finds_hits`（csv → `read_records` → `search_records(Keyword "张三")` 命中行数 ≥1 + cell 含「张三」）端到端验证「预处理 → 搜索」链路，修复用户反馈「预处理导入后在搜索界面无法搜到」的根因。
- **T6-2 移除各界面 FileToolbar**：删除 `frontend/src/components/FileToolbar.jsx`（-106 行）+ `App.jsx` 移除 `NO_TOOLBAR_VIEWS` 集合与 `<FileToolbar/>` 渲染分支；导入唯一入口收敛到 `PreprocessView` 内置「选择文件」按钮（v0.4.0 设计本意）；消除各界面顶部冗余的导入按钮。
- **T6-3 ToolsView 下拉栏**：`Sidebar.jsx` 的「Tools」项由普通 Menu item 改为 antd `Menu.SubMenu`，`children=[{key:"tools.sql",label:"SQL 解析"},{key:"tools.encrypt",label:"加密/解密"}]`（v0.7.0：原 `tools.regex` 子项已删除），点击「Tools」标题展开/折叠（`openKeys` 受控为 `state.sidebarToolsOpen`，默认折叠）；子项点击双重 dispatch `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB(<sql|encrypt>)`，`selectedKeys` 在 `activeView === "tools"` 时高亮 `tools.<toolsActiveTab>`；`ToolsView.jsx` 本体不再渲染视图内 `Select` 下拉（与 Sidebar SubMenu 语义重复），仅渲染 Card + 子工具内容，Card `title` 随 `toolsActiveTab` 切换；用户反馈「侧边导航栏的 Tools 有下拉功能」最终落到 Sidebar SubMenu 形态。
- **T6-4 SQL 盲注特征自动跳转**：core `looks_like_blind_probe(sql: &str) -> bool` 复用 `ascii_binary_regex` / `equality_regex` / `length_regex` 三类正则，不依赖 `response_body_size`（区别于 `extract_blind_probe`，可用于预处理阶段仅 SQL 文本场景）；Tauri `detect_sql_blind_features(headers, rows) -> {detected, samples}`；GUI `PreprocessView.handleImport` 命中即自动跳转到 SqlParseTool 并预填样本；仅本地正则匹配，不外发数据。
- **T6-5 RegexTool 语句→构造正则**（v0.7.0 已删除）：历史 v0.4.1 重构项——移除内置模板 Tab + 新增 `ConstructTab`（antd `TextArea` 语句 → `regexConstruct(statement)` → `pattern` `Paragraph` copyable + `matched_clues` `Tag` 列表 + 测试样例高亮）；core 新增 `crates/core/src/tools/regex_construct.rs::construct_regex(statement) -> Result<ConstructedRegex, CoreError>`。v0.7.0 整体删除，详见 v0.7.0 更新日志。

## v0.7.0 已落地

- **T24-1 删除正则解析功能**：用户明确要求删除。前端 `RegexTool.jsx` / `RegexConstructTab.jsx` 删除，`ToolsView.jsx` / `Sidebar.jsx` / `state.js` / `tauri.js` 同步清理（删 5 个 ACTION + 5 个 state 字段 + 5 个 reducer case）；Tauri `commands/tools.rs` 重写为仅 `parse_sql_tool`，`main.rs` 删除 `explain_regex` / `regex_construct` 两项注册（handler 39 → 37）；core `tools/mod.rs` 重写为仅 `encrypt` + `sql_parse`，删除 `regex_explain.rs` / `regex_construct.rs` / `regex_template.rs` 三文件。**明确保留** `SearchQuery::Regex`、masker 内部 regex（`regex_replace` / `regex_extract`）、logsign blind regex、`state.searchRegexInput` / `SEARCH_REGEX_*`——这些位置用 regex 但与 Tools/正则解析子工具无关。
- **T24-2 PreprocessView 列级 SQL 解析跳转**：预览表表头 `<Space>` 在 ✏ 改名按钮旁新增 `ConsoleSqlOutlined` 按钮，`handleColumnSqlParse(columnName)`：遍历 `records.rows` 按 `headers.indexOf(columnName)` 取列下标，收集所有非空 cell 值 → `parseSqlTool(inputs)` → 写 `SET_SQL_PARSE_INPUT` + `SET_SQL_PARSE_RESULT` → `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB("sql")`。与 T6-4「盲注自动跳转」互补：前者是用户针对单列主动触发，后者是导入时按全部 cell 探测。`SqlParseTool.jsx` 不修改，已读取这两个 state 字段。
- **T24-3 版本 bump + docs 全面清理**：4 处 manifest `0.6.8 → 0.7.0`；docs/00/01/02/03/04 + README + README_EN 同步删除正则解析条目 + 新增 v0.7.0 列级 SQL 跳转说明；`docs/versions/0.7.0/更新日志.md` + `docs/qa/versions/0.7.0/QA-审计报告.md` 落盘。
- **T24-4 Release QA + finalize**：5 维度 Release QA（功能 / 回归 / 构建 / 安全 / 文档）结论 qa_passed；`cargo test -p ruT0-data-kit-core --release` 464 passed + tauri 5 passed + npm build 3007 modules 0 error；commit + push origin main。

## v0.7.1 已落地

- **T25-1 双调用点 URL-decode-on-detection**：用户报告 URL-encoded SQLi payload（`username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1`）解析失败。根因：`parse_sqls` / `detect_sql_blind_features` 把 raw 输入直接喂纯文本正则（期望字面空格 ` ` / `>` / `#`），遇到 `%20` / `%3E` / `%23` 不匹配 → 0 探针。修复：新增 `crates/core/src/log/mod.rs::looks_like_url_encoded`（正则 `(?i)%[0-9a-f]{2}` + `OnceLock` 单例，复用既有 `regex::Regex` 零新依赖）；core `tools/sql_parse.rs::parse_sqls` 循环内对 `input.sql` 条件解码后喂 `extract_blind_probe` / `parse_payload`；Tauri `commands/log.rs::detect_sql_blind_features` 对 cell 条件解码后喂 `looks_like_blind_probe`，`samples` 改收集解码后形态（与 `parse_sqls` 入口一致，避免下游二次解码）。`SqlParseInput.sql` 字段语义放宽：caller 可直传 raw。对已解码输入零影响（`looks_like_url_encoded` 返回 `false` 走原路径，向后兼容 v0.5.0 ~ v0.7.0）。新增 8 core 单测 + 3 Tauri 单测。
- **T25-2 版本 bump + docs + Release QA + finalize**：4 处 manifest `0.7.0 → 0.7.1`；docs/00/02/03/04 + README + README_EN 同步新增 v0.7.1 说明；`docs/versions/0.7.1/更新日志.md` + `docs/qa/versions/0.7.1/QA-审计报告.md` 落盘；5 维度 Release QA 结论 qa_passed；`cargo test -p ruT0-data-kit-core --release` 472 passed + tauri 8 passed + npm build 3007 modules 0 error；commit + push origin main。

## v0.7.2 已落地

- **T26-1 前端 SqlParseTool `body|sql` 前缀解析 + UI 文案**：用户报告 URL-encoded payload 经 v0.7.1 URL 解码后仍无法完整还原数据库。根因：前端 `SqlParseTool.jsx:58` 对所有行构造 `responseBodySize: null`，而 `extract_blind_probe_with_line`（`crates/core/src/logsign/blind/probe.rs:61-64`）在 `response_body_size: None` 时 `return Vec::new()` → 0 探针 → 聚合空 → 还原空。盲注二分还原**必须**有 body_size 才能区分 true（`ascii>thr` 为真，body==true_size）vs false（body!=true_size）两簇。修复：`onParse` 每行用正则 `^(\d+)\|(.*)$` 解析 `<body_size>|<sql>` 前缀（如 `862|username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1`），命中则 `responseBodySize` 为整数，否则保持 `null`（向后兼容非盲注 payload：time/error/union/tautology/comment 仍走 `parse_payload` 兜底）。文件顶部注释 + 帮助文字 + placeholder 同步更新说明前缀语法。无新依赖（纯本地正则）。
- **T26-2 core 单测贯通验证 + gap 如实说明**：新增 2 个 core 单测到 `crates/core/src/tools/sql_parse.rs` tests 模块——(1) `parse_sqls_body_size_enables_full_reconstruction`：用既有 helper `ascii_binary_probes_for_char("database()", 1, b'p' as u32, 862, 875, "1.1.1.1")` 生成 3 true + 2 false 自洽序列（max_true=111, min_false=112, `111+1=112` ✓ 自洽），断言 5 探针 + `decoded_string.starts_with('p')` + schema=="p"；回归对照：同输入但 `response_body_size: None` → 0 探针 + 空 aggregated，证明 body_size 是贯通关键。(2) `parse_sqls_user_five_payloads_with_body_size_gap_documented`：用户提供的 5 条 payload（阈值 79/103/109 true 侧 body=862，112/115 false 侧 body=875），断言 5 探针 + aggregated 1 条 + position 1 `ascii_val=Some(112)` + `decoded_char=None` + `status="insufficient_probes"`。**gap 真相**：`max_true=109, min_false=112, 109+1=110≠112` → 严格自洽校验（`aggregate.rs:466-469`）不通过 → `insufficient_probes`。该校验是取证工具保守设计，patch **不放宽**；测试注释说明：补 110/111 两条探针即可自洽还原 'p'。
- **T26-3 版本 bump + docs + Release QA + finalize**：4 处 manifest `0.7.1 → 0.7.2` + Cargo.lock ×2 同步；docs/00/02/03/04 + README + README_EN 同步新增 v0.7.2 说明；`docs/versions/0.7.2/更新日志.md` + `docs/qa/versions/0.7.2/QA-审计报告.md` 落盘；5 维度 Release QA 结论 qa_passed；`cargo test -p ruT0-data-kit-core --release` 376 passed + 3 ignored + tauri 8 passed + npm build 3007 modules 0 error；commit + push origin main。

## v0.7.3 已落地

- **T27-1 扩展 detect_sql_blind_features 返回 body_size + source_ip + 去重 + 测试**：用户报告无法手工获得 `862|username=1'%20or%20ascii(substr((database()),1,1))%3E110%23&password=1` 这种带 body_size 前缀的数据。根因：log 源的 `size` 列就是 HTTP 响应 body 字节数、`ip` 列就是来源 IP（都已存在于 records），但 `detect_sql_blind_features`（`src-tauri/src/commands/log.rs:58-88`）扫描盲注探针时**丢弃**了 body_size 和 source_ip，只返回 `samples: Vec<String>`（纯 SQL 文本）。修复：扩展返回形状为 `samples: Vec<{ sql, body_size, source_ip }>`——从 `headers` 按 header 名定位 `size` 列 + `ip` 列，命中 `looks_like_blind_probe` 时从同行配对 body_size（空/`-`→None）+ source_ip（空→None）；按 `(sql, body_size, source_ip)` 三元组去重避免 query/decoded_query 列重复收集；samples 上限仍 50。新增 4 个 Tauri 单测 + 更新 3 个既有测试断言。返回形状变更零破坏（v0.5.x 删除自动跳转入口后无前端调用方）。
- **T27-2 前端 PreprocessView 新增「盲注自动提取」按钮 + tauri.js 注释更新**：`PreprocessView.jsx` 段 ④ 跳转区新增「盲注自动提取」按钮（`handleBlindAutoExtract`），import `detectSqlBlindFeatures` + `parseSqlTool`，点击后扫描全表 → 配对 body_size/ip → 拼 `body|sql` 文本 → 调 `parseSqlTool` → dispatch `SET_SQL_PARSE_INPUT` + `SET_SQL_PARSE_RESULT` + `SET_VIEW: tools` + `SET_TOOLS_ACTIVE_TAB: sql`；`!res.detected` → `message.warning` 不跳转；失败 `showError` 不清空。`tauri.js` `detectSqlBlindFeatures` 注释更新为新返回形状。非 log 源无 `size`/`ip` 列 → body_size=null 走 `parse_payload` 兜底。
- **T27-3 版本 bump + docs + Release QA + finalize**：4 处 manifest `0.7.2 → 0.7.3` + Cargo.lock ×2 同步；docs/00/02/03/04 + README + README_EN 同步新增 v0.7.3 说明；`docs/versions/0.7.3/更新日志.md` + `docs/qa/versions/0.7.3/QA-审计报告.md` 落盘；5 维度 Release QA 结论 qa_passed；`cargo test -p ruT0-data-kit-core --release` 376 passed + 3 ignored + tauri 12 passed + npm build 3007 modules 0 error；commit。

## 安装

### 构建依赖

- **Rust toolchain**（stable，推荐 1.75+）：通过 rustup 安装。
- **Tauri v2 CLI**：两种方式任选其一
  - `cargo install tauri-cli --version "^2"`（全局安装，命令 `cargo tauri`）
  - 直接用 npx 免安装：`npx --yes @tauri-apps/cli@latest <subcommand>`（本仓库采用这种方式，无需全局安装）
- **Node.js + npm**：前端构建需要（首次需联网拉 react/antd/@ant-design/icons/vite）
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

### 启动 GUI（开发模式）

> 前端基于 React + Vite + Ant Design，首次运行需先安装 npm 依赖（仅一次，联网拉取 react/antd/@ant-design/icons/vite）。

```sh
# 方式 A：cargo tauri（需先 cargo install tauri-cli --version "^2"）
cd frontend && npm install && cd ..
cargo tauri dev

# 方式 B：npx 免安装 tauri-cli（推荐，本仓库 CI/打包采用此方式）
cd frontend && npm install && cd ..
npx --yes @tauri-apps/cli@latest dev
```

### 打包发布版（产出 .app / .dmg / 裸二进制）

**一键打包（推荐，自动构建前端 + 编译后端 + 产出 bundle）：**

```sh
# 在仓库根目录执行（cwd 必须是仓库根，tauri-cli 会读 src-tauri/tauri.conf.json）
npx --yes @tauri-apps/cli@latest build
```

产物位置（macOS）：
- `.app` 包：`src-tauri/target/release/bundle/macos/RuT0DataKit.app`
- `.dmg` 安装包：`src-tauri/target/release/bundle/dmg/RuT0DataKit_<version>_aarch64.dmg`
- 裸二进制：`src-tauri/target/release/ruT0-data-kit`

**分步打包（手动控制前端构建，适合 CI 或排查问题）：**

```sh
# 1. 构建前端生产资源到 frontend/dist/
cd frontend && npm install && npm run build && cd ..

# 2. 编译 release 二进制（前端 dist 已内嵌）
cargo build --manifest-path src-tauri/Cargo.toml --release

# 3.（可选）单独产出 .app/.dmg bundle（跳过 beforeBuildCommand，因为前端已构建）
npx --yes @tauri-apps/cli@latest build --no-bundle   # 仅二进制
npx --yes @tauri-apps/cli@latest build                # 含 .app + .dmg
```

> **打包排错 tip**：如果 `npx @tauri-apps/cli build` 报 `npm --prefix frontend run build` 找不到 `frontend/frontend/package.json`，是因为 **tauri-cli 执行 `beforeBuildCommand` 时 cwd 是 `frontend/`，不是仓库根**，`--prefix frontend` 会被叠加成 `frontend/frontend`。正确做法：把 `src-tauri/tauri.conf.json` 的 `beforeBuildCommand` 从 `npm --prefix frontend run build` 改为 `npm run build`（`beforeDevCommand` 同理改为 `npm run dev`），本仓库 v0.5.0 已按此修正。

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

### GUI 数据脱敏（v0.5.x）

「数据脱敏」视图的段②「表头-脱敏映射」是动态配置：每个表头一行，现场选脱敏算子（scope）
并填参数，直接写会话级 `maskOverrides`，不进规则库、不落盘（导入新文件时清空）。无需先到
「规则管理」创建规则——脱敏是按数据现场填参数的一次性动作。

「规则管理」视图的 4 条脱敏模版（`template` / `split_template` / `regex_replace` /
`const_replace`，tag=`mask`，params=`None`）是**内置只读**的，每条带「试运行」面板：填参数 +
样例值 → 点「运行」调 `trial_mask` 命令 → 立即看到脱敏结果，用于在作用于真实数据前验证参数
是否正确（试运行只算单条样例值，不读文件、不落盘）。

可选算子（scope）：

| scope | 参数 | 适用 |
| --- | --- | --- |
| `template` | keep_prefix / keep_suffix / mask_min_len / min_len / max_len / mask_char / cjk | 通用替换脱敏（保留首尾 + 中间打码） |
| `split_template` | 上述 7 个 + separator / segment_index | 邮箱等切分后脱敏 |
| `regex_replace` | pattern / replacement / match_mode | 正则替换 |
| `const_replace` | with | 整列替换为常量 |

按《数据脱敏规范文档》（`tests/fixtures/samples/tips/附件/数据脱敏规范文档.pdf`），导入
`tests/fixtures/samples/csv/sample_mask_spec.csv` 后，每个表头选 `template` 算子并按下表填
参数即可复现 PDF §4.2 脱敏结果（编号/性别两列不选算子即不脱敏）：

| 表头 | keep_prefix | keep_suffix | mask_min_len | min_len | max_len | cjk | 结果示例 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 姓名 | — | — | — | — | — | true | 张三→张*、李小明→李*明、欧阳修华→欧**华 |
| 身份证号 | 6 | 4 | 8 | 18 | 18 | false | 110101199001011234→110101********1234 |
| 手机号 | 3 | 4 | 4 | 11 | 11 | false | 13812345678→138****5678 |
| 出生日期 | 8 | 0 | 2 | — | — | false | 1990-01-15→1990-01-** |
| 银行卡号 | 4 | 4 | 1 | — | — | false | 6222021234567890123→6222***********0123 |

> cjk=true 走中文姓名分支（n=2→首*、n=3→首*末、n≥3→首+*(n-2)+末），此时 keep_prefix/
> keep_suffix 被忽略，仅 mask_char / min_len / max_len guard 生效。其它规则靠 keep_prefix +
> keep_suffix + mask_min_len 三参数即可复现。空字段不填让后端默认生效。

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
| v0.3.0 | pcap 流量包敏感数据提取（tshark 子进程 + HTTP 字段提取 + 双重 URL 解码 + 自动 base64 字段解码 + 敏感扫描 + PcapView 四段 GUI） | 已发布 v0.3.0 |
| v0.4.0 | 7 界面架构性完整重构（统一预处理 6 类源 + 多标签规则引擎 + 统一搜索 SearchQuery 枚举 + Tools SQL 解析/正则解析） | 已发布 v0.4.0 |
| v0.4.1 | 5 项缺陷修复：数据流打通 + 移除各界面 FileToolbar + ToolsView 下拉栏 + SQL 盲注特征自动跳转 + RegexTool 语句→构造正则 | 已发布 v0.4.1 |
| v0.4.2 | 设置模块首期 + tshark 多平台自动检测 + 路径配置持久化 + 3 项 patch 修复 | 已发布 v0.4.2 |
| v0.4.3 | txt 兼容 + 数据提取独立模块（ExtractView）+ 规则引擎去绝对化（PhoneValidator 删白名单 + 新增 IpValidator） | 已发布 v0.4.3 |
| v0.5.0 | 架构性质升级：blind_aggregator/commands/operator 三大 God 文件拆分 + rules/patterns.rs 正则集中化 + 前端 state 领域切片 + ColumnRuleMapper 公共组件（依据代码质量审计报告 P0+P1+P2，零行为回归） | 已发布 v0.5.0 |
| v0.7.0 | 删除 Tools/正则解析子工具（前端 RegexTool/RegexConstructTab + Tauri explain_regex/regex_construct + core regex_explain/regex_construct/regex_template 全移除）+ PreprocessView 表头新增「列级 SQL 解析」按钮（取该列全部非空行 → `parseSqlTool` → 跳转 Tools/Sql 复用现有 UI） | 已发布 v0.7.0 |
| v0.7.1 | SQL 解析路径自动识别 URL-encoded 输入：新增 `looks_like_url_encoded`（`(?i)%[0-9a-f]{2}`）+ `parse_sqls` / `detect_sql_blind_features` 命中才调 `url_decode_twice` 双重解码再喂探针正则；caller 可直传 raw，对已解码输入零影响（patch，非破坏性） | 已发布 v0.7.1 |
| v0.7.2 | SqlParseTool 支持 `body\|sql` 前缀打通盲注还原链路：前端 `onParse` 每行用正则 `^(\d+)\|(.*)$` 解析 `<body_size>\|<sql>` 前缀（如 `862\|username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1`），无前缀行保持 `null` 向后兼容非盲注 payload；2 个 core 单测贯通验证（自洽序列还原 'p' + 用户 5 payload gap 如实断言）；聚合算法严格自洽校验 `max_true+1==min_false` 不放宽，gap=110/111 补齐即自洽还原 'p'（patch，非破坏性） | 已发布 v0.7.2 |
| v0.7.3 | PreprocessView 新增「盲注自动提取」按钮 + `detect_sql_blind_features` 返回 body_size/source_ip：扩展既有 Tauri 命令返回形状从 `samples: Vec<String>` 改为 `samples: Vec<{ sql, body_size, source_ip }>`——从同行按 header 名定位 `size` 列（HTTP 响应 body 字节数）+ `ip` 列（来源 IP）配对，按 `(sql, body_size, source_ip)` 三元组去重；PreprocessView 段 ④ 跳转区新增按钮（`handleBlindAutoExtract`），点击后扫描全表 → 拼 `body\|sql` 文本 → 调 `parseSqlTool` → 跳转 Tools/Sql；非 log 源无 size/ip 列 → body_size=null 走 `parse_payload` 兜底；新增 4 个 Tauri 单测；返回形状变更零破坏（v0.5.x 删除自动跳转入口后无前端调用方）（patch，非破坏性） | 已发布 v0.7.3 |
| v0.8.0 | 收尾发布 minor：v0.7.4 两处盲注 BUG 修复（Fix A `SqlParseInput` 加 `#[serde(rename_all = "camelCase")]` root cause — Tauri v2 嵌套 struct 字段走原生 serde，前端 camelCase `responseBodySize`/`sourceIp` 被静默丢 → `response_body_size: None` → 0 探针 → schema 空；v0.7.2 测试用 struct 字面量绕开 serde 故全绿但 GUI 从未跑通；Fix B `detect_sql_blind_features` 移除 50 截断 — 长 log 制造 gap → insufficient_probes）+ 手机号自定义前缀 v0.6.8 修订已交付索引（`phone`/`pinfo_phone` 支持 `prefixes` 参数自定义前 1-3 位号段，缺省默认 1 开头，无 "52" 拼留，本版本仅索引不改代码）（minor，非破坏性） | 已发布 v0.8.0 |

版本判定标准见 `docs/04-版本标准.md`。

## 安全与隐私

- 全部数据在本地处理，不调用任何远程接口；样本与规则不上传。
- 不引入 Python 运行时依赖；核心与扩展均为 Rust 原生。
- 输出文件路径由用户在 GUI 中显式指定，工具不自行外发。
- pcap 解析（v0.3.0）依赖系统 `tshark`，仍在本机执行。

## 许可证

待定（license 待确认）。
