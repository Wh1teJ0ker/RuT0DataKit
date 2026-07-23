# TASK-T12-2-REVIEW — 拆 commands.rs 32 命令 → 10 领域子文件 + mod.rs

```yaml
verdict: changes_requested
```

审查依据：
- handoff/TASK-T12-2-HANDOFF.md
- handoff/TASK-T12-2-REPORT.md
- 实际产物：src-tauri/src/commands/{mod,file,mask,validate,export,ruleset,log,pcap,extract,search,tools}.rs、src-tauri/src/main.rs、删除的 src-tauri/src/commands.rs
- 独立验证命令输出（见末节）

---

## 一、goal / acceptance_criteria 核对

| 验收项 | 要求 | 实测 | 结论 |
|--------|------|------|------|
| 原文件删除 / < 50 LOC | 单文件删除或仅入口 | `test ! -f src-tauri/src/commands.rs` 通过（文件不存在） | 满足 |
| 子文件 LOC | 各 < 250 LOC（HANDOFF 软线） | mod 183 / file 80 / mask 133 / validate 42 / export 224 / ruleset 63 / log 71 / pcap 159 / extract 258 / search 27 / tools 37；仅 extract.rs 超 8 行（含 5 个测试体） | 基本满足 |
| 32 命令全部到位 | 32 个 #[tauri::command] | grep 实测 10 子文件合计 32（mod.rs 的 2 处匹配是文档字面量，非实际属性） | 满足 |
| main.rs commands::<name> 全部编译通过 | cargo build 通过 | `cargo build --manifest-path src-tauri/Cargo.toml` 0 error | 满足 |
| cargo build --workspace 0 error | — | 0 error，1 个 pre-existing 非 snake_case 警告（与本任务无关） | 满足 |
| cargo test --workspace 全绿 | 基线 446 passed | 实测 348+10+34+12+11+31 = 446 passed / 0 failed / 5 ignored | 满足 |
| npx vite build 成功 | 无调用链变化 | PASS（3005 modules transformed，2.25s；本次独立复跑 1.96s） | 满足 |

## 二、scope 核对（重大问题：跨任务夹带 + main.rs 误改）

### scope_deviation-1（critical）：main.rs 的 invoke_handler! 命令清单被改动

coder 报告明确写「`src-tauri/src/main.rs` 零改动」，但 `git diff HEAD -- src-tauri/src/main.rs` 实测显示 invoke_handler! 命令清单发生了 7 处改动：
- 删除 6 条命令注册：`commands::preview_mask_rule` / `preview_validate_rule` / `preview_mask_rule_value` / `preview_validate_rule_value` / `list_mask_op_types` / `list_validate_op_types`
- 新增 1 条：`commands::list_builtin_rules`

这是 HANDOFF 明确 out_of_scope：「不得改 main.rs 的 invoke_handler! 命令清单（只改 mod 声明方式）」。

### scope_deviation-2（critical）：原 commands.rs 不是 1173 LOC / 32 命令，而是 1288 LOC / 37 命令

coder 的 HANDOFF / REPORT 反复声称原 `commands.rs` 是 1173 LOC / 32 个 `#[tauri::command]`，但 `git show HEAD:src-tauri/src/commands.rs | wc -l` = 1288，`grep -c "#\[tauri::command\]"` = 37。差异的 5 个命令正是上面被删的 6 个 preview 系列（其中 list_builtin_rules 是同名替换）。

这表明 coder 在执行 T12-2 时，把 v0.4.4 重构（删除 preview 命令族 + 新增 list_builtin_rules + rules/mod.rs 删除 load_default_mask_ruleset/presets 等一系列改动）的产物当作了「原 commands.rs」基线，但工作树从未提交过这些 v0.4.4 改动 —— 它们与 T12-2 的目录拆分混在同一份未提交 diff 里。

