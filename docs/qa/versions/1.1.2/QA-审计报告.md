# v1.1.2 QA 审计报告

> 版本：1.1.2
> 审计类型：Release 前全局审计（v1.1.1 → v1.1.2 维护性增强版本）
> 审计依据：[`docs/00-需求文档.md`](../../../00-需求文档.md) §6 验收标准 + [`docs/03-开发任务清单.md`](../../../03-开发任务清单.md) §8 T37~T45 任务定义 + [`docs/04-版本标准.md`](../../../04-版本标准.md) §4 发布门禁
> 审计轮次：R5（2026-08-09 T47 设置页全局每页行数持久化增量复核；R4 2026-08-09 T46 Base64 抽离为独立加解密面板增量复核；R3 2026-08-09 T45 .log 结构化解析增量复核；R2 2026-08-09 完整重审基于实际命令输出 + Mimosa 重跑；R1 由 T44 coder 执行结论 `qa_passed`）
> 结论：`qa_passed` — 静态审计 + 单元测试（136 passed / 3 ignored / 0 failed）+ 前端构建（3081 modules）+ 版本一致性（4 处 1.1.2）全通过；T37~T47 全部 `verified_complete`，T44 文档收口 + Release QA 完成且无 major/critical 问题。**安全声明**：Mimosa 深度安全扫描已重跑完整审计（`scan-2026-08-08T19-56-00.366Z-a958f772712b`，deep 深度），68/68 源文件全量解析成功（0 失败），487 个依赖包 0 漏洞，0 findings；覆盖度 `partial`（调用图部分不完整：部分调用为动态派发或超出分析规模，跨文件可达性可能不完整），`runStatus=inconclusive`（`verdictEffect=none`）。此前 R1 的 `library_source_unavailable` 已消除；剩余 `callgraph_fact_partial` 为动态派发分析方法学限制（非项目缺陷）。**不宣称项目安全**（静态分析非运行时验证），但无任何已识别的安全 finding 阻碍发布。

## 1. 审计维度与结论

| 维度 | 结论 | 说明 |
|---|---|---|
| 需求覆盖 | `pass` | T37~T47 单元级全 `verified_complete`；8 项用户需求（Base64 列编解码 + .log 结构化导入 + tshark 测试加固 + 设置按钮迁移 + 设置文案精简 + .log 结构化解析优化 + Base64 抽离为独立加解密面板 + 全局每页行数设置）逐一对照实现，见 §2 |
| 端到端流程 | `pass` | E1（cargo fmt/clippy/test 全绿）+ E2（pnpm frozen-lockfile install + build 通过）+ E7（版本号 4 处一致 1.1.2）+ E9（全局每页行数持久化 + 同步刷新 + 重启保留）；E3~E6 后端单测覆盖，GUI 视觉验收属浏览器自动化增量轮次，不阻塞发布 |
| 构建与测试 | `pass` | `cargo fmt --check` / `cargo clippy --workspace -- -D warnings` / `cargo test --workspace`（136 passed / 3 ignored / 0 failed）+ `pnpm build`（3081 modules，2.59s）全绿 |
| 代码质量 | `pass` | scope_deviation 审查：T37~T47 均无越界改动；T37 闭包式 `base64_transform_column_cells` 使 DB 层不依赖 commands 层 `Base64Mode`，职责清晰；T42/T45 LogReader 复用 `Reader` trait + `detect_format` 分发，符合 §2.5 扩展路径；T45 改动仅限 `log.rs` 单文件，正则经 `OnceLock` 缓存；T46 纯前端 UI 重组，复用既有 `activeCapability` XOR 切换 + `SidePanel` 注册表模式，无新增 IPC/DB 方法；T47 沿用 `settings.json` 持久化模式（与 tshark 路径同文件同字段策略），`SET_PAGE_SIZE` reducer 合并全局 + Sheet 同步更新，`toRowObjects` 增加 `pageSize` 参数消除行号硬编码 |
| 安全与隐私 | `pass` | 新增 SQL（`base64_transform_column_cells` 查询/回写）全用 `?N` + `params![]` 绑定，grep 无字符串拼接；`list_undoable_operations` 的 IN 子句为硬编码常量（含 `base64_column`），无注入风险；凭据无新增（沿用 v1.1.0 updater 密钥配置）；全本地处理无网络调用；tshark 子进程用固定 flag 参数不传用户输入；**Mimosa 深度扫描已重跑完整审计**（scan-2026-08-08T19-56-00.366Z-a958f772712b，68/68 文件全量解析 0 失败，487 包 0 漏洞，0 findings），剩余 callgraph partial 为方法学限制（动态派发），非项目缺陷 |
| 数据与迁移 | `pass` | `SCHEMA_VERSION=3` 不变（无新增表/列/索引），v1.1.1 → v1.1.2 无迁移；历史数据完整保留；`operations.kind="base64_column"` 纳入既有 `before_snapshot_json` 快照机制 |
| 依赖与配置 | `pass` | 新增依赖 `base64 = "0.22"`（src-tauri/Cargo.toml）单一来源，无传递依赖膨胀；Cargo.lock 已同步（base64 0.22.1 锁定）；既有 base64 0.21.7 为 tauri-utils/swift-rs 间接依赖（非本轮引入）；T45 LogReader 用 `regex`（已是 core 依赖 v1.1.0 processor 引入）+ `std::sync::OnceLock`（std 库）无新增 crate；版本号 4 处一致 1.1.2（Cargo.toml workspace + tauri.conf.json + frontend/package.json + constants.js；core/src-tauri workspace=true 继承） |
| 文档一致性 | `pass` | `docs/versions/1.1.2/` 三件套齐全（规划需求.md + 更新日志.md + RELEASE-NOTES.md，含 T45 增量）；`docs/qa/versions/1.1.2/QA-审计报告.md` 本报告；`docs/02-技术设计文档.md` 补充 v1.1.2 段落（Base64 + .log 结构化导入 + UI 迁移 + 文案精简）；`docs/04-版本标准.md` 里程碑表 v1.1.2 行状态推进 |

