# TASK-BOARD — v0.6.0 重整 + 覆盖发布

> 版本：v0.6.0
> 创建：2026-07-23（重整轮）
> 依赖版本：v0.5.0（release_complete @ c6033b0）
> 依据：用户 goal「对当前的所有改动做好梳理，然后进行优化，然后再重新提交到0.6.0，进行覆盖」

## 背景

主会话早前从 dangling commit `fa7e8c6` 恢复了 pinfo_phone 参数化校验器工作。
`fa7e8c6` 名义上是 T13-2（encrypt 命令），但 `git add -A` 混入了 5 类未提交改动：
pinfo_phone 校验器 + builtin ruleset 扩展、sql_reader 结构化还原、MaskColumnMapper
+ RulesView Table 重构 + state.js 级联初始化修复、encrypt 命令 + trial_mask。

本版本目标：把这堆「大杂烩」按逻辑组拆成干净 commit、优化代码、补完缺失的
T13-3 前端 encrypt 接线、bump 版本号 0.5.0→0.6.0，覆盖式重新发布 v0.6.0。

## 已确认的 baseline（fa7e8c6 状态）

- `cargo test --workspace`：393 + 10 + 34 + 12 + 11 + 31 = 491 全绿（3+2 ignored）
- `cargo build --workspace`：0 error，2 warning（extract_col_name_from_def 死代码 + crate 名非 snake_case）
- `npm --prefix frontend run build`：绿
- 版本号：0.5.0（5 manifest）
- pinfo_phone.rs / builtin.rs / sql_reader.rs / MaskColumnMapper.jsx / RulesView.jsx 均已就位

## 任务 DAG

```yaml
goal: 把 fa7e8c6 大杂烩按逻辑组重提交为干净 commits + 优化代码 + 补完前端 encrypt 接线 + bump 0.6.0 覆盖发布，全链路验收通过。
version: 0.6.0
depends_on_version: 0.5.0
prework:
  - id: PRE
    title: 主会话 soft reset fa7e8c6 → 按逻辑组重新提交 5 个干净 commits
    depends_on: []
    status: planned
    owner: main
    note: |
      主会话亲自执行（不派子 Agent）。git reset --soft fa7e8c6^ 后按文件组
      分批 git add + commit，产出 5 个干净 commit：
        C1 feat(rules): pinfo_phone 参数化校验器 + builtin ruleset 扩展 (T9-8)
        C2 feat(readers): sql_reader 结构化 dump 还原 (T9-9)
        C3 refactor(gui): MaskColumnMapper + RulesView Table + state 级联初始化 (T12-7)
        C4 feat(commands): encrypt/decrypt tauri 命令 + trial_mask 试运行 (T13-2)
        C5 chore: Cargo.lock 同步 crypto 依赖 + README 更新
      保留 fa7e8c6 的 d5f41c0(T13-1) 与 b7ca700(T13-4) 两个已有 commit 不动。
      soft reset 不改工作区内容，重提交后代码与 fa7e8c6 字节一致（除后续 T15 优化）。
tasks:
  - id: T15-1
    title: 代码优化清理（dead code + PDF 绝对化措辞）
    depends_on: [PRE]
    status: planned
    handoff: handoff/TASK-T15-1-HANDOFF.md
  - id: T15-2
    title: 前端 EncryptTool.jsx + tauri.js/state.js/Sidebar/ToolsView 接线（补完 T13-3）
    depends_on: [T15-1]
    status: planned
    handoff: handoff/TASK-T15-2-HANDOFF.md
  - id: T15-3
    title: 版本号 bump 0.5.0→0.6.0（5 manifest）+ docs/versions/0.6.0 同步
    depends_on: [T15-2]
    status: planned
    handoff: handoff/TASK-T15-3-HANDOFF.md
e2e_acceptance:
  - cargo test --workspace 全绿（基线 491 + T15 无新增失败）
  - cargo build --workspace 0 error（warning 不增）
  - cargo build --manifest-path src-tauri/Cargo.toml 0 error
  - npm --prefix frontend run build 成功
  - AES-CBC / Base64 / Hex 加密→解密互逆（T13-1 单测覆盖）
  - 批量列加密：指定列加密、未选列原样（T13-2 单测 + T15-2 前端预览）
  - 不外发数据：crates/core/src/tools/encrypt.rs 无 reqwest/http/ureq
  - 死代码 warning 清除：extract_col_name_from_def 不再 never used
  - pinfo_phone 参数化：params.prefixes 可覆盖默认 52 前缀（单测覆盖）
  - sql_reader 结构化还原：CREATE TABLE + INSERT VALUES → Records（e2e 覆盖）
  - 版本号 5 manifest 一致为 0.6.0
e2e_verification:
  - cargo test --workspace
  - cargo build --workspace
  - cargo build --manifest-path src-tauri/Cargo.toml
  - npm --prefix frontend run build
release_qa:
  required: true
  report: docs/qa/versions/0.6.0/QA-审计报告.md
  audit_scope:
    - 需求覆盖（encrypt + pinfo_phone + sql 还原 + RulesView Table + 版本号）
    - 端到端流程（单值试运行 + 批量列预览 + pinfo_phone 校验 + sql dump 还原）
    - 构建与测试（3 target 全绿、warning 不增）
    - 代码质量（模块独立、死代码清除、PDF 绝对化措辞清除）
    - 安全与隐私（纯本地、密钥不落盘、不外发数据）
    - 依赖与配置（crypto crate 版本钉定、Cargo.lock 同步）
    - 文档一致性（更新日志 / 规划需求 / 版本标准 / QA 报告对齐）
```

## 依赖说明

- **PRE**（主会话亲自做）：soft reset + 重提交，先建立干净 commit 历史
- **串行链**：PRE → T15-1（优化）→ T15-2（补完前端）→ T15-3（版本 bump + docs）
- 严格串行：T15-1 改的文件可能与 T15-2 有重叠（state.js / tauri.js），不并行

## E2E 验收项

| 项 | 验证命令 | 预期 |
|----|----------|------|
| 核心测试 | `cargo test --workspace` | 全绿（基线 491，T15 不增失败） |
| 后端编译 | `cargo build --workspace` | 0 error，warning 不增 |
| Tauri 编译 | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error |
| 前端构建 | `npm --prefix frontend run build` | ✓ built |
| 死代码清除 | `cargo build --workspace 2>&1 \| grep never_used` | 0 命中 extract_col_name_from_def |
| 算法互逆 | T13-1 单测 | AES-CBC / Base64 / Hex 加密→解密还原 |
| 批量列 | T13-2 单测 + T15-2 前端预览 | 指定列加密、未选列原样 |
| 不外发数据 | `grep -rn "reqwest\|http\|ureq" crates/core/src/tools/encrypt.rs` | 0 命中 |
| 版本号一致 | 5 manifest grep | 全部 0.6.0 |

## Release QA 门禁

- required: true
- report: docs/qa/versions/0.6.0/QA-审计报告.md
- audit_scope:
  - 需求覆盖（encrypt 工具 + pinfo_phone + sql 还原 + RulesView Table + 版本号 5 项落地）
  - 端到端流程（单值试运行 + 批量列预览 + pinfo_phone 校验 + sql dump 还原）
  - 构建与测试（3 target 全绿、warning 不增）
  - 代码质量（模块独立、死代码清除、PDF 绝对化措辞清除）
  - 安全与隐私（纯本地、密钥不落盘、不外发数据）
  - 依赖与配置（crypto crate 版本钉定、Cargo.lock 同步）
  - 文档一致性（更新日志 / 规划需求 / 版本标准 / QA 报告对齐）
