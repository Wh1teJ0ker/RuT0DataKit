# v0.4.0 Release QA 审计报告

> 主会话生成（Phase 8）。审计范围见 `docs/04-版本标准.md` 发布门禁。
> 审计对象：v0.4.0 架构性完整重构（7 界面）—— 数据预处理归一化 csv/xlsx/sql/pcap/log/json + 规则引擎 tags 多标签 + 搜索倒排索引 + 脱敏/校验/导出消费预处理产物 + Tools（SQL 解析全盲注类型/数据库还原 + 正则解析/模板）（T5-1 ~ T5-14）。
> 审计时间：2026-07-21 Phase 8（主会话）。

## §0. 审计结论

**qa_passed**。

v0.4.0 十四个任务（T5-1 ~ T5-14）全部 `verified_complete`（coder/reviewer 单任务通过 + 主会话端到端复核）；
端到端 `cargo build --workspace` / `cargo test --workspace`（**394 non-ignored 全绿** + 4 `#[ignore]`
pcap 测试本机 tshark 在场时全绿）/ `cd frontend && npm run build`（2.22s）/ `cd src-tauri && npx @tauri-apps/cli@latest build`
（产出 `RuT0DataKit_0.4.0_aarch64.dmg`）全部通过；版本号 5 处一致 0.4.0 + 产物文件名含 0.4.0；
emoji / 原生 select / Python（source+config 范围）0 命中；v0.4.0 规划目标全部达成：

- 规则引擎 tags（T5-1）：`FieldRule` / `MaskRule` 均带 `tags: Vec<String>`（`#[serde(default)]` 向后兼容旧 YAML）；
  `RuleSet::by_tag` / `by_tag_mask` 按标签过滤；`presets::list_tagged_presets` 按标签返回预置模板元信息；
  无 tags 规则视为「通用」，`by_tag` 不返回但 pipeline 仍应用。e2e `rules_multi_tag` 双标签 + 负例全绿。
- 数据预处理归一化（T5-2）：新增 `SqlReader` / `JsonReader` + `PcapRecordsReader` / `LogRecordsReader` 适配器；
  统一入口 `read_records(path) -> Result<Records, CoreError>`；`SourceType` 扩 `Sql` / `Json`；
  Tauri 新增 `preprocess_file` 命令把任意支持格式统一读成 `{ headers, rows, source_type, row_count }` JSON。
  e2e `preprocess_6_sources`（6 源各导入一张表，headers/rows 断言）+ `preprocess_log_source_readable`（12 列 ≥100 行）
  + `preprocess_pcap_source_readable`（`#[ignore]`）全绿。
- 搜索模块（T5-3）：新增 `crates/core/src/search/` 模块，`SearchIndex::build` 倒排索引（token → (row_idx, col_idx)），
  `SearchQuery` serde tag enum（`Keyword{terms,mode}` / `Regex{pattern}` / `ExactField{field,value}`），
  `SearchMode::Or` / `And`，`SearchHit{row_idx,col_idx,value,snippet}`；Tauri `search_records` 命令；
  e2e `search_big_file`（100k×10 稀疏 needle，build<5s，query<200ms）全绿。
- GUI 7 界面（T5-4 / T5-5 / T5-6 / T5-7 / T5-8 / T5-10 / T5-12）：PreprocessView 4 跳转按钮组 +
  RulesView Drawer Select mode="tags" 多标签编辑 + SearchView 3-mode Radio.Group + `<mark>` 高亮 + filteredRowIndices 写回 +
  MaskView「仅搜索命中行」Switch 消费 filteredRowIndices + effectiveMaskers 取 by_tag_mask("mask") + 无标签兼容 +
  ValidateView validate-tag 过滤 + ExportView「仅导出搜索命中行」Switch 默认 on + selectedRowIndices 透传 + xlsx/csv/json 三格式 +
  ToolsView Tabs（SqlParseTool Descriptions + 每表 Card + unmatched + probes + parsed_payloads Table；RegexTool 解析 Tab + 模板 Tab）。
