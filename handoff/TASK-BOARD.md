# TASK-BOARD — v0.6.1 行数显示 + RulesView 默认收起

> 版本：v0.6.1
> 创建：2026-07-24
> 依赖版本：v0.6.0（release_complete @ 3e59ea0）
> 依据：用户 goal「v0.6.1 第一，加一个小功能，原始数据和脱敏或者校验后的数据都要显示总行数有多少，第二，规则管理，默认应该是列表全部不打开，点击才打开」

## 任务 DAG

```yaml
goal: 4 视图统一加醒目总行数 + RulesView 默认全部收起，bump 0.6.1，全链路验收通过。
version: 0.6.1
depends_on_version: 0.6.0
tasks:
  - id: T16-1
    title: 4 视图（MaskView/ValidateView/PreprocessView/ExportView）统一加醒目总行数统计条
    depends_on: []
    status: planned
    handoff: handoff/TASK-T16-1-HANDOFF.md
  - id: T16-2
    title: RulesView 默认全部收起（expandedRowKeys 默认 []）
    depends_on: []
    status: planned
    handoff: handoff/TASK-T16-2-HANDOFF.md
  - id: T16-3
    title: 版本号 bump 0.6.0→0.6.1（5 manifest）+ docs 同步 + QA 报告
    depends_on: [T16-1, T16-2]
    status: planned
    owner: main
e2e_acceptance:
  - npm --prefix frontend run build 成功
  - cargo test --workspace 全绿（基线 491，T16 不动后端，应仍 491）
  - cargo build --workspace 0 error
  - cargo build --manifest-path src-tauri/Cargo.toml 0 error
  - 4 视图均显示醒目总行数（原始 / 脱敏后 / 校验后 / 导出源对应）
  - RulesView 首次渲染所有行收起
  - 版本号 5 manifest 一致为 0.6.1
e2e_verification:
  - npm --prefix frontend run build
  - cargo test --workspace
  - cargo build --workspace
  - cargo build --manifest-path src-tauri/Cargo.toml
release_qa:
  required: true
  report: docs/qa/versions/0.6.1/QA-审计报告.md
  audit_scope:
    - 需求覆盖（总行数 4 视图 + RulesView 默认收起）
    - 端到端流程（脱敏/校验/导出预览行数显示正确）
    - 构建与测试（4 target 全绿、warning 不增）
    - 代码质量（视觉权重提升、降级处理）
    - 文档一致性（更新日志 / 版本标准 / QA 报告对齐）
```

## 依赖说明

- T16-1 与 T16-2 改不同文件（MaskView/ValidateView/PreprocessView/ExportView vs RulesView），可并行
- T16-3 依赖 T16-1 + T16-2，最后做

## E2E 验收项

| 项 | 验证命令 | 预期 |
|----|----------|------|
| 前端构建 | `npm --prefix frontend run build` | ✓ built |
| 核心测试 | `cargo test --workspace` | 491 green |
| 后端编译 | `cargo build --workspace` | 0 error |
| Tauri 编译 | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error |
| 总行数显示 | 4 视图 grep `总行数` | 命中 |
| RulesView 默认收起 | `useState([])` | 命中 |
| 版本号一致 | 5 manifest grep | 全部 0.6.1 |

## Release QA 门禁

- required: true
- report: docs/qa/versions/0.6.1/QA-审计报告.md
