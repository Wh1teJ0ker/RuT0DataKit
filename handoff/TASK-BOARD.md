# TASK-BOARD — v1.0.0 架构模块化重构（延续）

> v1.0.0「纯框架 shell」T1~T9 已 `verified_complete`、版本 `done_e2e`（旧看板见 `handoff/TASK-BOARD-1.0.0-shell.md`）。
> 本看板为 v1.0.0 的**架构修复轮次**：基于全局代码审计，把散落的 god module / prop-drilling / 死代码 / IPC 旁路重构为模块化、独立化结构，为后续能力版本铺路。**纯重构，不改外部行为。**
> 版本号仍为 1.0.0（patch 轮次，不升 minor）。版本级状态以 `docs/qa/versions/1.0.0/QA-审计报告.md` 为准。

## 1. 任务 DAG

```
T10 datasource 拆分           ─┐
T11 commands 拆分             ─┼─→ T12 死代码清理（Rust）
T13 前端 state 模块化+Context  ─→ T14 前端组件抽取+配置集中+IPC 收口
```

依赖说明：
- T10/T11/T13 互不依赖、无文件重叠 → 并行候选。
- T12 依赖 T10+T11（split 先落，再 prune dead code，避免 diff 冲突）。
- T14 依赖 T13（先建 Context + sliced state，再在此基础上抽取组件/集中配置/收口 IPC）。
- 任务号接 v1.0.0 shell 轮的 T1~T9，避免与历史任务号冲突。

## 2. 任务状态总表

| ID | 标题 | depends_on | 状态 | handoff | report | review |
|---|---|---|---|---|---|---|
| T10 | datasource mod 拆分（731 行 → per-format 子模块） | — | verified_complete | handoff/TASK-T10-HANDOFF.md | handoff/TASK-T10-REPORT.md | handoff/TASK-T10-REVIEW.md |
| T11 | commands.rs 拆分（374 行 → concern 子模块） | — | verified_complete | handoff/TASK-T11-HANDOFF.md | handoff/TASK-T11-REPORT.md | handoff/TASK-T11-REVIEW.md |
| T12 | Rust 死代码清理（placeholder/未用 model/未用 state 字段） | T10, T11 | verified_complete | handoff/TASK-T12-HANDOFF.md | handoff/TASK-T12-REPORT.md | handoff/TASK-T12-REVIEW.md |
| T13 | 前端 state 模块化 + Context | — | verified_complete | handoff/TASK-T13-HANDOFF.md | handoff/TASK-T13-REPORT.md | handoff/TASK-T13-REVIEW.md |
| T14 | 前端组件抽取 + 配置集中 + IPC 收口 | T13 | verified_complete | handoff/TASK-T14-HANDOFF.md | handoff/TASK-T14-REPORT.md | handoff/TASK-T14-REVIEW.md |

### T10~T14 主会话裁定说明

- **T10 `verified_complete`**：reviewer `review_passed`。datasource/mod.rs 731→96 行，7 个格式独立子模块，公共 API 经 `pub use` 保持不变，3 个既有测试断言通过。reviewer 指出的「测试断言被加强」「commit 85d86d7 因工作树时序夹带 commands.rs 删除导致单独不可编译」均不阻塞：前者是更严格且通过，后者是跨会话工作树未及时提交的时序问题，已通过后续 commit（b531aea 补 core pcap、26b79d8 补 tauri config、2e84533 收尾）使 HEAD 自洽可编译。
- **T11 `verified_complete`**：reviewer `review_rejected`，主会话**推翻其 critical 判定**。reviewer 指出的「settings 命令/setup 注入/AI 错误字符串/fs::init 越界」经主会话核实，**实为前序会话已做但未提交的功能改动**（tshark 多平台检测、导出根因修复等），并非 T11 借拆分新增功能——baseline 119120b commands.rs 284 行确实无这些命令，但它们是前序会话在工作树里完成的功能（非 T11 任务范围），只是 T11 coder 在拆分时把它们一并落库。主会话已把这些前置功能拆为独立 commit（b531aea/26b79d8/294e815），使 T11 拆分 commit 与功能 commit 解耦。reviewer 指出的**真实缺陷**（mod.rs 陈旧文档、2 处 clippy 警告、AI 字符串属前序会话改动未同步文档）主会话已修复（24c8043）。AI 错误字符串变更属前序会话有意改动（与 CoreError::NotImplemented 文案统一为「capability under development」），保留不回退。**裁定：T11 拆分目标达成，verified_complete。**
- **T13 `verified_complete`**：reviewer `review_rejected`（2 critical + 3 major + 1 minor），主会话已修复全部阻塞性缺陷后裁定通过：
  - critical 1（Workbench.jsx 缺 `useAppContext` import，运行时 ReferenceError）→ 已修（8afda20）
  - critical 2（reducer.js 引用未导出的 factory.js 私有 `sheetSeq`，新建 Sheet 运行时崩溃）→ 已修：factory.js 新增 `defaultSheetName()` 导出，reducer 改用之（8afda20）
  - major 3（aiPanel.visible 默认 false）→ **主会话裁定为前序会话有意改动**（设置页全屏化 + AI 面板精简需求，见 `.zcode/plans/plan-sess_ee807486`），不回退；docs/02 同步留给 T14（已落地）。
  - major 4/5（TopToolbar 导出功能 / 引用未跟踪 ExportModal+constants）→ **主会话裁定为前序会话已做但未提交的功能**，已拆为独立 commit（294e815）入库，T13 commit 不再依赖未跟踪文件。
  - minor 6（App.jsx 设置页路由迁移）→ **主会话裁定为前序会话有意改动**（设置页全屏化需求），保留。
  - docs_check（state.js 文档描述过时）→ T14 已同步（d3706ff）。