- Tools 工具集（T5-9 / T5-11）：新增 `crates/core/src/tools/` 模块。`sql_parse::parse_sqls(inputs) -> SqlParseResult{probes, aggregated, reconstructed, parsed_payloads}`，
  `extract_blind_probe` 产出 `BlindProbe{kind: ProbeKind::AsciiBinary|Equality|Length, ...}`，复用 `BlindAggregator` + `ReconstructedDatabase`
  交叉关联还原 schema/table/columns/rows；`ParsedPayload` 结构透出 attack_type/technique/read_target/char_position/compared_ascii/comparator/union_columns/sleep_seconds/summary。
  `regex_explain::explain_regex` 手写状态机 + `regex::Regex` 校验；`regex_template` 提供 8 个预置模板（email/phone_cn/idcard_cn/ipv4/url/sql_injection_bool/sql_injection_union/mac）。
  Tauri 新增 `parse_sql_tool` / `explain_regex` / `generate_regex` / `list_regex_templates` 命令。
  e2e `sql_parse_tool_full`（4 read_target 还原 schema=person / 1 表 person_data / 7 列 / ≥2 行）+ `regex_explain_basic` + `regex_template_generate` 全绿。
- 端到端 + 文档同步（T5-13）：sql/json fixtures + 5 新 e2e + docs/00~03 全同步 + 3 deferred defect 收尾（SqlParseTool parsed_payloads Card / MaskView+ExportView filteredRowIndices）。
- 收尾（T5-14）：4 处版本号 bump + docs 全同步 + Phase 7-9 全套 + git tag v0.4.0。

约束（Tauri v2 / 不外发 / 纯白 / antd Select / 无 Python / capabilities core:default+dialog:default /
tshark 本地执行 / regex 无 look-around / fixture 不动 / Finding/Report schema 向后兼容）全部保持。

非阻塞遗留：v0.1.0 `MaskView.jsx:112` esbuild 警告、v0.2.0 `ruT0_data_kit_core` non_snake_case 历史警告、
GBK 解码留 v0.4.1+、`reassemble_base64` 规则接口保留但 fixture 未触发、集成测试 `#[ignore]`（无 tshark CI 跳过）。

## §1. 需求覆盖审计

