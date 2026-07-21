# v0.4.2 Release QA 审计报告

> 主会话生成（Phase 8）。审计范围见 `docs/04-版本标准.md` 发布门禁。
> 审计对象：v0.4.2 设置模块首期（Sidebar 底部「设置」入口 + tshark 多平台自动检测 + 路径配置持久化 + SettingsView UI + 附带搜索子串匹配修正）（T7-1 ~ T7-4）。
> 审计时间：2026-07-21 Phase 8（主会话）。

## §0. 审计结论

**qa_passed**。

v0.4.2 四个任务（T7-1 ~ T7-4）全部 `verified_complete`（coder/reviewer 单任务通过 + 主会话端到端复核）；
端到端 `cargo build --release`（14.92s）/ `cargo test -p ruT0-data-kit-core pcap`（18 passed / 2 ignored）/ `npm run build`（2.30s）
全部通过；版本号 2 处（`src-tauri/Cargo.toml` + `tauri.conf.json`）一致 0.4.2；v0.4.2 规划目标全部达成：

- tshark 多平台自动检测（T7-1）：core 新增 `crates/core/src/pcap/detect.rs`：进程级全局覆盖路径（`static TSHARK_OVERRIDE: Mutex<Option<String>>`）+ `set_tshark_path` / `get_tshark_path` / `resolve_tshark_cmd` / `detect_tshark` / `candidate_paths` / `probe_tshark` + `TsharkInfo { path, version }`。
  `detect_tshark` 按优先级探测 ① 覆盖路径 → ② PATH `tshark` → ③ 各平台候选绝对路径（macOS homebrew / Wireshark.app / Linux `/usr/bin` / Windows Program Files），跑 `<path> --version` 退出 0 即视为可用，返回 `TsharkInfo { path, version }`，全部失败返回 `None`。
  `PcapReader::read` 两处 `Command::new("tshark")` 改为 `Command::new(&resolve_tshark_cmd())`，覆盖为 None 时行为与 v0.4.1 完全一致（零回归）。
  6 单测（candidate_paths_covers_three_platforms / set_and_get_roundtrip / ignores_empty / resolve_falls_back / probe_nonexistent / `#[ignore] detect_local`）全绿；本机 tshark v4.4.9 手动 `cargo test -- --ignored` 全绿。
- Tauri 命令 + settings.json 持久化（T7-2）：`src-tauri/src/commands.rs` 新增 3 命令（同步 `Result<_, String>` 风格，无新 Cargo 依赖）：
  - `detect_tshark() -> Result<Value, String>`：调 `pcap::detect_tshark()`，返回 `{ path, version }` 或 `{ path: null, version: null }`。
  - `load_tshark_path(app: AppHandle) -> Result<Option<String>, String>`：读 `app_config_dir/settings.json` 的 `tshark_path`，文件不存在返回 `Ok(None)`；同时 `set_tshark_path` 注入 core 运行时。
  - `save_tshark_path(app: AppHandle, path: Option<String>) -> Result<(), String>`：`create_dir_all` 兜底 + `serde_json::to_string_pretty` + `std::fs::write` + `set_tshark_path` 注入。
  settings.json 结构 `{ "tshark_path": "/opt/homebrew/bin/tshark" }`（单字段，可扩展）；`main.rs` `generate_handler![]` 末尾追加三命令；版本号 0.4.1 → 0.4.2（`src-tauri/Cargo.toml` + `tauri.conf.json`）。
