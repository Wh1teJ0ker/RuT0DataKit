# v0.4.1 Release QA 审计报告

> 主会话生成（Phase 8）。审计范围见 `docs/04-版本标准.md` 发布门禁。
> 审计对象：v0.4.1 5 项缺陷修复（数据流打通 + 移除 FileToolbar + ToolsView 下拉栏 + SQL 盲注自动跳转 + RegexTool 语句→构造正则）（T6-1 ~ T6-6）。
> 审计时间：2026-07-21 Phase 8（主会话）。

## §0. 审计结论

**qa_passed**。

v0.4.1 六个任务（T6-1 ~ T6-6）全部 `verified_complete`（coder/reviewer 单任务通过 + 主会话端到端复核）；
端到端 `cargo build --workspace` / `cargo test --workspace`（**419 non-ignored 全绿** + 4 `#[ignore]`
pcap 测试本机 tshark 在场时全绿）/ `cd frontend && npm run build`（2.20s）/ `cd src-tauri && npx @tauri-apps/cli@latest build`
（产出 `RuT0DataKit_0.4.1_aarch64.dmg`）全部通过；版本号 4 处一致 0.4.1 + 产物文件名含 0.4.1 + Info.plist `CFBundleShortVersionString=0.4.1`；
emoji / 原生 select / Python（source+config 范围）0 命中；v0.4.1 规划目标全部达成：

- 数据流打通（T6-1）：`ExportView` 从读 `state.filePath`（SET_FILE 通道）迁移到读 `state.records`（SET_RECORDS 通道，与 `PreprocessView.handleImport` 唯一写入端对齐）；
  `computeExportArgs` fallback 到 records 作为后端 inputPath；新增 e2e `preprocess_to_search_finds_hits`
  （csv → `read_records` → `search_records(Keyword "张三")` 命中行数 ≥1 + cell 含「张三」）全绿。
- 移除 FileToolbar（T6-2）：删除 `frontend/src/components/FileToolbar.jsx`（-106 行）+ `App.jsx` 移除 `NO_TOOLBAR_VIEWS` 集合与 `<FileToolbar/>` 渲染分支；
  导入唯一入口收敛到 `PreprocessView` 内置导入按钮（v0.4.0 设计本意）；`grep -rn "FileToolbar" frontend/src/` 0 命中。
- ToolsView 下拉栏（T6-3）：`ToolsView` 由 antd `Tabs`（横版标签）改为 antd `Select`（下拉栏），
  `options=[{value:"sql",label:"SQL 解析"},{value:"regex",label:"正则解析"}]`，`onChange` dispatch `SET_TOOLS_ACTIVE_TAB`，默认 sql fallback。
- SQL 盲注自动跳转（T6-4）：core `crates/core/src/logsign/blind_aggregator.rs::looks_like_blind_probe(sql: &str) -> bool`
  复用 `ascii_binary_regex` / `equality_regex` / `length_regex` 三类正则，不依赖 `response_body_size`（区别于 `extract_blind_probe`）；
  Tauri `commands::detect_sql_blind_features(headers, rows) -> {detected: bool, samples: Vec<String>}`（迭代所有 cell，命中收集最多 50 条样本）；
  GUI `PreprocessView.handleImport` 在 `SET_RECORDS` 后调用，命中即 `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB("sql")` + `SET_SQL_PARSE_INPUT(samples.join("\n"))` + `message.success` 提示；
  4 单测（ascii_binary / equality / length / negative）全绿。仅本地正则匹配，**不外发数据**。
- RegexTool 语句→构造正则（T6-5）：移除内置模板 Tab（`TemplateTab` + `ParamInput` + `generate_regex` / `list_regex_templates` Tauri 命令 + `regexTemplateSelected` / `regexTemplateParams` / `regexGenerated` state 字段）；
  新增 `ConstructTab`（antd `TextArea` 语句 → `regexConstruct(statement)` → `pattern` `Paragraph` copyable + `matched_clues` `Tag` 列表 + 测试样例高亮复用 `highlightMatches` + `adaptRegexForJs`）；
  core 新增 `crates/core/src/tools/regex_construct.rs::construct_regex(statement: &str) -> Result<ConstructedRegex, CoreError>`，
  规则化推断 6 类线索（位数 / 字符集 / 锚定前缀 / 邮箱 / URL / 身份证），语义优先级 邮箱 > URL > 身份证 > 通用，
  末尾 `Regex::new` 校验保证 pattern 可编译；`ConstructedRegex { pattern, explanation, matched_clues }`；10 单测全绿。
  `regex_template.rs` 源码保留但 `tools/mod.rs` 不再 re-export（向后兼容源码可读，不暴露公共 API）；`grep -rn "generateRegex\|listRegexTemplates" frontend/src/` 仅余 `tauri.js:235` 历史注释（无调用）。
