# TASK-T5-REPORT — 导入流（CSV/XLSX → DB → Table）

## implemented_changes

### 后端（Rust）

- `crates/core/Cargo.toml`
  - 新增依赖 `csv = "1"`、`calamine = "0.26"`。
  - 新增 `[dev-dependencies] tempfile = "3"`（datasource 单测用）。
  - 说明：HANDOFF 要求 `calamine = "0.27"`，但 0.27 依赖的 `zip = "~2.5.0"` 已被 yank，无法解析；降级到 0.26（API 兼容，`Data` 枚举无 `Duration` 变体，已在 `cell_to_string` 中对齐）。这是实现过程中发现的与 HANDOFF 字面要求不符的必要偏差，仅版本号变化，功能等价。

- `crates/core/src/datasource/mod.rs`
  - 实现 `CsvReader`：`csv::ReaderBuilder`（`has_headers=false` + `flexible=true`），首行作表头，`read_all` 返回含表头行在内的 `Vec<Record>`（表头行 `row_idx=0` 的 fields 为列名自映射），`headers` 返回首行。
  - 实现 `XlsxReader`：`calamine::open_workbook_auto` 取首个工作表，首行作表头，`cell_to_string` 处理 `Data` 各变体（Empty/String/DateTimeIso/DurationIso/DateTime/Int/Float/Bool/Error；Float 整数去 `.0`）。
  - `detect_format(path)`：按扩展名 `.csv` → `CsvReader`、`.xlsx` → `XlsxReader`，其它返回 `CoreError::NotImplemented`。
  - 新增 3 个单测：`detect_format_routes_by_extension`、`csv_reader_headers_and_rows`、`csv_reader_flexible_columns`（全绿）。

- `src-tauri/src/commands.rs`
  - 新增 `use crate::db::Cell;` + `use ruT0_data_kit_core::datasource;`。
  - 新增 `ImportResult { session_id, sheet_id, row_count, headers }`（`#[serde(rename_all = "camelCase")]`）。
  - 新增 `PageData { headers, rows, total, page, page_size }`（`#[serde(rename_all = "camelCase")]`）。
  - 新增 `import_file(path, db: State<'_, DbManager>) -> Result<ImportResult, String>`：
    - `detect_format` → `headers()` + `read_all()`；
    - `db.create_session(name=file_stem, source_path=path, source_type=ext, row_count)`；
    - `db.create_sheet(session_id, name=file_stem, position=0)`；
    - 分批 `db.write_cells`（`IMPORT_BATCH_ROWS=5000`，每批 commit 一次，每行的列按 `headers` 顺序写入，空值过滤空字符串）；
    - `db.log_operation(Some(sheet_id), "import", params_json={path,row_count}, "{}")`；
    - 返回 `ImportResult`。
  - 新增 `get_sheet_data(sheet_id, page, page_size, db) -> Result<PageData, String>`：
    - `db.query_cells` + `db.count_rows`；
    - headers 从 `row_idx=0` 的 cell value 取（本页无首行时回查首页）；
    - 按 `row_idx` 分组（`BTreeMap` 保序）、组内按 `col_idx` 排序、缺列补 `None`，跳过表头行 `row_idx=0`；
    - 返回 `PageData`。
  - 未触碰 T6 的 `UpdateStatus`/`check_update`/`install_update`，未触碰 T7 的 `AiContext`/`AiSuggestion`/`ai_suggest`/`invoke_ai_op`。

- `src-tauri/src/lib.rs`
  - `generate_handler!` 在原有 4 项（check_update/install_update/ai_suggest/invoke_ai_op）后**追加** `commands::import_file`、`commands::get_sheet_data`，未移除任何已有项。

### 前端（React）

- `frontend/src/tauri.js`
  - 新增 `importFile(path)` → `invoke('import_file', { path })`。
  - 新增 `getSheetData(sheetId, page, pageSize)` → `invoke('get_sheet_data', { sheetId, page, pageSize })`。
  - 保留 T7 的 `aiSuggest`/`invokeAiOp`。

- `frontend/src/state.js`
  - 新增 `createSheetFromImport(result)` 工厂：用 `ImportResult` 填充 Sheet（id=sheetId、sessionId、headers、total=rowCount、rows 初始空）。
  - `ACTION` 新增 `IMPORT_SUCCESS`、`SET_SHEET_DATA`。
  - `reducer` 新增两 case：
    - `IMPORT_SUCCESS`：用 `createSheetFromImport` 构造 Sheet，同 sheetId 替换否则追加，切换 `activeSheetId`。
    - `SET_SHEET_DATA`：用 `PageData`（headers/rows/total/page/pageSize）更新对应 Sheet，rows 转为 antd 行对象（key、按 header 映射、status=default）。
  - `useAppState` 新增 `importSuccess`/`setSheetData` 回调并返回。
  - 初始 `sheets=[]`、`activeSheetId=null`（移除原 mock 注入，由真实导入流填充）。

- `frontend/src/components/layout/TopToolbar.jsx`
  - 「导入」按钮接入：`@tauri-apps/plugin-dialog` 的 `open`（filters csv/xlsx）→ `importFile(path)` → `onImport(importPayload)` → `message.success`；失败 `message.error`；`loading` 态防重入。
  - 其余按钮/能力按钮/布局不变。

- `frontend/src/App.jsx`
  - 顶层接入导入流：`handleImport(payload)` → `dispatch(IMPORT_SUCCESS)` → `getSheetData(sheetId,1,50)` → `dispatch(SET_SHEET_DATA)`。
  - `handleSetPage(page)`：翻页时 `setPage(page)` + `getSheetData(sheet.id,page,pageSize)` + `dispatch(SET_SHEET_DATA)`。
  - 将 `handleSetPage` 作为 `setPage` 传给 `Workbench`（替代原裸 `setPage`）。

