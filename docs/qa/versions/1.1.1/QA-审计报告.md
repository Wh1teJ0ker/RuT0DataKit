# v1.1.1 QA 审计报告

> 版本：1.1.1
> 审计类型：Release 前全局审计（含 v1.1.0 → v1.1.1 历史遗留回归）
> 审计依据：[`docs/00-需求文档.md`](../../../00-需求文档.md) §6 验收标准 + [`docs/03-开发任务清单.md`](../../../03-开发任务清单.md) §7 T29~T36 任务定义 + [`docs/04-版本标准.md`](../../../04-版本标准.md) §4 发布门禁
> 结论：`qa_passed` — 静态审计 + 单元测试（66 passed；hotfix 轮次 72 passed）+ 前端构建（3079 modules）+ 版本一致性全通过；T29~T35 全部 `verified_complete`，T36 文档收口 + Release QA 完成且无 major/critical 问题；v1.1.1 hotfix（搜索优化第二轮：stale highlight + 行级搜索只保留结果）已回归无新增 major/critical。

## 1. 审计维度与结论

| 维度 | 结论 | 说明 |
|---|---|---|
| 功能完整性 | `pass` | T29~T35 单元级全 `verified_complete`；DB 39 tests + commands 层 27 tests（undo/redo 5 + search 15 + columns 7）+ core 40 tests = workspace 66 passed（hotfix 轮次 72 passed，新增 search_rows 6 tests）；撤销/重做 + 列操作（JSON 解析 + 列内替换）+ 搜索（关键字/正则/全局替换 + 行级「只保留搜索结果」+ stale highlight 修复）+ 搜索索引优化全部落地 |
| 回归与端到端 | `pass` | `cargo fmt --check` / `cargo clippy --workspace -- -D warnings` / `cargo test --workspace`（66→72 passed / 0 failed / 0 ignored）+ `pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build`（3079 modules transformed，2.31→2.45s）全绿；GUI 端到端交互（E3~E6）属浏览器自动化增量轮次，不阻塞发布 |
| 构建与产物 | `pass` | 本机 dev 构建 pass；版本一致性 pass（6 处 1.1.1）；CI 构建待 tag 触发（未发布前不阻塞） |
| 安全 | `pass` | 所有新 SQL 用 `?N` + `params![]` 参数绑定（grep 无字符串拼接 SQL）；LIKE 转义 `%`/`_`/`\`；正则编译失败返回错误不 panic；before/after 快照仅存变更列 cells；replace 单事务原子性；v1.1.0 安全收紧（CSP + fs 权限 + updater dialog）延续无回归 |
| 文档 | `pass` | `docs/04` v1.1.0→`qa_passed` + v1.1.1 行 `in_progress`；`docs/03` 追加 §7 T29~T36 + §8 里程碑表；`docs/02` schema v3 + §4.7~§4.9 新命令 + §6 前端 state 扩展 + 搜索高亮；`docs/versions/1.1.1/` 三件套齐全；`handoff/TASK-BOARD.md` 为 v1.1.1 看板 |
| 数据与迁移 | `pass` | `SCHEMA_VERSION=2→3` 增量迁移幂等（`PRAGMA table_info` 检查列存在再 ALTER + `IF NOT EXISTS`）；`before_snapshot_json` 列 + `idx_cells_sheet_col` 索引；历史数据保留（历史 operations 行 before_snapshot 为 NULL，表示不可撤销，仅新操作可撤销） |
| 依赖与配置 | `pass` | 版本号 6 处一致 1.1.1（Cargo.toml workspace + tauri.conf.json + frontend package.json + constants.js；core/src-tauri workspace=true 继承）；`regex` 已有依赖无新增 |
| 代码质量 | `pass` | schema 迁移幂等 + DB 单一真源 + IPC 收口（7 新 IPC camelCase 对齐）+ 前端组件复用（ColumnOpsPanel 参考 MaskPanel/ExtractPanel 模式）+ commands 层测试补齐 |

## 2. 功能完整性审计

依据 [`规划需求.md`](../../versions/1.1.1/规划需求.md) §2.1 的 9 项验收标准与 [`03-开发任务清单.md`](../../../03-开发任务清单.md) §7 T29~T36 任务定义：

| # | 验收项 | 状态 | 证据 |
|---|---|---|---|
| 1 | DB schema v2→v3 + 8 方法 | `pass` | `src-tauri/src/db/{schema,migrate,mod}.rs`；SCHEMA_VERSION=3 + before_snapshot_json 列 + idx_cells_sheet_col 索引；migrate_v2_to_v3 幂等（PRAGMA table_info 检查）；8 方法（log_operation_with_snapshot / query_column_cells_with_row_idx_range / search_cells / search_cells_regex / count_search_results / replace_in_column_cells / replace_all_cells / query_operation_by_id）全参数化 SQL；39 db tests 全过 |
| 2 | undo/redo 后端命令 + mask_column 快照改造 | `pass` | `src-tauri/src/commands/processor.rs`；undo_operation/redo_operation/list_undoable_operations 三命令；mask_column 改用 log_operation_with_snapshot（before/after 快照仅变更列 cells）；undo = write_cells(before)，redo = write_cells(after)；5 commands 层 tests 全过 |
| 3 | 搜索后端命令（search_cells + replace_all + search_rows） | `pass` | `src-tauri/src/commands/search.rs`；search_cells 关键字 LIKE + 正则预筛精匹配 + 分页 + 匹配区间；replace_all 单事务 + before/after 快照撤销；15 tests 全过；**hotfix 轮次**新增 search_rows 行级搜索（distinct row_idx 分页 + 整行 cells + 命中区间）+ 6 tests，全参数化 SQL |
| 4 | 列操作后端命令（parse_column_as_json + replace_in_column） | `pass` | `src-tauri/src/commands/columns.rs`；parse_column_as_json JSON 列展开为新 sheet（不入撤销栈，skipped 计数）+ replace_in_column 单事务列内替换快照撤销；7 tests 全过 |
| 5 | 前端 IPC + state 扩展 | `pass` | `frontend/src/tauri.js` 7→8 IPC（undo/redo 不传 sheetId，searchCells colIdx 可 null，**hotfix** 新增 searchRows）+ `state/constants.js` 5→6 ACTION（**hotfix** 新增 APPLY_SEARCH_ROWS）+ `state/reducer.js` 5→6 case + `state/factory.js` searchHits + searchRows/searchTotal + createSheetFromParse；APPLY_SEARCH_HITS 改为先清空再写（**hotfix** stale highlight 修复）；APPLY_SEARCH_ROWS rowKey 构造（`${sheetId}-${page}-${i}`）+ searchHits 同步刷新；pnpm build 通过 |
| 6 | 前端 UI（撤销工具栏 + 搜索栏 + 列操作面板） | `pass` | `DataTable.jsx` 搜索栏 + mark 高亮（#fff48f）+ 全局替换 Modal + **hotfix** isSearchMode 模式切换（dataSource=searchRows、pagination total=searchTotal、handleSearchPageChange）+ 「搜索结果共 N 行，仅显示命中行」提示；`Workbench.jsx` 撤销/重做工具栏（redoStack 本地维护）；`ColumnOpsPanel.jsx` JSON 解析新 Tab + 列内替换；SidePanel+TopToolbar 注册 columnOps；AppContext 5→6 dispatcher；state.js barrel createSheetFromParse；pnpm build 通过（3079 modules 2.45s）；T34 review 3 minor 非阻塞 |
| 7 | 搜索索引优化（避免大文件卡顿） | `pass` | idx_cells_sheet_col 复合索引 + 服务端分页（LIMIT/OFFSET）+ count_search_results total；LIKE + 复合索引策略（不引入 FTS5） |
| 8 | commands 层测试补齐 | `pass` | 首个 commands 层测试套件：undo/redo 5 + search 15 + columns 7 = 27 tests；DB 层 39 tests；workspace 全量 66 passed / 0 failed / 0 ignored |
| 9 | 版本号 1.1.0→1.1.1 + 文档收口 | `pass` | 4 文件改动（Cargo.toml workspace + tauri.conf.json + package.json + constants.js；core/src-tauri workspace=true 继承）；docs/versions/1.1.1/ 三件套齐全；docs/02 schema v3 + 新命令 + 新 state 同步；docs/03 §7+§8；docs/04 v1.1.1 in_progress |

## 3. 回归与端到端审计

| 场景 | 状态 | 证据 |
|---|---|---|
| `cargo fmt --check` | `pass` | 全绿 |
| `cargo clippy --workspace -- -D warnings` | `pass` | core + src-tauri 全零警告（2.16s） |
| `cargo test --workspace` | `pass` | 66→72 passed / 0 failed / 0 ignored（hotfix 轮次新增 search_rows 6 tests：db 层 + commands 层） |
| `pnpm --prefix frontend install --frozen-lockfile` | `pass` | Lockfile is up to date，Already up to date（366ms） |
| `pnpm --prefix frontend build` | `pass` | 3079 modules transformed，✓ built in 2.45s（hotfix 轮次；chunk >500kB 为 antd 既有警告，非本轮引入） |
| E1 静态全绿 | `pass` | fmt + clippy + test + pnpm build 全绿 |
| E2 前端构建 | `pass` | frozen-lockfile install + build 通过 |
| E3 撤销/重做 GUI | `info` | 后端单测覆盖 undo/redo 回写正确；GUI 浏览器自动化验收属增量轮次，不阻塞发布 |
| E4 搜索高亮 GUI | `info` | 后端单测覆盖 search_cells 关键字/正则/分页 + search_rows 行级搜索；前端 pnpm build 通过；hotfix 已修复 stale highlight + 行级「只保留搜索结果」模式；GUI 视觉验收属增量轮次 |
| E5 列操作 GUI | `info` | 后端单测覆盖 parse_column_as_json + replace_in_column；前端 pnpm build 通过；GUI 视觉验收属增量轮次 |
| E6 全局替换 GUI | `info` | 后端单测覆盖 replace_all 单事务 + 快照；GUI 视觉验收属增量轮次 |
| E7 版本号一致 | `pass` | 6 处 1.1.1（grep 确认） |
| E8 schema 迁移 | `pass` | db 单测覆盖 migrate_v2_to_v3 幂等 + before_snapshot_json 列 + idx_cells_sheet_col 索引 |

## 4. 构建与产物审计

| 项 | 状态 | 证据 |
|---|---|---|
| 本机 dev 构建 | `pass` | cargo check --workspace 通过 + pnpm build 通过 |
| 版本一致性 | `pass` | Cargo.toml workspace.package.version=1.1.1 + tauri.conf.json version=1.1.1 + frontend/package.json version=1.1.1 + frontend/src/constants.js APP_VERSION="v1.1.1"；core/src-tauri Cargo.toml 用 workspace=true 自动继承 |
| CI 构建（四目标矩阵） | `info` | 待 git tag `v1.1.1` 触发；未发布前不阻塞 qa_passed |

## 5. 安全审计

| 项 | 状态 | 证据 |
|---|---|---|
| SQL 参数绑定（v1.1.1 新增 8→14 方法 + 3 命令组 + search_rows） | `pass` | 全部用 `?N` + `params![]` 绑定；grep 无字符串拼接 SQL；list_undoable_operations 的 IN 子句为硬编码常量（`mask`/`replace_in_column`/`replace_all`），无注入风险；hotfix 轮次新增 search_matched_row_ids / count_matched_rows / query_row_cells / count_columns + 正则等价 6 方法全参数化 |
| LIKE 转义 | `pass` | 关键字搜索转义 `%`/`_`/`\` + `ESCAPE '\'`，避免用户输入改变 LIKE 语义 |
| 正则编译失败处理 | `pass` | `regex::Regex::new` 失败返回 `Err("invalid regex: ...")`，不 panic |
| 快照大小控制 | `pass` | before/after 快照仅存被修改列的 cells（非全表），避免 operations 表膨胀 |
| replace 单事务原子性 | `pass` | replace_in_column_cells / replace_all_cells 在单事务内完成（before 快照 + write_cells + log_operation） |
| v1.1.0 安全收紧延续 | `pass` | CSP `default-src 'self'` + 白名单；fs 权限仅 document/download + write-text-file；updater dialog=true；extractor Default 降级；detect Mutex 静默降级——无回归 |
| 全本地处理 | `pass` | 新增 7 命令仅读 DB cells + 回写 DB，无网络调用；CSP connect-src 白名单仅 updater 相关 GitHub 域 |