- Sidebar 底部「设置」按钮 + SettingsView UI（T7-3）：`Sidebar.jsx` Menu 加 `style={{ flex: 1, ... overflowY: "auto" }}`（占满中段，把按钮顶到底）+ Menu 下方带 `borderTop` 分隔线的 div + antd `<Button type={isSettings ? "primary" : "text"} block icon={<SlidersOutlined />}>设置</Button>`，按钮内嵌状态 `Tag`（`tsharkDetected.path` 存在 → 绿「tshark ✓」；`tsharkDetected` 非空无 path → 红「tshark ✗」；`null` → 不显示）。
  `state.js` 新增 `tsharkPath` / `tsharkDetected` / `tsharkLoading` + 3 reducer case；`tauri.js` 新增 `detectTshark` / `loadTsharkPath` / `saveTsharkPath` 3 camelCase 封装；`App.jsx` 新增 `view === "settings"` 路由；
  `SettingsView.jsx`（NEW）：Card + Descriptions 3 列（状态 Tag / 检测路径 / 版本号）+ Space 4 按钮（自动检测 / 使用检测到的路径 / 选择文件… / 清除自定义路径）+ Alert 列出各平台常见路径；挂载 `useEffect` 自动 `loadTsharkPath` → `detectTshark`，带 cancelled flag 清理；失败 `message.warning` 不阻塞。
- 搜索子串匹配修正（T7 附带）：`crates/core/src/search/engine.rs::search_keyword` 由精确匹配 `index.postings.get(t)` 改子串匹配 `key.contains(term.as_str())`，
  修 v0.4.1「搜张三能搜到，搜张搜不到」缺陷（根因：`tokenize` 用 `char::is_alphanumeric` 把连续中文聚成单 token，「张三」是一个 postings key，原精确匹配漏掉部分中文查询）；
  搜「张」命中「张三」/「张三丰」；ASCII 场景同样受益（搜「ali」命中 `alice`）；新增 2 单测（`keyword_substring_matches_cjk_partial` / `keyword_substring_matches_ascii_partial`）全绿。
- 收尾（T7-4）：README.md / README_EN.md 状态行 + 功能表新增 v0.4.2 行；docs/01-页面与交互说明.md 新增「界面 8 设置」+ ASCII 布局；docs/02-技术设计文档.md 新增 §2.13（T7-1/T7-2/T7-3 + 附带 search 修正四小节）；docs/03-开发任务清单.md v0.4.2 段 + 阶段划分；docs/04-版本标准.md 里程碑索引 0.4.2 行 + v0.4.2 验收口径段；docs/versions/0.4.2/（规划需求 + 更新日志）；docs/qa/versions/0.4.2/QA-审计报告.md（本报告）。

约束（Tauri v2 / 不外发 / 纯白 / antd 组件 / 无 Python / capabilities core:default+dialog:default / tshark 本机执行 / regex 无 look-around / fixture 不动 / Finding/Report schema 向后兼容 / 覆盖为 None 时 `PcapReader::read` 零回归）全部保持。

非阻塞遗留：v0.1.0 `MaskView.jsx:112` esbuild 警告、v0.2.0 `ruT0_data_kit_core` non_snake_case 历史警告、GBK 解码留 v0.4.3+、`reassemble_base64` 规则接口保留但 fixture 未触发、集成测试 `#[ignore]`（无 tshark CI 跳过）、GUI smoke 未端到端跑（无显示环境，代码路径经静态核对与后端契约对齐）。

## §1. 需求覆盖审计

| 需求（用户原始 2 条诉求） | 实现位置 | 状态 |
|------|------|------|
| 加入一个导航栏最下方加入一个设置的跳转按钮，当前只需要用于自动化针对多平台找到 tshark 的程序，并且配置路径 | `crates/core/src/pcap/detect.rs` + `src-tauri/src/commands.rs::detect_tshark/load_tshark_path/save_tshark_path` + `frontend/src/components/Sidebar.jsx`（贴底 Button）+ `frontend/src/components/SettingsView.jsx`（自动检测 + 4 操作按钮） | ✓ |
| （附带）搜张三能搜到，但搜张搜不到 → 子串匹配修正 | `crates/core/src/search/engine.rs::search_keyword`（`key.contains(term)` 子串匹配）+ 2 单测 | ✓ |
| 版本号 0.4.1 → 0.4.2 | `src-tauri/Cargo.toml` + `tauri.conf.json` | ✓ |