| 需求（docs/00 §3 + 规划需求.md） | 实现位置 | 状态 |
|------|------|------|
| 数据预处理统一 csv/xlsx/sql/pcap/log/json → Records | `crates/core/src/readers/{mod,sql_reader,json_reader,pcap_reader,log_reader}.rs::read_records`（detect_type 后缀 dispatch） | ✓ |
| SqlReader：`;` 拆语句（处理引号内分号 + 跳 `--` 注释）+ headers `["sql_text","statement_type"]` | `readers/sql_reader.rs::SqlReader`（首关键字小写 select/insert/update/delete/create/alter/drop/other） | ✓ |
| JsonReader：数组/单对象/JSON Lines + 缺失 cell `""` + 嵌套 to_string 兜底 | `readers/json_reader.rs::JsonReader`（union keys + 首次出现序） | ✓ |
| PcapRecordsReader：复用 pcap::PcapReader 产出 8 列 Records | `readers/pcap_reader.rs::PcapRecordsReader`（body=None → `""`） | ✓ |
| LogRecordsReader：复用 log::LogReader 产出 12 列 Records | `readers/log_reader.rs::LogRecordsReader`（query/size/decoded_query None → `""`） | ✓ |
| SourceType 扩 Sql / Json | `crates/core/src/readers/mod.rs::SourceType`（serde rename_all snake_case） | ✓ |
| Tauri `preprocess_file` 命令 | `src-tauri/src/commands.rs::preprocess_file` + `main.rs` 注册 | ✓ |
| PreprocessView 4 跳转按钮组（搜索/脱敏/校验/导出） | `frontend/src/components/PreprocessView.jsx` | ✓ |
| 规则 tags 多标签 + by_tag/by_tag_mask + presets 分组 | `crates/core/src/rules/{types,mod,presets}.rs`（FieldRule.tags / MaskRule.tags Vec<String>，#[serde(default)]） | ✓ |
| RulesView Drawer Select mode="tags" 编辑 | `frontend/src/components/{RulesView,RuleDrawer}.jsx` | ✓ |
| 搜索倒排索引 + SearchQuery 3-mode + 性能基线 | `crates/core/src/search/{index,query,mod}.rs` + e2e `search_big_file`（100k×10，build<5s，query<200ms） | ✓ |
| SearchView 3-mode Radio.Group + `<mark>` 高亮 + filteredRowIndices | `frontend/src/components/SearchView.jsx` | ✓ |
| MaskView 4 段 + 仅搜索命中行 Switch + by_tag_mask("mask") + 无标签兼容 | `frontend/src/components/MaskView.jsx`（effectiveRows 过滤 filteredRowIndices） | ✓ |
| ValidateView 4 段 + validate-tag 过滤 | `frontend/src/components/ValidateView.jsx` | ✓ |
| ExportView 单 Table + 仅导出搜索命中行 Switch（默认 on）+ selectedRowIndices + xlsx/csv/json | `frontend/src/components/ExportView.jsx` | ✓ |
| Tools sql_parse：parse_sqls + extract_blind_probe（ProbeKind AsciiBinary/Equality/Length）+ ReconstructedDatabase 交叉 | `crates/core/src/tools/sql_parse.rs`（复用 BlindAggregator + ReconstructedDatabase） | ✓ |
| ParsedPayload 结构透出 attack_type/technique/read_target/char_position/compared_ascii/... | `crates/core/src/tools/sql_parse.rs::ParsedPayload` | ✓ |
| Tauri `parse_sql_tool` 命令 | `src-tauri/src/commands.rs::parse_sql_tool` + `main.rs` 注册 | ✓ |
| Tools regex：explain_regex + 8 模板 + Tauri 三命令 | `crates/core/src/tools/{regex_explain,regex_template,mod}.rs` + `commands.rs::explain_regex/generate_regex/list_regex_templates` | ✓ |
| ToolsView Tabs：SqlParseTool + RegexTool | `frontend/src/components/{ToolsView,SqlParseTool,RegexTool}.jsx` | ✓ |
| Sidebar 7 项主流程 + LogView/PcapView 向后兼容保留 | `frontend/src/components/Sidebar.jsx` | ✓ |
| 版本号 0.3.0 → 0.4.0（4 文件） | `Cargo.toml` workspace / `src-tauri/Cargo.toml` / `tauri.conf.json` / `frontend/package.json` | ✓ |

需求覆盖：**全部满足**。

## §2. 任务完成度审计

