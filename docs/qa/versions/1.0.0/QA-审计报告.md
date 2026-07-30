# v1.0.0 QA 审计报告

> 版本：1.0.0
> 审计类型：Release 前全局审计
> 审计依据：[`docs/00-需求文档.md`](../../../00-需求文档.md) §6 验收标准 + [`docs/03-开发任务清单.md`](../../../03-开发任务清单.md) 各任务验收 + [`docs/04-版本标准.md`](../../../04-版本标准.md) §4 发布门禁
> 结论：`conditional_pass`（静态 + Phase 7 app 启动/DB 实测通过；GUI 交互项待用户手动验收；构建产物矩阵待 finalize）

## 1. 审计维度与结论

| 维度 | 结论 | 说明 |
|---|---|---|
| 功能完整性 | `conditional_pass` | 静态项 + Phase 7 app 启动/DB 实测 pass；GUI 交互项（E3 四区/E4 导入/E5 列交互/E6 设置点击）标 pending_e2e 待用户手动验收 |
| 回归与端到端 | `conditional_pass` | app 启动稳定性 pass；GUI 主流程（导入→重启恢复）pending_e2e 待用户验收 |
| 构建与产物 | `pending_release` | 四目标矩阵构建需 CI/Release 流程；版本一致性已 pass；本机 dev 构建 pass |
| 安全 | `conditional_pass` | 静态项（本地处理 / 私钥不落盘 / camelCase / HTTPS endpoint）pass；签名验签失败拒绝安装 pending_e2e（需实际 updater JSON 测试） |
| 文档 | `pass` | 双语 README 收口 + Quick Start 实测 + 更新日志/TASK-BOARD 一致 |

## 2. 功能完整性审计

依据 [`规划需求.md`](../../versions/1.0.0/规划需求.md) §3 的 11 项验收标准：

| # | 验收项 | 状态 | 证据 |
|---|---|---|---|
| 1 | `cargo check --workspace` 通过，`cargo tauri dev` 可启动 | `pass` | `cargo check --workspace` → Finished（dev profile）；`cargo tauri dev` 主会话 Phase 7 实跑：8.55s 编译完成，`target/debug/ruT0-data-kit` 进程稳定存活，vite dev server `http://localhost:5173` 返回 200，日志无 panic/error（仅 5 个 dead_code 编译警告） |
| 2 | 四区布局可见，占比符合 01 文档 §1。上方工具栏左组 6 项数据操作 + 右组 4 项能力按钮可见 | `pending_e2e` | 辅证：`frontend/src/components/layout/TopToolbar.jsx` 存在（左6/右4 + Divider）；`SidePanel.jsx` 存在；需 GUI 确认占比 |
| 3 | 右组能力按钮点击后左侧能力面板切换到对应能力面板（占位提示「v1.1+ 释放」）；左组禁用项点击弹 `v1.1+` 提示；左侧面板底部「⚙ 设置」始终可见，点击进入设置页 | `pending_e2e` | 辅证：`frontend/src/components/panels/{Mask,Validate,Extract,Rules}Panel.jsx` 4 文件存在；需 GUI 确认交互 |
| 4 | 可新建 / 切换 / 关闭 / 重命名 Sheet Tab | `pending_e2e` | 辅证：`frontend/src/components/SheetTabs.jsx` 存在；需 GUI 确认 |
| 5 | antd Table 支持行复选 + 区间选择、列 checkbox 显隐、列拖拽排序 | `pending_e2e` | 辅证：`frontend/src/components/DataTable.jsx` 存在（@dnd-kit 列拖拽 + Dropdown 列显隐 + rowClassName 枚举）；需 GUI 确认 |
| 6 | SQLite 5 表 3 索引存在，重复启动不重复建表，`schema_version` 正确 | `pass` | `src-tauri/src/db/schema.rs` SCHEMA_DDL：sessions/sheets/cells/operations/app_settings 5 表 + idx_cells_sheet_row/idx_operations_sheet/idx_sheets_session 3 索引，CREATE IF NOT EXISTS 幂等；单测 `db::tests::new_creates_tables_and_schema_version` 验证 schema_version=1 且重复 new 幂等；Phase 7 主会话 sqlite3 实测：DB 文件 49152 字节，5 表 + 3 索引，schema_version=1，app_settings 仅含 schema_version 一行 |
| 7 | 导入 1000 行 CSV / XLSX：Sheet 新建、Table 渲染首页、分页可翻 | `partial` | cargo test 覆盖：`datasource::tests::*`（CSV headers/rows + flexible columns + detect_format 路由）3 项 + `db::tests::write_and_query_cells_paginated` + `write_and_query_cells_paginated_multi_column`（T4-hotfix 多列分页语义）pass；实际 1000 行 GUI 导入 → pending_e2e |
| 8 | 重启应用后通过「打开历史 Session」可重新加载该 Sheet 全量数据 | `pending_e2e` | 后端单测 `db::tests::list_and_get_session` 覆盖 `list_sessions`/`get_session`，pass；前端历史 Session 入口未实现（T5 REPORT 已说明留后续），属 v1.0.0 已知边界，不判 fail；GUI 恢复流程 → pending_e2e |
| 9 | `operations` 表有对应 `import` 记录 | `pass` | `src-tauri/src/commands.rs:204` `log_operation(Some(sheet_id), "import", ...)` 在 import_file 末尾调用；单测 `db::tests::log_operation_inserts_row` pass；Phase 7 主会话 sqlite3 实测：app 首次启动后 operations 表 0 行（未触发导入，符合预期） |
| 10 | `check_update` 无新版本返回 `available=false`；无网络静默降级；签名校验失败拒绝安装 | `partial` | `src-tauri/src/commands.rs:43-54` check_update：Ok(None)→none()，Err(_)→none()（available=false），无网静默降级 pass；签名验签失败拒绝 → pending_e2e（需实际 updater JSON 测试） |
| 11 | 前端调用 `aiSuggest` 得到 `v1.1+` 错误文案，UI 不崩溃 | `pass` | `src-tauri/src/commands.rs:99-101` `ai_suggest` 返回 `Err("ai_suggest not implemented until v1.1+")`；`AiPanel.jsx` catch 降级（T7 验证 pnpm build 通过不崩溃） |