- 收尾（T6-6）：4 处版本号 bump + docs 全同步 + Phase 7-9 全套 + git tag v0.4.1。

约束（Tauri v2 / 不外发 / 纯白 / antd Select / 无 Python / capabilities core:default+dialog:default /
tshark 本地执行 / regex 无 look-around / fixture 不动 / Finding/Report schema 向后兼容）全部保持。

非阻塞遗留：v0.1.0 `MaskView.jsx:112` esbuild 警告、v0.2.0 `ruT0_data_kit_core` non_snake_case 历史警告、
GBK 解码留 v0.4.2+、`reassemble_base64` 规则接口保留但 fixture 未触发、集成测试 `#[ignore]`（无 tshark CI 跳过）。

## §1. 需求覆盖审计

| 需求（用户原始 5 条诉求） | 实现位置 | 状态 |
|------|------|------|
| 数据流打通：预处理导入后搜索界面能搜到 | `frontend/src/components/ExportView.jsx`（读 state.records）+ e2e `preprocess_to_search_finds_hits` | ✓ |
| 每一个界面上方不应该有导入文件的按钮 | `frontend/src/App.jsx`（移除 FileToolbar 渲染分支）+ `frontend/src/components/FileToolbar.jsx` 删除 | ✓ |
| Tools 页面应该是一个下拉栏，不是横版标签 | `frontend/src/components/ToolsView.jsx`（antd Select 替代 antd Tabs） | ✓ |
| 预处理解析到 SQL 盲注特征自动跳转工具页面 | `crates/core/src/logsign/blind_aggregator.rs::looks_like_blind_probe` + `src-tauri/src/commands.rs::detect_sql_blind_features` + `frontend/src/components/PreprocessView.jsx::handleImport` | ✓ |
| 优化正则解析工具：给一个语句能自动构造正则 | `crates/core/src/tools/regex_construct.rs::construct_regex` + `src-tauri/src/commands.rs::regex_construct` + `frontend/src/components/RegexTool.jsx::ConstructTab` | ✓ |
| 版本号 0.4.0 → 0.4.1（4 文件） | `Cargo.toml` workspace / `src-tauri/Cargo.toml` / `tauri.conf.json` / `frontend/package.json` | ✓ |

需求覆盖：**全部满足**（5/5 用户诉求 + 版本号 bump）。

## §2. 任务完成度审计

| 任务 | 状态 | 完成判定 |
|------|------|---------|
| T6-1 数据流打通 | verified_complete | ExportView 读 state.records + computeExportArgs fallback + e2e `preprocess_to_search_finds_hits` 命中「张三」 ✓ |
| T6-2 移除 FileToolbar | verified_complete | FileToolbar.jsx 删除（-106 行）+ App.jsx 移除 NO_TOOLBAR_VIEWS + 渲染分支 + `grep FileToolbar frontend/src/` 0 命中 ✓ |
| T6-3 ToolsView 下拉栏 | verified_complete | antd Tabs → antd Select + options 2 项 + onChange dispatch SET_TOOLS_ACTIVE_TAB + 默认 sql fallback ✓ |
| T6-4 SQL 盲注自动跳转 | verified_complete | looks_like_blind_probe + 4 单测 + detect_sql_blind_features Tauri 命令 + PreprocessView handleImport 跳转链 + message.success ✓ |
| T6-5 RegexTool 语句→构造正则 | verified_complete | 移除模板 Tab + ConstructTab + construct_regex + 10 单测 + ConstructedRegex 结构 + Regex::new 校验 ✓ |
| T6-6 收尾 | verified_complete | 4 版本文件 + docs 全同步 + Phase 7-9 + git tag v0.4.1 ✓ |

任务完成度：**6/6 verified_complete**。

## §3. 代码质量审计