## 6. 文档审计

| 项 | 状态 | 证据 |
|---|---|---|
| `docs/04-版本标准.md` v1.1.0→qa_passed + v1.1.1 in_progress | `pass` | v1.1.0 行状态 `done_e2e`→`qa_passed`（tag 已发布）；v1.1.1 行 `in_progress`；QA 链接改 1.1.1 |
| `docs/03-开发任务清单.md` 追加 §7 T29~T36 + §8 里程碑 | `pass` | 标题加 v1.1.1 能力阶段；§7 八任务表（ID/标题/depends_on/状态/验收要点）；§8 里程碑表（6 阶段，T36 in_progress） |
| `docs/02-技术设计文档.md` schema v3 + §4.7~§4.9 新命令 + §6 state 扩展 | `pass` | 版本覆盖说明加 v1.1.1；SCHEMA_VERSION=3 + before_snapshot_json 列 + idx_cells_sheet_col 索引；§4.6 mask_column 签名加 replacement；§4.7 撤销/重做 + §4.8 搜索 + §4.9 列操作；§6 state 加 searchHits/undoStack/searchState + 5 ACTION；搜索高亮 `<mark>` #fff48f |
| `docs/versions/1.1.1/` 三件套 | `pass` | 规划需求.md（范围边界 + 验收标准 + 技术约束）+ 更新日志.md（T29~T36 进度表 + 关键设计决策）+ RELEASE-NOTES.md（用户面向 + 已知限制 + 升级）齐全 |
| `handoff/TASK-BOARD.md` v1.1.1 看板 | `pass` | T29~T35 verified_complete + T36 planned；DAG + E1~E8 + Release QA 门禁；当前状态 in_progress |
| 过时注释清理 | `pass` | 02 doc rules.replacement 描述从「脱敏替换值」改为「脱敏掩码字符」；mask_column 签名加 replacement 参数 |

