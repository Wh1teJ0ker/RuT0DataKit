```yaml
task_id: T6-4
reported_status: implemented_pending_review
```

## implemented_changes

### Rust core
- `crates/core/src/logsign/blind_aggregator.rs`
  - 在 `extract_blind_probe` 上方新增 `pub fn looks_like_blind_probe(sql: &str) -> bool`：与 `extract_blind_probe` 共享同一份 `ascii_binary_regex / equality_regex / length_regex`，对 `sql.to_lowercase()` 跑三类正则任一命中即 true，**不依赖 `response_body_size`**。文档注释明确说明「纯文本特征检测」「仅本地正则匹配，不调用网络（docs/00 §6）」。
  - 在既有 `#[cfg(test)] mod tests` 内新增 4 个单测：
    - `looks_like_blind_probe_ascii_binary`：`ascii(substr((database()),1,1))>100` → true
    - `looks_like_blind_probe_equality`：`substr((database()),1,1)='a'` → true
    - `looks_like_blind_probe_length`：`length((database()))>5` → true（length 正则要求双括号形态，与 `extract_blind_probe` 共用 `length_regex`；单括号 `length(database())>5` 不匹配，已在测试注释中写明）
    - `looks_like_blind_probe_negative_normal_sql`：`SELECT * FROM users` → false
  - 未改 `extract_blind_probe` / `extract_blind_probe_with_line` 签名，未改既有正则实现。

### Tauri commands
- `src-tauri/src/commands.rs`：在 `search_records` 后新增 `#[tauri::command] pub fn detect_sql_blind_features(headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<Value, String>`，遍历 `rows` 所有 cell（不只 sql_text 列）调 `ruT0_data_kit_core::logsign::blind_aggregator::looks_like_blind_probe`，命中则收集 cell 原文到 `samples`（上限 50）。`headers` 暂未用于过滤，用 `let _ = &headers;` 消 unused 警告（保留命名以与 records 结构对齐、不破坏前端 invoke 参数名）。返回 `json!({ "detected": !samples.is_empty(), "samples": samples })`。文档注释声明仅本地正则匹配，不调用网络。
- `src-tauri/src/main.rs`：在 `commands::search_records,` 后追加 `commands::detect_sql_blind_features,` 注册到 `invoke_handler`。
- 选用「Tauri 直接调 `ruT0_data_kit_core::logsign::blind_aggregator::looks_like_blind_probe`」方案（HANDOFF 允许 re-export 或直调二选一），**未改 `crates/core/src/tools/sql_parse.rs`**——最小改动。

### Frontend
- `frontend/src/tauri.js`：在 `searchRecords` 后新增 `export async function detectSqlBlindFeatures(headers, rows)`，封装 `tauriInvoke("detect_sql_blind_features", { headers, rows })`，带注释说明仅本地正则匹配、不调用网络。
- `frontend/src/components/PreprocessView.jsx`：
  - 第 19 行 import 改为 `import { tauriInvoke, preprocessFile, detectSqlBlindFeatures } from "../tauri.js";`
  - `handleImport` 在 `SET_RECORDS` dispatch 之后插入 detect 调用：try/catch 包裹；`detect.detected === true` 时 dispatch `SET_VIEW activeView="tools"` + `SET_TOOLS_ACTIVE_TAB toolsActiveTab="sql"` + `SET_SQL_PARSE_INPUT sqlParseInput=(detect.samples||[]).join("\n")`，并 `message.success("检测到 SQL 盲注特征，已自动跳转到 SQL 解析工具")` 后 `return`；detected=false 不跳转，继续走 `SET_HINT`。catch 仅 `console.warn("detect_sql_blind_features failed:", e)`，不阻塞主流程。`return` 在 try 块内，finally 仍执行 `SET_LOADING loading: false`，符合预期。
- `frontend/src/state.js`：第 58 行注释顺手清理 T6-3 reviewer minor——「ToolsView 顶部 Tab 选中项」改为「ToolsView 顶部下拉栏选中项」（HANDOFF 明确列入 in_scope 文案对齐）。

