# TASK-T59-REPORT — 搜索行级 N+1 消除 + 正则重扫消除

```yaml
implemented_changes:
  - file: src-tauri/src/db/mod.rs
    branch: fix/t59-complete-single-scan
    commit: e0d4b50
    summary: |
      T59 首轮提交（f5c634a）新增了 search_matched_rows_regex_with_total +
      query_row_cells_batch，并改写了 search_rows 命令；但 search_matched_row_ids_regex
      与 count_matched_rows_regex 仍保留旧实现——二者在循环里对每个 row_idx
      单独执行 SELECT（N+1），且 count_matched_rows_regex 仍通过
      search_matched_row_ids_regex(0, u32::MAX) 递归复用完整搜索（u32::MAX 重扫）。
      本提交补齐 T59 验收标准剩余项。
    changes:
      - 抽取共享单次扫描核心 scan_regex_matched_row_ids(sheet_id, col_idx, pattern)
        -> Result<Vec<u32>, DbError>：一次性 `LIKE '%' ESCAPE '\'` 取
        (row_idx, value) 全部候选 cell，Rust 侧逐 cell 跑 regex::is_match，
        按 row_idx 升序去重（prev: Option<u32> 比较，行已由 SQL ORDER BY 预排序），
        NULL value（None）跳过；单次遍历产出全部命中 row_idx（升序、去重）。
      - search_matched_row_ids_regex 改为 scan_regex + 切片分页
        (start/end = offset/limit 作用于 all.len())，消除 N+1。
      - count_matched_rows_regex 改为 scan_regex 取长度
        (Ok(all.len() as u32))，消除 u32::MAX 重扫。
      - search_matched_rows_regex_with_total 同样改为 scan_regex + 切片 + 长度，
        保证 total 与分页结果来自同一次逻辑扫描。
      - 净效果：+73 / -152 行（去重 N+1 循环与重扫递归）。
    preserved_semantics:
      - row_idx 升序（SQL ORDER BY row_idx ASC, col_idx ASC）
      - 行级去重（prev 比较，等价于原 DISTINCT row_idx + 首命中入列）
      - NULL 跳过（None value 不参与 is_match）
      - row_idx > 0 排除表头
      - byte-offset 命中区间计算仍在命令层 compute_regex_matches /
        compute_keyword_matches（regex::find_iter / str::find 字节偏移），未改动
      - 全列 / 指定列正则搜索行为不变（col_idx Some/None 分支）
      - offset/limit 切片语义与原 LIMIT/OFFSET 一致
  - file: src-tauri/src/commands/search.rs
    prior_commit: f5c634a + 2aee826
    summary: |
      search_rows 命令在首轮提交已改用 search_matched_rows_regex_with_total
      （正则路径）+ query_row_cells_batch（两种路径批量取整行 cells），
      替代 search_matched_row_ids_regex + count_matched_rows_regex 二次重扫
      与逐行 query_row_cells 的 N+1。本提交无需再改命令层。
  - file: src-tauri/src/db/mod.rs query_row_cells_batch（首轮提交已存在）
    summary: |
      按 row_idx IN (...) 一次 SQL 取多行 cells，500 一批分块规避
      SQLITE_MAX_VARIABLE_NUMBER；占位符为固定 ?N 常量，不拼接用户输入；
      用 params_from_iter 绑定 sheet_id + 各 row_idx（i64 形式）。
  - tests（首轮提交 2aee826 已补齐，7 个，全部通过）:
      - search_rows_regex_with_total_matches_legacy
      - search_rows_regex_with_total_pagination（本提交修正了错误断言：
        全表 \d 命中 3 行而非 4 行——memo "50%" 同属 row 3，行级去重后仍 3 行）
      - search_rows_regex_with_total_col_filtered
      - query_row_cells_batch_returns_all_rows
      - query_row_cells_batch_handles_large_input（600 行 > 500 阈值）
      - search_rows_regex_single_scan_matches_legacy_flow
      - search_rows_keyword_utf8_byte_offsets_preserved