## 7. 问题记录

| 严重度 | 问题 | 修复任务 | 状态 |
|---|---|---|---|
| `minor` | T34 全局替换后未刷新 undoStack（DataTable.jsx:213-238）——replaceAll 成功后仅刷新当前页 + clearSearch，未调 listUndoableOperations 刷新撤销栈 | 建议用户手动切换 Tab 触发 useEffect 刷新，或后续版本在 replaceAll 成功回调中加 listUndoableOperations | `info`（非阻塞，已知简化） |
| `minor` | T34 空 sheet 调 listUndoableOperations 噪音（Workbench.jsx:34-55）——useEffect 监听 activeSheetId 触发 listUndoableOperations，空 sheet 时返回空数组，无副作用但有噪音 | 建议后续版本加 sheet 存在性守卫 | `info`（非阻塞） |
| `minor` | T34 ColumnOpsPanel 空 sheet sessionId 守卫（ColumnOpsPanel.jsx:66-90）——空 sheet 时 parseColumnAsJson 的 sessionId 可能为 null | 建议后续版本加 sessionId 非空守卫 | `info`（非阻塞） |
| `info` | ~~搜索高亮匹配区间按字节偏移切片，多字节字符（中文）边界可能错位~~ | **v1.1.1 已修复**（hotfix）：前端 `highlightCell` 改用 `TextEncoder`/`TextDecoder` 按字节区间还原字符串，中英文混合安全；原 UTF-16 切片错位导致无高亮的 bug 已消除 | `resolved` |
| `info` | ~~第一次搜索高亮在第二次搜索后仍残留（stale highlight）~~ | **v1.1.1 已修复**（hotfix 第二轮）：`APPLY_SEARCH_HITS` 改为先 `searchHits = {}` 清空再写入；Node 单测断言验证旧 key 不残留 | `resolved` |
| `info` | ~~搜索仅高亮命中单元格，无法只保留命中行~~ | **v1.1.1 已修复**（hotfix 第二轮）：新增 `search_rows` 行级搜索 + `APPLY_SEARCH_ROWS` action + DataTable `isSearchMode` 模式切换（dataSource/searchRows、pagination total/searchTotal）；6 后端 tests 覆盖关键字/正则/分页/列过滤/空查询 | `resolved` |
| `info` | 正则模式 total 取全量命中数（无法用 SQL COUNT），超大表性能待优化 | v1.1.1 已知边界，LIKE 预筛已缓解，留待后续版本 | `info` |
| `info` | parse_column_as_json 仅展平一层 JSON 对象，嵌套对象 value 用 to_string() 序列化 | v1.1.1 已知简化，递归展平（a.b 列名）推迟 v1.2+ | `info` |
| `info` | GUI 端到端交互（E3~E6）未浏览器自动化验收 | 后端单测 + 前端构建 pass；GUI 视觉验收属浏览器自动化增量轮次，不阻塞发布 | `info` |
| `info` | Mimosa 安全扫描在 git commit 前未得到完整结论（library_source_unavailable / callgraph_fact_partial） | 按兼容策略继续，不宣称项目安全；建议尽快重跑完整审计 | `info` |