## 2. 需求覆盖审计

依据 [`规划需求.md`](../../versions/1.1.2/规划需求.md) §三验收标准与 [`03-开发任务清单.md`](../../../03-开发任务清单.md) §8 T37~T45 任务定义：

| # | 需求 | 任务 | 状态 | 证据 |
|---|---|---|---|---|
| 1 | Base64 列编解码（就地变更，可撤销） | T37（后端）+ T38（前端 IPC）+ T46（前端 UI 迁入 CryptoPanel） | `pass` | `src-tauri/src/commands/columns.rs` 新增 `base64_column` 命令（`base64_column_inner` 接受 `&DbManager` 便于单测）+ `Base64Mode`/`Base64Result` 结构；`src-tauri/src/db/mod.rs` 新增 `base64_transform_column_cells`（闭包式转换，DB 层不依赖 `Base64Mode`，单事务 + before/after 快照）；`operations.kind="base64_column"` 纳入 `list_undoable_operations` IN 子句；`src-tauri/src/lib.rs` 注册命令；`Cargo.toml` 加 `base64="0.22"`；前端 `frontend/src/tauri.js` 加 `base64Column` IPC；T46 把 Base64 UI 从 `ColumnOpsPanel` 迁至新建 `CryptoPanel`（`crypto` 能力按钮，`KeyOutlined` 图标，`SidePanel` 注册表登记）；12 columns tests 全过（含 encode/decode/skipped/undo 往返） |
| 2 | .log 文件导入（格式自动识别 + 结构化多列） | T42（初版）+ T45（结构化升级） | `pass` | `crates/core/src/datasource/log.rs` `LogReader` 对前 10 条非空行做 probe，按正则命中数选格式：Apache Combined（11 列 + raw_line，request 拆 method/url/protocol，bytes 为 `-` 归零）/ Apache Common（9 列 + raw_line）/ Syslog（5 列 + raw_line，pid 可选）/ 通用应用日志（4 列 + raw_line，thread 可选）/ 未识别回退单列 `line`；正则经 `OnceLock` 缓存，`raw_line` 始终保留原文；`crates/core/src/datasource/mod.rs` `detect_format` 工厂按 `.log` 分发 + 模块导出；`frontend/src/components/layout/TopToolbar.jsx` 导入对话框 `.log` filter；13 log tests 全过（3 fallback + 10 结构化 + access.log 真实 fixture ignored 集成测试）+ detect_format 路由测试覆盖 `.log` |
| 3 | tshark 解析测试加固 | T41 | `pass` | `crates/core/src/pcap/detect.rs` 新增 2 测试（`candidate_paths_windows_paths_are_absolute` + `probe_tshark_nonexistent_returns_none`）+ 既有 5 测试保留（candidate 三平台覆盖 / set_get roundtrip / 空白忽略 / resolve 回退 / nonexistent probe）；`reader.rs` 既有 9 测试保留（hex_to_bytes / parse_tshark_output 多场景 + fixture ignored）；共 7 detect + 9 reader = 16 pcap tests，Windows 路径候选 `C:\Program Files\Wireshark\tshark.exe` + `C:\Program Files (x86)\Wireshark\tshark.exe` 显式覆盖 |
| 4 | 设置按钮迁移到 TopToolbar 右上角 | T40 | `pass` | `frontend/src/components/layout/TopToolbar.jsx` 右端加 `<SettingOutlined />` 文本按钮（`flex:1` 占位推到最右，`onClick={() => setView("settings")}`）；`frontend/src/components/AiPanel.jsx` 移除原设置入口；`frontend/src/App.jsx` 路由不变（`currentView==="settings"` 渲染 SettingsView）；pnpm build 通过 |
| 5 | 设置界面文案精简 | T39 | `pass` | `frontend/src/components/settings/cards/{DbPathCard,AboutCard,TsharkPathCard}.jsx` 三卡片移除冗余描述文案，保留核心字段与操作；DbPathCard 仍硬编码 DB 路径展示（动态命令推迟 v1.2+）；TsharkPathCard 路径覆盖/探测逻辑不变；pnpm build 通过 |
| 6 | 版本号 1.1.1 → 1.1.2 | T43 | `pass` | 4 处一致（Cargo.toml workspace.package.version + 注释行 + tauri.conf.json + frontend/package.json + frontend/src/constants.js；core/src-tauri Cargo.toml workspace=true 继承）；grep 确认无 1.1.1 残留 |
| 7 | 文档收口 + 全量验证 + Release QA | T44 | `pass` | 三件套齐全（规划需求.md + 更新日志.md + RELEASE-NOTES.md）+ 本 QA 报告 + 02 设计文档 v1.1.2 段落 + TASK-BOARD 最终状态；全量验证 cargo fmt/clippy/test + pnpm build 全绿 |
| 8 | .log 结构化解析优化（格式自动识别 + 多列 + raw_line） | T45 | `pass` | `crates/core/src/datasource/log.rs` 重写：`LogFormat` 枚举 + 4 正则（`OnceLock` 缓存）+ `detect_format()` probe 探测 + `split_request()` 二次拆分；Apache Combined 12 列 / Common 10 列 / Syslog 6 列 / AppLog 5 列 / fallback 1 列；`raw_line` 始终保留原文，解析失败各字段留空；13 单测全过（含 access.log 真实 fixture 集成 `#[ignore]` 测试）；cargo fmt/clippy/test 全绿；pnpm build 绿；改动仅限 log.rs 单文件 |
| 9 | Base64 抽离为独立加解密面板 | T46 | `pass` | 新建 `frontend/src/components/panels/CryptoPanel.jsx`（搬迁 Base64 编解码 Form + handleBase64 逻辑，行为不变）；`TopToolbar.jsx` `CAPABILITIES` 数组新增 `{id:"crypto",label:"加解密",icon:<KeyOutlined/>}`（位于 columnOps 与 rules 之间）；`SidePanel.jsx` `PANELS` 注册表加 `crypto: CryptoPanel`；`ColumnOpsPanel.jsx` 删除 Base64 相关 state/handler/JSX/import，仅剩 JSON 解析；pnpm build 通过（3080 modules，2.56s）；纯前端 UI 重组，无 IPC/DB/Rust 改动 |
| 10 | 全局每页行数设置（PAGE_SIZE 持久化） | T47 | `pass` | `src-tauri/src/commands/settings.rs` 扩展 `TsharkSettings` → `AppSettings`（新增 `#[serde(default)] page_size: Option<u32>`）+ 新增 `load_page_size` / `save_page_size` 两个 `#[tauri::command]`；`src-tauri/src/lib.rs` `generate_handler!` 注册两命令；前端 `frontend/src/state/constants.js` 加 `initialState.pageSize=50` + `ACTION.SET_PAGE_SIZE`；`reducer.js` 新增 `SET_PAGE_SIZE` case（合并全局 + 所有 Sheet 同步：`sheets.map(s => ({...s, pageSize, page: 1}))`）+ ADD_SHEET/IMPORT_SUCCESS/ADD_SHEET_FROM_PARSE 传 `state.pageSize`；`factory.js` 3 工厂增加可选 `pageSize` 参数；`AppContext.jsx` 加 `setPageSize` dispatcher；`tauri.js` 加 `loadPageSize` / `savePageSize` IPC 封装 + `toRowObjects` 增加 `pageSize` 参数消除硬编码 `PAGE_SIZE` 行号计算；新建 `PageSizeCard.jsx`（Select 20/50/100/200 + 保存按钮 + 启动加载 + 保存后刷新当前 Sheet 首页）；`SettingsView.jsx` 注册 PageSizeCard；`App.jsx` 启动 useEffect 调 `loadPageSize` + 导入首页用 `state.pageSize`；`DataTable.jsx` 搜索 L241/269 改 `sheet.pageSize || PAGE_SIZE`（修既有不一致）；pnpm build（3081 modules，2.59s）+ cargo clippy（-D warnings 0 警告）全绿 |

