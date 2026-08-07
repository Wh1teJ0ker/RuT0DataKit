# TASK-BOARD — v1.1.1 能力阶段（撤销 + 列操作 + 搜索 + 测试优化）

> v1.1.0 已 `qa_passed` 并发布 tag `v1.1.0`（旧看板归档于 `handoff/archive/TASK-BOARD-v1.1.0.md`）。
> 本看板为 v1.1.1：在 v1.1.0 之上落地四大功能。
> 版本号 1.1.1。版本级状态以 `docs/qa/versions/1.1.1/QA-审计报告.md` 为准。

## 1. 任务 DAG

```
T29 DB schema v2→v3 (before_snapshot + 搜索索引 + 列操作辅助方法)
   ├─→ T30 undo/redo 后端命令
   ├─→ T31 搜索后端命令 (search_cells + replace_all)
   ├─→ T32 列操作后端命令 (parse_column_as_json + replace_in_column)
   │       (T30/T31/T32 三者无文件重叠，可并行分派)
   └─→ T33 前端 IPC + state 扩展（依赖 T30/T31/T32 全部）
            └─→ T34 前端 UI（撤销工具栏 + 搜索栏 + 列操作面板）
                     └─→ T35 版本号升级 1.1.0→1.1.1（6 处）
                              └─→ T36 文档收口 + 全量验证 + Release QA
```

依赖说明：
- T29 是地基（schema 迁移 + 新方法），T30/T31/T32 都依赖它。
- T30/T31/T32 互不依赖（不同命令文件 / 不同 DB 方法），可并行分派 coder。
- T33 汇聚三个后端命令的 IPC 封装 + state action。
- T34 依赖 T33 的 IPC + state。
- T35/T36 收尾。

## 2. 任务状态总表

| ID | 标题 | depends_on | 状态 | handoff | report | review |
|---|---|---|---|---|---|---|
| T29 | DB schema v2→v3（before_snapshot + idx_cells_sheet_col + 7+1 个新方法） | — | verified_complete | handoff/TASK-T29-HANDOFF.md（已清理） | handoff/TASK-T29-REPORT.md（已清理） | handoff/TASK-T29-REVIEW.md (review_passed, 已清理) |
| T30 | undo/redo 后端命令（undo_operation / redo_operation / list_undoable_operations + mask_column 改造） | T29 | verified_complete | handoff/TASK-T30-HANDOFF.md（已清理） | handoff/TASK-T30-REPORT.md（已清理） | handoff/TASK-T30-REVIEW.md (review_passed, 已清理) |
| T31 | 搜索后端命令（search_cells + replace_all） | T29 | verified_complete | handoff/TASK-T31-HANDOFF.md（已清理） | handoff/TASK-T31-REPORT.md（已清理） | handoff/TASK-T31-REVIEW.md (review_rejected→fix→review_passed, 已清理) |
| T32 | 列操作后端命令（parse_column_as_json + replace_in_column） | T29 | verified_complete | handoff/TASK-T32-HANDOFF.md（已清理） | handoff/TASK-T32-REPORT.md（已清理） | handoff/TASK-T32-REVIEW.md (review_passed, 已清理) |
| T33 | 前端 IPC + state 扩展（7 IPC + 5 ACTION + factory searchHits） | T30, T31, T32 | verified_complete | handoff/TASK-T33-HANDOFF.md（已清理） | handoff/TASK-T33-REPORT.md（已清理） | handoff/TASK-T33-REVIEW.md (review_passed, 已清理) |
| T34 | 前端 UI（撤销工具栏 + 搜索栏 + 列操作面板 + 单元格高亮） | T33 | verified_complete | handoff/TASK-T34-HANDOFF.md（已清理） | handoff/TASK-T34-REPORT.md（已清理） | handoff/TASK-T34-REVIEW.md (review_passed, 3 minor, 已清理) |
| T35 | 版本号升级 1.1.0→1.1.1（6 处一致） | T34 | verified_complete | —（主会话直接实施，4 文件改动 + core/src-tauri workspace=true 继承） | — | — |
| T36 | 文档收口 + 全量验证 + Release QA | T35 | planned | — | — | — |