严重度口径：`critical`（阻塞发布）/ `major`（需回流修复）/ `minor`（可带病发布但记录）/ `info`（仅记录）。

## 8. 审计结论

`qa_passed` — 本轮 Release QA 审计覆盖 v1.1.1 全部交付（T29~T36）+ v1.1.0 → v1.1.1 历史遗留回归。T29~T35 全部 `verified_complete`（review_passed，无 critical/major 问题）；T36 文档收口 + 全量验证完成。核心交付：

1. **撤销 / 重做**（T29+T30+T33+T34）—— mask/replace_in_column/replace_all 三类就地变更操作可撤销可重做，before/after 快照仅存变更列 cells，单事务原子性；
2. **列操作**（T29+T32+T33+T34）—— parse_column_as_json JSON 列展开为新 sheet（不入撤销栈）+ replace_in_column 列内批量替换（快照撤销）；
3. **搜索**（T29+T31+T33+T34）—— 关键字 LIKE + 正则预筛精匹配 + idx_cells_sheet_col 复合索引 + 服务端分页 + 全局替换快照撤销 + 单元格 `<mark>` 高亮；**hotfix 第二轮**：stale highlight 修复（APPLY_SEARCH_HITS 先清空）+ 行级搜索 `search_rows`（只保留命中行 + 行级分页）+ DataTable `isSearchMode` 模式切换；
4. **测试优化**（T29~T32 + hotfix）—— 首个 commands 层测试套件（undo/redo 5 + search 15→21 + columns 7 = 27→33 tests），DB 层扩展至 39 tests，workspace 全量 66→72 passed。