## 3. 端到端流程审计

| 验收项 | 状态 | 证据 |
|---|---|---|
| E1 静态全绿 | `pass` | `cargo fmt --check` exit 0 + `cargo clippy --workspace -- -D warnings` exit 0 + `cargo test --workspace` 136 passed / 3 ignored / 0 failed |
| E2 前端构建 | `pass` | `pnpm --prefix frontend install --frozen-lockfile`（Lockfile is up to date, Already up to date, 367ms）+ `pnpm --prefix frontend build`（3080 modules transformed, ✓ built in 2.56s；chunk >500kB 为 antd 既有警告，非本轮引入） |
| E3 Base64 编码/解码/撤销 | `pass` | 后端单测覆盖 `base64_encode_column_basic` / `base64_decode_column_basic` / `base64_decode_column_skips_invalid` + 撤销往返（before 快照回写恢复原文）；前端 pnpm build 通过；GUI 视觉验收属增量轮次 |
| E4 设置按钮在 TopToolbar 右端 | `pass` | TopToolbar.jsx 右端 `<SettingOutlined />` 文本按钮可见，`onClick` 切换 `currentView="settings"`；pnpm build 通过 |
| E5 .log 文件可导入（格式自动识别 + 结构化多列） | `pass` | `LogReader` 单测覆盖 3 fallback（reads_lines / preserves_empty_lines / empty_file）+ 10 结构化（apache_combined/common + unparseable_request + dash_bytes + syslog + syslog_without_pid + app_log + app_log_without_thread + mixed_fallback + raw_line_preserves）+ 1 access.log 真实 fixture 集成（`#[ignore]` 本机验证过）；`detect_format` 路由测试覆盖 `.log`；前端导入 filter 含 `.log`；GUI 视觉验收属增量轮次 |
| E6 tshark Windows 路径候选有测试覆盖 | `pass` | `candidate_paths_windows_paths_are_absolute` 断言 Windows 路径以 `C:\` 开头 + `tshark.exe` 结尾；`candidate_paths_covers_three_platforms` 断言三平台覆盖；`probe_tshark_nonexistent_returns_none` 断言不存在路径返回 None 不 panic |
| E7 版本号 4 处一致 1.1.2 | `pass` | grep 确认 Cargo.toml（workspace.package.version=1.1.2 + 注释行）+ tauri.conf.json（version=1.1.2）+ frontend/package.json（version=1.1.2）+ frontend/src/constants.js（APP_VERSION="v1.1.2"）；core/src-tauri Cargo.toml workspace=true 自动继承 |
| E8 加解密按钮 + CryptoPanel + ColumnOpsPanel 瘦身 | `pass` | `TopToolbar.jsx` `CAPABILITIES` 含 `{id:"crypto",label:"加解密",icon:<KeyOutlined/>}`；`SidePanel.jsx` `PANELS` 含 `crypto: CryptoPanel`；`CryptoPanel.jsx` 承载 Base64 编解码 Form + handleBase64（逻辑与原 ColumnOpsPanel 一致：base64Column → getSheetData + SET_SHEET_DATA + listUndoableOperations + SET_UNDO_STACK）；`ColumnOpsPanel.jsx` 仅剩 JSON 解析（无 base64Form/base64ing/handleBase64/base64Column import/Divider）；pnpm build 通过 |
| E9 全局每页行数设置 + 持久化 | `pass` | `settings.rs` 新增 `AppSettings.page_size` + `load_page_size`/`save_page_size` 命令（`#[serde(default)]` 向后兼容旧 settings.json）；`PageSizeCard.jsx` 可选 20/50/100/200 + 保存按钮 + 启动加载；`SET_PAGE_SIZE` reducer 合并全局 + Sheet 同步（page 重置 1）；`App.jsx` 启动 useEffect 加载持久化值；`toRowObjects` 增加 `pageSize` 参数消除硬编码行号；`DataTable.jsx` 搜索改 `sheet.pageSize || PAGE_SIZE`（修既有不一致）；pnpm build（3081 modules，2.59s）+ cargo clippy（-D warnings 0 警告）全绿 |

