# TASK-BOARD — v1.0.0 框架阶段

> 本看板服务于 [`docs/03-开发任务清单.md`](../docs/03-开发任务清单.md) 定义的 v1.0.0「纯框架 shell」MVP。所有任务的单任务状态以本看板为准，版本级状态以 [`docs/qa/versions/1.0.0/QA-审计报告.md`](../docs/qa/versions/1.0.0/QA-审计报告.md) 为准。

## 1. 任务 DAG

```
T1 工作区脚手架
 ├─ T2 四区布局 shell ─────────────┐
 ├─ T4 SQLite 持久层 ──┐            │
 │                     ├─ T5 导入流 │
 T3 Sheet/Tab + Table ─┘            │
 ├─ T6 Tauri updater 插件           │
 ├─ T7 AI 占位 IPC 契约             ├─ T9 双语 README + docs + QA
 └─ T8 设置页 + 能力面板占位提示 ───┘
```

依赖说明：
- T1 为所有任务前置。
- T2/T3/T4/T6/T7 在 T1 `verified_complete` 后可并行启动。
- T5 依赖 T3 + T4。
- T8 依赖 T2 + T6。
- T9 依赖 T1~T8 全部 `verified_complete`。

## 2. 任务状态总表

| ID | 标题 | depends_on | 状态 | handoff | report | review |
|---|---|---|---|---|---|---|
| T1 | 工作区脚手架 | — | verified_complete | —（已闭环） | — | — |
| T2 | 四区布局 shell | T1 | verified_complete | —（已闭环） | — | — |
| T3 | Sheet/Tab + antd Table | T1 | verified_complete | —（已闭环） | — | — |
| T4 | SQLite 持久层 | T1 | verified_complete | —（已闭环，含 T4-hotfix edb3ad6） | — | — |
| T5 | 导入流 | T3, T4 | verified_complete | —（已闭环） | — | — |
| T6 | Tauri updater 插件 | T1 | verified_complete | —（已闭环） | — | — |
| T7 | AI 占位 IPC 契约 | T1 | verified_complete | —（已闭环） | — | — |
| T8 | 设置页 + 能力面板占位提示 | T2, T6 | verified_complete | —（已闭环） | — | — |
| T9 | 双语 README + docs + QA | T1~T8 | in_progress | [HANDOFF](TASK-T9-HANDOFF.md) | — | — |

状态词：`planned` / `in_progress` / `implemented_not_verified` / `partially_complete` / `blocked` / `in_review` / `review_passed` / `review_rejected` / `verified_complete` / `not_complete`

## 3. 端到端验收项（Phase 7）

- E1：`cargo check --workspace` + `cargo test --workspace` 全绿。
- E2：`pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build` 通过。
- E3：`cargo tauri dev` 启动，四区布局可见（工具栏左组 6 项 + 右组 4 项 / 动态能力面板 / Workbench / AI 占位）。
- E4：导入 1000 行 CSV → Sheet 新建 → Table 渲染首页 50 行 → 分页可翻 → 重启后历史 Session 可重新加载（持久化）。
- E5：右组能力按钮点击切换左侧能力面板，4 项面板统一 `v1.1+` 占位提示；左组禁用项点击弹 `v1.1+` 提示。
- E6：设置页 4 卡片渲染，「检查更新」可触发 `check_update`，DB 路径与实际文件一致，「在 Finder 中显示」可打开。
- E7：AI 面板调用 `aiSuggest` 得到 `v1.1+` 文案，UI 不崩溃。
- E8：`sqlite3 <db> .schema` 显示 5 表 3 索引；`operations` 表有 `import` 记录。
- E9：无网络时 `check_update` 静默降级返回 `available=false`，不弹错。

## 4. 端到端验证命令

```sh
cargo check --workspace
cargo test --workspace
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
cargo tauri dev  # 手动核验 E3~E8
sqlite3 ~/Library/Application\ Support/com.rut0.datakit/ruT0datakit.db ".schema"
```

## 5. Release QA 门禁（Phase 8）

- required: true
- report: `docs/qa/versions/1.0.0/QA-审计报告.md`
- audit_scope:
  - 需求覆盖（00/01 文档的 v1.0.0 范围是否全落地）
  - 端到端流程（E1~E9 是否全过）
  - 构建与测试（workspace + frontend build + cargo test）
  - 代码质量（嵌套 struct `rename_all = "camelCase"`、模块边界、out_of_scope 遵守）
  - 安全与隐私（数据不外发、updater 仅拉签名产物、私钥不落盘仓库）
  - 数据与迁移（5 表 3 索引、schema_version、事务回滚）
  - 依赖与配置（版本号 1.0.0 一致、rust-toolchain 固定、tauri.conf.json 配置）
  - 文档一致性（docs / README / 更新日志 / QA 报告互不冲突）
- 门禁：全部维度通过且无 major/critical 问题 → `qa_passed`；否则 `qa_failed` 回流修复。

## 6. 版本状态机

| 阶段 | 触发条件 | 目标状态 |
|---|---|---|
| 单任务通过 | reviewer `review_passed` + 主会话确认下游未破坏 | `verified_complete`（T1~T8） |
| 端到端通过 | T1~T8 全 `verified_complete` + E1~E9 全过 | `done_e2e` |
| 版本 QA 通过 | Release QA 审计落盘且结论通过 | `qa_passed` |
| 版本完成 | `qa_passed` + 版本文档同步 | `release_complete` |

## 7. 进度同步约定

- 每个单任务 `verified_complete` 后：删除该任务 HANDOFF/REPORT/REVIEW 三件套；同步 `docs/versions/1.0.0/更新日志.md` 对应行。
- 端到端通过后：更新日志全行 `verified_complete`；TASK-BOARD 保留。
- Release QA `qa_passed` 且版本状态同步后：删除 TASK-BOARD.md。