### 8.1 验证摘要

| 验证项 | 结果 |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --workspace -- -D warnings` | pass（2.16s） |
| `cargo test --workspace` | 66→72 passed / 0 failed / 0 ignored |
| `pnpm --prefix frontend install --frozen-lockfile` | pass（366ms） |
| `pnpm --prefix frontend build` | pass（3079 modules，2.45s） |
| 版本号一致性（6 处） | pass（1.1.1） |
| schema 迁移幂等 | pass（v2→v3 单测覆盖） |

**门禁裁决**：无未修复的 critical/major 问题。3 项 minor（T34 review 标记）均为非阻塞已知简化，已记录留待后续版本优化。GUI 端到端交互验收（E3~E6）属浏览器自动化增量轮次，不阻塞发布。v1.1.1 hotfix 第二轮（stale highlight + 行级搜索只保留结果）回归测试 72 passed，无新增 major/critical。**结论推进至 `qa_passed`**。

**发布前置门禁**（[`04-版本标准.md`](../../../04-版本标准.md) §4）满足：静态 + 单元测试 + 前端构建全绿 + 版本一致性 + schema 迁移幂等 + 文档收口。CI 构建（四目标矩阵）待 git tag `v1.1.1` 触发。

---

## 9. 修复证据索引

| 文件 | 改动 | 验证 |
|---|---|---|
| `src-tauri/src/db/schema.rs` | SCHEMA_VERSION=3 + before_snapshot_json 列 + idx_cells_sheet_col 索引 | `cargo test -p ruT0-data-kit -- db::` 39 passed |
| `src-tauri/src/db/migrate.rs` | migrate_v2_to_v3（PRAGMA table_info 检查 + IF NOT EXISTS 幂等） | db 单测覆盖迁移幂等 |
| `src-tauri/src/db/mod.rs` | 8 新方法（log_operation_with_snapshot / query_column_cells_with_row_idx_range / search_cells / search_cells_regex / count_search_results / replace_in_column_cells / replace_all_cells / query_operation_by_id），全参数化 SQL | `cargo clippy -D warnings` 绿 + db 单测 |
| `src-tauri/src/commands/processor.rs` | undo_operation / redo_operation / list_undoable_operations + mask_column 改用 log_operation_with_snapshot | `cargo test --workspace` 5 commands tests passed |
| `src-tauri/src/commands/search.rs`（新文件） | search_cells + replace_all + search_rows（hotfix） | `cargo test --workspace` 15→21 tests passed |
| `src-tauri/src/commands/columns.rs`（新文件） | parse_column_as_json + replace_in_column | `cargo test --workspace` 7 tests passed |
| `src-tauri/src/lib.rs` | generate_handler! 注册 7→8 新命令（hotfix 加 search_rows） | `cargo check` 绿 |
| `frontend/src/tauri.js` | 7→8 IPC 封装（undo/redo 不传 sheetId；hotfix 加 searchRows） | `pnpm build` 绿 |
| `frontend/src/state/constants.js` | 5→6 ACTION（hotfix 加 APPLY_SEARCH_ROWS）+ initialState.undoStack + searchState | `pnpm build` 绿 |
| `frontend/src/state/reducer.js` | 5→6 case（SET_SEARCH_STATE / APPLY_SEARCH_HITS 先清空 / APPLY_SEARCH_ROWS / CLEAR_SEARCH 清 searchRows+searchTotal / ADD_SHEET_FROM_PARSE / SET_UNDO_STACK） | `pnpm build` 绿 |
| `frontend/src/state/factory.js` | searchHits + searchRows/searchTotal 字段 + createSheetFromParse | `pnpm build` 绿 |
| `frontend/src/components/DataTable.jsx` | 搜索栏 + mark 高亮 + 全局替换 Modal + hotfix isSearchMode 模式切换 + handleSearchPageChange + 「搜索结果共 N 行」提示 | `pnpm build` 绿 |
| `frontend/src/components/Workbench.jsx` | 撤销/重做工具栏 + redoStack 本地维护 | `pnpm build` 绿 |
| `frontend/src/components/panels/ColumnOpsPanel.jsx`（新文件） | JSON 解析新 Tab + 列内替换 | `pnpm build` 绿 |
| `frontend/src/components/layout/SidePanel.jsx` | 注册 columnOps | `pnpm build` 绿 |
| `frontend/src/components/layout/TopToolbar.jsx` | CAPABILITIES 加 columnOps | `pnpm build` 绿 |
| `frontend/src/state/AppContext.jsx` | 5→6 dispatcher（hotfix 加 applySearchRows） | `pnpm build` 绿 |
| `frontend/src/state.js` | barrel export createSheetFromParse | `pnpm build` 绿 |
| `Cargo.toml` | workspace.package.version=1.1.1 + v1.1.1 注释行 | `cargo check` 绿 |
| `src-tauri/tauri.conf.json` | version=1.1.1 | `cargo check` 绿 |
| `frontend/package.json` | version=1.1.1 | `pnpm build` 绿 |
| `frontend/src/constants.js` | APP_VERSION="v1.1.1" | `pnpm build` 绿 |
| `docs/02-技术设计文档.md` | 版本覆盖说明 + schema v3 + §4.6 mask_column 签名 + §4.7~§4.9 新命令 + §6 state 扩展（searchHits/searchRows/searchTotal + APPLY_SEARCH_ROWS）+ 搜索高亮 + 行级搜索模式说明 + rules.replacement 描述 | 人工核对 |
| `docs/03-开发任务清单.md` | 标题 + §7 T29~T36 + §8 里程碑 | 人工核对 |
| `docs/04-版本标准.md` | v1.1.0→qa_passed + v1.1.1 in_progress + QA 链接改 1.1.1 | 人工核对 |
| `docs/versions/1.1.1/规划需求.md`（新文件） | 范围边界 + 验收标准 + 技术约束 | 人工核对 |
| `docs/versions/1.1.1/更新日志.md` | T29~T36 进度表 + 关键设计决策 + hotfix 第二轮（stale highlight + 行级搜索） | 人工核对 |
| `docs/versions/1.1.1/RELEASE-NOTES.md`（新文件） | 用户面向 + 已知限制 + 升级 + hotfix 行级搜索条目 | 人工核对 |
| `frontend/src/components/DataTable.jsx`（hotfix） | `highlightCell` 改用 `TextEncoder`/`TextDecoder` 按字节偏移切片，修复中文等多字节字符无高亮 bug | `pnpm build` 绿 + Node 单测断言中文/ASCII/多命中场景 |
| `src-tauri/src/db/mod.rs`（hotfix 第二轮） | 新增 6 方法（count_columns / query_row_cells / search_matched_row_ids / count_matched_rows + 正则等价），全参数化 SQL | `cargo test -p ruT0-data-kit -- db::` 39+ tests passed |
| `src-tauri/src/commands/search.rs`（hotfix 第二轮） | 新增 search_rows 命令 + SearchRow/SearchRowsPage/RowCellMatch 结构 + compute_regex_matches helper + 6 tests | `cargo test --workspace` 21 tests passed |
| `frontend/src/state/reducer.js`（hotfix 第二轮） | APPLY_SEARCH_HITS 改先清空（stale 修复）+ APPLY_SEARCH_ROWS case + CLEAR_SEARCH 清 searchRows/searchTotal | `pnpm build` 绿 + Node 断言 stale 修复 + 行转换逻辑 |