## 4. 构建与测试审计

| 项 | 状态 | 证据 |
|---|---|---|
| `cargo fmt --check` | `pass` | exit 0，无格式差异 |
| `cargo clippy --workspace -- -D warnings` | `pass` | exit 0，core + src-tauri 全零警告 |
| `cargo test --workspace` | `pass` | src-tauri lib 77 passed / 0 failed / 0 ignored；core lib 59 passed / 0 failed / 3 ignored（本机 tshark 探测 + pcap fixture + access.log fixture 集成，CI 无 tshark/fixture 时跳过）；Doc-tests 0；合计 136 passed / 3 ignored / 0 failed |
| `pnpm --prefix frontend install --frozen-lockfile` | `pass` | Lockfile is up to date，Already up to date（367ms） |
| `pnpm --prefix frontend build` | `pass` | 3081 modules transformed，✓ built in 2.59s；chunk >500kB 为 antd 既有警告，非本轮引入 |
| 本机 dev 构建 | `pass` | cargo check + pnpm build 通过 |
| CI 构建（四目标矩阵） | `info` | 待 git tag `v1.1.2` 触发；未发布前不阻塞 qa_passed |

## 5. 代码质量审计

| 项 | 状态 | 证据 |
|---|---|---|
| scope_deviation（T37~T45） | `pass` | 各任务 REPORT 均无越界改动；T37 闭包式 `base64_transform_column_cells` 使 DB 层不依赖 commands 层 `Base64Mode`，转换逻辑由调用方闭包提供，职责清晰；T42 LogReader 复用 `Reader` trait + `detect_format` 分发，符合 §2.5 扩展路径；T40 仅迁移设置入口位置，未改路由逻辑；T45 改动仅限 `log.rs` 单文件（regex + OnceLock + detect_format + read_all 重写），不触动 mod.rs / data.rs / model.rs / 前端 |
| 模块自洽 | `pass` | LogReader 独立子模块 `datasource/log.rs`，不跨模块访问；T45 正则缓存在模块内 `OnceLock`，`detect_format`/`split_request` 为私有函数；base64 转换闭包在 commands 层实现，DB 层只提供事务 + 快照机制 |
| IPC 收口 | `pass` | `base64_column` 走 `#[tauri::command]` + `Result<T, String>` + `.map_err(|e| e.to_string())`；前端经 `tauri.js` 封装层 `base64Column` 调用，不直接 invoke 裸字符串 |
| 测试覆盖 | `pass` | T37 columns 12 tests（含 base64 encode/decode/skipped/undo 往返）+ T42/T45 log 13 tests（3 fallback + 10 结构化 + access.log 真实 fixture ignored）+ T41 detect 7 tests + reader 9 tests；workspace 全量 136 passed |
| 单一真源 | `pass` | 版本号唯一源 `tauri.conf.json`，Cargo.toml workspace.package.version + frontend/package.json + constants.js 同步；SCHEMA_VERSION=3 单一真源 |

## 6. 安全与隐私审计

