# TASK-T4-HOTFIX-REVIEW — SQLite 持久层分页语义修复验收

verdict:
  - review_passed

scope_check:
  - 无越界。本次 hotfix 仅修改 `src-tauri/src/db/mod.rs`（diff stat: 52+/6-）。
  - `src-tauri/src/db/schema.rs` / `migrate.rs` / `error.rs`：未改动（git diff --stat 为空）。
  - `docs/`：未改动（属 T9 收口范围，hotfix 不应触碰，符合预期）。
  - `handoff/TASK-T6-HANDOFF.md` / `TASK-T7-HANDOFF.md` / `TASK-T8-HANDOFF.md`：未改动。
  - 工作树中其它改动（`commands.rs` / `frontend/*` / `crates/core/*` / `lib.rs` / `Cargo.lock`）均为 T5 coder 已有产物，非本次 hotfix 产物；本次 hotfix 只新增 `db/mod.rs` 的 SQL + 注释 + 1 测试，符合主会话判定的「T4 DB 层最小必要修复」范围。
  - `handoff/TASK-T4-HOTFIX-REPORT.md` 与 `handoff/TASK-T5-REPORT.md` 均存在于工作树（未删除）。

docs_check:
  - docs/ 无需同步：本次只修正 DB 查询语义 bug，不改变对外契约或用法（`count_rows`/`query_cells` 签名、返回类型均不变）。docs 永久文档由 T9 统一收口，hotfix 阶段不动文档符合规范。

defects:
  - 无阻塞问题。
  - 非阻塞（minor，无需 coder 行动，仅记录）：
    - severity: minor
    - file: src-tauri/src/db/mod.rs:380
    - issue: 新测试 `write_and_query_cells_paginated_multi_column` page1 断言用 `c.row_idx <= 1` 而非精确集合 `{0,1}`，稍弱于理想断言。
    - impact: 无实际影响——组合 `len()==6` + 每行固定 3 列 + 构造时 row 0..4 全部存在，等价于 `row_idx ∈ {0,1}`；旧实现下该断言仍会失败（旧 page1 返回 2 cells 而非 6），确实暴露原 bug。
    - fix: 可选加强为 `assert!(p1.iter().all(|c| c.row_idx == 0 || c.row_idx == 1));`，不强制。

review_evidence:
  - `count_rows`（src-tauri/src/db/mod.rs:152-160）：SQL `SELECT COUNT(DISTINCT row_idx) FROM cells WHERE sheet_id = ?1`，返回真实行数（含表头 row_idx=0）。单列 Sheet 下 `COUNT(DISTINCT row_idx) == COUNT(*)`（每行恰 1 cell），向后兼容成立。
  - `query_cells`（src-tauri/src/db/mod.rs:94-126）：SQL 改为行级分页——子查询 `SELECT DISTINCT row_idx FROM cells WHERE sheet_id=?1 ORDER BY row_idx ASC LIMIT ?2 OFFSET ?3` 取本页 row_idx 集合，外层 `WHERE sheet_id=?1 AND row_idx IN (...) ORDER BY row_idx ASC, col_idx ASC` 取这些行全部列。参数绑定 `params![sheet_id, limit, offset]` → `?1`=sheet_id, `?2`=limit, `?3`=offset，正确。单列 Sheet 下子查询 row_idx 集合即原 cell row 集合，行为不变，向后兼容成立。空 sheet → 子查询空 → `row_idx IN (空)` → 0 行，边界正确。
  - 新测试 `write_and_query_cells_paginated_multi_column`（src-tauri/src/db/mod.rs:356-392）：构造 3 列 × 5 行 = 15 cells；断言 count==5（旧 `COUNT(*)` 返回 15，会失败→暴露 bug）；page1==6 cells row≤1、page2==6 cells row∈[2,3]、page3==3 cells row==4、page4==0。旧 `query_cells` page1 返回 2 cells（LIMIT 2 作用在 cell 上）而非 6，会失败→暴露 bug。断言集合正确。

verification_run:
  - `cargo check --workspace`：PASS（5 个 pre-existing warnings，无 error，均为 dead_code/非 snake_case，与本次改动无关）。
  - `cargo test --workspace`：PASS。db 模块 9 passed（含新增 `write_and_query_cells_paginated_multi_column`，原 8 个全绿，`write_and_query_cells_paginated` 单列场景 + `write_cells_upsert_on_conflict` 均通过，无回归）；datasource 3 passed；其它 0。
  - `git diff --stat -- src-tauri/src/db/mod.rs`：仅 1 文件 52+/6-。
  - `git diff --stat -- docs/ handoff/TASK-T6/T7/T8-HANDOFF.md src-tauri/src/db/{schema,migrate,error}.rs`：空，确认未越界。

impact_on_T5:
  - `get_sheet_data`（src-tauri/src/commands.rs:220-229）调用 `db.count_rows`（total）与 `db.query_cells`（每页 cells）。修复后 `total` 为真实行数，每页返回 `page_size` 行的全部列，T5 AC3/AC5/AC10 在静态层面成立（GUI 实际核验仍属主会话 Phase 7 E4/E8）。
  - `import_file`（commands.rs 写入路径）不调用这两个方法，不受影响。

conclusion:
  - goal 满足：`count_rows` 返回行数、`query_cells` 行级分页，均修复。
  - acceptance_criteria（来自 T5 reviewer 转交的 T4 缺陷）满足。
  - 无关键缺陷，无非阻塞缺陷需 coder 行动。
  - 无越界改动。
  - 验证充分（check + test 全过）。
  - 文档无需同步（契约未变，docs 归 T9）。
  - 建议主会话判定 T4-hotfix verified_complete，并据此重开/推进 T5 的 review 通过判定。
