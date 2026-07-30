# TASK-T5-HANDOFF — 导入流（CSV/XLSX → DB → Table）

> 本 HANDOFF 与最新代码基线（T6 verified_complete @ 8a976f6）对齐。T5 依赖 T3（工作台 UI）+ T4（持久层）均 `verified_complete`，T6 也已 `verified_complete`。T5 与 T7 存在文件重叠（`commands.rs` + `lib.rs` + `frontend/src/tauri.js`），主会话已将 T7 排在 T5 之前：**T5 必须在 T7 `verified_complete` 后启动**，届时 `commands.rs` 将已含 `ai_suggest`/`invoke_ai_op`，`tauri.js` 将已含 `aiSuggest`/`invokeAiOp`，T5 在其上追加导入命令与封装。

## task_id
T5

## goal
打通「选文件 → 识别 → 新建 Session/Sheet → 写 SQLite cells → 渲染 Table」全链路。v1.0.0 支持 CSV 与 XLSX 两种格式，首行作为表头。

## in_scope
允许新增/修改的文件：

- `crates/core/src/datasource/mod.rs`（T1 骨架）：实现 `Reader` trait 的 CSV/XLSX 读取：
  - CSV：用 `csv` crate（**需在 `crates/core/Cargo.toml` 加 `csv = "1"` 依赖**）。
  - XLSX：用 `calamine` crate（**需在 `crates/core/Cargo.toml` 加 `calamine = "0.27"` 依赖**）。
  - `read_all` 返回 `Vec<Record>`（或等价结构，含表头 + 行数据）；`headers` 返回表头列表。
  - `detect_format` 返回 CSV/XLSX/NotImplemented。
- `src-tauri/src/commands.rs`（扩展 T6+T7 产物）：新增两个命令：
  - `import_file(path: String, db: State<'_, DbManager>) -> Result<ImportResult, String>`：
    - 用 `tauri-plugin-dialog` 已注册（T6 装配）的前端 `open` 选文件后传入路径；本命令负责解析 + 写库。
    - 调 `datasource::detect_format` → 对应 `Reader` → `read_all`。
    - `db.create_session`（name=文件名 stem，source_path，source_type=csv/xlsx，row_count）。
    - `db.create_sheet`（session_id，name=文件名，position=0）。
    - `db.write_cells`（sheet_id, 批量 Cell，每 5000 行 commit 一次）。
    - `db.log_operation`（sheet_id, kind="import", params_json={path,row_count}, result_json={}）。
    - 返回 `ImportResult { sessionId, sheetId, rowCount, headers }`（camelCase）。
  - `get_sheet_data(sheet_id: i64, page: u32, page_size: u32, db: State<'_, DbManager>) -> Result<PageData, String>`：
    - `db.query_cells`（sheet_id, page, page_size）→ 按 row_idx 分组为 `Vec<Vec<Option<String>>>`。
    - `db.count_rows`（sheet_id）→ total。
    - headers：从首行 cell 或从 sheets 表的 `column_order`/首行推导；v1.0.0 可从首行 cell 取（row_idx=0）。
    - 返回 `PageData { headers, rows, total, page, pageSize }`（camelCase）。
  - 定义 `ImportResult`/`PageData` struct，`#[serde(rename_all = "camelCase")]`。
  - **不要动 T6 的 `UpdateStatus`/`check_update`/`install_update`，不要动 T7 的 `AiContext`/`AiSuggestion`/`ai_suggest`/`invoke_ai_op`**。
- `src-tauri/src/lib.rs`：扩展 `generate_handler!` 追加 `commands::import_file`、`commands::get_sheet_data`（T6 两项 + T7 两项保留）。
- `frontend/src/tauri.js`（扩展 T7 产物）：新增 `importFile(path)` → `invoke('import_file', { path })`，`getSheetData(sheetId, page, pageSize)` → `invoke('get_sheet_data', { sheetId, page, pageSize })`。保留 T7 的 `aiSuggest`/`invokeAiOp`。
- `frontend/src/state.js`（T3 产物，扩展）：导入成功后 dispatch 新增 action：
  - 可复用 `ADD_SHEET`（T3 已实现），但需用 `importFile` 返回的 `ImportResult` 填充 Sheet 字段（headers/rowCount），并切换 `activeSheetId`。
  - 新增 action `IMPORT_SUCCESS`（或复用 `ADD_SHEET` + `SET_ACTIVE_SHEET`），payload 含 `sessionId/sheetId/rowCount/headers`。mock 数据标记的 `// TODO(T5): replace mock` 在导入成功路径替换为真实数据。
- `frontend/src/components/Workbench.jsx` 或 `TopToolbar.jsx`（按需）：接入导入按钮 → `tauri.open` 选文件 → `importFile` → dispatch `IMPORT_SUCCESS` → `getSheetData` 首页 → 渲染 DataTable。**不许破坏 T2/T3 已有布局与交互**。

## out_of_scope
明确不许动的：

