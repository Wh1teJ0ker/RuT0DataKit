# TASK-BOARD — v0.6.0 重整 + 覆盖发布（release_complete）

> 版本：v0.6.0
> 创建：2026-07-23（重整轮）
> 完成：2026-07-24
> 依赖版本：v0.5.0（release_complete @ c6033b0）
> 依据：用户 goal「对当前的所有改动做好梳理，然后进行优化，然后再重新提交到0.6.0，进行覆盖」
> **状态：release_complete（qa_passed）**

## 背景

主会话早前从 dangling commit `fa7e8c6` 恢复了 pinfo_phone 参数化校验器工作。
`fa7e8c6` 名义上是 T13-2（encrypt 命令），但 `git add -A` 混入了 5 类未提交改动：
pinfo_phone 校验器 + builtin ruleset 扩展、sql_reader 结构化还原、MaskColumnMapper
+ RulesView Table 重构 + state.js 级联初始化修复、encrypt 命令 + trial_mask。

本版本目标：把这堆「大杂烩」按逻辑组拆成干净 commit、优化代码、补完缺失的
T13-3 前端 encrypt 接线、bump 版本号 0.5.0→0.6.0，覆盖式重新发布 v0.6.0。

## 任务 DAG（全部 done）

```yaml
goal: 把 fa7e8c6 大杂烩按逻辑组重提交为干净 commits + 优化代码 + 补完前端 encrypt 接线 + bump 0.6.0 覆盖发布，全链路验收通过。
version: 0.6.0
depends_on_version: 0.5.0
prework:
  - id: PRE
    title: 主会话 soft reset fa7e8c6 → 按逻辑组重新提交 6 个干净 commits
    status: done
    commits:
      - c43235e chore(handoff): TASK-BOARD 重写
      - a2e9f09 feat(rules): pinfo_phone 参数化校验器 (T9-8)
      - 5b1af51 feat(readers): sql_reader 结构化 dump 还原 (T9-9)
      - cd41b18 refactor(gui): MaskColumnMapper + RulesView Table + state 级联 (T12-7)
      - 17ae025 feat(commands): encrypt/decrypt tauri 命令 (T13-2)
      - 74dc849 chore: Cargo.lock + README 同步
tasks:
  - id: T15-1
    title: 代码优化清理（dead code + PDF 绝对化措辞）
    status: done  # verified_complete + reviewer pass @ 52adcff
  - id: T15-2
    title: 前端 EncryptTool.jsx + tauri.js/state.js/Sidebar/ToolsView 接线（补完 T13-3）
    status: done  # verified_complete + reviewer pass @ 97bfbac
  - id: T15-3
    title: 版本号 bump 0.5.0→0.6.0（5 manifest）+ docs 同步 + QA 报告
    status: done  # @ 5176e02
```

## E2E 验收结果

| 项 | 验证命令 | 结果 |
|----|----------|------|
| 核心测试 | `cargo test --workspace` | 491 passed / 0 failed / 7 ignored ✅ |
| 后端编译 | `cargo build --workspace` | 0 error，2 warning（历史遗留 crate 名） ✅ |
| Tauri 编译 | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error ✅ |
| 前端构建 | `npm --prefix frontend run build` | vite 3008 modules built ✅ |
| 死代码清除 | `cargo build 2>&1 \| grep never_used` | 0 命中 extract_col_name_from_def ✅ |
| 算法互逆 | T13-1 单测 | AES-CBC / Base64 / Hex 加密→解密还原 ✅ |
| 批量列 | T13-2 单测 + T15-2 前端 | 指定列加密、未选列原样 ✅ |
| 不外发数据 | `grep reqwest\|http\|ureq encrypt.rs` | 0 命中 ✅ |
| 代码层 PDF 措辞 | `grep PDF crates/ frontend/src/` | 0 命中 ✅ |
| 版本号一致 | 5 manifest grep | 全部 0.6.0 ✅ |

## Release QA 门禁

- required: true → **passed**
- report: `docs/qa/versions/0.6.0/QA-审计报告.md`（结论 qa_passed）
- 五维度全部通过：需求覆盖 / 端到端 / 构建测试 / 代码质量 / 文档一致性 / 安全隐私

## 最终提交链（v0.6.0）

```
5176e02 chore(v0.6.0): T15-3 版本号 0.5.0→0.6.0 (5 manifest) + docs 同步 + QA 报告
633f2a1 chore(handoff): 清理 T15-1/T15-2 三件套
97bfbac feat(gui): T15-2 补齐 EncryptTool 前端接线
f1f161b chore(handoff): cleanup T15-1 three-piece set
52adcff refactor(core): 清理死代码 + PDF 绝对化措辞中性化 (T15-1)
74dc849 chore: Cargo.lock 同步 crypto 依赖 + README
17ae025 feat(commands): encrypt/decrypt tauri 命令 (T13-2)
cd41b18 refactor(gui): MaskColumnMapper + RulesView Table + state 级联 (T12-7)
5b1af51 feat(readers): sql_reader 结构化 dump 还原 (T9-9)
a2e9f09 feat(rules): pinfo_phone 参数化校验器 + builtin ruleset 扩展 (T9-8)
c43235e chore(handoff): clean stale T12 three-piece sets + rewrite TASK-BOARD
d5f41c0 feat(core/tools): add AES-CBC/Base64/Hex encrypt module (T13-1)
b7ca700 docs(v0.6.0): add 规划需求 + 更新日志骨架 + 里程碑索引行 (T13-4)
```

## 收尾

- 三件套（HANDOFF/REPORT/REVIEW）全部清理，仅保留本 TASK-BOARD 作为版本归档。
- `docs/04-版本标准.md` v0.6.0 里程碑行状态 `release_complete`。
- `docs/versions/0.6.0/更新日志.md` 版本状态 `release_complete`。
- v0.6.0 覆盖发布完成。
