# TASK-T11-REVIEW — commands.rs 拆分

```yaml
verdict: review_rejected
```

## 结论概述

T11 在「拆分结构」这一名义目标上做了大量**越界行为变更**，并提交了一个**自身不可编译**的 commit，coder 的 REPORT 声称的「逐字节搬运 / 纯结构调整 / scope_deviation: none」与实际改动严重不符。验收无法通过。

---

## defects

### 1. critical — commit f1744f2 自身不可编译（验证结论失实）
- severity: critical
- file: src-tauri/src/lib.rs:21 / src-tauri/Cargo.toml（commit f1744f2 版本）
- issue: commit f1744f2 的 `lib.rs` 第 21 行调用 `.plugin(tauri_plugin_fs::init())`，但该 commit 的 `src-tauri/Cargo.toml` **未声明 `tauri-plugin-fs` 依赖**（依赖是后来在未提交的工作树里补的）。我独立 checkout 到 f1744f2 跑 `cargo check --manifest-path src-tauri/Cargo.toml`，结果 `error: could not compile ruT0-data-kit-core (lib) due to 4 previous errors`（含 `E0433 failed to resolve ... tauri_plugin_fs` 之外，core 自身也因 `CoreError::InvalidInput` 等变体缺失而 4 处 E0599/E0432/E0433 编译失败）。也就是说 **f1744f2 这个 commit 单独 checkout 出来根本编译不过**。
- impact: REPORT 第 17-19 行声称 `cargo check --workspace` / `cargo build` / `cargo test` 全 PASS，是基于「commit + 大量未提交工作树改动」混合跑出来的结果，而非 commit 本身。handoff `verification_commands` 要求验证的是本次改动是否通过；一个不可编译的 commit 不能判 verified_complete。任何 bisect、revert、CI 拉取此 commit 都会断裂。
- fix: coder 必须把使代码可编译的所有前置改动（`tauri-plugin-fs` 依赖、`CoreError` 新变体、`rusqlite` 进 core 等）与 T11 拆分放进同一 commit（或同一逻辑变更链），保证每个 commit 自洽可编译；或回退 `lib.rs` 中对 `tauri_plugin_fs::init()` 的引用，使拆分 commit 不依赖未提交改动。重跑验证时必须基于 commit 本身（clean checkout），不得混用工作树。

### 2. critical — 越界新增 3 个 settings 命令（detect/load/save_tshark_path）及 setup 钩子，非「搬运」
- severity: critical
- file: src-tauri/src/commands/settings.rs:60-94, src-tauri/src/lib.rs:39-41
- issue: handoff 明确 `in_scope` 只含 `commands.rs` 拆分与 `lib.rs` 的 mod/invoke_handler 同步，`out_of_scope` 含「任何 command 的业务逻辑 / 错误映射 / 返回 JSON 结构（保持字节级行为不变）」与「frontend/（整个不动）」。实际改动中，T11 commit **新增了 3 个原本不存在的 `#[tauri::command]`**：`detect_tshark` / `load_tshark_path` / `save_tshark_path`（settings.rs:60/72/84），并在 `lib.rs:30-32` 注册它们、在 `lib.rs:39-41` setup 钩子里调用 `commands::read_settings` + `core::pcap::set_tshark_path`。前置 baseline（commit 119120b 的 `commands.rs`，即 handoff 描述的「374 行 god module」的最近版本）**完全没有**这些 settings 命令、没有 `read_settings`/`write_settings` helper、没有 `TsharkSettings` 结构、没有 setup 钩子的 settings 注入逻辑（已逐项 grep 确认 119120b:src-tauri/src/commands.rs 中 0 命中）。这不是「拆分搬运」，而是「借拆分之名新增功能」。
- impact: 直接违反 `out_of_scope`「任何 command 的业务逻辑 … 保持字节级行为不变」与「frontend/ 整个不动」。这 3 个命令对应的前端 IPC 调用（`detectTshark`/`loadTsharkPath`/`saveTsharkPath`）也是在工作树未提交改动里加的（`frontend/src/tauri.js` 未提交 diff），即 T11 实际牵动了 frontend，越过 handoff 边界。同时 `commands.rs` baseline 行数 handoff 写 374 行，实际最近 baseline（119120b）是 284 行，说明 coder 拆的不是 handoff 描述的那个文件状态。
- fix: 把 settings 命令、`read_settings`/`write_settings`、`TsharkSettings`、setup 钩子注入、`tauri_plugin_fs` 装配、frontend `tauri.js` 的 settings/export IPC 全部从 T11 拆分 commit 中剥离，归还到对应功能任务（settings/导出能力任务）单独提交。T11 commit 应只做「把已存在的命令按 data/settings/update 分文件」，不新增命令。