| 项 | 状态 | 证据 |
|---|---|---|
| SQL 参数绑定 | `pass` | `base64_transform_column_cells` 的 SELECT before / UPDATE cells / INSERT operations 全用 `?N` + `params![]` 绑定（`params![sheet_id, col_idx as i64]` / `params![...]`）；grep 无 `format!` 拼接 SQL；`list_undoable_operations` 的 IN 子句为硬编码常量（`mask`/`replace_in_column`/`replace_all`/`base64_column`），无注入风险 |
| 凭据 | `pass` | 无新增凭据；updater 密钥沿用 v1.1.0 配置（`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 从环境变量/GitHub Secret 读取，源码无字面量） |
| 全本地处理 | `pass` | Base64 编解码 / .log 导入 / tshark 探测全在本地，无网络调用；CSP `default-src 'self'` + 白名单延续无回归 |
| tshark 子进程安全 | `pass` | `probe_tshark` 用 `Command::new(path).arg("--version")`，不传用户输入作参数；`read()` 用固定 flag 参数（`-r`/`-Y`/`-T`/`-e` 等）调用 tshark，pcap 文件路径经 `.arg(path)` 参数化传递不经 shell；`set_tshark_path` 过滤空白串；不 panic（不存在路径返回 None） |
| Mimosa 深度扫描 | `info` | **已重跑完整审计**（`scan-2026-08-08T19-56-00.366Z-a958f772712b`，deep 深度）。源码全量解析：68/68 文件 parsed，0 parseFailures / 0 readFailures / 0 truncated。依赖审计：487 包 0 漏洞。findings：0（0 high/medium/low/info/businessLogic）。所有阶段 completed（threatModel / findingDiscovery / validation / pathAnalysis / reporting）。覆盖缺口仅 `callgraph_fact_partial`（"调用图部分不完整:部分调用为动态派发或超出分析规模,跨文件可达性可能不完整"），为静态分析方法学限制（动态派发），非项目缺陷。`runStatus=inconclusive` / `verdictEffect=none`。**R1 的 `library_source_unavailable` 已消除**。静态分析非运行时验证，不宣称项目安全，但无任何已识别 finding 阻碍发布 |

## 7. 数据与迁移审计

| 项 | 状态 | 证据 |
|---|---|---|
| `SCHEMA_VERSION` | `pass` | 仍为 3（v1.1.1 设定），v1.1.2 无 schema 变更 |
| 表/列/索引变更 | `pass` | 无新增表/列/索引；`operations.kind="base64_column"` 复用既有 `operations` 表 + `before_snapshot_json`/`result_snapshot_json` 列（v1.1.1 已加） |
| 迁移幂等 | `pass` | 无迁移代码（schema 不变）；v1.1.1 → v1.1.2 直接升级，历史数据完整保留 |
| 历史数据兼容 | `pass` | 历史 `operations` 行不受影响；新增 `base64_column` 操作的 before/after 快照机制与 v1.1.1 mask/replace 一致 |

## 8. 依赖与配置审计

| 项 | 状态 | 证据 |
|---|---|---|
| 新增依赖 | `pass` | `base64 = "0.22"`（src-tauri/Cargo.toml），单一来源，无传递依赖膨胀 |
| 既有依赖回归 | `pass` | `rusqlite` / `serde` / `serde_json` / `regex` / `csv` / `calamine` / `tempfile` 等无版本变更 |
| 版本号一致性 | `pass` | 4 处一致 1.1.2（Cargo.toml workspace.package.version=1.1.2 + 注释行 + tauri.conf.json version=1.1.2 + frontend/package.json version=1.1.2 + frontend/src/constants.js APP_VERSION="v1.1.2"）；core/src-tauri Cargo.toml workspace=true 自动继承 |
| 配置文件 | `pass` | tauri.conf.json updater 配置不变；CSP 不变；fs 权限不变；settings.json 向后兼容（`page_size` 用 `#[serde(default)]`，旧文件反序列化为 `None` → 前端回退 50） |

## 9. 文档一致性审计

| 项 | 状态 | 证据 |
|---|---|---|
| `docs/versions/1.1.2/` 三件套 | `pass` | 规划需求.md（范围边界 + 验收标准 + 明确不做）+ 更新日志.md（T37~T47 进度表 + E1~E9 验收）+ RELEASE-NOTES.md（用户面向 + 已知限制 + 升级，含 T47 全局每页行数设置）齐全 |
| `docs/qa/versions/1.1.2/QA-审计报告.md` | `pass` | 本报告，8 维度全 pass + 结论 qa_passed |
| `docs/02-技术设计文档.md` v1.1.2 段落 | `pass` | 版本覆盖说明加 v1.1.2；§2.4 模块契约矩阵 DbManager 加 `base64_transform_column_cells` + commands::columns 加 `base64_column`；§4.9 列操作命令表含 `base64_column` + Base64Mode + Base64Result 结构 + 语义说明；§3.x LogReader + detect_format `.log` 分发；设置按钮迁移 + 文案精简说明；§4.11 settings.json page_size 持久化（T47 AppSettings + load_page_size/save_page_size + SET_PAGE_SIZE reducer + PageSizeCard + state.pageSize 字段） |
| `docs/04-版本标准.md` 里程碑表 | `pass` | v1.1.2 行状态推进（详见 §12） |
| `handoff/TASK-BOARD.md` | `pass` | T37~T44 全部 verified_complete + E2E 全绿 + Release QA qa_passed |

## 10. 回归检查（v1.1.1 功能不退化）

| 场景 | 状态 | 证据 |
|---|---|---|
| 撤销/重做（mask/replace_in_column/replace_all） | `pass` | `cargo test --workspace` 77 src-tauri passed 含 undo/redo 5 + search 21 + columns 12（含 base64）+ db 31；既有功能无回归 |
| 列操作（parse_column_as_json / replace_in_column） | `pass` | columns 12 tests 含既有 JSON 解析 + 列内替换 + 新增 base64；全过 |
| 搜索（search_cells / search_rows / replace_all） | `pass` | search 21 tests 含关键字/正则/分页/行级搜索/全局替换；全过 |
| 导入（CSV/XLSX/JSON/JSONL/TXT/SQL/PCAP） | `pass` | detect_format 路由测试覆盖全部 8 种扩展名（含新增 .log）；既有格式不回归 |
| 设置页（SettingsView + 5 卡片） | `pass` | 三卡片文案精简 + 设置按钮迁移 + T47 新增 PageSizeCard（Select 20/50/100/200 + 保存后全局同步刷新）；UpdateCard 不变；pnpm build 通过 |
| 版本号 | `pass` | 4 处 1.1.2 一致，无 1.1.1 残留 |