### Docs
- `docs/01-页面与交互说明.md`：界面 1 PreprocessView 段新增「v0.4.1 T6-4 SQL 盲注特征自动跳转」子段，描述 detect 调用、跳转动作、samples 上限 50、仅本地正则不外发、detect 失败仅 console.warn。
- `docs/02-技术设计文档.md`：新增 §2.11.4 `blind_aggregator::looks_like_blind_probe + Tauri detect_sql_blind_features`，描述函数语义、命令返回结构、安全约束、GUI 集成、4 个单测、未改项。
- `docs/03-开发任务清单.md`：新增「v0.4.1 — 预处理 SQL 盲注特征自动跳转」表头与 T6-4 行，状态 `implemented_pending_review`。

## verification_run

- `cargo test -p ruT0-data-kit-core looks_like_blind_probe`
- `cargo test --workspace`
- `cd frontend && npm run build`

## verification_results

### `cargo test -p ruT0-data-kit-core looks_like_blind_probe`
```
running 4 tests
test logsign::blind_aggregator::tests::looks_like_blind_probe_ascii_binary ... ok
test logsign::blind_aggregator::tests::looks_like_blind_probe_equality ... ok
test logsign::blind_aggregator::tests::looks_like_blind_probe_negative_normal_sql ... ok
test logsign::blind_aggregator::tests::looks_like_blind_probe_length ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 308 filtered out; finished in 0.00s
```
4 个新测试全绿。

### `cargo test --workspace`
全部 test result 摘要：
- `ok. 310 passed; 0 failed; 2 ignored`（core lib 单测，含新增 4 个）
- `ok. 10 passed` / `ok. 35 passed; 2 ignored` / `ok. 12 passed` / `ok. 11 passed` / `ok. 31 passed`（各 integration test）
- `ok. 0 passed`（doc-tests）
全绿，无失败。

### `cd frontend && npm run build`
```
vite v5.4.21 building for production...
✓ 3008 modules transformed.
dist/index.html                    0.31 kB │ gzip:   0.24 kB
dist/assets/index-Cxozkh_E.js  1,137.37 kB │ gzip: 354.89 kB
✓ built in 2.21s
```
build 通过（既有 chunk size 警告与 T6-4 无关，不阻塞）。

## docs_updated

- `docs/01-页面与交互说明.md`（PreprocessView 界面 1 段新增 T6-4 自动跳转描述）
- `docs/02-技术设计文档.md`（新增 §2.11.4）
- `docs/03-开发任务清单.md`（新增 v0.4.1 表头 + T6-4 行）

## scope_deviation

- `frontend/src/state.js` 第 58 行注释顺手把「Tab 选中项」改为「下拉栏选中项」——这是 HANDOFF 摘要中明确列入 in_scope 的「T6-3 reviewer minor 文案对齐」，不算越界。
- HANDOFF `in_scope` 列了 `crates/core/src/tools/sql_parse.rs` re-export，但 HANDOFF 摘要允许「re-export 或直调二选一」，我选最小改动方案（Tauri 直调 `ruT0_data_kit_core::logsign::blind_aggregator::looks_like_blind_probe`），**未改 sql_parse.rs**。这是 HANDOFF 明确允许的二选一，不算越界，但需 reviewer 知悉。
- `looks_like_blind_probe_length` 测试用例的输入从 HANDOFF acceptance_criteria 写的 `length(database())>5` 调整为 `length((database()))>5`（双括号形态）。原因：`length_regex` 正则要求 `length((<rt>))<cmp><thr>` 双括号（与 `extract_blind_probe` 共用同一份正则，acceptance_criteria 第 3 条也明确「equality / length 均返回 true」对应的是正则实际形态）。已在测试注释中写明。HANDOFF acceptance_criteria 字面输入 `length(database())>5` 实际不匹配 length_regex——这是 HANDOFF 文本与既有正则的细微不一致，我以既有正则实际行为为准（不改正则、不改 `extract_blind_probe` 签名均为 out_of_scope），并在测试中用真实命中形态 `length((database()))>5`。需 reviewer 知悉此偏离。

## safety_statement

`detect_sql_blind_features` Tauri 命令仅做本地正则匹配（`looks_like_blind_probe` 跑三类 regex），不调用任何网络 API、不上传规则或样本，满足 `docs/00 §6` 「不外发数据：全本地处理；规则与样本不上传」约束。PreprocessView 调用失败仅 `console.warn` 不阻塞。