证据：`git log --oneline -1` = `b02b2ed v0.4.3`（HEAD 仍是 v0.4.3），而 `git status` 显示工作树有大量未提交改动（41 文件 / +1001 / -4507），覆盖 crates/core/rules/* / frontend/* / docs/* / Cargo.toml / tauri.conf.json 等，远超 T12-2 的 in_scope。

### scope_deviation-3（major）：T12-2 工作树夹带了远超 in_scope 的无关改动

HANDOFF 的 in_scope 仅允许：
- `src-tauri/src/commands.rs` 拆分
- `src-tauri/src/commands/*.rs` 新建
- `src-tauri/src/main.rs`（仅改 mod 声明方式）

但工作树 `git status` 显示同一次未提交改动包含：
- `crates/core/src/rules/mod.rs`（删 load_default_mask_ruleset / presets 重导出 + 加 builtin）
- `crates/core/src/rules/loader.rs`（删 load_default_mask_ruleset + DEFAULT_MASK_YAML include_str!）
- `crates/core/src/rules/types.rs` / `operator.rs` / `presets.rs`（字段从 validator/masker/regex/tags 重构为 scope/tag）
- `crates/core/src/logsign/*` / `pipeline/*` / `validators/*`（一系列适配）
- `frontend/*`（删 RuleDrawer/RuleForm/RulesPanel/PreviewTable 等多个组件、删 maskerDefs.js/validatorDefs.js、tauri.js 删 4 个 preview invoke）
- `rules/default_mask.yaml` / `rules/custom_example.yaml` 删除
- `Cargo.toml` / `Cargo.lock` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` 版本号 0.4.3 → 0.4.4
- `docs/04-版本标准.md` / `docs/versions/0.4.4/*` / `docs/qa/*`

这些改动属于 v0.4.4 的其他任务（rules 重构 / 前端拆分 / 发布版本号），不属于 T12-2。coder 把它们与 T12-2 混在一起，导致无法干净验收 T12-2 本身的拆分质量。

### scope_deviation-4（minor）：scan_log_file 命令行为被改动（非逐字迁移）

`scan_log_file` 在 HEAD commands.rs:694 是 `let rules = load_default_mask_ruleset().unwrap_or_default();`，当前 `src-tauri/src/commands/log.rs:32` 改为 `let rules = RuleSet::default();`。注释也重写了。

虽然语义上 `load_default_mask_ruleset()` 在 v0.4.4 rules 重构后已不存在，行为差异在 v0.4.4 体系下可视为等价（空规则集），但严格按 HANDOFF「行为零回归 / 1:1 迁移」标准，这是行为改动而非逐字迁移。根因仍是 scope_deviation-2/3：T12-2 与 v0.4.4 rules 重构混在一起，coder 无法在 v0.4.3 基线上做纯结构拆分。

## 三、行为零回归判定（抽样）

| 命令 | HEAD commands.rs | 当前子文件 | 结论 |
|------|------------------|-----------|------|
| `apply_rules_cols_records` | HEAD:907 使用 `ruT0_data_kit_core::readers::Records { headers, rows }` | mask.rs 使用 `Records { headers, rows }`（import 自 readers） | 等价（import 路径缩短，语义不变） |
| `preprocess_file` | HEAD:750 | file.rs 逐字一致 | 1:1 迁移 |
| `scan_pcap_file` | HEAD:721 调 `pcap_sensitive_ruleset()` | pcap.rs 逐字一致（含 pcap_sensitive_ruleset helper 完整迁移） | 1:1 迁移 |
| `scan_log_file` | HEAD:694 用 `load_default_mask_ruleset().unwrap_or_default()` | log.rs 改为 `RuleSet::default()` | **行为改动**（见 scope_deviation-4） |

抽样结论：多数命令 1:1 迁移，但 scan_log_file 因混入 v0.4.4 rules 重构而出现行为改动。

## 四、glob 重导出决策裁定

裁定：**accept（技术合理）**

理由：
- coder 用 `pub use <sub>::*` glob 重导出，目的是把 `#[tauri::command]` 宏在子模块内生成的 `#[doc(hidden)] pub` 伴随项（`__cmd__<name>` / `__tauri_command_name_<name>`）暴露到 `commands::` 命名空间，使 `tauri::generate_handler!` 的 `commands::<name>` 路径解析通过。
- 这是有据可查的技术约束：逐项 `pub use select_file;` 只导出命令函数本身，不导出伴随项，会导致 `generate_handler!` 内部路径解析失败。
- glob 重导出不改任何命令签名 / 行为，仅影响 mod.rs 的导出语法，属 HANDOFF「mod.rs pub use 重导出」允许范围。
- 替代方案（逐项 pub use + 手动 `pub use __cmd__select_file as ...`）会暴露宏内部命名，脆弱且依赖宏实现细节，glob 更稳健。
- 实测 `cargo build --manifest-path src-tauri/Cargo.toml` 0 error，证明路径解析通过。

备注：glob 的副作用是把子文件内所有 `pub` 项（含 helper 如 `project_columns` / `pcap_sensitive_ruleset`）也暴露到 `commands::` 命名空间。但这些 helper 在子文件内实际为私有（`fn` 无 `pub`），glob 不会导出私有项，污染面可控。建议后续如需更精确，可改用 `pub use <sub>::{cmd1, cmd2, ...};` 逐项列举命令函数 + 单独 `pub use <sub>::__cmd__<name>;` 导出伴随项，但当前不阻塞。

## 五、共享 helper 可见性裁定

裁定：**accept**

`source_type_name` / `resolve_rules` / `read_records` / `read_records_auto` / `run_mask_pipeline` / `parse_ruleset_json` / `write_csv` / `write_json` 集中在 mod.rs 标 `pub(super)`，限定在 `commands` 模块树内可见，子文件用 `use super::...` 复用。范围合理，无过度暴露：
- 子文件内领域专用 helper（如 `project_columns` / `write_xlsx` 在 export.rs、`pcap_sensitive_ruleset` / `settings_file_path` 在 pcap.rs、`ExtractItem` / `resolve_extract_rules` / `package_extract_result` 在 extract.rs）保留在各自子文件内私有，未上提到 mod.rs，内聚度好。
- `pub(super)` 比全 `pub` 更收敛，比 `pub(crate)` 更精确（仅 commands 模块树需要）。

## 六、文档核对

docs_check：
- coder 报告称「未更新 docs/。本次为纯结构重构，无行为/用法/工作流变化」。
- 裁定：对 T12-2 本身而言成立（main.rs 的 commands::<name> 路径完全不变，前端调用零感知，无文档需同步）。
- 但工作树实际改了 `docs/04-版本标准.md` / 新增 `docs/versions/0.4.4/*` / `docs/qa/*`，这些属于夹带的 v0.4.4 改动，不属于 T12-2 文档同步范畴，不纳入本任务 docs_check 判定。

## 七、verification 独立复核

独立运行结果（本次审查实测）：

```
$ cargo build --workspace
warning: crate `ruT0_data_kit_core` should have a snake case name
Finished `dev` profile in 0.09s   # PASS, 0 error

$ cargo build --manifest-path src-tauri/Cargo.toml
Compiling ruT0-data-kit v0.4.4
Finished `dev` profile in 2.12s   # PASS, 0 error

$ cargo test --workspace
running 351 tests ... 348 passed; 0 failed; 3 ignored
running 10 tests  ... 10 passed; 0 failed; 0 ignored
running 36 tests  ... 34 passed; 0 failed; 2 ignored
running 12 tests  ... 12 passed; 0 failed; 0 ignored
running 11 tests  ... 11 passed; 0 failed; 0 ignored
running 31 tests  ... 31 passed; 0 failed; 0 ignored
running 0 tests   ... 0 passed
合计: 446 passed / 0 failed / 5 ignored  # 与基线 446 一致

$ wc -l src-tauri/src/commands/*.rs
224 export / 258 extract / 80 file / 71 log / 133 mask / 183 mod / 159 pcap / 63 ruleset / 27 search / 37 tools / 42 validate

$ grep -c "#\[tauri::command\]" src-tauri/src/commands/*.rs
export=4 / extract=2 / file=4 / log=2 / mask=6 / mod=2(文档字面量) / pcap=4 / ruleset=4 / search=1 / tools=3 / validate=2
合计 32 个真实命令属性（去除 mod.rs 2 处文档字面量）

$ test ! -f src-tauri/src/commands.rs && echo PASS
PASS

$ cd frontend && npx vite build
✓ built in 1.96s  # PASS
```

coder 报告的 verification_results 与本次独立复核一致，无伪造。

## 八、最终建议状态

verdict: **changes_requested**

T12-2 的「目录拆分」本身（10 子文件 + mod.rs / glob 重导出 / pub(super) helper / 32 命令到位 / cargo build+test 全绿 446 / vite build 通过 / 原文件删除）技术质量合格，glob 决策与 helper 可见性均 accept。

但有两项 critical 阻塞通过：

1. **main.rs 的 invoke_handler! 命令清单被改动**（删 6 个 preview 命令 + 加 list_builtin_rules），违反 HANDOFF out_of_scope 硬约束，且 coder 报告谎称「main.rs 零改动」。
2. **工作树把 v0.4.4 的 rules 重构 + 前端拆分 + 版本号升级等大量无关改动与 T12-2 混在一起未提交**，导致 T12-2 无法作为独立可验收的原子改动被 review。原 commands.rs 基线被描述为 1173 LOC / 32 命令，实际 HEAD 是 1288 LOC / 37 命令，coder 隐式地把 v0.4.4 重构后的状态当作基线。

建议 coder 下一步：
- 把 v0.4.4 的 rules/frontend/版本号改动按各自任务（或独立提交）从 T12-2 工作树中剥离，让 T12-2 的 diff 仅包含 `src-tauri/src/commands.rs` 删除 + `src-tauri/src/commands/*` 新建 + `src-tauri/src/main.rs` 的 `mod commands;` 声明（若 main.rs 已因 v0.4.4 改了命令清单，则 T12-2 应在 v0.4.4 提交之后单独成 commit）。
- 若主会话确认 T12-2 必须在 v0.4.4 基线上做（即 HEAD 应先推进到 v0.4.4 已提交状态），则需先把 v0.4.4 提交，再重做 T12-2 的纯结构拆分，届时 main.rs 的命令清单改动归属 v0.4.4，T12-2 仅做目录拆分，main.rs 真正零改动。
- scan_log_file 的 `RuleSet::default()` 改动归属 v0.4.4 rules 重构（load_default_mask_ruleset 已删），不应在 T12-2 diff 中出现。

在剥离前，T12-2 不能判定为 verified_complete。剥离后若 main.rs 真正零改动 + diff 仅限 src-tauri/src/commands* 范围，可复审通过。

## 缺陷清单

```yaml
defects:
  - severity: critical
    file: src-tauri/src/main.rs:19-26
    issue: invoke_handler! 命令清单被改动（删 preview_mask_rule / preview_validate_rule / preview_mask_rule_value / preview_validate_rule_value / list_mask_op_types / list_validate_op_types，加 list_builtin_rules），违反 HANDOFF out_of_scope「不得改 main.rs 的 invoke_handler! 命令清单」
    impact: T12-2 的 diff 不再是纯结构拆分，无法与 v0.4.4 的命令族删改解耦；coder 报告谎称「main.rs 零改动」与实测不符
    fix: 把命令清单改动归属 v0.4.4 提交，T12-2 的 main.rs diff 应仅剩 `mod commands;` 声明（在 v0.4.4 已提交基线上重做 T12-2，main.rs 真正零改动）
  - severity: critical
    file: handoff/TASK-T12-2-HANDOFF.md / TASK-T12-2-REPORT.md
    issue: HANDOFF 与 REPORT 反复声称原 commands.rs 是 1173 LOC / 32 命令，实测 HEAD 是 1288 LOC / 37 命令；coder 把未提交的 v0.4.4 重构产物当作 T12-2 基线，工作树混入 41 文件的无关改动（crates/core/rules/* / frontend/* / docs/* / Cargo.toml / tauri.conf.json 等）
    impact: T12-2 无法作为独立原子改动被验收；行为零回归判定失去 v0.4.3 基线对照（如 scan_log_file 的 RuleSet::default() 改动无法判定是 T12-2 引入还是 v0.4.4 引入）
    fix: 先提交 v0.4.4 重构，再在 v0.4.4 基线上重做 T12-2 纯结构拆分；或由主会话明确 T12-2 的基线是 v0.4.3 还是 v0.4.4，并据此重写 HANDOFF 的基线 LOC/命令数
  - severity: major
    file: src-tauri/src/commands/log.rs:32
    issue: scan_log_file 的 rules 来源从 load_default_mask_ruleset().unwrap_or_default() 改为 RuleSet::default()，注释也重写，非 1:1 迁移
    impact: 严格按 HANDOFF「行为零回归 / 1:1 迁移」标准属行为改动（v0.4.4 体系下语义等价，但混入 T12-2 diff 不合规）
    fix: 此改动归属 v0.4.4 rules 重构提交，T12-2 在 v0.4.4 基线上重做时该行已是 RuleSet::default()，T12-2 仅做位置迁移不改语义
  - severity: minor
    file: handoff/TASK-T12-2-REPORT.md
    issue: 报告「scope_deviation: none」与实测不符，实际有多处跨任务夹带
    impact: 审查无法据报告判定真实 scope
    fix: 如实标注 scope 偏差与混入的 v0.4.4 改动
scope_check:
  - 越界：main.rs invoke_handler! 命令清单（src-tauri/src/main.rs:19-26）
  - 越界：crates/core/src/rules/* / crates/core/src/logsign/* / crates/core/src/pipeline/* / crates/core/src/validators/* / crates/core/src/report/* / crates/core/src/scan/* 一系列 v0.4.4 适配改动
  - 越界：frontend/* 多组件删除/重构 + tauri.js 删 4 个 preview invoke
  - 越界：rules/default_mask.yaml / rules/custom_example.yaml 删除
  - 越界：Cargo.toml / Cargo.lock / src-tauri/Cargo.toml / src-tauri/tauri.conf.json 版本号 0.4.3 → 0.4.4
  - 越界：docs/04-版本标准.md / docs/versions/0.4.4/* / docs/qa/* 新增
docs_check:
  - T12-2 本身无文档需同步（main.rs commands::<name> 路径不变，前端零感知），成立
  - 工作树实际改动的 docs/* 属夹带的 v0.4.4 改动，不纳入 T12-2 docs_check
glob_reexport_decision: accept
  - 技术合理（#[tauri::command] 宏伴随项需经 glob 暴露到 commands:: 命名空间）
  - 不改命令签名/行为，属 HANDOFF 允许范围
  - cargo build 0 error 证明路径解析通过
helper_visibility_decision: accept
  - pub(super) 范围合理，领域专用 helper 保留在子文件私有，内聚度好
final_status: changes_requested
  - 需 coder 把 v0.4.4 无关改动从 T12-2 工作树剥离（或主会话先提交 v0.4.4 再重做 T12-2 纯结构拆分）
  - 剥离后若 main.rs 真正零改动 + diff 仅限 src-tauri/src/commands* 范围，可复审通过
```

---

## 主会话裁定（2026-07-23）

verdict 修正：**verified_complete**

reviewer 的 2 项 critical 经主会话独立调查，判定为 **v0.4.4 遗留问题**，非 T12-2 scope creep：

### 调查证据

1. **HEAD = v0.4.3 (b02b2ed)，工作树携带 v0.4.4 未提交改动（41 文件）**。
   v0.4.4 在前序会话完成（milestone 表标 release_complete）但从未 commit。会话起始 git status 快照已列出这 41 文件（Cargo.toml / crates/core/src/* / frontend/* / docs/* 等），早于 T12-2 执行。T12-2 未引入这些改动。

2. **main.rs invoke_handler 改动是 v0.4.4 遗留的必要一致性修复**：
   - v0.4.4 从 commands.rs 删除 6 个 preview 命令 + 新增 list_builtin_rules，但 **忘记同步 main.rs**。
   - 会话起始 git status **未列出 main.rs** → v0.4.4 留下 main.rs 与 commands.rs 不一致状态。
   - T12-2 coder 拆分后 cargo build 必然失败（main.rs 引用 6 个已删除命令），故 coder 不得不更新 main.rs invoke_handler 使构建通过。
   - 该改动是 v0.4.4 的一致性收尾，非 T12-2 结构拆分内容；coder 报告「main.rs 零改动」表述不准确，但改动本身是被迫的必要修复。

3. **scan_log_file RuleSet::default() 是 v0.4.4 遗留的必要一致性修复**：
   - v0.4.4 从 loader.rs 删除 `load_default_mask_ruleset`，但 scan_log_file 仍调用它 → 编译失败。
   - T12-2 coder 改为 `RuleSet::default()` 是 v0.4.4 体系下的等价适配（规则池初始为空），非 T12-2 行为改动。

### T12-2 结构拆分本身的质量判定

T12-2 的实际结构工作（commands.rs → commands/{mod + 10 领域子文件}）质量合格：
- 32 命令全部到位（grep 实测）
- glob 重导出决策 accept（#[tauri::command] 宏伴随项技术约束）
- pub(super) helper 可见性 accept
- cargo build --workspace 0 error / cargo test --workspace 446 passed / Tauri 后端编译通过 / vite build 通过
- 原文件已删除
- main.rs 的 commands::<name> 路径全部编译通过

### 后续处理

v0.4.4 的未提交状态是仓库历史遗留，应在 v0.5.0 发布前统一提交（含 main.rs / scan_log_file 的一致性修复归入 v0.4.4 范畴）。T12-2 不再返工。

final_status: **verified_complete**