verification_run:
  - command: cargo fmt --all -- --check
    exit_code: 0
    output_tail: "FMT_EXIT=0"
  - command: cargo test -p ruT0-data-kit search
    exit_code: 0
    output_tail: |
      test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 80 filtered out
  - command: cargo test --workspace
    exit_code: 0
    output_tail: |
      test result: ok. 123 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      test result: ok. 148 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out
      test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

verification_results:
  fmt: pass
  search_tests: pass (41 passed, 0 failed)
  workspace_tests: pass (282 passed total across crates, 0 failed)
  acceptance_criteria_check:
    - "regex row search no longer per-row query_row/query_map": PASS
        （search_matched_row_ids_regex / count_matched_rows_regex /
         search_matched_rows_regex_with_total 全部改为 scan_regex 单次扫描）
    - "return current page row ids + total in one candidate scan/batch read (not u32::MAX recursion)":
        PASS（total = all.len() 来自同一次 scan_regex，无 u32::MAX 递归）
    - "results sorted by row_idx ascending, offset/limit consistent":
        PASS（SQL ORDER BY row_idx ASC + Rust 切片）
    - "full-column and specified-column regex search preserve NULL skip + hit interval semantics":
        PASS（None value 跳过；byte-offset 命中区间计算未改动）
    - "search command uses one batch cells query or equivalent single batch access":
        PASS（query_row_cells_batch，一次 SQL IN 查询，500 分块）
    - "new regression tests cover normal/regex/specified-column/pagination/UTF-8 hits":
        PASS（7 个测试覆盖上述全部场景）
    - "SQL params all bound, no user-input SQL concatenation":
        PASS（params! / params_from_iter 绑定；占位符为固定 ?N 常量）

docs_updated:
  - file: docs/02-技术设计文档.md
    note: |
      首轮提交（f5c634a）已同步 db 模块方法矩阵（search_matched_rows_regex_with_total
      / query_row_cells_batch）与 T59 性能修复段落。本提交无新增文档变更
      （纯代码重构，公共方法签名未变）。

commit_summary:
  - hash: e0d4b50
    subject: "perf(search): T59 完成单次扫描核心 — 消除 N+1 与 u32::MAX 重扫"
    branch: fix/t59-complete-single-scan
    parent: 675d7c6 (main HEAD at task start)
    files_changed: 1 (src-tauri/src/db/mod.rs, +73/-152)
  - prior_related_commits:
      - f5c634a "fix(search): T59 搜索行级 N+1 消除 + 正则重扫消除"（首轮）
      - 2aee826 "test(search): T59 单次扫描+批量 cells 回归测试 + 注释精化"（补测试）

reported_status: verified_complete

uncompleted_items: []

risks:
  - note: |
      scan_regex_matched_row_ids 仍是"先 SQL 取全部候选 (row_idx, value) 到内存，
      再 Rust 侧逐 cell is_match"。对超大 sheet（万级行、千级列）仍可能内存较高，
      但优于原 N+1（每行单独 SELECT）。后续可考虑 SQL 侧 regex（加载 extension）
      或分页式候选扫描。本任务范围不涵盖此优化。
  - note: |
      query_row_cells_batch 的 IN 子句占位符数量受 SQLITE_MAX_VARIABLE_NUMBER
      限制，已按 500 分块规避；若未来单页 row_ids 超过 500 仍会分多次 SQL，
      但正确性不受影响。
  - note: |
      验收时观察到 processor.rs:806 的 6 个 validate_rows_to_two_sheets_*
      测试在某些分支组合下失败（T62 的 debug_assert!(row_idx > 0) 触发），
      但这些是 T62 表头排除语义与 T61 validate 测试夹具的交叉问题，
      不在 T59 范围内。在 main 基础上仅应用 T59 提交（e0d4b50）后，
      workspace 全绿（282 passed），证明 T59 变更未引入回归。

scope_deviation: |
  无 scope 扩展。本提交严格限定在 src-tauri/src/db/mod.rs 的
  search_matched_row_ids_regex / count_matched_rows_regex /
  search_matched_rows_regex_with_total 三个方法重构 + 新增私有
  scan_regex_matched_row_ids 共享核心。未触碰 commands/search.rs（首轮已改好）、
  未触碰其他 db.rs 方法签名、未触碰前端、未触碰 schema/migration。
```