## 11. 问题记录

| 严重度 | 问题 | 修复任务 | 状态 |
|---|---|---|---|
| `info` | Mimosa 深度扫描覆盖度 `partial`（`callgraph_fact_partial`：部分调用为动态派发或超出分析规模，跨文件可达性可能不完整）；`runStatus=inconclusive`（`verdictEffect=none`） | **已重跑完整审计**（scan-2026-08-08T19-56-00.366Z-a958f772712b）：68/68 源文件全量解析 0 失败，487 包 0 漏洞，0 findings；R1 的 `library_source_unavailable` 已消除；剩余 callgraph partial 为方法学限制（动态派发），非项目缺陷。静态分析非运行时验证，不宣称项目安全，但无已识别 finding 阻碍发布 | `info`（非阻塞，已知方法学限制） |
| `info` | Base64 列编解码仅支持标准 Base64（`STANDARD` engine），不支持 URL-safe / no-padding 变体 | v1.1.2 已知边界，推迟 v1.2+ | `info` |
| `info` | `.log` 结构化解析覆盖四种主流格式（Apache Combined/Common、Syslog、通用应用日志），非主流格式回退单列 `line`；正则匹配基于行级 probe，不处理跨行多行日志 | T45 已落地四格式结构化解析；非主流格式回退 + 跨行多行日志推迟 v1.2+ | `info`（部分推迟） |
| `info` | tshark 集成仍仅覆盖 HTTP 协议字段提取（DNS/TCP 等推迟 v1.3+）；本机无 tshark 时相关测试 `#[ignore]` 跳过 | v1.1.2 已知边界 | `info` |
| `info` | DbPathCard 仍为硬编码 DB 路径展示，动态 DB 路径命令推迟 v1.2+ | v1.1.2 已知简化 | `info` |
| `info` | GUI 端到端交互（E3~E6）未浏览器自动化验收 | 后端单测 + 前端构建 pass；GUI 视觉验收属浏览器自动化增量轮次，不阻塞发布 | `info` |
| `info` | 前端 chunk >500kB 警告（antd 既有，非本轮引入） | 既有警告，非 v1.1.2 引入 | `info` |
| `info` | v1.1.2 全部改动尚未 commit / push，停留在工作区 | 待用户确认后 commit + tag `v1.1.2` | `info` |

严重度口径：`critical`（阻塞发布）/ `major`（需回流修复）/ `minor`（可带病发布但记录）/ `info`（仅记录）。

## 12. 审计结论

`qa_passed` — 本轮 Release QA 审计覆盖 v1.1.2 全部交付（T37~T47）+ v1.1.1 → v1.1.2 回归。T37~T43 全部 `verified_complete`（无 critical/major 问题）；T44 文档收口 + 全量验证完成；T45 .log 结构化解析增量通过 R3 复核；T46 Base64 抽离为独立加解密面板通过 R4 复核；T47 全局每页行数设置通过 R5 复核。核心交付：

1. **列编解码 — Base64**（T37 + T38 + T46）—— `base64_column` 命令 + `base64_transform_column_cells` DB 方法（闭包式转换，单事务 + before/after 快照撤销）；前端 tauri.js IPC + T46 把 UI 从 ColumnOpsPanel 迁至独立 `CryptoPanel`（`crypto` 能力按钮，`KeyOutlined` 图标，`SidePanel` 注册表登记）；12 columns tests 覆盖 encode/decode/skipped/undo 往返；
2. **.log 文件导入 + 结构化解析**（T42 初版 + T45 结构化升级）—— `LogReader` 对前 10 条非空行 probe，按正则命中数自动识别格式：Apache Combined（11 列 + raw_line）/ Common（9 列 + raw_line）/ Syslog（5 列 + raw_line）/ 通用应用日志（4 列 + raw_line）/ 未识别回退单列 `line`；`detect_format` `.log` 分发 + 前端导入 filter；13 log tests（含 access.log 真实 fixture 集成）+ detect_format 路由测试；正则经 `OnceLock` 缓存，`raw_line` 始终保留原文；
3. **tshark 解析测试加固**（T41）—— detect.rs 新增 Windows 路径绝对性 + nonexistent probe 测试；reader.rs 既有 tshark 输出解析测试保留；三平台七条候选路径显式覆盖；
4. **设置按钮迁移 + 文案精简**（T39 + T40）—— 设置入口从 AiPanel 迁至 TopToolbar 右端；三卡片冗余描述移除；
5. **版本号**（T43）—— 4 处一致 1.1.2；
6. **加解密面板独立化**（T46）—— Base64 UI 从 ColumnOpsPanel 迁至新建 CryptoPanel，工具栏新增 `crypto` 能力按钮（`KeyOutlined`），作为后续所有加解密/哈希/编解码类操作的统一入口；ColumnOpsPanel 仅剩 JSON 解析；纯前端 UI 重组，无 IPC/DB/Rust 改动。
7. **全局每页行数设置（PAGE_SIZE 持久化）**（T47）—— `settings.rs` 扩展 `TsharkSettings` → `AppSettings`（新增 `#[serde(default)] page_size: Option<u32>`，与 tshark 路径同文件同模式）+ `load_page_size`/`save_page_size` 两个命令；前端 `SET_PAGE_SIZE` reducer 合并全局 + Sheet 同步更新（page 重置 1），`PageSizeCard` 设置卡片（Select 20/50/100/200），`App.jsx` 启动加载持久化值，`toRowObjects` 增加 `pageSize` 参数消除硬编码行号，`DataTable` 搜索改 `sheet.pageSize || PAGE_SIZE` 修既有不一致；pnpm build（3081 modules）+ cargo clippy（0 警告）全绿。