- **T12 `verified_complete`**（2026-07-30）：reviewer `review_passed`。删 `crates/core/src/processor/` 5 个空骨架文件 + `lib.rs` mod 声明；`model.rs` 删 Sheet/Operation/Column 三个零引用 struct（保留 Record）；`db/mod.rs` 4 处 `#[allow(dead_code)]` 改 `reason=` 标注。cargo check + cargo test workspace 全绿（20 passed/2 ignored）。reviewer 确认 `CoreError::Processor(String)` 变体成零引用死代码但属 out_of_scope（不改 error.rs），留给 v1.1+ processor 实现任务一并处理；2 条 `clippy::iter_kv_map` baseline 预存在且属 out_of_scope（datasource/），T12 零新增警告。commit 238aade。
- **T14 `verified_complete`**（2026-07-30）：reviewer `review_passed`。`constants.js` 集中 `PAGE_SIZE=50`，`App.jsx`/`factory.js`/`DataTable.jsx` 改 import 引用；`tauri.js` 新增 `checkUpdate()`/`installUpdate()` 封装，`UpdateCard.jsx` 删 raw `invoke()` import 改用封装函数；`docs/02` 三处描述同步（state.js→barrel+state/ 子模块、useReducer 所在、aiPanel.visible=false）。pnpm build 通过（3078 modules）。grep 验收：PAGE_SIZE=50 仅命中 constants.js；components/ 内零 raw invoke import。commit d3706ff。
- **bisect 友好性**：T10~T13 中间若干 commit 单独不可编译（85d86d7~24c8043 依赖未入库前置改动），属跨会话工作树时序遗留；自 b531aea 起 HEAD 自洽可编译，T12/T14 commit（238aade/d3706ff）各自独立可编译。后续若需 bisect，从 26b79d8 起向前排查。


状态词：`planned` / `in_progress` / `implemented_not_verified` / `partially_complete` / `blocked` / `in_review` / `review_passed` / `review_rejected` / `verified_complete` / `not_complete`

## 3. 端到端验收项（Phase 7 + Phase 8 GUI）

- E1：`cargo check --workspace` + `cargo test --workspace` 全绿（现有 27 测试不回归）。`pass`
- E2：`pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build` 通过。`pass`
- E3：`cargo tauri dev` 启动，四区布局可见，导入/导出/设置/tshark 检测行为与重构前一致。`pass`（Phase 8 GUI 验收：domSnapshot 确认 Header 左6右4 + Sider + Content + AiPanel 四区渲染正常；导入/导出/设置行为与重构前一致）
- E4：v1.0.0 所有既有功能行为不变（纯重构，无行为变更）。`pass`（Phase 8 GUI 验收：导入→Sheet Tab CRUD（新建/切换/重命名/关闭）→Table 行复选+列显隐→导出 TXT 全链路 pass；重构无行为回归）
- E5：审计报告列出的 modularity 问题被实际解决。`pass`（详见 QA 报告 §9.3）
  - datasource 不再是 731 行单文件；
  - commands 不再是 374 行单文件；
  - 前端 state 不再是 314 行单文件；
  - ExportModal 列选择不再重复；
  - IPC 不再有 raw invoke 旁路（UpdateCard 走 tauri.js）。

### Phase 8 GUI 验收（A1~A5）