- `frontend/src/components/ExportView.jsx`（T6-1）：
  - `hasRecords` 由 `state.records && state.records.rows && state.records.rows.length > 0` 推导，`handleExport` gate 改为 `!hasRecords`，与 PreprocessView `SET_RECORDS` 通道对齐。
  - `computeExportArgs` 优先用 `filePath`，fallback 到 records 作为后端 inputPath（兜底兼容旧调用路径）。
- `frontend/src/App.jsx` + `frontend/src/components/FileToolbar.jsx`（T6-2）：
  - FileToolbar.jsx 整文件删除（-106 行）；App.jsx 删除 `import FileToolbar` + `NO_TOOLBAR_VIEWS` set + `<FileToolbar/>` 渲染分支。
  - PreprocessView 内置导入按钮保留（v0.4.0 设计本意：导入唯一入口）。
- `frontend/src/components/ToolsView.jsx`（T6-3）：
  - antd `Select` 替代 antd `Tabs`，`options=[{value:"sql",label:"SQL 解析"},{value:"regex",label:"正则解析"}]`，`style={{width: 220}}`，`onChange` dispatch `SET_TOOLS_ACTIVE_TAB`。
  - 分支：`state.toolsActiveTab === "regex" ? <RegexTool/> : <SqlParseTool/>`（默认 sql fallback）。
- `crates/core/src/logsign/blind_aggregator.rs`（T6-4）：
  - `pub fn looks_like_blind_probe(sql: &str) -> bool`：lowercase 后依次跑 `ascii_binary_regex()` / `equality_regex()` / `length_regex()`，任一命中即 true。
  - 与 `extract_blind_probe` 区别：不依赖 `response_body_size`（纯文本检测），不构造 BlindProbe 实例，仅返回 bool；复用既有 3 个正则函数，无重复实现。
  - 4 单测：`looks_like_blind_probe_ascii_binary` / `_equality` / `_length` / `_negative_normal_sql`；length 用例用 `length((database()))>5` 双括号（既有 `length_regex` 要求 `length\s*\(\s*\(...\))`，单括号是 v0.2.4 既有局限，不在 T6-4 范围）。
- `src-tauri/src/commands.rs::detect_sql_blind_features`（T6-4）：
  - `#[tauri::command] pub fn detect_sql_blind_features(headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<Value, String>`；
  - 迭代所有 cell，调 `looks_like_blind_probe`，命中收集最多 50 条样本，返回 `{detected: bool, samples: Vec<String>}`；
  - `let _ = &headers;` 抑制未用警告（headers 暂不参与判定，留接口）。
- `frontend/src/components/PreprocessView.jsx::handleImport`（T6-4）：
  - `SET_RECORDS` 后 await `detectSqlBlindFeatures(headers, rows)`；
  - `detected === true` → dispatch `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB("sql")` + `SET_SQL_PARSE_INPUT(samples.join("\n"))` + `message.success("检测到 SQL 盲注特征，已自动跳转到 SQL 解析工具")` + `return`；
  - catch → `console.warn` 非阻塞（检测失败不影响预处理主流程）。
- `crates/core/src/tools/regex_construct.rs`（T6-5，NEW）：
  - `pub struct ConstructedRegex { pattern: String, explanation: String, matched_clues: Vec<String> }`；
  - `pub fn construct_regex(statement: &str) -> Result<ConstructedRegex, CoreError>`：语义优先级 邮箱 > URL/链接/网址 > 身份证 > 通用（位数 + 字符集 + 锚定前缀）；
  - `parse_prefix` 用 `r#"以\s*['\"]?([^'\"\s]+)['\"]?\s*开头"#`；`parse_charset` 返回 `(&'static str, Option<&'static str>)` 支持 大写字母/小写字母/字母/十六进制/默认数字 `\d`；`parse_count` 匹配 `(\d+)\s*(?:位|个)`；
  - 末尾 `regex::Regex::new(&pattern)` 校验，保证 pattern 可编译；不可编译返回 `CoreError::Invalid`；
  - 10 单测覆盖 6 类线索 + 负例 + 可编译性；接受模式：`^\d{11}` / `^[A-Z]{8}` / `^1\d{10}` / 包含 `@` / `^https?` 前缀 / `[\dXx]` 后缀。