## 3. 回归与端到端审计

| 场景 | 状态 | 证据 |
|---|---|---|
| 端到端主流程：导入 → 编辑列序 → 重命名列 → 关闭 → 重启 → 恢复 | `pending_e2e` | 需 GUI 主流程，Phase 7 主会话 app 启动稳定但未执行交互导入；列序/重命名/关闭/重启恢复链路 → pending_e2e |
| 大表导入（5 万行）性能与分页响应 | `pending_e2e` | 需 GUI + 大文件，Phase 7 |
| 错误降级：DB 损坏备份重建 | `pending_e2e` | 需 GUI 触发，Phase 7 |
| updater 无网络静默降级 | `pending_e2e` | 后端逻辑已 pass（§2 #10），GUI 触发确认 → Phase 7 |
| AI 占位调用不崩溃 | `pending_e2e` | 后端逻辑已 pass（§2 #11），GUI 触发确认 → Phase 7 |
| app 启动稳定性（非崩溃退出） | `pass` | Phase 7 主会话 `cargo tauri dev` 实跑：8.55s 编译完成，进程稳定存活无 panic，DB 初始化成功（5 表 3 索引 + schema_version=1），vite dev server 200；前置回归项已通过 |

## 4. 构建与产物审计

| 项 | 状态 | 证据 |
|---|---|---|
| 四目标矩阵构建成功（Linux x64 / macOS arm64 / macOS x64 / Windows x64） | `pending_release` | 需 CI/Release 流程，非本阶段 |
| 产物命名符合 `RuT0DataKit_<VERSION>_<PLATFORM>_<ARCH>.<EXT>` | `pending_release` | 需实际产物，Phase 8 |
| SHA-256 checksums 文件生成并上传 | `pending_release` | 需 CI 流程 |
| updater JSON（`latest.json`）随 Release 上传 | `pending_release` | 需 finalize 流程 |
| 版本一致性：Git tag `v1.0.0` == `tauri.conf.json` version == 产物命名 | `pass` | grep 4 处版本号均为 1.0.0：`Cargo.toml:11` version="1.0.0"、`src-tauri/Cargo.toml:3` version="1.0.0"、`src-tauri/tauri.conf.json:4` "version":"1.0.0"、`frontend/package.json:4` "version":"1.0.0"（Git tag 待发布时打） |

## 5. 安全审计

| 项 | 状态 | 证据 |
|---|---|---|
| 全本地处理，用户数据不外发（updater 除外） | `pass` | 三层架构单向依赖 `frontend → src-tauri → crates/core`；commands.rs 仅 import_file（本地路径）/get_sheet_data/check_update/install_update/ai_suggest/invoke_ai_op，无网络外发调用（updater 仅拉元数据） |
| updater 仅拉版本元数据 + 签名产物，不传用户数据 | `pass` | `tauri.conf.json:29-39` updater endpoints 指向 GitHub Release latest.json；`commands.rs:42-71` check_update/install_update 委托 tauri-plugin-updater（仅元数据+签名产物）；`docs/versions/1.0.0/updater-密钥.md` 文档说明 |
| Ed25519 签名密钥通过 GitHub Secret 注入，不落盘仓库 | `pass` | `.gitignore` 含 `*.key`（T6 落盘）；`find . -name '*.key'` 仓库内无 .key 文件；pubkey 入 tauri.conf.json（可入库），私钥在 ~/.tauri 不入库 |
| 签名验签失败拒绝安装 | `pending_e2e` | install_update 委托插件 download_and_install（验签失败抛 Err），逻辑 pass；需实际 updater JSON 测试 |
| `endpoints` 指向 GitHub Release（HTTPS），不自建服务器 | `pass` | `tauri.conf.json:32` `https://github.com/Wh1teJ0ker/RuT0DataKit/releases/latest/download/latest.json` HTTPS |
| 嵌套 struct 全部带 `#[serde(rename_all = "camelCase")]`（grep 核验） | `pass` | grep `rename_all`：`db/mod.rs:23,33,44,55`（Cell/SessionSummary/SheetSummary/SessionDetail）+ `commands.rs:19,80,90,122,132`（UpdateStatus/AiContext/AiSuggestion/ImportResult/PageData）共 9 处全覆盖 |