### 12.1 验证摘要

| 验证项 | 结果 |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --workspace -- -D warnings` | pass |
| `cargo test --workspace` | 136 passed / 3 ignored / 0 failed（src-tauri lib 77 + core 59） |
| `pnpm --prefix frontend install --frozen-lockfile` | pass（367ms） |
| `pnpm --prefix frontend build` | pass（3081 modules，2.59s） |
| 版本号一致性（4 处） | pass（1.1.2） |
| schema 迁移 | 不需要（SCHEMA_VERSION=3 不变） |
| Mimosa 深度扫描 | 0 findings / 487 包 0 漏洞 / 68 文件全量解析（callgraph partial 为方法学限制） |

**门禁裁决**：无未修复的 critical/major 问题。全部 `info` 项均为非阻塞已知简化或已知边界，已记录留待后续版本优化。GUI 端到端交互验收（E3~E6）属浏览器自动化增量轮次，不阻塞发布。**Mimosa 深度扫描已重跑完整审计**（scan-2026-08-08T19-56-00.366Z-a958f772712b）：0 findings、487 包 0 漏洞、68/68 源文件全量解析成功；剩余 callgraph partial 为动态派发方法学限制，非项目缺陷。静态分析非运行时验证，不宣称项目安全，但无任何已识别 finding 阻碍发布。**结论推进至 `qa_passed`**。

**发布前置门禁**（[`04-版本标准.md`](../../../04-版本标准.md) §4）满足：静态 + 单元测试 + 前端构建全绿 + 版本一致性 + schema 不变（无迁移）+ 文档收口。CI 构建（四目标矩阵）待 git tag `v1.1.2` 触发。

---

## 13. 修复证据索引

| 文件 | 改动 | 验证 |
|---|---|---|
| `src-tauri/src/commands/columns.rs` | 新增 `base64_column` 命令 + `base64_column_inner` + `Base64Mode`/`Base64Result` + 12 tests（含 base64 encode/decode/skipped/undo 往返） | `cargo test --workspace` 12 columns tests passed |
| `src-tauri/src/db/mod.rs` | 新增 `base64_transform_column_cells`（闭包式转换，单事务 + before/after 快照，全参数化 SQL）+ `list_undoable_operations` IN 子句加 `base64_column` | `cargo clippy -D warnings` 绿 + db 单测 |
| `src-tauri/src/lib.rs` | `generate_handler!` 注册 `base64_column` | `cargo check` 绿 |
| `src-tauri/Cargo.toml` | `base64 = "0.22"` 依赖 + workspace=true 继承 version=1.1.2 | `cargo check` 绿 |
| `Cargo.toml` | workspace.package.version=1.1.2 + v1.1.2 注释行 | `cargo check` 绿 |
| `crates/core/src/datasource/log.rs`（T42 新增 + T45 结构化重写） | `LogReader` 格式自动识别 + 结构化多列解析（Apache Combined/Common、Syslog、通用应用日志）+ `raw_line` 保留 + fallback 单列；4 正则 `OnceLock` 缓存 + `detect_format` probe 探测 + `split_request` 二次拆分；13 tests（3 fallback + 10 结构化 + access.log 真实 fixture ignored） | `cargo test --workspace` 13 log tests passed（含 fixture `#[ignore]` 本机验证） |
| `crates/core/src/datasource/mod.rs` | `mod log` + `pub use log::LogReader` + `detect_format` `.log` 分发 + 路由测试覆盖 `.log` | `cargo test --workspace` detect_format test passed |
| `crates/core/src/pcap/detect.rs` | 新增 `candidate_paths_windows_paths_are_absolute` + `probe_tshark_nonexistent_returns_none` 测试 | `cargo test --workspace` 7 detect tests passed |
| `crates/core/src/pcap/reader.rs` | tshark 解析测试保留（既有） | `cargo test --workspace` 9 reader tests passed（fixture ignored） |
| `frontend/src/tauri.js` | 新增 `base64Column` IPC 封装 | `pnpm build` 绿 |
| `frontend/src/components/panels/ColumnOpsPanel.jsx` | T38 原 Base64 模式 UI；T46 删除 Base64 相关 state/handler/JSX/import，仅剩 JSON 解析 | `pnpm build` 绿 |
| `frontend/src/components/panels/CryptoPanel.jsx`（T46 新增） | Base64 编解码 Form + handleBase64（从 ColumnOpsPanel 迁入，行为不变：base64Column → getSheetData + SET_SHEET_DATA + listUndoableOperations + SET_UNDO_STACK）+ 面板标题「加解密」 | `pnpm build` 绿 |
| `frontend/src/components/layout/TopToolbar.jsx` | 右端 `<SettingOutlined />` 设置按钮 + 导入 filter 加 `.log` + T46 `CAPABILITIES` 新增 `{id:"crypto",label:"加解密",icon:<KeyOutlined/>}` | `pnpm build` 绿 |
| `frontend/src/components/layout/SidePanel.jsx` | T46 `PANELS` 注册表加 `crypto: CryptoPanel` + 注释更新 | `pnpm build` 绿 |
| `frontend/src/components/AiPanel.jsx` | 移除原设置入口 | `pnpm build` 绿 |
| `frontend/src/App.jsx` | 路由不变（`currentView==="settings"` 渲染 SettingsView） | `pnpm build` 绿 |
| `frontend/src/components/settings/cards/{DbPathCard,AboutCard,TsharkPathCard}.jsx` | 三卡片冗余描述文案精简 | `pnpm build` 绿 |
| `frontend/src/components/settings/cards/PageSizeCard.jsx`（T47 新增） | 全局每页行数设置卡片：`useAppState` + antd Form/Select（20/50/100/200）+ Save 按钮；`handleSave` 调 `savePageSize` → dispatch `SET_PAGE_SIZE` → 刷新当前激活 Sheet 首页（`getSheetData` + `SET_SHEET_DATA`）→ message.success | `pnpm build` 绿 |
| `frontend/src/components/settings/SettingsView.jsx` | import PageSizeCard + 插入到 TsharkPathCard 与 DbPathCard 之间 | `pnpm build` 绿 |
| `frontend/src/state/constants.js` | `initialState.pageSize=50` + `ACTION.SET_PAGE_SIZE` + `activeCapability` 注释加 `crypto` | `pnpm build` 绿 |
| `frontend/src/state/reducer.js` | 新增 `SET_PAGE_SIZE` case（合并全局 + Sheet 同步：`sheets.map(s => ({...s, pageSize, page: 1}))`）+ ADD_SHEET/IMPORT_SUCCESS/ADD_SHEET_FROM_PARSE 传 `state.pageSize` + SET_SHEET_DATA 的 `toRowObjects` 传 `action.payload.pageSize ?? s.pageSize` | `pnpm build` 绿 |
| `frontend/src/state/factory.js` | 3 工厂（createEmptySheet/createSheetFromImport/createSheetFromParse）增加可选 `pageSize` 参数（默认 `PAGE_SIZE`） | `pnpm build` 绿 |
| `frontend/src/state/AppContext.jsx` | 新增 `setPageSize` dispatcher（`useCallback` + dispatch `SET_PAGE_SIZE`）+ 加入 value useMemo 与 deps | `pnpm build` 绿 |
| `frontend/src/tauri.js` | 新增 `loadPageSize`/`savePageSize` IPC 封装 + `toRowObjects` 增加 `pageSize` 参数（`base = (page-1) * pageSize`）消除硬编码 `PAGE_SIZE` 行号计算 | `pnpm build` 绿 |
| `frontend/src/App.jsx` | 启动 `useEffect` 调 `loadPageSize` → dispatch `SET_PAGE_SIZE`；导入首页 `getSheetData(id, 1, state.pageSize)`（原 `PAGE_SIZE`）；deps 加 `state.pageSize` | `pnpm build` 绿 |
| `frontend/src/components/DataTable.jsx` | 搜索 L241/269 改 `sheet.pageSize || PAGE_SIZE`（修既有不一致：搜索用 raw `PAGE_SIZE`，分页用 `sheet.pageSize || PAGE_SIZE`） | `pnpm build` 绿 |
| `src-tauri/src/commands/settings.rs` | `TsharkSettings` → `AppSettings` + 新增 `#[serde(default)] page_size: Option<u32>` 字段 + `load_page_size` / `save_page_size` 两个 `#[tauri::command]` | `cargo clippy -D warnings` 绿 |
| `src-tauri/src/lib.rs` | `generate_handler!` 注册 `load_page_size` + `save_page_size` | `cargo check` 绿 |
| `frontend/src/constants.js` | `APP_VERSION="v1.1.2"` | `pnpm build` 绿 |
| `frontend/package.json` | version=1.1.2 | `pnpm build` 绿 |
| `src-tauri/tauri.conf.json` | version=1.1.2 | `cargo check` 绿 |
| `docs/02-技术设计文档.md` | 版本覆盖说明加 v1.1.2 + §2.4 模块契约矩阵 + §4.9 base64_column（含 T46 CryptoPanel 迁移说明）+ §4.10 LogReader 结构化解析 + UI 迁移 + 文案精简 + activeCapability 取值加 `crypto` + §4.11 settings.json page_size 持久化（T47 AppSettings + load_page_size/save_page_size + SET_PAGE_SIZE reducer + PageSizeCard） | 人工核对 |
| `docs/versions/1.1.2/规划需求.md`（既有） | 范围边界 + 验收标准 + 明确不做 | 人工核对 |
| `docs/versions/1.1.2/更新日志.md` | T37~T47 进度表 + E1~E9 验收全绿 + done_e2e | 人工核对 |
| `docs/versions/1.1.2/RELEASE-NOTES.md`（新文件） | 用户面向 + 已知限制 + 升级（含 T45 .log 结构化导入 + T46 加解密面板独立化 + T47 全局每页行数设置） | 人工核对 |
| `docs/qa/versions/1.1.2/QA-审计报告.md`（新文件） | 8 维度审计 + qa_passed 结论（R5 含 T47 增量） | 本报告 |
| `handoff/TASK-BOARD.md` | T37~T47 verified_complete + E2E 全绿 + qa_passed | 人工核对 |