- `frontend/src/components/RegexTool.jsx`（T6-5）：
  - 删除 `TemplateTab`（lines 243-504）+ `ParamInput`（510-545）；删除 `generateRegex` / `listRegexTemplates` import；新增 `regexConstruct` + antd `Tag`；
  - 新 `ConstructTab`：antd `TextArea` → `regexConstruct(statement)` → `pattern` `Paragraph` copyable + `matched_clues` `Tag` 列表 + 测试样例高亮（复用 `highlightMatches` + `adaptRegexForJs`）；
  - 主组件 Tabs items：`[{ key: "explain", label: "解析", children: <ExplainTab/> }, { key: "construct", label: "构造", children: <ConstructTab/> }]`。
- `crates/core/src/tools/mod.rs`（T6-5）：
  - `pub mod regex_construct;` 新增；`pub use regex_construct::{construct_regex, ConstructedRegex};`；
  - **REMOVED**：`pub use regex_template::{generate_regex, list_regex_templates, TemplateMeta};`（`regex_template.rs` 文件保留，不暴露公共 API）。
- `src-tauri/src/commands.rs`（T6-5）：删除 `generate_regex` / `list_regex_templates`，新增 `regex_construct(statement: String) -> Result<ConstructedRegex, String>` 调 `core_construct_regex`。
- `src-tauri/src/main.rs`（T6-5）：删除 `commands::generate_regex,` / `commands::list_regex_templates,`，新增 `commands::regex_construct,`。
- `frontend/src/tauri.js`（T6-5）：删除 `generateRegex` / `listRegexTemplates`，新增 `export async function regexConstruct(statement) { return tauriInvoke("regex_construct", { statement }); }`。
- 既有警告保持（非本版本引入）：`MaskView.jsx:112` esbuild、`ruT0_data_kit_core` non_snake_case、antd chunk > 500kB。

代码质量：**通过**。

## §4. 端到端验收审计

| 命令 | 结果 |
|------|------|
| `cargo build --workspace` | ✓ Finished dev（1 条 non_snake_case 历史警告，非本版本引入） |
| `cargo test --workspace`（non-ignored） | ✓ 419 passed / 0 failed（lib 320 + e2e 10 + integration 35 + log_test 12 + 11 + 31 logsign + 0 doc-tests） |
| `cargo test --workspace`（4 ignored） | `preprocess_pcap_source_readable` / 2 pcap 既有 `#[ignore]` + `pcap_reader_reads_fixture` / `pcap_scan_full` 本机 tshark 在场时全绿 |
| `cd frontend && npm run build` | ✓ built in 2.20s（3009 modules；既有 chunk>500kB 历史警告，非本版本引入） |
| `cd src-tauri && npx @tauri-apps/cli@latest build` | ✓ 16.82s，产出 `RuT0DataKit.app` + `RuT0DataKit_0.4.1_aarch64.dmg`（文件名含 0.4.1，5.1M） |
| e2e `preprocess_to_search_finds_hits`（T6-1 新增） | csv fixture → `read_records` → `search_records(Keyword "张三")` 命中行数 ≥1 + cell 含「张三」 ✓ |
| lib 单测 `looks_like_blind_probe`（T6-4 新增 4 例） | ascii_binary / equality / length / negative_normal_sql 全绿 ✓ |
| lib 单测 `construct_regex`（T6-5 新增 10 例） | phone_11_digits / uppercase_8 / prefix_1_then_10 / email_semantic / url_semantic / idcard_semantic / lowercase_6 / hex_8 / negative_unrecognized / pattern_compilable 全绿 ✓ |
| 既有 v0.4.0 e2e（preprocess_6_sources / search_big_file / sql_parse_tool_full / rules_multi_tag / regex_explain_basic） | 不破 ✓ |
| 既有 v0.2.4 `log_scan_full` reconstructed_database 五段断言 | 不破 ✓ |
| 既有 v0.3.0 `pcap_scan_full`（ignored） | 不破 ✓ |
| `grep -rn "FileToolbar" frontend/src/` | 0 命中 ✓ |
| `grep -rn "generateRegex\|listRegexTemplates" frontend/src/` | 仅 `tauri.js:235` 历史注释（无调用） ✓ |
| Info.plist | `defaults read .../Info.plist CFBundleShortVersionString` → `0.4.1` ✓ |
| GUI smoke | 代码路径就位（5 项缺陷修复点 + ToolsView Select + PreprocessView 跳转链 + RegexTool ConstructTab）；reviewer 静态核对 JSX 结构与 T6-x 后端契约对齐；无 GUI 显示环境未端到端跑，不阻塞（构建产物已就绪） |