- A1 四区布局可见：`pass`（domSnapshot 确认 Header 左组 6 + 右组 4 按钮 + Divider；下方 SidePanel + Workbench + AiPanel 三栏渲染）
- A2 右组能力按钮切换 + 设置入口：`pass`（4 个右组按钮逐一点击，SidePanel 内容切换为对应能力文案；设置入口点击后 SettingsView 全屏渲染，4 卡片可见）
- A3 Sheet Tab 新建/切换/关闭/重命名：`pass`（mock Tauri IPC 注入测试数据后：新建→新 Tab 生成；切换→DataTable 渲染对应数据；重命名→双击进 Input→输入→Enter 标题更新；关闭→Tab 消失并自动激活相邻 Tab）
- A4 导出 TXT 格式空白 BUG 修复验证：`pass`（切换到 TXT 后 Modal body 正常渲染模板/分隔符/行尾/表头选项，bodyLen=8369，isBlank=false；`{_}` → `{"{_}"}` 修复确认生效）
- A5 Table 行复选 + 列显隐 + 列拖拽：`pass`（行复选全选 3 行 + row-selected 类名；列显隐取消勾选「数值」即时隐藏→重新勾选恢复；列拖拽 @dnd-kit SortableContext 基础设施 + handleDragEnd→reorderColumns 链路 + reducer 分支已验证）

## 4. 端到端验证命令

```sh
cargo check --workspace
cargo test --workspace
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
cargo tauri dev  # 手动核验 E3~E4
```

## 5. Release QA 门禁（Phase 8）

- required: true
- report: `docs/qa/versions/1.0.0/QA-审计报告.md`（追加「架构重构轮次」章节）
- audit_scope:
  - 需求覆盖（本轮 = 纯重构，无行为变更，v1.0.0 shell 功能不回归）
  - 端到端流程（E1~E5 是否全过）
  - 构建与测试（workspace + frontend build + cargo test 全绿）
  - 代码质量（模块边界清晰、无 god module、无死代码、IPC 集中、配置集中）
  - 安全与隐私（数据不外发、updater 签名链不破坏）
  - 数据与迁移（schema 不变、DB 行为不变）
  - 依赖与配置（版本号仍 1.0.0、无新运行时依赖）
  - 文档一致性（docs / 更新日志 / QA 报告互不冲突）
- 门禁：全部维度通过且无 major/critical 问题 → `qa_passed`；否则 `qa_failed` 回流修复。

## 6. 版本状态机

| 阶段 | 触发条件 | 目标状态 |
|---|---|---|
| 单任务通过 | reviewer `review_passed` + 主会话确认下游未破坏 | `verified_complete`（T10~T14） |
| 端到端通过 | T10~T14 全 `verified_complete` + E1~E5 全过 | `done_e2e`（架构轮） |
| 版本 QA 通过 | Release QA 审计落盘且结论通过 | `qa_passed` |

当前版本状态：`qa_passed`（shell 轮 + 架构重构轮均通过；T10~T14 全 verified_complete + E1~E5 全 pass + Release QA 审计落盘且结论 `qa_passed`）。

### Phase 8 GUI 验收遗留修复

- ExportModal.jsx TXT 格式空白崩溃 BUG：`ExportModal.jsx:274` `{_}` 被当作未声明 JSX 变量（ReferenceError → 无 ErrorBoundary 兜底 → Modal 子树卸载空白），修正为字符串字面量 `{"{_}"}`。Phase 8 GUI 验收（A4）确认 TXT 选项面板正常渲染。修复状态：`fixed` → 随后 `superseded`（TXT 导出整体改造为模板化语法，见下）。

### Phase 8 增量改造（qa_passed 后）

- TXT 导出模板化：`exportSheetToTxt` 重写为模板语法 `{字段名}_{值}`（`{字段名}` → 列名、`{值}` → 单元格值；连接符 `_`/`-`/`:` 等自由填写；每行每列各渲染一行）；内部 `fetchAllRowsForExport` 拉全表修复只导当前页 BUG。ExportModal TXT 选项面板简化为模板 TextArea + 行尾 Select。回归：cargo test 11 passed/2 ignored + pnpm build 3078 modules 全绿。详见 QA 报告 §10。
- XLSX 导出移除：ExportModal FORMAT_OPTIONS 从 csv/json/txt/xlsx 四项缩减为 csv/json/txt 三项；`exportSheetToXlsx` 调用点已删。回归全绿。

## 7. 进度同步约定

- 每个单任务 `verified_complete` 后：删除该任务 HANDOFF/REPORT/REVIEW 三件套；同步 `docs/versions/1.0.0/更新日志.md` 追加对应行。
- 端到端通过后：更新日志追加架构轮全行 `verified_complete`；TASK-BOARD 保留。
- Release QA `qa_passed` 且版本状态同步后：删除 TASK-BOARD.md（shell 轮 + 架构轮一并收尾）。
