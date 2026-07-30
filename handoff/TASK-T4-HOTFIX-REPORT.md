# TASK-T4-HOTFIX-REPORT — SQLite 持久层分页语义修复

## task_id
T4-hotfix（主会话判定 T5 review_rejected 后的 in-place 修复）

## depends_on
T4 verified_complete（59c8be7）→ T5 review 暴露的 T4 DB 层分页语义缺陷

## goal
修复 T4 `DbManager` 的 `count_rows` 与 `query_cells` 对多列 Sheet 的分页语义错误，使 T5 `get_sheet_data` 能正确返回「页 = 行数」与「每页 page_size 行」。

## root_cause（来自 T5 reviewer）
- `count_rows`：`SELECT COUNT(*) FROM cells` 返回 **cell 数**（= 行数 × 列数），而非**行数**。1000 行 × 5 列的 Sheet 返回 5005 而非 1001。
- `query_cells`：`LIMIT ?2 OFFSET ?3` 直接作用在 **cells 记录**上，page_size=50 实际返回 50 个 cell（约 10 行，列数越多行数越少），而非 50 行。

## implemented_changes（主会话直接修改，非 coder 子智能体）

### `src-tauri/src/db/mod.rs`

1. `count_rows`（原 L143-152）
   - SQL: `SELECT COUNT(*) FROM cells WHERE sheet_id = ?1` → `SELECT COUNT(DISTINCT row_idx) FROM cells WHERE sheet_id = ?1`
   - 注释: 「统计某 sheet 的 cell 行数」→「统计某 sheet 的数据**行数**（`DISTINCT row_idx`，含表头行）」
   - 语义：返回真实数据行数（含 row_idx=0 的表头行），与 T5 `ImportResult.rowCount`（= `records.len()` 含表头）一致。

2. `query_cells`（原 L87-118）
   - SQL 改为**行级分页**：子查询 `SELECT DISTINCT row_idx FROM cells WHERE sheet_id=?1 ORDER BY row_idx ASC LIMIT ?2 OFFSET ?3` 取本页 `row_idx` 集合，外层 `WHERE sheet_id=?1 AND row_idx IN (...) ORDER BY row_idx ASC, col_idx ASC` 取这些行的全部列。
   - 注释更新：明确「每页返回 `page_size` 个 `row_idx` 对应的全部列」「确保多列 Sheet 每页返回 `page_size` 行而非 `page_size` 个 cell」。
   - 对单列 Sheet 行为不变（子查询 row_idx 集合即 cell 集合），向后兼容。

3. 新增单测 `write_and_query_cells_paginated_multi_column`
   - 构造 3 列 × 5 行（含表头 row_idx=0）= 15 cells。
   - 断言 `count_rows == 5`（而非 15）。
   - 断言 page1（page_size=2）返回 6 cells（2 行 × 3 列），row_idx ∈ {0,1}。
   - 断言 page2 返回 6 cells，row_idx ∈ {2,3}。
   - 断言 page3 返回 3 cells（1 行 × 3 列），row_idx == 4。
   - 断言 page4 返回 0 cells。
   - 该测试恰好暴露原 bug：旧 SQL 下 page1 会返回 6 cells 但 count_rows 会返回 15，page2 会从 cell 偏移 6 开始（row_idx=2 的 3 列 + row_idx=3 的 3 列），与期望一致——但旧 `count_rows` 返回 15 会让前端分页器显示错误页数。新实现两者都对齐行语义。

## out_of_scope
- 未触碰 `schema.rs` / `migrate.rs` / `error.rs`。
- 未触碰 T5 的 `commands.rs`（`import_file`/`get_sheet_data` 逻辑无需改，它们调用的 `count_rows`/`query_cells` 语义已正确）。
- 未触碰 T6/T7/T8 任何产物。
- 未触碰前端。
- 未触碰 docs（docs 由 T9 收口）。

## verification_run
1. `cargo check --workspace` — PASS（5 pre-existing warnings，无 error）
2. `cargo test --workspace` — PASS
   - db: 9 passed（含新增 `write_and_query_cells_paginated_multi_column`，原 8 个全绿无回归）
   - datasource: 3 passed
   - 其它: 0
3. `git diff -- src-tauri/src/db/mod.rs` — 仅改 `count_rows` SQL + `query_cells` SQL + 注释 + 新增 1 测试，无其它改动

## scope_deviation
- 无。仅修 T4 在 `db/mod.rs` 的两处 SQL + 注释 + 1 个新测试，属 T4 原始范围（持久层查询语义）的最小必要修复，未扩展到任何其它文件。

## impact_on_T5
- T5 的 `get_sheet_data`（commands.rs:220）现在能拿到正确的 `total`（行数）与每页 `page_size` 行数据。
- T5 的 `import_file`（commands.rs:146）写入逻辑无需改。
- T5 的 AC3/AC5/AC10（分页正确性、1000 行 CSV 分页）现在在静态层面成立（GUI 实际核验仍属主会话 Phase 7 E4/E8）。

## reported_status
verified_complete（建议）——主会话直接实施 + 独立验证全过；最终完成由主会话判定。