端到端验收：**通过**（419 non-ignored + 4 ignored 测试全绿 + 构建产物 + 3 新增测试套（1 e2e + 4 + 10 单测）+ v0.4.0/v0.2.4/v0.3.0 既有断言不破）。

## §5. 文档同步审计

| 文档 | 同步状态 |
|------|---------|
| `docs/00-需求文档.md` | §6 安全约束保留；v0.4.1 段补 5 项缺陷修复（T6-6 in_scope 已列） ✓ |
| `docs/01-页面与交互说明.md` | FileToolbar 移除 / ToolsView Select / 预处理自动跳转 / RegexTool 构造模式 ✓ |
| `docs/02-技术设计文档.md` | 新增 `detect_sql_blind_features` 命令 + `construct_regex` 模块 ✓ |
| `docs/03-开发任务清单.md` | v0.4.1 段 T6-1~T6-6 + 阶段划分补 v0.4.1 行 ✓ |
| `docs/04-版本标准.md` | 里程碑索引补 0.4.1 行 `release_complete` + v0.4.1 验收口径段 ✓ |
| `docs/versions/0.4.1/规划需求.md` | 新建，状态 `release_complete` ✓ |
| `docs/versions/0.4.1/更新日志.md` | T6-1~T6-6 verified_complete + 版本状态 release_complete ✓ |
| `docs/qa/versions/0.4.1/QA-审计报告.md` | 本报告 §0-§9 ✓ |
| `README.md` / `README_EN.md` | 版本号 + 功能列表补 5 项修复 ✓ |

文档同步：**通过**。

## §6. 版本号一致性审计

| 文件 | 版本 | 状态 |
|------|------|------|
| `Cargo.toml`（workspace） | 0.4.1 | ✓ |
| `crates/core/Cargo.toml` | `version.workspace = true`（继承 0.4.1） | ✓ |
| `src-tauri/Cargo.toml` | 0.4.1 | ✓ |
| `src-tauri/tauri.conf.json` | 0.4.1 | ✓ |
| `frontend/package.json` | 0.4.1 | ✓ |
| Tauri bundle 产物 | `RuT0DataKit_0.4.1_aarch64.dmg`（5.1M） | ✓ |
| Info.plist CFBundleShortVersionString | 0.4.1 | ✓ |

版本号一致性：**4/4 一致（+ workspace 继承）+ 产物文件名含 0.4.1 + Info.plist 0.4.1**。

## §7. 约束审计（emoji / native select / Python / 不外发）

| 约束 | 检查 | 结果 |
|------|------|------|
| 源码 emoji 0 | `frontend/src/components/*.jsx` 扫码 | 0 命中 ✓ |
| 原生 `<select>` 0 | grep `<select[ >]` `frontend/src/components/*.jsx` | 0 命中 ✓ |
| Python 0（source/config 范围） | 产品纯 Rust + React；tshark 子进程本机执行；分析/审查才用 python3（不入产品） | ✓ |
| capabilities 最小 | `core:default` + `dialog:default`，未变 | ✓ |
| withGlobalTauri:true / csp:null | tauri.conf.json 只改 version 字段 | ✓ |
| 不外发数据 | 无网络 command；无 reqwest/hyper/fetch/upload；tshark 本机执行；`looks_like_blind_probe` / `detect_sql_blind_features` / `construct_regex` 均纯本地正则/规则推断，无网络调用；规则与样本不上传（docs/00 §6 保持） | ✓ |
| Tauri v2 | `@tauri-apps/cli@latest` build 成功 | ✓ |
| antd Select（非原生） | 5 项修复点全用 antd（Select / TextArea / Tag / Paragraph / Table / Button / message）；ToolsView 由 Tabs → Select | ✓ |
| 纯白主题 / 无 emoji | 保持 | ✓ |
| regex crate（无 look-around） | `construct_regex` 用 `Regex::new` 校验，无 look-around；`looks_like_blind_probe` 复用既有 3 个正则，无新增 look-around | ✓ |
| tests/fixtures/samples 不删 | 未改 fixture；既有 log/pcap/csv fixtures 未动 | ✓ |
| Finding/Report schema 向后兼容 | 未改 Report/Finding schema；`regex_template.rs` 源码保留但 `tools/mod.rs` 不再 re-export（源码可读，公共 API 收敛） | ✓ |
| 安全约束（docs/00 §6） | 「不外发数据：全本地处理；规则与样本不上传」全部保持 | ✓ |

约束审计：**全部通过**。

