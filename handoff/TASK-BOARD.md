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
| T10 | datasource mod 拆分（731 行 → per-format 子模块） | — | planned | handoff/TASK-T10-HANDOFF.md | — | — |
| T11 | commands.rs 拆分（374 行 → 4 子模块） | — | planned | handoff/TASK-T11-HANDOFF.md | — | — |
| T12 | Rust 死代码清理（placeholder/未用 model/未用 state 字段） | T10, T11 | planned | handoff/TASK-T12-HANDOFF.md | — | — |
| T13 | 前端 state 模块化 + Context | — | planned | handoff/TASK-T13-HANDOFF.md | — | — |
| T14 | 前端组件抽取 + 配置集中 + IPC 收口 | T13 | planned | handoff/TASK-T14-HANDOFF.md | — | — |

状态词：`planned` / `in_progress` / `implemented_not_verified` / `partially_complete` / `blocked` / `in_review` / `review_passed` / `review_rejected` / `verified_complete` / `not_complete`

## 3. 端到端验收项（Phase 7）

- E1：`cargo check --workspace` + `cargo test --workspace` 全绿（现有 27 测试不回归）。
- E2：`pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build` 通过。
- E3：`cargo tauri dev` 启动，四区布局可见，导入/导出/设置/tshark 检测行为与重构前一致。
- E4：v1.0.0 所有既有功能行为不变（纯重构，无行为变更）。
- E5：审计报告列出的 modularity 问题被实际解决：
  - datasource 不再是 731 行单文件；
  - commands 不再是 374 行单文件；
  - 前端 state 不再是 314 行单文件；
  - ExportModal 列选择不再重复；
  - IPC 不再有 raw invoke 旁路（UpdateCard 走 tauri.js）。

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

当前版本状态：`done_e2e`（shell 轮）；架构重构轮 `planned`。

## 7. 进度同步约定

- 每个单任务 `verified_complete` 后：删除该任务 HANDOFF/REPORT/REVIEW 三件套；同步 `docs/versions/1.0.0/更新日志.md` 追加对应行。
- 端到端通过后：更新日志追加架构轮全行 `verified_complete`；TASK-BOARD 保留。
- Release QA `qa_passed` 且版本状态同步后：删除 TASK-BOARD.md（shell 轮 + 架构轮一并收尾）。