- **不许动 DB 层**（T4，`src-tauri/src/db/` 全部）。如需新增 DB 方法，改 `db/mod.rs` 属 T4 范围扩展，**本任务通过现有 `query_cells`/`write_cells`/`count_rows`/`create_session`/`create_sheet`/`log_operation` 完成**，不新增 DB 方法。
- **不许动 updater**（T6）。
- **不许动 AI 命令**（T7）。
- **不许动布局组件结构**（T2/T3 的四区布局、Sheet/Tab、DataTable 列拖拽/选区/状态高亮骨架保持不变；本任务只接入导入触发路径，不改交互骨架）。
- **不许实现导出/格式转换/撤销/列操作真实逻辑**（v1.1+）。
- **不许动 docs/**、**不许动 handoff/ 其它任务文件**。

## acceptance_criteria
1. `crates/core/src/datasource/mod.rs` 实现 CSV + XLSX 读取，`read_all`/`headers` 返回正确数据，`detect_format` 识别 `.csv`/`.xlsx`。
2. `import_file` 命令：解析文件 → `create_session` + `create_sheet` + `write_cells`（事务批量，5000 行/批 commit）+ `log_operation(kind="import")` → 返回 `ImportResult`。
3. `get_sheet_data` 命令：分页查询 → 按 row_idx 分组 → 返回 `PageData{headers,rows,total,page,pageSize}`。
4. `ImportResult`/`PageData` 均 `#[serde(rename_all = "camelCase")]`。
5. 前端导入按钮 → 选文件 → `importFile` → dispatch → `getSheetData` → DataTable 渲染首页 50 行，分页可翻。
6. 重启应用后通过「打开历史 Session」（list_sessions + open_session 路径）可重新加载该 Sheet 全量数据（持久化验证）。
7. `operations` 表有对应 `import` 记录（`sqlite3 .dump` 或查询核验）。
8. `cargo check --workspace` + `cargo test --workspace` + `pnpm --prefix frontend run build` 通过。
9. T6/T7 产物未被破坏：`grep -n 'check_update\|install_update\|ai_suggest\|invoke_ai_op' src-tauri/src/lib.rs` 仍全部命中。
10. 导入 1000 行 CSV 样例：Sheet 新建、Table 渲染首页 50 行、分页可翻、重启后历史可加载、operations 有 import 记录。

## verification_commands
```sh
# 1. 编译 + 测试
cargo check --workspace
cargo test --workspace

# 2. 前端构建
pnpm --prefix frontend run build

# 3. 命令与类型核对
grep -n 'import_file\|get_sheet_data\|ImportResult\|PageData' src-tauri/src/commands.rs
grep -n 'generate_handler' src-tauri/src/lib.rs

# 4. serde camelCase
grep -n 'rename_all' src-tauri/src/commands.rs

# 5. 前端封装
grep -n 'importFile\|getSheetData' frontend/src/tauri.js

# 6. datasource 实现
grep -n 'csv\|calamine\|read_all\|detect_format' crates/core/src/datasource/mod.rs

# 7. T6/T7 回归
grep -n 'check_update\|install_update\|ai_suggest\|invoke_ai_op' src-tauri/src/lib.rs

# 8. DB 持久化核验（导入后）
sqlite3 ~/Library/Application\ Support/com.rut0.datakit/ruT0datakit.db "SELECT kind, COUNT(*) FROM operations GROUP BY kind"
sqlite3 ~/Library/Application\ Support/com.rut0.datakit/ruT0datakit.db ".schema"
```

GUI 导入核验（1000 行 CSV → Sheet → Table → 分页 → 重启）由主会话环境补；coder 至少编译 + 测试 + 构建通过，并用样例 CSV 文件描述导入路径。

## files_likely_to_change
- `crates/core/src/datasource/mod.rs`（实现 CSV/XLSX Reader）
- `crates/core/Cargo.toml`（加 csv + calamine 依赖）
- `src-tauri/src/commands.rs`（新增 import_file/get_sheet_data + ImportResult/PageData）
- `src-tauri/src/lib.rs`（generate_handler 追加 2 项）
- `frontend/src/tauri.js`（新增 importFile/getSheetData 封装）
- `frontend/src/state.js`（IMPORT_SUCCESS action 或复用 ADD_SHEET）
- `frontend/src/components/Workbench.jsx` 或 `layout/TopToolbar.jsx`（接入导入按钮触发路径）

## risks
- **commands.rs/tauri.js 与 T7 重叠**：主会话已将 T7 排在 T5 之前，T5 启动时 T7 已 `verified_complete`，`commands.rs` 已含 AI 命令，`tauri.js` 已含 `aiSuggest`/`invokeAiOp`。T5 追加导入命令与封装，不要删除 T7 产物。
- **CSV/XLSX 依赖未在 Cargo.toml**：`crates/core/Cargo.toml` 当前无 `csv`/`calamine`，T5 需新增。
- **大表批量写性能**：5000 行/批 commit，避免单事务过大；`write_cells` 已是事务批量 upsert，分批由调用方控制（可在 `import_file` 内分批调用 `write_cells`）。
- **headers 推导**：XLSX 表头可能在首行，也可能在指定行；v1.0.0 统一取首行。若首行非表头，v1.1+ 再加配置。
- **State<'_, DbManager> 注入**：`import_file`/`get_sheet_data` 命令需从 Tauri State 拿 `DbManager`（T4 已 `app.manage(db_manager)`）。命令签名 `db: tauri::State<'_, DbManager>`。

## depends_on
[T3, T4]（T6/T7 已 verified_complete，T5 在 T7 之后启动避免文件冲突）

## status
planned