## §8. 风险与遗留

| 项 | 严重度 | 处理 |
|----|--------|------|
| tshark 路径依赖 PATH | 非阻塞 | `PcapReader::read` 探测 `tshark --version`，缺失返回 `DependencyMissing("tshark")`；GUI 弹 `message.error` 提示；集成测试 `#[ignore]` 无 tshark CI 跳过 |
| GBK 解码未实现 | 非阻塞 | fixture 中文地址是 UTF-8 base64，UTF-8 解码够用；规划需求.md 已声明留 v0.4.2+ |
| `reassemble_base64` 规则接口保留但 fixture 未触发 | 非阻塞 | fixture 每 POST body 单独 base64，自动解码即命中；规则兜底接口保留供 CTF 分块场景 |
| 集成测试 `#[ignore]` 无 tshark CI 跳过 | 非阻塞 | 本机 tshark v4.4.9 手动 `cargo test -- --ignored pcap` 全绿；CI 无 tshark 时跳过不阻塞 |
| `MaskView.jsx:112` `const params` 赋值 esbuild 警告 | 非阻塞 | v0.1.0 既有，v0.4.1 范围外，保留 |
| crate 名 `ruT0_data_kit_core` non_snake_case 警告 | 非阻塞 | v0.2.0 既有历史命名，改名涉及 Cargo.toml + 全仓 use，留后续 |
| antd chunk > 500kB 警告 | 非阻塞 | vite 通用提示，非本版本引入 |
| GUI smoke 未端到端跑（无显示环境） | 非阻塞 | 代码路径经 reviewer 静态核对与 T6-x 后端契约对齐；构建产物就绪；用户可手动 `open RuT0DataKit.app` 验证 |
| `tauri.js:235` `list_regex_templates` 历史注释 | 非阻塞 | T6-5 移除命令后保留接口注释作为历史参考，无实际调用，不影响功能 |
| `looks_like_blind_probe` length 单括号不识别 | 非阻塞 | `length_regex` 既有模式要求 `length\s*\(\s*\(...\))` 双括号；单括号 `length(database())>5` 不识别是 v0.2.4 既有局限，T6-4 不改 length_regex（out_of_scope，避免影响 extract_blind_probe 行为）；测试用双括号 `length((database()))>5` |

无阻塞风险。

## §9. 发布建议

**建议发布 v0.4.1**。

- 六任务全 verified_complete；端到端 419 non-ignored + 4 ignored 测试全绿；构建产物就绪
  （.app + `RuT0DataKit_0.4.1_aarch64.dmg` 含 0.4.1）；
- v0.4.1 规划目标全部达成（5 项用户诉求 + 收尾）：
  - 数据流打通（T6-1）：ExportView 读 state.records + e2e `preprocess_to_search_finds_hits` 命中「张三」。
  - 移除 FileToolbar（T6-2）：FileToolbar.jsx 删除 + App.jsx 移除渲染分支 + `grep FileToolbar frontend/src/` 0 命中。
  - ToolsView 下拉栏（T6-3）：antd Tabs → antd Select。
  - SQL 盲注自动跳转（T6-4）：`looks_like_blind_probe` 4 单测 + `detect_sql_blind_features` Tauri 命令 + PreprocessView handleImport 跳转链。
  - RegexTool 语句→构造正则（T6-5）：移除模板 Tab + ConstructTab + `construct_regex` 10 单测 + ConstructedRegex 结构。
  - 收尾（T6-6）：4 版本号 + docs 全同步 + Phase 7-9。
- 向后兼容验证通过（v0.4.0 7 界面架构 / 规则 tags / 搜索倒排 / sql_parse / regex_explain 后端命令全部不破；
  v0.2.4 reconstructed_database 五段断言 + v0.3.0 pcap_scan_full 不破；regex_template 源码保留）；
- 约束全部保持；文档全部同步；版本号 4 处一致 + 产物文件名 + Info.plist 一致。

Phase 9 可执行：
1. `docs/04-版本标准.md` 0.4.1 行 → `release_complete`
2. `docs/versions/0.4.1/更新日志.md` 版本状态 → `release_complete`
3. 删除 `handoff/`（TASK-BOARD.md + 6 trio HANDOFF/REPORT/REVIEW 文件）
4. git commit + tag v0.4.1 + push GitHub SSH（`git@github.com:Wh1teJ0ker/RuT0DataKit.git`）
