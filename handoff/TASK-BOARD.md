# v1.2.1 任务看板

> 版本：v1.2.1（PATCH，缺陷修复 + 三轮架构优化）｜SCHEMA_VERSION 不变（=5）｜不打 tag（仅用户显式指令时）
> 需求文档：`docs/versions/1.2.1/规划需求.md`
> **任务 ID 按版本重置：阶段一 T1–T4，阶段二 T5–T10，阶段三 T11–T15**

## 目标

三阶段交付：
1. **阶段一**：解决 8.5 MB 单行无换行 TXT 文件导致前端卡顿/冻结的性能问题。
2. **阶段二**：修复手机号规则管理与实际功能数据不一致（空名单误过滤 7xx 手机号）；
   拆分 4 个 god-file（extract_ops.rs / func_validator.rs / rules/mod.rs / RulesPanel.jsx）；
   提取共享 helper 消除 ~150 行重复；删除 ~850 行死代码。
3. **阶段三**：第三轮架构优化 — Rust 后端表驱动去重 + 正则缓存 + 共享 helper 收敛；
   前端共享 hooks + AppContext 精简 + 3 个 god-file 拆分 + 常量集中 + 文档漂移修正。

## 任务 DAG

| 任务 | 标题 | 依赖 | 状态 | handoff |
|------|------|------|------|---------|
| T1 | TxtReader 按行/定长分块解析 | — | verified_complete | ~~deleted~~ |
| T2 | DataTable 大单元格截断 + highlightCell 防护 | — | verified_complete | ~~deleted~~ |
| T3 | ExtractPanel 超大输入不再主线程同步冻结 | — | verified_complete | ~~deleted~~ |
| T4 | 构建验证 + 版本号升级 + 提交 | T1, T2, T3 | verified_complete | ~~deleted~~ |
| T5 | 手机号校验一致性修复 | — | verified_complete | ~~deleted~~ |
| T6 | 删除死代码 | — | verified_complete | ~~deleted~~ |
| T7 | 拆分 func_validator.rs + rules/mod.rs | T5, T6 | verified_complete | ~~deleted~~ |
| T8 | extract_ops 提取复用 helper | T6, T7 | verified_complete | ~~deleted~~ |
| T9 | 拆分 RulesPanel.jsx | T5 | verified_complete | ~~deleted~~ |
| T10 | 构建验证 + 版本文档 | T5–T9 | verified_complete | ~~deleted~~ |
| T11 | Rust 后端去重 + 性能优化 | — | verified_complete | ~~deleted~~ |
| T12 | 前端共享 hooks + state 精简 | — | verified_complete | ~~deleted~~ |
| T13 | 前端 god-file 拆分 | T12 | verified_complete | ~~deleted~~ |
| T14 | 文档漂移修正 | T11, T13 | verified_complete | ~~deleted~~ |
| T15 | 构建验证 + 版本文档 | T11–T14 | verified_complete | ~~deleted~~ |

依赖说明：阶段一 T1/T2/T3 互无文件重叠，为并行候选；T4 是集成门禁。
阶段二 T5∥T6 无文件重叠可串行快速推进；T7 依赖 T5+T6；T8 依赖 T6+T7；T9 仅依赖 T5；
T10 是集成门禁，必须等 T5–T9 全部 `verified_complete`。
阶段三 T11∥T12 无文件重叠可并行；T13 依赖 T12（拆分需 hooks 先就位）；
T14 依赖 T11+T13（文档描述最终代码结构）；T15 是集成门禁。

## 端到端验收项（Phase 7）

### 阶段一（T1–T4）

1. `npx vite build` 通过（前端零构建错误）
2. `cargo check` 通过（后端零编译错误）
3. 现有 CSV/JSON/XLSX/LOG 格式行为不变（不新增/不删除 IPC，不改 SCHEMA_VERSION）
4. 8.5 MB 单行 TXT 经 T1 修复后：`TxtReader::read()` 产出多行（≥2 行），不再是 1 行 1 cell
5. DataTable 对超长单元格截断显示（ellipsis），`highlightCell` 不对超阈值 cell 执行 `TextEncoder.encode`
6. ExtractPanel `handleRun` 对超阈值输入跳过主线程同步正则并提示用户改用「提取并校验到新 Tab」

### 阶段二（T5–T10）

7. `check_phone_prefix("79912345678", &[])` → `true`（空名单=不过滤）
8. `is_valid_phone("79912345678", &[])` → `true`；`…, &["134"])` → `false`
9. `validate_extracted_with_params(PhonePrefix{[]}, "79912345678")` → `(true, "")`
10. UI 标签统一「留空=不限」（RulesPanel/ExtractPanel/ValidatePanel/MaskPanel）
11. `func_validator.rs` 拆为 `validators/` 子模块（8 文件）；`rules/mod.rs` 拆为 `rule.rs` + `registry.rs` + `builtins.rs`
12. `extract_ops.rs` 共享 `validate_single`/`query_all_data_cells`/`read_headers`/`write_sheet`，extract_ops+mask_ops 共用
13. `RulesPanel.jsx` ≤ 500 行容器 + `rule/` 子目录（RuleDetail + RuleTest + params/ + constants.js）
14. `grep -r "extractColumn\|validateColumn\|PiiExtractor" frontend/src` 无结果
15. `npx vite build` + `cargo check` + `cargo test` 通过

### 阶段三（T11–T15）

16. `builtins.rs` ≤ 100 行（表驱动 `BUILTIN_RULES` 切片）
17. `is_valid_birth_format` 使用 `OnceLock<Regex>` 缓存
18. `grep -c 'conn.lock().expect' src-tauri/src/db/*.rs` = 0
19. `grep -rn 'normalize_gender' src-tauri/src/commands/processor/extract_ops.rs` 无结果
20. `cargo clippy --all-targets` 零 warning
21. `grep -rn 'state.sheets.find' frontend/src/components/` ≤ 2 处
22. AppContext.jsx ≤ 80 行
23. `DataTable.jsx` ≤ 300 行；`ValidatePanel.jsx` ≤ 250 行；`ExportModal.jsx` ≤ 200 行
24. `grep -rn 'func_validator\|extractor\.rs\|validate_column\|extract_column' docs/02 docs/03` = 0
25. 模块树与 `find crates/core/src/processor -name '*.rs'` 一致
26. IPC 表与 `generate_handler!` 命令列表一致
27. `npx vite build` + `cargo check` + `cargo test` + `cargo clippy --all-targets` 通过

## 端到端验证命令

```bash
cd /Users/joker/code/RuT0DataKit/frontend && npx vite build
cd /Users/joker/code/RuT0DataKit && cargo check
cd /Users/joker/code/RuT0DataKit && cargo test
```

## Release QA

- required: true（仅用户显式指令 `/tag v1.2.1` 时触发）
- report: `docs/qa/versions/1.2.1/QA-审计报告.md`
- audit_scope: 需求覆盖 / 端到端流程 / 构建与测试 / 代码质量 / 安全与隐私 / 数据与迁移 / 依赖与配置 / 文档一致性

## commit 约定

> 用户明确要求：后续 commit 尽量不要出现 T1-T10 这类任务号前缀。
> commit message 使用 Conventional Commits（`type(scope): subject`），subject 中**不嵌入任务号**。
