# RuT0DataKit

面向数据安全 CTF 场景的快速数据脱敏 / 解析工具集。核心用 Rust 实现，桌面端基于 Tauri v2，
全部处理在本地完成，不上传任何样本或规则。本仓库面向数据安全竞赛与红队场景中的
"敏感数据快速清洗 / 解析" 需求，不依赖 Python 运行时。

> **当前状态：v0.4.2 设置模块首期（T7-1 ~ T7-4 verified_complete，已 release_complete）。** v0.4.2 在 v0.4.1 5 项缺陷修复的基础上，新增 Sidebar 底部「设置」入口与 tshark 多平台自动检测 + 路径配置：
> - **Sidebar 底部「设置」入口（T7-3）**：`Sidebar.jsx` 给 antd Menu 加 `flex:1` 占满中段，下方分隔线 + `Button block` 把「设置」顶到底部；选中态用 `type=primary`；按钮内嵌 tshark 状态 Tag（绿=已检测 / 红=未检测）一眼可见。点击 dispatch `SET_VIEW("settings")`，App.jsx 新增 `view === "settings"` 分支渲染 SettingsView。
> - **tshark 多平台自动检测（T7-1）**：core 新增 `crates/core/src/pcap/detect.rs`：进程级全局覆盖路径（`Mutex<Option<String>>`）+ `set_tshark_path` / `get_tshark_path` / `resolve_tshark_cmd` / `detect_tshark` / `candidate_paths`。`detect_tshark` 按优先级探测覆盖路径 → PATH `tshark` → 各平台候选绝对路径（macOS homebrew / Wireshark.app / Linux /usr/bin / Windows Program Files），跑 `<path> --version` 退出 0 即视为可用，返回 `TsharkInfo { path, version }`。`PcapReader::read` 两处 `Command::new("tshark")` 改为 `Command::new(&resolve_tshark_cmd())`，覆盖为 None 时行为与 v0.4.1 完全一致（零回归）。
> - **路径配置持久化（T7-2）**：Tauri 新增 3 命令：`detect_tshark` / `load_tshark_path` / `save_tshark_path`，配置写 `app_config_dir/settings.json`（`{ "tshark_path": "..." }`），读写同时调 `set_tshark_path` 注入运行时立即生效（无需重启 app）。零新依赖（复用 `tauri::Manager::path()` + `std::fs`，不引入 `tauri-plugin-store` / `tauri-plugin-fs` / `which` crate）。
> - **SettingsView UI（T7-3）**：`frontend/src/components/SettingsView.jsx` Card + Descriptions 显示状态 / 当前生效路径 / 版本；操作按钮组「自动检测 / 使用检测到的路径 / 选择文件... / 清除自定义路径」；挂载时自动调 `loadTsharkPath` + `detectTshark` 灌入状态；Alert 提示各平台常见路径参考。
> - **搜索子串匹配修正（T7 附带）**：v0.4.1 用户反馈「搜张三能搜到，搜张搜不到」——根因 `tokenize` 把中文聚成整 token，原 `search_keyword` 精确匹配 postings key 漏命中。改为子串匹配：`key.contains(term)` 即命中，搜「张」命中 `张三`/`张三丰`，ASCII 场景同样受益（搜「ali」命中 `alice`）。新增 CJK + ASCII 子串测试覆盖。
> 安全约束保持：tshark 探测/路径配置全本地，不调用网络；settings.json 仅写本地 app_config_dir；规则与样本不上传。版本状态约定见 `docs/04-版本标准.md`。

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
- **Tools 页面（界面 7）**：antd Tabs 下拉两个子工具——(a) **SQL 解析**：`parse_sqls(inputs) -> SqlParseResult { probes, aggregated, reconstructed, parsed_payloads }`，`extract_blind_probe(sql, response_body_size, source_ip)` 抽 `BlindProbe`（AsciiBinary / Equality / Length 三类），`reconstruct_database` 关联 4 类 read_target 自动还原数据库 schema/tables；非盲注 payload 走 `parse_payload` 兜底。(b) **正则解析**：`explain_regex(pattern) -> Vec<RegexTokenDesc>` 手写逐字符扫描 + 8 内置模板（v0.4.1 起改为语句→构造正则）。
- **跨视图 state 不丢**：`App.jsx` 顶层 `useReducer` 内存常驻，`SET_VIEW` 只切 view 不重置数据；搜索命中行号、脱敏结果、校验结果、SQL 解析结果均跨视图保留。
- **不外发数据**：全本地处理；规则与样本不上传（保留 v0.1.0 §6 安全约束）。