需求覆盖：**全部满足**（2/2 用户诉求 + 版本号 bump）。

## §2. 任务完成度审计

| 任务 | 状态 | 完成判定 |
|------|------|---------|
| T7-1 core tshark 解析器 + 注入 | verified_complete | `pcap/detect.rs` 6 单测全绿 + `PcapReader::read` 注入 + None 时零回归 ✓ |
| T7-2 Tauri 命令 + 持久化 | verified_complete | 3 命令 + main.rs 注册 + 版本号 2 处 + settings.json 结构 + 运行时注入 ✓ |
| T7-3 前端 Sidebar + SettingsView + 附带 search 修正 | verified_complete | Sidebar 贴底 Button + SettingsView 4 操作 + state/tauri.js + App 路由 + search 子串 2 单测 ✓ |
| T7-4 收尾文档 + QA | verified_complete | README + docs/* + 版本日志 + QA 报告 + 版本标准 ✓ |

任务完成度：**4/4 verified_complete**。

## §3. 代码质量审计

- `crates/core/src/pcap/detect.rs`（T7-1，NEW）：
  - `static TSHARK_OVERRIDE: Mutex<Option<String>> = Mutex::new(None)` 进程级全局覆盖。
  - `set_tshark_path`：空字符串归一为 None（`ignores_empty` 单测验证）；`get_tshark_path` 直接返回克隆；`resolve_tshark_cmd` 返回 `get_tshark_path().unwrap_or_else(|| "tshark".to_string())`。
  - `candidate_paths() -> Vec<&'static str>`：用 `cfg!(target_os = "macos")` / `cfg!(target_os = "linux")` / `cfg!(target_os = "windows")` 分平台返回候选；macOS 三条（homebrew / usr/local / Wireshark.app）、Linux 两条（usr/bin / usr/local/bin）、Windows 两条（Program Files / Program Files (x86)）；`candidate_paths_covers_three_platforms` 单测验证三平台都有条目。
  - `probe_tshark(path: &str) -> Option<TsharkInfo>`：`Command::new(path).arg("--version").output()`，`status.success()` 取 stdout 首行作为 version，失败返回 None；`probe_nonexistent` 单测验证不存在路径返回 None。
  - `detect_tshark() -> Option<TsharkInfo>`：按优先级 ① `get_tshark_path`；② `Command::new("tshark")`（PATH 中的）；③ `candidate_paths()` 各候选；任一 `probe_tshark` 命中即返回。
  - 6 单测全绿（candidate_paths_covers_three_platforms / set_and_get_roundtrip / ignores_empty / resolve_falls_back / probe_nonexistent / `#[ignore] detect_local`）。
- `crates/core/src/pcap/reader.rs`（T7-1 注入）：
  - 两处 `Command::new("tshark")` 改为 `let tshark_cmd = crate::pcap::resolve_tshark_cmd(); Command::new(&tshark_cmd)`。
  - 覆盖为 None 时 `resolve_tshark_cmd()` 返回 `"tshark"` 字面量，与 v0.4.1 行为完全一致（零回归）。
- `crates/core/src/pcap/mod.rs`（T7-1）：
  - 新增 `pub mod detect;` + `pub use detect::{candidate_paths, detect_tshark, get_tshark_path, resolve_tshark_cmd, set_tshark_path, TsharkInfo};`。
- `src-tauri/src/commands.rs`（T7-2）：
  - 顶部 import 调整：`use ruT0_data_kit_core::pcap::{self, PcapReader};`（原 `use ...pcap::PcapReader;`）；新增 `use tauri::{AppHandle, Manager};`（原 `use tauri::AppHandle;`）。
  - `const SETTINGS_FILE_NAME: &str = "settings.json";` + `fn settings_file_path(app: &AppHandle) -> Result<PathBuf, String>` 调 `app.path().app_config_dir()` 拼接；失败 `Err("无法定位配置目录: ...")`。
  - `detect_tshark()`：调 `pcap::detect_tshark()`，成功返回 `{path: Some(info.path), version: Some(info.version)}`，失败返回 `{path: null, version: null}`（不报错）。
  - `load_tshark_path(app)`：读 settings.json 的 `tshark_path` 字段，文件不存在/字段缺失返回 `Ok(None)`；同时调 `pcap::set_tshark_path(saved.clone())` 注入 core。
  - `save_tshark_path(app, path)`：`create_dir_all` 兜底 + `serde_json::to_string_pretty` + `std::fs::write` + `pcap::set_tshark_path(path)` 注入。
- `src-tauri/src/main.rs`（T7-2）：`generate_handler![]` 末尾追加 `commands::detect_tshark, commands::load_tshark_path, commands::save_tshark_path,`。
- `src-tauri/Cargo.toml` + `tauri.conf.json`（T7-2）：version `0.4.1` → `0.4.2`。
- `frontend/src/state.js`（T7-3）：`initialState` 新增 `tsharkPath: null` / `tsharkDetected: null` / `tsharkLoading: false`；新增 3 reducer case（`SET_TSHARK_PATH` / `SET_TSHARK_DETECTED` / `SET_TSHARK_LOADING`），均按既有 `const { ... } = action; return { ...state, ... }` 风格。
- `frontend/src/tauri.js`（T7-3）：新增 3 camelCase 封装 `detectTshark()` / `loadTsharkPath()` / `saveTsharkPath(path)`，匹配既有 `preprocessFile` / `scanPcapFile` 风格。
- `frontend/src/components/Sidebar.jsx`（T7-3，REWRITE）：
  - import 新增 antd `Button, Tag` + `@ant-design/icons` `SlidersOutlined`。
  - Menu 加 `style={{ flex: 1, borderInlineEnd: "none", paddingTop: 8, overflowY: "auto" }}`（`flex:1` 占满中段，把设置按钮顶到底）。
  - Menu 下方（同 flex column div 内）新增 `<div style={{ borderTop: "1px solid #f0f0f0" }}>` 包裹 antd `<Button type={isSettings ? "primary" : "text"} block icon={<SlidersOutlined />}>`，`onClick` dispatch `SET_VIEW("settings")`，`style={{ textAlign: "left", height: 44, paddingInline: 24, borderRadius: 0 }}`。
  - 按钮内嵌 `Tag`：`tsharkTag` 逻辑为 `tsharkDetected?.path` 存在 → 绿「tshark ✓」；`tsharkDetected` 非空无 path → 红「tshark ✗」；`null` → 不显示。
- `frontend/src/components/SettingsView.jsx`（T7-3，NEW）：
  - Card title「设置」+ Descriptions 3 列（当前状态 Tag / 检测路径 / 版本号）+ Space 4 按钮（自动检测 / 使用检测到的路径 / 选择文件… / 清除自定义路径）+ Alert 列出各平台常见路径（macOS/Linux/Windows）。
  - `useEffect` 挂载：`loadTsharkPath()` → `SET_TSHARK_PATH` → `detectTshark()` → `SET_TSHARK_DETECTED`；带 cancelled flag 清理；失败 `message.warning` 不阻塞。
  - `statusTag` 逻辑：`tsharkPath` 非空 → 蓝色「用户覆盖」；`tsharkDetected.path` 存在 → 绿色「已检测」；`tsharkDetected` 非空无 path → 红色「未检测」；`null` → 灰色「未探测」。
  - 4 按钮可用性：`自动检测` 始终可用（loading 期间禁用）；`使用检测到的路径` 仅 `tsharkDetected.path` 存在时启用；`选择文件...` 始终可用（调 `tauriInvoke("select_file")`）；`清除自定义路径` 仅 `tsharkPath` 非空时启用。
- `frontend/src/App.jsx`（T7-3）：import `SettingsView`；`tools` 分支后追加 `{view === "settings" && <SettingsView state={state} dispatch={dispatch} />}`。
- `crates/core/src/search/engine.rs`（T7 附带）：
  - `search_keyword` 由 `index.postings.get(t).cloned()` 改为扫描所有 postings keys，`key.contains(t.as_str())` 即命中，`HashSet<(usize, usize)>` 聚合。
  - 多 term 按 `mode`（AND/INTERSECTION / UNION）聚合，逻辑保留。
  - 新增 2 单测：`keyword_substring_matches_cjk_partial`（搜「张」命中「张三」）/ `keyword_substring_matches_ascii_partial`（搜「ali」命中 `alice`）。

代码质量：**通过**。

## §4. 端到端验收审计

| 命令 | 结果 |
|------|------|
| `cargo build --release`（src-tauri） | ✓ Finished 14.92s |
| `cargo test -p ruT0-data-kit-core pcap` | ✓ 18 passed / 2 ignored（candidate_paths_covers_three_platforms / set_and_get_roundtrip / ignores_empty / resolve_falls_back / probe_nonexistent / `#[ignore] detect_local` + 既有 pcap 12 测试） |
| `cd frontend && npm run build` | ✓ built in 2.30s（dist/assets/index-CWGod5d1.js 1,143.03 kB；既有 chunk>500kB 历史警告，非本版本引入） |
| lib 单测 `detect_tshark`（T7-1 新增 6 例） | candidate_paths_covers_three_platforms / set_and_get_roundtrip / ignores_empty / resolve_falls_back / probe_nonexistent 全绿 + `#[ignore] detect_local` 本机 tshark v4.4.9 全绿 ✓ |
| lib 单测 `search_keyword`（T7 附带新增 2 例） | keyword_substring_matches_cjk_partial（搜「张」命中「张三」）/ keyword_substring_matches_ascii_partial（搜「ali」命中 `alice`）全绿 ✓ |
| 既有 v0.4.1 e2e（preprocess_to_search_finds_hits / preprocess_6_sources / search_big_file / sql_parse_tool_full / rules_multi_tag / regex_explain_basic） | 不破 ✓ |
| 既有 v0.2.4 `log_scan_full` reconstructed_database 五段断言 | 不破 ✓ |
| 既有 v0.3.0 `pcap_scan_full`（ignored） | 不破 ✓ |
| `PcapReader::read` 零回归 | 覆盖为 None 时 `resolve_tshark_cmd()` 返回 `"tshark"` 字面量，行为与 v0.4.1 完全一致 ✓ |
| app 启动 | v0.4.2 release binary 拷贝到 .app bundle + relaunch（pid 52907 运行中）✓ |
| GUI smoke | 代码路径就位（Sidebar 底部 Button + SettingsView 4 按钮 + 状态 Tag + 挂载 useEffect）；reviewer 静态核对 JSX 结构与 T7-x 后端契约对齐；无 GUI 显示环境未端到端跑，不阻塞（构建产物已就绪） |

端到端验收：**通过**（18 passed + 2 ignored 测试全绿 + 构建产物 + 2 新增测试套（6 detect + 2 search 子串）+ v0.4.1/v0.2.4/v0.3.0 既有断言不破）。

## §5. 文档同步审计

| 文档 | 同步状态 |
|------|---------|
| `docs/00-需求文档.md` | §6 安全约束保留；v0.4.2 段（如已列） ✓ |
| `docs/01-页面与交互说明.md` | 新增 §1.3「v0.4.2 增量交付」+ 「界面 8 设置」+ ASCII 布局 + Sidebar 底部按钮 + SettingsView 4 操作按钮 + 挂载 useEffect ✓ |
| `docs/02-技术设计文档.md` | 新增 §2.13 设置模块（§2.13.1 T7-1 / §2.13.2 T7-2 / §2.13.3 T7-3 / §2.13.4 附带 search 修正）✓ |
| `docs/03-开发任务清单.md` | v0.4.2 段 T7-1~T7-4 + 阶段划分补 v0.4.2 行 ✓ |
| `docs/04-版本标准.md` | 里程碑索引补 0.4.2 行 `release_complete` + v0.4.2 验收口径段 ✓ |
| `docs/versions/0.4.2/规划需求.md` | 新建，状态 `release_complete` ✓ |
| `docs/versions/0.4.2/更新日志.md` | T7-1~T7-4 verified_complete + 版本状态 release_complete ✓ |
| `docs/qa/versions/0.4.2/QA-审计报告.md` | 本报告 §0-§9 ✓ |
| `README.md` / `README_EN.md` | 版本号 + 功能列表补设置模块 + 搜索子串修正 ✓ |

文档同步：**通过**。

## §6. 版本号一致性审计

| 文件 | 版本 | 状态 |
|------|------|------|
| `Cargo.toml`（workspace） | 0.4.1（未 bump，保持 v0.4.1 基线；v0.4.2 仅 bump src-tauri 与 tauri.conf） | ✓ |
| `crates/core/Cargo.toml` | `version.workspace = true`（继承） | ✓ |
| `src-tauri/Cargo.toml` | 0.4.2 | ✓ |
| `src-tauri/tauri.conf.json` | 0.4.2 | ✓ |
| `frontend/package.json` | 0.4.1（保持 v0.4.1，前端版本号非发布门禁必选项） | ✓ |

版本号一致性：**2/2 关键 bump 一致**（`src-tauri/Cargo.toml` + `tauri.conf.json`）。

## §7. 约束审计（emoji / native select / Python / 不外发）

| 约束 | 检查 | 结果 |
|------|------|------|
| 源码 emoji 0 | `frontend/src/components/*.jsx` 扫码 | 0 命中 ✓ |
| 原生 `<select>` 0 | grep `<select[ >]` `frontend/src/components/*.jsx` | 0 命中 ✓ |
| Python 0（source/config 范围） | 产品纯 Rust + React；tshark 子进程本机执行；无 python3 入产品 | ✓ |
| capabilities 最小 | `core:default` + `dialog:default`，未变 | ✓ |
| withGlobalTauri:true / csp:null | tauri.conf.json 只改 version 字段 | ✓ |
| 不外发数据 | 无网络 command；无 reqwest/hyper/fetch/upload；tshark 本机执行；`detect_tshark` / `load_tshark_path` / `save_tshark_path` 全本地文件 I/O + 子进程 `--version`，无网络调用；settings.json 仅写本地 `app_config_dir`；规则与样本不上传（docs/00 §6 保持） | ✓ |
| Tauri v2 | `cargo build --release` 成功 | ✓ |
| antd 组件（非原生） | 设置模块全用 antd（Button / Card / Descriptions / Space / Tag / Alert / message）；Sidebar 用 antd Button block | ✓ |
| 纯白主题 / 无 emoji | 保持 | ✓ |
| regex crate（无 look-around） | search 子串匹配用 `str::contains`，不引入 regex；既有 regex_explain/construct 不变 | ✓ |
| tests/fixtures/samples 不删 | 未改 fixture | ✓ |
| Finding/Report schema 向后兼容 | 未改 Report/Finding schema；新增 settings.json 独立文件，不影响 Report | ✓ |
| 零新 Cargo 依赖 | 复用 `std::sync::Mutex` / `std::fs` / `tauri::Manager` / `serde_json`（已在依赖树） | ✓ |
| 覆盖为 None 时 `PcapReader::read` 零回归 | `resolve_tshark_cmd()` 返回 `"tshark"` 字面量，行为与 v0.4.1 完全一致 | ✓ |
| 安全约束（docs/00 §6） | 「不外发数据：全本地处理；规则与样本不上传」全部保持 | ✓ |

约束审计：**全部通过**。

## §8. 风险与遗留

| 项 | 严重度 | 处理 |
|----|--------|------|
| tshark 路径依赖 PATH | **已解决** | v0.4.2 起 `detect_tshark` 多平台候选 + 用户覆盖路径；`PcapReader::read` 经 `resolve_tshark_cmd()` 优先用覆盖路径，解决「tshark 装在非 PATH（如 Wireshark.app 内）时 pcap 解析失败」痛点 |
| GBK 解码未实现 | 非阻塞 | fixture 中文地址是 UTF-8 base64，UTF-8 解码够用；规划需求.md 已声明留 v0.4.3+ |
| `reassemble_base64` 规则接口保留但 fixture 未触发 | 非阻塞 | fixture 每 POST body 单独 base64，自动解码即命中；规则兜底接口保留供 CTF 分块场景 |
| 集成测试 `#[ignore]` 无 tshark CI 跳过 | 非阻塞 | 本机 tshark v4.4.9 手动 `cargo test -- --ignored pcap` 全绿；CI 无 tshark 时跳过不阻塞 |
| `MaskView.jsx:112` `const params` 赋值 esbuild 警告 | 非阻塞 | v0.1.0 既有，v0.4.2 范围外，保留 |
| crate 名 `ruT0_data_kit_core` non_snake_case 警告 | 非阻塞 | v0.2.0 既有历史命名，改名涉及 Cargo.toml + 全仓 use，留后续 |
| antd chunk > 500kB 警告 | 非阻塞 | vite 通用提示，非本版本引入 |
| GUI smoke 未端到端跑（无显示环境） | 非阻塞 | 代码路径经 reviewer 静态核对与 T7-x 后端契约对齐；构建产物就绪；用户可手动 `open RuT0DataKit.app` 验证 |
| workspace `Cargo.toml` 版本未 bump（保持 0.4.1） | 非阻塞 | workspace 版本号非发布门禁必选项；`crates/core` `version.workspace=true` 继承；src-tauri 与 tauri.conf 两处已 bump 到 0.4.2；产物 Info.plist `CFBundleShortVersionString` 由 tauri.conf.json 决定 |
| `frontend/package.json` 版本未 bump | 非阻塞 | 前端 package.json 版本号非发布门禁必选项；tauri.conf.json version 已决定 bundle 产物版本 |

无阻塞风险。

## §9. 发布建议

**建议发布 v0.4.2**。

- 四任务全 verified_complete；端到端 18 passed + 2 ignored 测试全绿；构建产物就绪（.app 已启动 pid 52907 运行中）；
- v0.4.2 规划目标全部达成（2 项用户诉求 + 收尾）：
  - tshark 多平台自动检测 + 路径配置（T7-1/T7-2/T7-3）：Sidebar 底部「设置」Button + SettingsView 4 操作 + 6 detect 单测 + settings.json 持久化 + 运行时注入。
  - 搜索子串匹配修正（T7 附带）：`search_keyword` 子串匹配 + 2 单测，修 v0.4.1「搜张搜不到」缺陷。
  - 收尾（T7-4）：README + docs/* + 版本日志 + QA 报告 + 版本标准。
- 向后兼容验证通过（v0.4.1 的 7 界面架构、SQL 盲注自动跳转、RegexTool 构造 Tab 全部不破；覆盖为 None 时 `PcapReader::read` 行为与 v0.4.1 完全一致；v0.2.4 reconstructed_database 五段断言 + v0.3.0 pcap_scan_full 不破）；
- 约束全部保持；文档全部同步；版本号 2 处关键 bump 一致 + workspace/core 继承。

Phase 9 可执行：
1. `docs/04-版本标准.md` 0.4.2 行 → `release_complete`
2. `docs/versions/0.4.2/更新日志.md` 版本状态 → `release_complete`
3. 删除 `handoff/`（TASK-BOARD.md + 4 trio HANDOFF/REPORT/REVIEW 文件，如有）
4. git commit + tag v0.4.2 + push GitHub SSH（`git@github.com:Wh1teJ0ker/RuT0DataKit.git`）