状态词：`planned` / `in_progress` / `implemented_not_verified` / `partially_complete` / `blocked` / `in_review` / `review_passed` / `review_rejected` / `verified_complete` / `not_complete`

## 3. 端到端验收项

- E1：`cargo fmt --check` + `cargo clippy --workspace -- -D warnings` + `cargo test --workspace` 全绿（含 commands 层新增测试）。
- E2：`pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build` 通过。
- E3：`cargo tauri dev` 启动 → 导入 CSV → 脱敏 → 点撤销 → cells 恢复 → 点重做 → cells 重新脱敏。
- E4：搜索框输入关键字 → 命中单元格高亮；正则模式可用；大文件（>1万行）搜索无明显卡顿。
- E5：列操作面板 → 选 JSON 列 → 解析为新 Tab → 新 Tab 含展开列；列内替换 → 该列命中值替换。
- E6：全局替换 Modal → from/to → 全表命中替换 → 撤销可恢复。
- E7：版本号 6 处一致 1.1.1。
- E8：重启应用后 schema 从 v2 迁移到 v3，历史数据保留。

## 4. 端到端验证命令

```sh
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
cargo tauri dev  # 手动核验 E3~E6
```

## 5. Release QA 门禁

- required: true
- report: `docs/qa/versions/1.1.1/QA-审计报告.md`
- audit_scope:
  - 需求覆盖（撤销 mask/replace + 列操作 JSON/replace + 搜索 keyword/regex + 全局替换 + 测试优化）
  - 端到端流程（E1~E8 是否全过）
  - 构建与测试（workspace + frontend build + cargo test 全绿 + commands 层测试新增）
  - 代码质量（schema 迁移幂等 + DB 单一真源 + IPC 收口 + 前端组件复用）
  - 安全与隐私（SQL 参数绑定 + LIKE 转义 + 正则编译失败处理 + 快照仅存变更列）
  - 数据与迁移（SCHEMA_VERSION 2→3 增量迁移幂等 + before_snapshot 列 + idx_cells_sheet_col）
  - 依赖与配置（版本号 6 处一致 1.1.1 + regex 已有依赖）
  - 文档一致性（docs / 更新日志 / QA 报告互不冲突）
- 门禁：全部维度通过且无 major/critical 问题 → `qa_passed`；否则 `qa_failed` 回流修复。

## 6. 版本状态机

| 阶段 | 触发条件 | 目标状态 |
|---|---|---|
| 单任务通过 | reviewer `review_passed` + 主会话确认下游未破坏 | `verified_complete`（T29~T36） |
| 端到端通过 | T29~T36 全 `verified_complete` + E1~E8 全过 | `done_e2e` |
| 版本 QA 通过 | Release QA 审计落盘且结论通过 | `qa_passed` |

当前版本状态：`in_progress`（T29~T35 verified_complete；T36 待启动）。

## 7. 进度同步约定

- 每个单任务 `verified_complete` 后：同步 `docs/versions/1.1.1/更新日志.md` 追加对应行。
- 端到端通过后：更新日志追加全行 `verified_complete`；TASK-BOARD 保留。
- Release QA `qa_passed` 且版本状态同步后：归档 TASK-BOARD.md 至 `handoff/archive/`。

## 8. 关键设计决策（用户确认）

- 搜索索引：LIKE + 复合索引 + 分页（不引入 FTS5）。
- 撤销范围：仅就地变更操作（mask / replace_in_column / replace_all）；import / parse_json / validate / extract 不入撤销栈。
- 安全约束：所有 SQL 用 `?N` + `params![]` 参数绑定；LIKE 转义 `%`/`_`/`\`；正则编译失败返回错误不 panic；快照仅存变更列 cells。