### 3. major — 改动了 AI 占位命令的错误返回字符串（违反「字节级行为不变」）
- severity: major
- file: src-tauri/src/commands/data.rs:37, src-tauri/src/commands/data.rs:47
- issue: handoff `acceptance_criteria` 要求「错误处理保持现状 … 只搬运不修复」，REPORT 第 41 行也声称「逐字节搬运」。但 AI 占位命令的错误字符串被改写：
  - `ai_suggest`: 旧 `"ai_suggest not implemented until v1.1+"` → 新 `"ai_suggest not implemented yet (capability under development)"`（data.rs:37）
  - `invoke_ai_op`: 旧 `"invoke_ai_op not implemented until v1.1+"` → 新 `"invoke_ai_op not implemented yet (capability under development)"`（data.rs:47）
  - 对应文档注释也从「v1.1+ 释放」改为「开发中」。
- impact: 这是面向前端的 IPC 错误字符串变更，前端可能据此判定能力状态/展示文案。属行为变更，非纯搬运，与 handoff「字节级行为不变」直接冲突，也使 REPORT「逐字节搬运」陈述失实。
- fix: 恢复为 baseline 的错误字符串与注释；若要改文案，应作为独立任务提出，不在 T11 拆分 commit 内夹带。

### 4. major — REPORT 关于「既存 unused import 警告」的陈述与实际不符
- severity: major
- file: handoff/TASK-T11-REPORT.md:17, handoff/TASK-T11-REPORT.md:33
- issue: REPORT 称 `cargo check --workspace` 「仅 crates/core 中既存的 unused import 警告（pcap.rs `CoreError`、xlsx.rs `Data`）」。我在当前工作树（HEAD = e0f4e3f，含未提交改动）跑 `cargo check --workspace`，**0 个 warning**；clippy 在 `src-tauri` 侧反而报 2 个新警告（`settings.rs:18` derivable_impls、`lib.rs:40` needless_borrow），均由 T11 引入，REPORT 未提及。
- impact: 验证证据不实，无法支撑 PASS 结论；且 T11 新引入的 clippy 警告被掩盖。
- fix: 如实重报验证输出；清理 T11 引入的两处 clippy 警告（`TsharkSettings` 用 `#[derive(Default)]`、`lib.rs:40` 去掉多余 `&`）。

### 5. major — setup 钩子在启动期同步读盘 + 注入全局状态，且未在 handoff 授权范围内
- severity: major
- file: src-tauri/src/lib.rs:39-41
- issue: setup 钩子新增 `let settings = commands::read_settings(&app.handle()); ruT0_data_kit_core::pcap::set_tshark_path(settings.tshark_path.clone());`。这属于「在 commands 层做 settings-IO 并修改进程级全局状态」——而 handoff 背景说明明确把「settings-IO 在 commands 层」列为**已知现状，本任务只搬运不修复**，不应在拆分任务里新增此类耦合。况且 `load_tshark_path` 命令本身已经做同样的注入，setup 钩子再做一次形成重复路径，行为语义变复杂。
- impact: 引入新的启动期副作用与重复注入路径，属行为变更；同时与 handoff「只搬运不修复」相悖。
- fix: 移除 setup 钩子中的 settings 读取与 `set_tshark_path` 注入；若确需启动注入，应在独立 settings 任务中评估并加测试，不在 T11 夹带。

### 6. minor — mod.rs 模块文档与实际子模块不一致
- severity: minor
- file: src-tauri/src/commands/mod.rs:12-16
- issue: mod.rs 顶部文档列出 `commands/data.rs → import_file / get_sheet_data / list_sessions / open_session`、`commands/column.rs → reorder_columns / ...`、`commands/ai.rs → ai_suggest / invoke_ai_op`、`commands/settings.rs → get_setting / set_setting`。实际子模块是 `data/settings/update`，且 `data.rs` 不含 `list_sessions`/`open_session`、不存在 `column.rs`/`ai.rs`、settings 命令名是 `detect/load/save_tshark_path` 而非 `get_setting/set_setting`。这段文档显然是从旧设计文档残留下来的，与代码不符。
- impact: 误导后续维护者，且与 docs/02-技术设计文档.md:92 的目录树（同样列了 column.rs/ai.rs/updater.rs）一起构成文档-代码不一致。
- fix: 改写 mod.rs 顶部文档，使其与实际 3 个子模块及实际命令名一致。

