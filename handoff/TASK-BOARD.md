# TASK-BOARD — v0.6.1 行数显示 + RulesView 默认收起（release_complete）

> 版本：v0.6.1
> 创建：2026-07-24
> 完成：2026-07-24
> 依赖版本：v0.6.0（release_complete @ 3e59ea0）
> 依据：用户 goal「v0.6.1 第一，加一个小功能，原始数据和脱敏或者校验后的数据都要显示总行数有多少，第二，规则管理，默认应该是列表全部不打开，点击才打开」
> **状态：release_complete（qa_passed）**

## 任务 DAG（全部 done）

```yaml
goal: 4 视图统一加醒目总行数 + RulesView 默认全部收起，bump 0.6.1，全链路验收通过。
version: 0.6.1
depends_on_version: 0.6.0
tasks:
  - id: T16-1
    title: 4 视图统一加醒目总行数统计条
    status: done  # verified_complete + reviewer pass @ e5b631c
  - id: T16-2
    title: RulesView 默认全部收起
    status: done  # verified_complete + reviewer pass @ 7f3ca25
  - id: T16-3
    title: 版本号 bump 0.6.0→0.6.1（5 manifest）+ docs 同步 + QA 报告
    status: done  # @ 88a02e6
```

## E2E 验收结果

| 项 | 验证命令 | 结果 |
|----|----------|------|
| 前端构建 | `npm --prefix frontend run build` | vite 3008 modules built ✅ |
| 核心测试 | `cargo test --workspace` | 491 passed / 0 failed / 5 ignored ✅ |
| 后端编译 | `cargo build --workspace` | 0 error，2 warning（历史遗留） ✅ |
| Tauri 编译 | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error ✅ |
| 总行数显示 | 4 视图 grep `总行数/原始数据 N 行/脱敏后 N 行/校验后 N 行` | 全命中 ✅ |
| RulesView 默认收起 | `useState([])` + `setExpandedKeys([])` | 命中 ✅ |
| 版本号一致 | 5 manifest grep | 全部 0.6.1 ✅ |

## Release QA 门禁

- required: true → **passed**
- report: `docs/qa/versions/0.6.1/QA-审计报告.md`（结论 qa_passed）
- 五维度全部通过：需求覆盖 / 端到端 / 构建测试 / 代码质量 / 文档一致性 / 安全隐私

## 最终提交链（v0.6.1）

```
88a02e6 chore(v0.6.1): T16-3 版本号 0.6.0→0.6.1 (5 manifest) + docs 同步 + QA 报告
7f3ca25 feat(gui): T16-2 RulesView 规则列表默认全部收起
e5b631c feat(gui): T16-1 4 视图统一加醒目总行数统计条
3e59ea0 chore(v0.6.0): Phase 9 release_complete — TASK-BOARD 收尾 + 版本状态同步（v0.6.0 基线）
```

## 收尾

- 三件套（HANDOFF/REPORT/REVIEW）全部清理，仅保留本 TASK-BOARD 作为版本归档。
- `docs/04-版本标准.md` v0.6.1 里程碑行状态 `release_complete`。
- `docs/versions/0.6.1/更新日志.md` 版本状态 `release_complete`。
- v0.6.1 发布完成。