v0.4.1 5 项缺陷修复已落地：
- **T6-1 数据流打通**：`ExportView` 从读 `state.filePath`（SET_FILE 通道，v0.4.0 实现漏写）迁移到读 `state.records`（SET_RECORDS 通道，与 `PreprocessView.handleImport` 唯一写入端对齐）；`computeExportArgs` fallback 到 records 作为后端 inputPath；新增 e2e `preprocess_to_search_finds_hits`（csv → `read_records` → `search_records(Keyword "张三")` 命中行数 ≥1 + cell 含「张三」）端到端验证「预处理 → 搜索」链路，修复用户反馈「预处理导入后在搜索界面无法搜到」的根因。
- **T6-2 移除各界面 FileToolbar**：删除 `frontend/src/components/FileToolbar.jsx`（-106 行）+ `App.jsx` 移除 `NO_TOOLBAR_VIEWS` 集合与 `<FileToolbar/>` 渲染分支；导入唯一入口收敛到 `PreprocessView` 内置「选择文件」按钮（v0.4.0 设计本意）；消除各界面顶部冗余的导入按钮。
- **T6-3 ToolsView 下拉栏**：`Sidebar.jsx` 的「Tools」项由普通 Menu item 改为 antd `Menu.SubMenu`，`children=[{key:"tools.sql",label:"SQL 解析"},{key:"tools.regex",label:"正则解析"}]`，点击「Tools」标题展开/折叠（`openKeys` 受控为 `state.sidebarToolsOpen`，默认折叠）；子项点击双重 dispatch `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB(<sql|regex>)`，`selectedKeys` 在 `activeView === "tools"` 时高亮 `tools.<toolsActiveTab>`；`ToolsView.jsx` 本体不再渲染视图内 `Select` 下拉（与 Sidebar SubMenu 语义重复），仅渲染 Card + 子工具内容，Card `title` 随 `toolsActiveTab` 切换为 `Tools - SQL 解析` / `Tools - 正则解析`；用户反馈「侧边导航栏的 Tools 有下拉功能」最终落到 Sidebar SubMenu 形态。
- **T6-4 SQL 盲注特征自动跳转**：core `looks_like_blind_probe(sql: &str) -> bool` 复用 `ascii_binary_regex` / `equality_regex` / `length_regex` 三类正则，不依赖 `response_body_size`（区别于 `extract_blind_probe`，可用于预处理阶段仅 SQL 文本场景）；Tauri `detect_sql_blind_features(headers, rows) -> {detected, samples}`；GUI `PreprocessView.handleImport` 命中即自动跳转到 SqlParseTool 并预填样本；仅本地正则匹配，不外发数据。
- **T6-5 RegexTool 语句→构造正则**：移除内置模板 Tab + 新增 `ConstructTab`（antd `TextArea` 语句 → `regexConstruct(statement)` → `pattern` `Paragraph` copyable + `matched_clues` `Tag` 列表 + 测试样例高亮）；core 新增 `crates/core/src/tools/regex_construct.rs::construct_regex(statement) -> Result<ConstructedRegex, CoreError>`，规则化推断 6 类线索（位数 / 字符集 / 锚定前缀 / 邮箱 / URL / 身份证），语义优先级 邮箱 > URL > 身份证 > 通用，末尾 `Regex::new` 校验保证 pattern 可编译；10 单测全绿。用户反馈「不是要求内置模板，而是我给出一个语句，能自动化帮我构造」。

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
| v0.3.0 | pcap 流量包敏感数据提取（tshark 子进程 + HTTP 字段提取 + 双重 URL 解码 + 自动 base64 字段解码 + 敏感扫描 + PcapView 四段 GUI） | 已发布 v0.3.0 |
| v0.4.0 | 7 界面架构性完整重构（统一预处理 6 类源 + 多标签规则引擎 + 统一搜索 SearchQuery 枚举 + Tools SQL 解析/正则解析） | 已发布 v0.4.0 |
| v0.4.1 | 5 项缺陷修复：数据流打通 + 移除各界面 FileToolbar + ToolsView 下拉栏 + SQL 盲注特征自动跳转 + RegexTool 语句→构造正则 | 已发布 v0.4.1 |

版本判定标准见 `docs/04-版本标准.md`。

## 安全与隐私

- 全部数据在本地处理，不调用任何远程接口；样本与规则不上传。
- 不引入 Python 运行时依赖；核心与扩展均为 Rust 原生。
- 输出文件路径由用户在 GUI 中显式指定，工具不自行外发。
- pcap 解析（v0.3.0）依赖系统 `tshark`，仍在本机执行。

## 许可证

待定（license 待确认）。
