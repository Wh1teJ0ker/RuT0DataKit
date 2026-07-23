# TASK-BOARD — v0.5.0 架构性质升级（低耦合 / 高内聚专项）

> 版本：v0.5.0（架构升级：God 文件拆分 + 正则集中化 + 前端状态切片）
> 创建：2026-07-23
> 依赖版本：v0.4.4（release_complete）
> 依据：`docs/qa/代码质量审计报告.md` P0+P1+P2 全套

## 用户目标

以 `docs/qa/代码质量审计报告.md` 为依据，解决当前代码库的高内聚 / 低耦合问题，优化架构性质缺陷，进化到下一个小版本 v0.5.0，作为专门的架构升级版本。

## 任务 DAG

```yaml
goal: 按 docs/qa/代码质量审计报告.md 的 P0+P1+P2 路线图，拆分 3 个 God 文件 + 集中化散布正则 + 前端状态切片 + 抽公共组件，在保持行为零回归前提下提升内聚度、降低耦合。
version: 0.5.0
depends_on_version: 0.4.4
tasks:
  - id: T12-1
    title: 拆 blind_aggregator.rs 1790 LOC → 3 子文件（probe/aggregate/reconstruct）
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T12-1-HANDOFF.md
    report: handoff/TASK-T12-1-REPORT.md
    review: handoff/TASK-T12-1-REVIEW.md (review_passed)
  - id: T12-2
    title: 拆 commands.rs 32 命令 → 8 领域子文件
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T12-2-HANDOFF.md
    report: handoff/TASK-T12-2-REPORT.md
    review: handoff/TASK-T12-2-REVIEW.md (verified_complete after main-session ruling: main.rs/scan_log_file changes are v0.4.4 leftovers, not T12-2 scope creep)
  - id: T12-3
    title: 拆 operator.rs 976 LOC → mask_op.rs + validate_op.rs 分文件
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T12-3-HANDOFF.md
    report: handoff/TASK-T12-3-REPORT.md
    review: handoff/TASK-T12-3-REVIEW.md (verified_complete)
  - id: T12-4
    title: 建 rules/patterns.rs 集中正则表 + 消除 phone/ip 正则散布
    depends_on: [T12-3]
    status: verified_complete
    handoff: handoff/TASK-T12-4-HANDOFF.md
    report: handoff/TASK-T12-4-REPORT.md
    review: handoff/TASK-T12-4-REVIEW.md (verified_complete; 16 regex verified char-identical, 450 passed)
  - id: T12-5
    title: 前端 state.js 按领域切片 + action 常量化
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T12-5-HANDOFF.md
    report: handoff/TASK-T12-5-REPORT.md
    review: handoff/TASK-T12-5-REVIEW.md (verified_complete; 50 dispatch strings all matched, 13 domains)
  - id: T12-6
    title: 抽 ColumnRuleMapper 公共组件消除 MaskView/ValidateView 重复
    depends_on: [T12-5]
    status: verified_complete
    handoff: handoff/TASK-T12-6-HANDOFF.md
    report: handoff/TASK-T12-6-REPORT.md
    review: handoff/TASK-T12-6-REVIEW.md (verified_complete; MaskView 323, ValidateView 294, mapper 253)
```

## 依赖说明

- **并行候选（无文件重叠）**：T12-1（logsign）/ T12-2（src-tauri/commands）/ T12-3（rules/operator）/ T12-5（frontend/state）
- **串行链**：T12-3 → T12-4（均改 rules 模块 + operator.rs 区域）；T12-5 → T12-6（均改前端 state + MaskView/ValidateView）

## E2E 验收项

| 项 | 验证命令 | 预期 |
|----|----------|------|
| 核心测试 | `cargo test --workspace` | 全绿（v0.4.4 基线 445 passed） |
| 后端编译 | `cargo build --workspace` | 0 error |
| Tauri 编译 | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error |
| 前端构建 | `cd frontend && npx vite build` | ✓ built |
| God 文件消除 | `wc -l` 三大文件 | blind_aggregator 拆后主文件 < 200 LOC；commands.rs 拆后主文件 < 100 LOC；operator.rs 拆后删除或 < 50 LOC |
| 正则散布消除 | `grep -rn "1\\\\d{10}\|1\[3-9\]" crates/core/src/validators/phone.rs crates/core/src/scan/mod.rs crates/core/src/rules/operator.rs` | 0 命中（统一引用 patterns.rs） |
| 行为零回归 | e2e.rs + 全部单测 | 全绿，无行为变化 |

## Release QA 门禁

- required: true
- report: docs/qa/versions/0.5.0/QA-审计报告.md
- audit_scope:
  - 需求覆盖（P0+P1+P2 全部 6 项落地）
  - 端到端流程（行为零回归）
  - 构建与测试（3 target 全绿）
  - 代码质量（God 文件消除 + 内聚度提升量化）
  - 文档一致性（更新日志 / 规划需求 / 版本标准对齐）