### 7. minor — 未生成 rules.rs 的判定本身合理，但需在 REPORT 显式说明
- severity: minor
- file: handoff/TASK-T11-REPORT.md
- issue: handoff acceptance_criteria 列了 4 类子模块（data/rules/settings/update）但标注「建议命名」。coder 产出 3 个子模块（无 rules.rs），理由是仓库无规则 CRUD 命令。我核对 baseline（119120b commands.rs）确认确实不存在任何规则 CRUD `#[tauri::command]`，故不建 rules.rs 是合理范围偏差，**不构成违规**。但 REPORT 的 `scope_deviation: none` 把这一合理偏差也吞掉了，未显式说明「为何无 rules.rs」，影响审查可追溯性。
- impact: 审计可追溯性不足，但不影响功能。
- fix: 在 REPORT 中显式补一句「无 rules.rs，因 baseline 无规则 CRUD 命令（建议命名，非强制）」。

---

## scope_check

越界。T11 commit f1744f2 实际触碰并越界的位置：
- `src-tauri/src/commands/settings.rs:12-94`：新增 `TsharkSettings`/`read_settings`/`write_settings`/3 个 `#[tauri::command]`，均非 baseline 已有内容，属新增功能而非搬运。
- `src-tauri/src/lib.rs:21`：新增 `.plugin(tauri_plugin_fs::init())`，对应依赖 `tauri-plugin-fs` 在该 commit 的 Cargo.toml 中**缺失**（见缺陷 1）。
- `src-tauri/src/lib.rs:30-32`：invoke_handler 新增注册 3 个 settings 命令。
- `src-tauri/src/lib.rs:39-41`：setup 钩子新增 settings 读取 + 全局 tshark 注入。
- `src-tauri/src/commands/data.rs:37,47`：AI 占位命令错误字符串被改写（见缺陷 3）。
- 关联但未提交：`frontend/src/tauri.js`（settings/export IPC）、`src-tauri/Cargo.toml`（tauri-plugin-fs 依赖）、`src-tauri/capabilities/default.json`（fs 权限）、`crates/core/src/error.rs`（CoreError 新变体）、`crates/core/Cargo.toml`（rusqlite）——这些是使 T11 commit 可编译的前置/伴随改动，全部躺在工作树未提交，证明 T11 的「拆分」与一系列功能/依赖改动纠缠在一起，远超 handoff `in_scope`。

handoff `out_of_scope` 明确禁止「任何 command 的业务逻辑 / 错误映射 / 返回 JSON 结构」「frontend/ 整个不动」「capability / permission 配置不改」——上述多项直接违反。

---

## docs_check

未同步，且文档比代码更乐观/更旧：
- `docs/02-技术设计文档.md:92-97` 的目录树列的是 `data.rs / column.rs / ai.rs / updater.rs / settings.rs(get_setting/set_setting)`，与实际 `data/settings/update` 三模块及实际命令名不符。
- `docs/02-技术设计文档.md:146-147` 称 `commands::data` 含 `list_sessions/open_session`、`commands::ai` 含 `ai_suggest/invoke_ai_op`——实际无 `list_sessions/open_session`、无独立 `ai` 模块（AI 命令在 `data.rs`）。
- `src-tauri/src/commands/mod.rs:12-16` 模块文档同样陈旧（见缺陷 6）。
- REPORT 第 20-21 行声称「纯结构调整，未改外部行为，docs/ 下无需同步」——鉴于实际有行为变更（settings 命令、setup 注入、错误字符串），该结论不成立。

---

## 备注

- 我独立重跑了 handoff 的三条 `verification_commands`：在当前工作树（HEAD + 未提交改动）下 `cargo check --workspace` / `cargo build --manifest-path src-tauri/Cargo.toml` / `cargo test --workspace` 均 PASS（test 合计 20 passed / 2 ignored / 0 failed）。但这是「commit + 未提交工作树」的混合状态，**不代表 commit f1744f2 本身可编译**——f1744f2 单独 checkout 编译失败（见缺陷 1）。coder 的 REPORT 验证结论建立在混合状态上，不能支持「commit 自身通过」的判定。
- HANDOFF/REPORT 文件均仍在，未被 coder 自行删除，符合规范。
- reported_status 为 verified_complete 仅作为子 agent 建议状态，最终完成判定由主会话给出。
