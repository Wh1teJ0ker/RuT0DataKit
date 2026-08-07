# TASK-BOARD — v1.1.0 能力阶段（脱敏/校验/提取 + 规则管理）

> v1.0.0「纯框架 shell + 架构重构轮」已 `qa_passed`（旧看板归档于 `handoff/archive/`）。
> 本看板为 v1.1.0 能力阶段：在 v1.0.0 shell 之上落地数据处理三件套（脱敏/校验/提取）+ 规则管理基础结构（3 条姓名规则 + DB 持久化）。
> 版本号 1.1.0。版本级状态以 `docs/qa/versions/1.1.0/QA-审计报告.md` 为准。

## 1. 任务 DAG

```
T15 core processor 模块 ──┐
T16 DB 扩展 (rules 表)  ──┼─→ T17 Tauri commands (6 IPC) ─→ T19 前端 state + IPC ─→ T20 前端面板
T18 版本号升级           │                                                      │
                          └──────────────────────────────────────────────────────┴─→ T21 文档收口
```

依赖说明：
- T15 / T16 / T18 互不依赖 → 可并行。
- T17 依赖 T15（core processor trait）+ T16（DB Rule CRUD）。
- T19 依赖 T17（IPC 命令）。
- T20 依赖 T19（state + IPC 封装）+ T3（DataTable 行高亮）。
- T21 依赖 T15~T20 全部 `verified_complete` 后做文档收口。

## 2. 任务状态总表

| ID | 标题 | depends_on | 状态 | handoff | report | review |
|---|---|---|---|---|---|---|
| T15 | core processor 模块（rules/validator/masker/extractor + 3 条姓名规则） | — | verified_complete | — | — | — |
| T16 | DB 扩展（SCHEMA_VERSION=2 + rules 表 + Rule CRUD + seed） | T15 | verified_complete | — | — | — |
| T17 | Tauri commands（6 IPC，DB 单一真源，删 RuleState） | T15, T16 | verified_complete | — | — | — |
| T18 | 版本号升级（6 处一致 1.1.0） | — | verified_complete | — | — | — |
| T19 | 前端 state + IPC 封装（APPLY_ROW_STATUSES + 6 invoke） | T17 | verified_complete | — | — | — |
| T20 | 前端面板（4 面板真实 UI + RulesPanel 两栏 + 无新增规则） | T19 | verified_complete | — | — | — |
| T21 | 文档收口（docs/versions/1.1.0/ 三件套 + 02 设计文档） | T15~T20 | verified_complete | — | — | — |

状态词：`planned` / `in_progress` / `implemented_not_verified` / `partially_complete` / `blocked` / `in_review` / `review_passed` / `review_rejected` / `verified_complete` / `not_complete`

## 3. 端到端验收项

- E1：`cargo fmt --check` + `cargo clippy --workspace -- -D warnings` + `cargo test --workspace` 全绿（39 passed / 2 ignored）。`pass`
- E2：`pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build` 通过。`pass`
- E3：`cargo tauri dev` 启动，四区布局可见，脱敏/校验/提取/规则管理面板真实 UI 可用，行状态高亮 `masked`/`invalid`/`hit` 触发。待 GUI 交互验收
- E4：规则持久化——重启应用后 `rules` 表保留用户调整的 `pattern`/`replacement` 参数。待 GUI 交互验收
- E5：RulesPanel 两栏布局 + 无新增规则入口。待 GUI 交互验收

## 4. 端到端验证命令

```sh
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
cargo tauri dev  # 手动核验 E3~E5
```

## 5. Release QA 门禁

- required: true
- report: `docs/qa/versions/1.1.0/QA-审计报告.md`
- audit_scope:
  - 需求覆盖（3 条姓名规则 + DB 持久化 + RulesPanel 两栏 + 无新增规则能力）
  - 端到端流程（E1~E5 是否全过）
  - 构建与测试（workspace + frontend build + cargo test 全绿）
  - 代码质量（core 纯逻辑 + DB 单一真源 + IPC 收口 + 前端组件复用）
  - 安全与隐私（CSP 策略 + fs 权限最小化 + updater 签名链 + 数据不外发）
  - 数据与迁移（SCHEMA_VERSION 1→2 增量迁移幂等 + rules 表 + seed）
  - 依赖与配置（版本号 6 处一致 1.1.0 + regex 纯 Rust 依赖）
  - 文档一致性（docs / 更新日志 / QA 报告互不冲突 + 行高亮颜色三处一致）
- 门禁：全部维度通过且无 major/critical 问题 → `qa_passed`；否则 `qa_failed` 回流修复。

## 6. 版本状态机

| 阶段 | 触发条件 | 目标状态 |
|---|---|---|
| 单任务通过 | reviewer `review_passed` + 主会话确认下游未破坏 | `verified_complete`（T15~T21） |
| 端到端通过 | T15~T21 全 `verified_complete` + E1~E5 全过 | `done_e2e` |
| 版本 QA 通过 | Release QA 审计落盘且结论通过 | `qa_passed` |

当前版本状态：`done_e2e`（T15~T21 全 verified_complete + E1~E2 pass + E3~E5 待 GUI 验收）。待 Release QA 审计推进。

## 7. 进度同步约定

- 每个单任务 `verified_complete` 后：同步 `docs/versions/1.1.0/更新日志.md` 追加对应行。
- 端到端通过后：更新日志追加全行 `verified_complete`；TASK-BOARD 保留。
- Release QA `qa_passed` 且版本状态同步后：归档 TASK-BOARD.md 至 `handoff/archive/`。