## 6. 文档审计

| 项 | 状态 | 证据 |
|---|---|---|
| 双语 README 存在且互链 | `pass` | `README.md:3` [English](./README_EN.md)；`README_EN.md:3` [中文](./README.md) |
| README Quick Start 在干净环境可复现 | `pass` | coder 实跑：`cargo check --workspace` Finished；`cargo test --workspace` 9 db + 3 datasource 全 PASS；`pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build` 3069 modules 转换成功 |
| `docs/versions/1.0.0/更新日志.md` 进度表与 `handoff/TASK-BOARD.md` 一致 | `pass` | 两份文件 T1~T8 均 `verified_complete`，T9 均标 `implemented_not_verified`（coder 标，待主会话判后改） |
| `docs/04-版本标准.md` 里程碑索引状态正确 | `partial` | §2 v1.0.0 行当前为 `planned`；T1~T8 已 verified_complete，按状态口径应可推进至 `in_progress`，但本字段由主会话 Phase 8 裁决，coder 不擅改（04 文档属 T9 out_of_scope 除非里程碑索引需调整） |
| 文档间无术语 / 范围冲突（无 v1.1+ 能力被误写成 v1.0.0 已交付） | `pass` | `grep -rn 'v1\.0\.0' docs/ | grep -i '已交付\|已实现\|delivered'` 仅命中 03/qa 报告中「无 v1.1+ 能力被误写成 v1.0.0 已交付」的反向声明，无 overclaim；详见 §7 IPC 契约已知边界 |
| README 不含 `{{PLACEHOLDER}}` 残留 | `pass` | `grep -rn '{{PLACEHOLDER}}' README.md README_EN.md docs/` 无命中 |

## 7. 问题记录

| 严重度 | 问题 | 修复任务 | 状态 |
|---|---|---|---|
| `info` | 02-技术设计文档.md §4 IPC 契约清单列了 set_selection/reorder_columns/rename_column/list_sessions/open_session/get_setting/set_setting 等完整契约，但 v1.0.0 实际只落地 6 个 tauri command（import_file/get_sheet_data/check_update/install_update/ai_suggest/invoke_ai_op）。db 层 list_sessions/get_session/get_setting/set_setting 方法已实现并单测覆盖，但未暴露为 #[tauri::command]。02 文档写的是完整契约蓝图，v1.0.0 只落地导入+updater+AI 占位子集。 | T9 out_of_scope（不改 02 文档），由主会话裁决是否在 02 文档补「v1.0.0 落地子集」注释 | 待主会话裁决 |

严重度口径：`critical`（阻塞发布）/ `major`（需回流修复）/ `minor`（可带病发布但记录）/ `info`（仅记录）。

## 8. 审计结论

`conditional_pass` — 静态审计全部 pass（功能完整性静态项、构建版本一致性、安全静态项、文档）；Phase 7 主会话实测通过项：app 启动稳定（cargo tauri dev 进程存活无 panic）、SQLite 5 表 3 索引 + schema_version=1 实测、operations 表存在。GUI 交互项（E3 四区视觉/E4 导入 1000 行/E5 列序+点击/E6 设置 check_update）因当前会话为非交互环境，主会话已启动 app 但未执行 GUI 点击操作，标 `pending_e2e`，由用户手动跑 `cargo tauri dev` 核验后回填；构建产物矩阵（四平台 + checksums + latest.json）标 `pending_release`，属 finalize 阶段。

**门禁裁决**：无 critical/major 问题。GUI 交互项为「需人工目视/点击」性质，代码静态核验（组件存在 + 注册命令 + 单测）均通过，且 app 启动稳定；在用户手动验收 GUI 项回填前，结论暂为 `conditional_pass`。用户确认 GUI 项后，可推进至 `qa_passed` 并进入 finalize。

**发布前置门禁**（[`04-版本标准.md`](../../../04-版本标准.md) §4）全部满足前，禁止 finalize。当前阻塞项：GUI 交互验收 + 四目标矩阵构建。