## verification_run

实际运行命令（cwd=`/Users/joker/Code/RuT0DataKit`）：

1. `cargo check --workspace`
2. `cargo test --workspace`
3. `pnpm --prefix frontend run build`
4. `grep -n 'import_file\|get_sheet_data\|ImportResult\|PageData' src-tauri/src/commands.rs`
5. `grep -n 'generate_handler' src-tauri/src/lib.rs`
6. `grep -n 'rename_all' src-tauri/src/commands.rs`
7. `grep -n 'importFile\|getSheetData' frontend/src/tauri.js`
8. `grep -n 'csv\|calamine\|read_all\|detect_format' crates/core/src/datasource/mod.rs`
9. `grep -n 'check_update\|install_update\|ai_suggest\|invoke_ai_op' src-tauri/src/lib.rs`
10. sqlite3 持久化核验：**未运行**（需先在 GUI 环境实际导入一次数据才有 operations 记录；coder 环境无 DB 实例，且 acceptance_criteria 8/10 的 GUI 导入核验由主会话环境补，HANDOFF 第 94 行已明确）。

## verification_results

| # | 命令 | 结果 | 摘要 |
|---|------|------|------|
| 1 | `cargo check --workspace` | PASS | Finished，仅 5 条 pre-existing warning（snake_case crate 名、db 未用方法），无 error |
| 2 | `cargo test --workspace` | PASS | 8（db）+ 3（datasource）+ 0（main/doctest）全绿，0 failed |
| 3 | `pnpm --prefix frontend run build` | PASS | vite build ✓ 3069 modules，dist 产物生成（chunk size warning 非错误） |
| 4 | grep commands.rs | PASS | 命中 import_file(146,149)、get_sheet_data(220,225)、ImportResult(123,149,207)、PageData(133,225,277) |
| 5 | grep generate_handler | PASS | 命中 lib.rs:18 |
| 6 | grep rename_all | PASS | 命中 5 处（UpdateStatus/AiContext/AiSuggestion/ImportResult/PageData） |
| 7 | grep tauri.js | PASS | 命中 importFile(32)、getSheetData(43) |
| 8 | grep datasource | PASS | 命中 csv/calamine/read_all/detect_format |
| 9 | grep T6/T7 回归 | PASS | 命中 check_update(19)/install_update(20)/ai_suggest(21)/invoke_ai_op(22) |
| 10 | sqlite3 持久化 | 未运行 | 见上 |

## docs_updated

无。HANDOFF `out_of_scope` 明确「不许动 docs/」。

## acceptance_criteria 证据映射

1. CSV+XLSX 读取 + detect_format：`crates/core/src/datasource/mod.rs` 的 `CsvReader`/`XlsxReader`/`detect_format`，单测 `csv_reader_headers_and_rows`/`detect_format_routes_by_extension` 通过。
2. import_file 全链路：`commands.rs:146` `import_file`，调 `create_session`+`create_sheet`+分批 `write_cells`（5000/批）+`log_operation(kind="import")`。
3. get_sheet_data 分页+分组：`commands.rs:220` `get_sheet_data`，`query_cells`+`count_rows`，BTreeMap 按 row_idx 分组、跳过表头行。
4. camelCase：`ImportResult`(commands.rs:122) / `PageData`(commands.rs:132) 均 `#[serde(rename_all = "camelCase")]`，grep #6 命中。
5. 前端导入按钮→选文件→importFile→dispatch→getSheetData→渲染：`TopToolbar.jsx` handleImport + `App.jsx` handleImport/handleSetPage + `state.js` IMPORT_SUCCESS/SET_SHEET_DATA。编译/构建通过；GUI 实际渲染首页 50 行 + 翻页由主会话环境补。
6. 重启后历史 Session 加载：依赖 `list_sessions`/`get_session`（T4 已实现，未动）；本任务未新增打开历史 Session 的前端入口（属 T8/后续范围，HANDOFF in_scope 未要求前端历史列表 UI）。后端 `get_sheet_data` 已支持任意 sheet_id 分页查询，可被历史加载路径复用。
7. operations 表 import 记录：`import_file` 末尾 `db.log_operation(Some(sheet_id), "import", ...)`；sqlite3 核验未运行（见上）。
8. cargo check + cargo test + pnpm build：全 PASS（见上表）。
9. T6/T7 产物未破坏：grep #9 命中 4 项全在；`generate_handler!` 含 6 项（4 旧 + 2 新）。
10. 1000 行 CSV 导入 GUI 核验：coder 环境无法运行 GUI，由主会话环境补。

## reported_status

verified_complete（建议状态）——coder 侧要求的全部验证命令（cargo check/test + pnpm build + 6 条 grep）均通过；sqlite3/GUI 核验属主会话环境范围，已标注未运行原因。最终完成由主会话判定。

## scope_deviation

- `calamine` 版本由 HANDOFF 指定的 `0.27` 降为 `0.26`：因 0.27 的传递依赖 `zip = "~2.5.0"` 已被 crates.io yank，无法解析；0.26 API 兼容（`Data` 枚举少一个 `Duration` 变体，已对齐），功能等价。仅版本号偏差，未越界。
- `crates/core/Cargo.toml` 新增 `[dev-dependencies] tempfile = "3"`：为 datasource 单测所需，未在 HANDOFF 显式列出但属于 in_scope 文件的最小必要扩展。
- 前端 `state.js` 移除了原 mock 初始 Sheet（`initialState.sheets = []`）：HANDOFF in_scope 明确「mock 数据标记的 `// TODO(T5): replace mock` 在导入成功路径替换为真实数据」，此为该要求的直接落地。