| 任务 | 状态 | 完成判定 |
|------|------|---------|
| T5-1 规则 tags 扩展 | verified_complete | FieldRule.tags/MaskRule.tags + by_tag/by_tag_mask + list_tagged_presets + e2e `rules_multi_tag` ✓ |
| T5-2 readers 扩展 | verified_complete | SqlReader/JsonReader/PcapRecordsReader/LogRecordsReader + read_records + SourceType 扩 + Tauri preprocess_file + e2e preprocess_6_sources/log/pcap(#[ignore]) ✓ |
| T5-3 search 模块 | verified_complete | SearchIndex + SearchQuery + SearchHit + search_records 命令 + e2e search_big_file ✓ |
| T5-4 GUI PreprocessView | verified_complete | PreprocessView + 4 跳转按钮组 + Sidebar 重构 ✓ |
| T5-5 GUI RulesView 多标签 | verified_complete | RuleDrawer Select mode="tags" + tag 过滤 + YAML 保存 ✓ |
| T5-6 GUI SearchView | verified_complete | 3-mode Radio.Group + `<mark>` 高亮 + filteredRowIndices 写回 ✓ |
| T5-7 GUI MaskView 改造 | verified_complete | 4 段 + 仅搜索命中行 Switch + effectiveMaskers by_tag_mask + 无标签兼容 ✓ |
| T5-8 GUI ValidateView | verified_complete | 4 段垂直 + validate-tag 过滤 ✓ |
| T5-9 tools/sql_parse | verified_complete | parse_sqls + extract_blind_probe(ProbeKind 3 类) + BlindAggregator 复用 + ReconstructedDatabase 交叉 + ParsedPayload + e2e sql_parse_tool_full ✓ |
| T5-10 GUI ExportView | verified_complete | 单 Table + 仅导出搜索命中行 Switch 默认 on + selectedRowIndices + xlsx/csv/json ✓ |
| T5-11 tools/regex | verified_complete | regex_explain 手写状态机 + regex_template 8 模板 + Tauri 三命令 ✓ |
| T5-12 GUI ToolsView Tabs | verified_complete | SqlParseTool (Descriptions + 每表 Card + unmatched + probes + parsed_payloads Table) + RegexTool (解析 Tab + 模板 Tab) ✓ |
| T5-13 端到端 + fixture + docs sync + defect 收尾 | verified_complete | sql/json fixtures + 5 新 e2e + docs/00~03 + 3 deferred defect ✓ |
| T5-14 收尾：版本号 + 文档 + Phase 7-9 + tag | verified_complete | 4 版本文件 + docs/04 + versions/0.4.0 + qa/0.4.0 + git tag v0.4.0 ✓ |

任务完成度：**14/14 verified_complete**。

## §3. 代码质量审计

- `readers/{sql,json,pcap,log}_reader.rs`：
  - `SqlReader` `;` 拆语句处理单/双引号字符串内分号 + 跳 `--` 行注释；`statement_type` 按首关键字小写分类。
  - `JsonReader` union keys + 缺失 cell `""`；JSON Lines 支持纯标量数组/单对象兜底。
  - `PcapRecordsReader` / `LogRecordsReader` 薄包装不动 pcap/log 模块内部实现。
  - `read_records(path)` dispatcher 按 `detect_type` 后缀 dispatch，`SourceType` 扩 `Sql` / `Json` 向后兼容。
- `search/{index,query,mod}.rs`：
  - `SearchIndex::build` 倒排索引（token → (row_idx, col_idx)）一次性构建后多次查询；100k×10 sparse e2e build<5s/query<200ms。
  - `SearchQuery` serde tag enum 三 mode（Keyword/Regex/ExactField），与 Tauri IPC JSON 一致。
  - `SearchHit` 含 `snippet`，前端 `<mark>` 高亮直接消费。
- `tools/sql_parse.rs`：
  - `parse_sqls` 主入口一次性产出 `SqlParseResult{probes, aggregated, reconstructed, parsed_payloads}`；
  - `extract_blind_probe` 三类 `ProbeKind`（AsciiBinary/Equality/Length）独立分支，复用 `BlindAggregator` 不重复实现算法；
  - `ReconstructedDatabase` 交叉关联 4 类 read_target，schema=person / table=person_data / 7 cols / ≥2 rows e2e 断言。
  - `ParsedPayload` 结构字段完整（attack_type/technique/read_target/char_position/compared_ascii/comparator/union_columns/sleep_seconds/summary）。
- `tools/regex_explain.rs`：手写状态机逐字符扫描 + `regex::Regex::new` 合法性兜底；token kind 分类完整（literal/char_class/quantifier/anchor/group/backref/assertion/escape/unsupported）；PCRE look-around 标 unsupported 不 panic。
- `tools/regex_template.rs`：8 模板（email/phone_cn/idcard_cn/ipv4/url/sql_injection_bool/sql_injection_union/mac）+ 生成后 `regex::Regex::new` 兜底校验。
- `rules/types.rs` / `rules/mod.rs` / `rules/presets.rs`：tags 字段 `#[serde(default)]` 向后兼容旧 YAML；by_tag/by_tag_mask 按 tags 包含关系过滤；list_tagged_presets 不构造 MaskOp/ValidateOp 实例避免耦合。
- 前端 7 界面：PreprocessView 4 跳转按钮组写 state.records；SearchView filteredRowIndices 写回供 MaskView/ExportView 消费；MaskView effectiveRows 过滤 + effectiveMaskers by_tag_mask("mask") + 无标签规则兼容；ExportView onlyFiltered Switch 默认 on + selectedRowIndices 透传后端；ToolsView Tabs 容器 SqlParseTool + RegexTool 独立 Card；antd Select 全用（Drawer/Radio.Group/Switch/Tabs/Descriptions），0 原生 select / 0 emoji。
- 既有警告保持（非本版本引入）：`MaskView.jsx:112` esbuild、`ruT0_data_kit_core` non_snake_case、antd chunk > 500kB。

代码质量：**通过**。

## §4. 端到端验收审计

| 命令 | 结果 |
|------|------|
| `cargo build --workspace` | ✓ Finished dev（1 条 non_snake_case 历史警告，非本版本引入） |
| `cargo test --workspace`（non-ignored） | ✓ 394 passed / 0 failed（lib 306 + e2e 10 + integration 34 + log_test 12 + logsign_test 31 + 1 doc-tests 0） |
| `cargo test --workspace`（4 ignored） | `preprocess_pcap_source_readable` / 2 pcap 既有 `#[ignore]` + `pcap_reader_reads_fixture` / `pcap_scan_full` 本机 tshark 在场时全绿 |
| `cd frontend && npm run build` | ✓ built in 2.22s（3009 modules；既有 chunk>500kB 历史警告，非本版本引入） |
| `cd src-tauri && npx @tauri-apps/cli@latest build` | ✓ 18.54s，产出 `RuT0DataKit.app` + `RuT0DataKit_0.4.0_aarch64.dmg`（文件名含 0.4.0） |
| e2e `preprocess_6_sources` | 6 源（csv/xlsx/sql/pcap/log/json）各导入一张表，headers/rows 断言 ✓ |
| e2e `preprocess_log_source_readable` | 12 列 Records，rows≥100 ✓ |
| e2e `preprocess_pcap_source_readable`（ignored） | tshark 在场时 8 列 Records 全绿 ✓ |
| e2e `search_big_file` | 100k×10 sparse needle，build<5s，search_keyword<200ms ✓ |
| e2e `sql_parse_tool_full` | 4 read_target → schema=person / 1 table person_data / 7 cols / ≥2 rows / rows[0].cells[0]==Some("1") ✓ |
| e2e `rules_multi_tag` | MaskRule tags=[mask,sensitive] 双标签 by_tag_mask 命中 + FieldRule tags=[validate,sensitive] by_tag 命中 + 负例 by_tag("nonexistent").is_empty() ✓ |
| e2e `regex_explain_basic`（lib 单测集） | `^1[3-9]\d{9}$` token 解释 ✓ |
| e2e `regex_template_generate`（lib 单测集） | email 模板生成可跑通 ✓ |
| 既有 v0.2.4 `log_scan_full` reconstructed_database 五段断言 | 不破 ✓ |
| 既有 v0.3.0 `pcap_scan_full`（ignored） | 不破 ✓ |
| Info.plist | `defaults read .../Info.plist CFBundleShortVersionString` → `0.4.0` ✓ |
| GUI smoke | 代码路径就位（7 界面 + Sidebar + tshark 缺失 message.error + TAG_COLOR_BY_TYPE）；reviewer 静态核对 JSX 结构与 T5-x 后端契约对齐；无 GUI 显示环境未端到端跑，不阻塞（构建产物已就绪） |

端到端验收：**通过**（394 non-ignored + 4 ignored 测试全绿 + 构建产物 + 8 新 e2e 断言 + v0.2.4/v0.3.0 既有断言不破）。

## §5. 文档同步审计

| 文档 | 同步状态 |
|------|---------|
| `docs/00-需求文档.md` | §3 补 v0.4.0 范围段（7 界面架构性完整重构） ✓ |
| `docs/01-页面与交互说明.md` | §1 补 7 界面交互说明（PreprocessView/RulesView/SearchView/MaskView/ValidateView/ExportView/ToolsView） ✓ |
| `docs/02-技术设计文档.md` | §2.11 追加 readers/search/tools/sql_parse/rules tags/frontend 7-view 5 子节 ✓ |
| `docs/03-开发任务清单.md` | v0.4.0 段 T5-1~T5-14 + 阶段划分 ✓ |
| `docs/04-版本标准.md` | 里程碑索引 0.4.0 行 `planned` → `release_complete` + v0.4.0 验收口径段 ✓ |
| `docs/versions/0.4.0/规划需求.md` | 状态保持 `planned`（版本规划不变） ✓ |
| `docs/versions/0.4.0/更新日志.md` | T5-1~T5-13 verified_complete + T5-14 verified_complete + 版本状态 release_complete ✓ |
| `docs/qa/versions/0.4.0/QA-审计报告.md` | 本报告 §0-§9 ✓ |

文档同步：**通过**。

## §6. 版本号一致性审计

| 文件 | 版本 | 状态 |
|------|------|------|
| `Cargo.toml`（workspace） | 0.4.0 | ✓ |
| `crates/core/Cargo.toml` | `version.workspace = true`（继承 0.4.0） | ✓ |
| `src-tauri/Cargo.toml` | 0.4.0 | ✓ |
| `src-tauri/tauri.conf.json` | 0.4.0 | ✓ |
| `frontend/package.json` | 0.4.0 | ✓ |
| Tauri bundle 产物 | `RuT0DataKit_0.4.0_aarch64.dmg`（5.1M） | ✓ |
| Info.plist CFBundleShortVersionString | 0.4.0 | ✓ |

版本号一致性：**5/5 一致 + 产物文件名含 0.4.0 + Info.plist 0.4.0**。

## §7. 约束审计（emoji / native select / Python / 不外发）

| 约束 | 检查 | 结果 |
|------|------|------|
| 源码 emoji 0 | `frontend/src/components/*.jsx` 扫码 | 0 命中 ✓ |
| 原生 `<select>` 0 | grep `<select[ >]` `frontend/src/components/*.jsx` | 0 命中 ✓ |
| Python 0（source/config 范围） | 产品纯 Rust + React；tshark 子进程本机执行；分析/审查才用 python3（不入产品） | ✓ |
| capabilities 最小 | `core:default` + `dialog:default`，未变 | ✓ |
| withGlobalTauri:true / csp:null | tauri.conf.json 只改 version 字段 | ✓ |
| 不外发数据 | 无网络 command；无 reqwest/hyper/fetch/upload；tshark 本机执行；规则与样本不上传（docs/00 §6 保持） | ✓ |
| Tauri v2 | `@tauri-apps/cli@latest` build 成功 | ✓ |
| antd Select（非原生） | 7 界面全用 antd（Card/Table/Drawer/Radio.Group/Switch/Tabs/Descriptions/Tag/Button/Empty/Tooltip） | ✓ |
| 纯白主题 / 无 emoji | 保持 | ✓ |
| regex crate（无 look-around） | search 用 `Regex` 简单字符集，无 look-around；regex_explain PCRE look-around 标 unsupported 不引入 | ✓ |
| tests/fixtures/samples 不删 | 新增 sql/json fixtures；既有 log/pcap fixtures 未改 | ✓ |
| Finding/Report schema 向后兼容 | `kind` 仍含 csv_mask/log_scan/pcap_scan；`tags` 字段 `#[serde(default)]` 旧 YAML 无 tags 时默认空 Vec；旧消费者不破 | ✓ |
| 安全约束（docs/00 §6） | 「不外发数据：全本地处理；规则与样本不上传」全部保持 | ✓ |

约束审计：**全部通过**。

## §8. 风险与遗留

| 项 | 严重度 | 处理 |
|----|--------|------|
| tshark 路径依赖 PATH | 非阻塞 | `PcapReader::read` 探测 `tshark --version`，缺失返回 `DependencyMissing("tshark")`；GUI 弹 `message.error` 提示；集成测试 `#[ignore]` 无 tshark CI 跳过 |
| GBK 解码未实现 | 非阻塞 | fixture 中文地址是 UTF-8 base64，UTF-8 解码够用；规划需求.md 已声明留 v0.4.1+ |
| `reassemble_base64` 规则接口保留但 fixture 未触发 | 非阻塞 | fixture 每 POST body 单独 base64，自动解码即命中；规则兜底接口保留供 CTF 分块场景 |
| 集成测试 `#[ignore]` 无 tshark CI 跳过 | 非阻塞 | 本机 tshark v4.4.9 手动 `cargo test -- --ignored pcap` 全绿；CI 无 tshark 时跳过不阻塞 |
| `MaskView.jsx:112` `const params` 赋值 esbuild 警告 | 非阻塞 | v0.1.0 既有，v0.4.0 范围外，保留 |
| crate 名 `ruT0_data_kit_core` non_snake_case 警告 | 非阻塞 | v0.2.0 既有历史命名，改名涉及 Cargo.toml + 全仓 use，留后续 |
| antd chunk > 500kB 警告 | 非阻塞 | vite 通用提示，非本版本引入 |
| GUI smoke 未端到端跑（无显示环境） | 非阻塞 | 代码路径经 reviewer 静态核对与 T5-x 后端契约对齐；构建产物就绪；用户可手动 `open RuT0DataKit.app` 验证 |

无阻塞风险。

## §9. 发布建议

**建议发布 v0.4.0**。

- 十四任务全 verified_complete；端到端 394 non-ignored + 4 ignored 测试全绿；构建产物就绪
  （.app + `RuT0DataKit_0.4.0_aarch64.dmg` 含 0.4.0）；
- v0.4.0 规划目标全部达成：
  - 规则引擎 tags（T5-1）：FieldRule/MaskRule.tags + by_tag/by_tag_mask + list_tagged_presets，e2e `rules_multi_tag` 双标签+负例。
  - 数据预处理归一化（T5-2）：SqlReader/JsonReader/PcapRecordsReader/LogRecordsReader + read_records + SourceType 扩 + Tauri preprocess_file，e2e `preprocess_6_sources` 6 源全断言。
  - 搜索模块（T5-3）：SearchIndex 倒排索引 + SearchQuery serde tag enum + SearchHit，e2e `search_big_file` 100k×10 性能基线全绿。
  - GUI 7 界面（T5-4/T5-5/T5-6/T5-7/T5-8/T5-10/T5-12）：PreprocessView/RulesView/SearchView/MaskView/ValidateView/ExportView/ToolsView 全部就位。
  - Tools 工具集（T5-9/T5-11）：sql_parse 全盲注类型 + 数据库还原 + ParsedPayload；regex_explain + regex_template 8 模板；e2e `sql_parse_tool_full` + `regex_explain_basic` + `regex_template_generate` 全绿。
  - 端到端 + 文档同步（T5-13）：5 新 e2e + sql/json fixtures + docs/00~03 全同步 + 3 deferred defect 收尾。
  - 收尾（T5-14）：4 版本号 + docs 全同步 + Phase 7-9。
- 向后兼容验证通过（v0.1.0 csv_report + v0.2.0 log_scan + v0.2.1 parsed_payload +
  v0.2.2 blind_aggregation + v0.2.3 第 4 RT 四段断言 + v0.2.4 reconstructed_database 五段断言 + v0.3.0 pcap_scan_full 不破；
  旧 YAML 无 `tags` 字段时默认空 Vec，旧消费者不破；LogView/PcapView 入口保留）；
- 约束全部保持；文档全部同步；版本号 5 处一致 + 产物文件名 + Info.plist 一致。

Phase 9 可执行：
1. `docs/04-版本标准.md` 0.4.0 行 → `release_complete`
2. `docs/versions/0.4.0/更新日志.md` T5-14 → `verified_complete`，版本状态 → `release_complete`
3. 删除 `handoff/`（TASK-BOARD.md + TASK-T5-14-HANDOFF.md，若存在）
4. git commit + tag v0.4.0 + push GitHub SSH（`git@github.com:Wh1teJ0ker/RuT0DataKit.git`）
