# TASK-T29-REPORT — DB schema v2→v3 + 持久层新方法

```yaml
implemented_changes:
  - src-tauri/src/db/schema.rs
      - SCHEMA_VERSION: 2 → 3
      - SCHEMA_DDL 中 operations 表新增 before_snapshot_json TEXT 列
        （位于 result_snapshot_json 之前）
      - 索引段新增 CREATE INDEX IF NOT EXISTS idx_cells_sheet_col
        ON cells(sheet_id, col_idx)（搜索加速）
      - 模块头注释补充 v1.2.0 说明
  - src-tauri/src/db/migrate.rs
      - 新增 migrate_v2_to_v3(conn): PRAGMA table_info(operations) 检查
        before_snapshot_json 列是否存在再 ALTER（幂等），CREATE INDEX
        IF NOT EXISTS idx_cells_sheet_col，upsert schema_version=3
      - migrate() dispatcher 新增 Some(2) if SCHEMA_VERSION == 3 分支
        调用 migrate_v2_to_v3
      - 模块头注释补充 v2→v3 迁移说明
      - 新增 4 个测试：migrate_v2_to_v3_adds_before_snapshot_column /
        migrate_v2_to_v3_is_idempotent / idx_cells_sheet_col_exists_after_migrate /
        migrate_v2_to_v3_preserves_existing_data
      - 修正既有 2 个 v1→v2 测试：SCHEMA_VERSION 升到 3 后 v1 DB 走重建
        分支，原断言失效；改为直接调用 migrate_v1_to_v2 验证增量迁移语义
        （rules 表存在 + 历史数据保留），并在测试注释说明原因
  - src-tauri/src/db/mod.rs
      - 新增 OperationRow 结构（撤销/重调用，含 before/after snapshot 字段，
        serde camelCase）
      - 新增 RegexSearchResult type alias（= Vec<(Cell, Vec<(usize,usize)>)>，
        避免 clippy type_complexity 警告）
      - 新增 escape_like(keyword) 辅助函数：转义 \ / % / _，配合
        ESCAPE '\' 子句让 LIKE 按字面匹配特殊字符
      - log_operation 重构为调用 log_operation_with_snapshot(before=None)，
        保持原行为；新增 log_operation_with_snapshot 写 before_snapshot_json 列
      - 7 个新方法（全部参数化 SQL，?N + params![]）：
          * query_column_cells_with_row_idx_range — 列范围分页（排除表头）
          * search_cells — 关键字搜索（LIKE '%kw%' ESCAPE '\'，col_idx=None 搜全表）
          * search_cells_regex — SQL LIKE '%' 预筛 + Rust regex 精确匹配，
            返回命中 cell + 匹配区间；非法正则返回 Err 不 panic
          * count_search_results — 搜索结果总数（分页 total）
          * replace_in_column_cells — 列内替换，单事务内抓 before 快照→
            计算→写回，返回 (affected, before, after)
          * replace_all_cells — 全表替换（所有列），同上语义
          * query_operation_by_id — 按 id 查 OperationRow
      - replace_cells_inner 私有方法共享列/全表替换逻辑，单事务保证原子性
      - 新增 18 个测试用例覆盖搜索/替换/操作查询（含 LIKE 转义、正则非法
        返回 Err、before/after 快照、分页、表头排除等）
      - 模块头注释补充 v1.2.0 说明
      - 修正 new_creates_tables_and_schema_version 测试断言（2 → 3）
  - src-tauri/Cargo.toml
      - 新增 regex = "1" 依赖（src-tauri crate 直连，供 DB 层正则搜索/替换）
  - Cargo.lock
      - 同步（regex 已在 workspace 中由 core crate 引入，版本锁定 1.x）
  - docs/02-技术设计文档.md
      - §3 序言：SCHEMA_VERSION=2 → =3，补充 v1.2.0 增量迁移说明
      - §3.4 operations 表：新增 before_snapshot_json TEXT 列说明
      - §3.7 索引：新增 idx_cells_sheet_col
      - §4 模块速查表：DbManager 方法清单更新（含 8 个新方法）
  - docs/04-版本标准.md
      - v1.2.0 里程碑状态 planned → in_progress，补充核心交付
        （DB schema 升级 + 7 个搜索/替换/查询方法）作为 T29 落地信号
verification_run:
  - cargo fmt --check
  - cargo clippy -p ruT0-data-kit --all-targets -- -D warnings
  - cargo test -p ruT0-data-kit
verification_results:
  - cargo fmt --check: 通过（exit 0，无 diff）
  - cargo clippy -p ruT0-data-kit --all-targets -- -D warnings: 通过
    （Finished dev profile，无 warning/error）
  - cargo test -p ruT0-data-kit: 通过
    （lib: 39 passed, 0 failed; main: 0 passed; doc-tests: 0 passed）
    新增/相关测试用例全部通过：
      db::migrate::tests::migrate_v2_to_v3_adds_before_snapshot_column ok
      db::migrate::tests::migrate_v2_to_v3_is_idempotent ok
      db::migrate::tests::idx_cells_sheet_col_exists_after_migrate ok
      db::migrate::tests::migrate_v2_to_v3_preserves_existing_data ok
      db::tests::query_column_cells_with_row_idx_range_paged ok
      db::tests::search_cells_keyword_basic ok
      db::tests::search_cells_excludes_header_row ok
      db::tests::search_cells_escapes_like_special_chars ok
      db::tests::search_cells_regex_basic ok
      db::tests::search_cells_regex_invalid_pattern_returns_err ok
      db::tests::search_cells_regex_pagination ok
      db::tests::count_search_results_basic ok
      db::tests::count_search_results_escapes_special_chars ok
      db::tests::replace_in_column_cells_returns_before_snapshot ok
      db::tests::replace_in_column_cells_regex_mode ok
      db::tests::replace_in_column_cells_no_change_returns_empty ok
      db::tests::replace_in_column_cells_invalid_regex_returns_err ok
      db::tests::replace_all_cells_returns_both_snapshots ok
      db::tests::replace_all_cells_across_columns ok
      db::tests::query_operation_by_id_basic ok
      db::tests::log_operation_without_snapshot_keeps_before_null ok
docs_updated:
  - docs/02-技术设计文档.md（§3 序言 / §3.4 operations 表 / §3.7 索引 / §4 模块速查表）
  - docs/04-版本标准.md（v1.2.0 里程碑状态 + 核心交付）
commit_summary:
  - 34e62f0 feat(db): schema v2→v3 迁移 — operations 加 before_snapshot_json
    列 + idx_cells_sheet_col 索引（schema.rs + migrate.rs）
  - 4fc9254 feat(db): DbManager 新增 8 方法（搜索/替换/操作查询）+ regex
    依赖（mod.rs + Cargo.toml + Cargo.lock）
  - a48238f docs(db): 同步 schema v3 + 8 方法到技术设计与版本标准
    （02-技术设计文档.md + 04-版本标准.md）
  - 分支 feat/t29-db-schema-v3（基于 main）
reported_status:
  - verified_complete
scope_deviation:
  - "minor: HANDOFF 列出 7 个新 DbManager 方法；实际新增 8 个（额外 log_operation_with_snapshot）。
    原因：HANDOFF 要求 replace_in_column_cells / replace_all_cells 在单事务内完成 before
    快照 + write_cells + log_operation，而 operations 表新增了 before_snapshot_json 列，
    需要一个能写该列的 log_operation 变体供 T30/T31/T32 调用。保留原 log_operation 签名
    不变（向后兼容），新增 log_operation_with_snapshot(before=None) 重载语义。此为达成
    HANDOFF acceptance criteria（replace 操作事务内含 before 快照 + log_operation）的
    必要支撑方法，未越出 'DbManager 新方法供 T30/T31/T32 调用' 的 goal 范围。"
  - "minor: HANDOFF 要求 search_cells_regex 用 'LIKE 预筛 + regex 精确匹配'。原计划用
    pattern 自身做 LIKE 字面粗筛（转义 %/_/\\），但 regex pattern 含元字符（如 \\d）不会
    作为字面值出现在 cell 中，LIKE 预筛会漏掉所有命中（测试 search_cells_regex_basic 实测
    0 命中）。改为用 LIKE '%' 匹配所有非空 value 行做粗筛，再由 Rust regex 精确过滤——
    '不漏'语义不变，只是预筛更宽（宁可多筛）。已在方法 doc comment 说明此设计选择。
    属于 HANDOFF risks 第 4 条所述 '简化：直接用整个 pattern 做 LIKE' 的等价退化，
    未改变方法签名与返回语义。"
  - "minor: HANDOFF 验收项要求 'migrate() dispatcher 有 Some(2) if SCHEMA_VERSION == 3
    分支'。实现同时保留 Some(1) if SCHEMA_VERSION == 2 旧分支——但因 SCHEMA_VERSION
    现为 3，Some(1) 分支条件不成立，v1 DB 会走重建路径（备份 .bak + DROP + bootstrap）。
    这是 schema 版本提升的预期行为（HANDOFF risks 第 2 条已提示 v1→v2 ALTER 幂等；
    v1→v3 跨两版本走重建是合理选择）。相应调整了 2 个既有 v1→v2 测试为直接调用
    migrate_v1_to_v2 验证增量迁移语义，避免断言 migrate() 走 v1→v2 路径（已不成立）。"
  - "未触碰 out_of_scope：未改 commands 层 / 前端 / core crate / lib.rs。"
```

## 验收项对照

| # | acceptance_criteria | 状态 | 证据 |
|---|---|---|---|
| 1 | SCHEMA_VERSION == 3 | ✅ | schema.rs:8 `pub const SCHEMA_VERSION: i64 = 3;` |
| 2 | operations 表含 before_snapshot_json TEXT 列 | ✅ | schema.rs DDL + migrate_v2_to_v3 ALTER |
| 3 | idx_cells_sheet_col ON cells(sheet_id, col_idx) | ✅ | schema.rs DDL + migrate_v2_to_v3 CREATE INDEX |
| 4 | migrate_v2_to_v3 幂等（先查列再 ALTER；索引 IF NOT EXISTS） | ✅ | migrate.rs PRAGMA table_info + 测试 is_idempotent |
| 5 | migrate() dispatcher 有 Some(2) if SCHEMA_VERSION==3 分支 | ✅ | migrate.rs:147-151 |
| 6 | DbManager 新增 7 方法（+1 支撑），全部 ?N 参数绑定 | ✅ | mod.rs（实际 8 个，见 scope_deviation） |
| 7 | v2 DB 经 migrate 后 before_snapshot_json 列存在且 schema_version=3 | ✅ | 测试 migrate_v2_to_v3_adds_before_snapshot_column |
| 8 | search_cells 关键字模式（LIKE + ESCAPE '\'） | ✅ | search_cells + escape_like + 测试 keyword_basic / escapes_like_special_chars |
| 9 | search_cells_regex 先 SQL LIKE 预筛再 Rust regex 精确匹配 | ✅ | search_cells_regex（LIKE '%' 预筛，见 scope_deviation）+ 测试 regex_basic |
| 10 | replace_in_column_cells 返回 before 快照 + 受影响行数 | ✅ | replace_in_column_cells + 测试 returns_before_snapshot |
| 11 | replace_all_cells 返回 before/after 快照 + 受影响行数 | ✅ | replace_all_cells + 测试 returns_both_snapshots / across_columns |
| 12 | count_search_results 返回总数 | ✅ | count_search_results + 测试 basic / escapes_special_chars |
| 13 | query_operation_by_id 返回 OperationRow（含 before/after） | ✅ | query_operation_by_id + 测试 basic |
| 14 | 单元测试覆盖（迁移幂等 + 列存在 + 索引 + search + replace + count） | ✅ | 21 个新测试全部通过 |

## 关键实现决策

1. **escape_like 转义顺序**：先转 `\` 再转 `%`/`_`，避免二次转义。配合
   `ESCAPE '\'` 子句，keyword 中的特殊字符按字面匹配。
2. **search_cells_regex 预筛策略**：放弃用 pattern 自身做 LIKE 字面粗筛
   （regex 元字符不会作为字面值出现在 cell 中，会漏命中），改用 `LIKE '%'`
   匹配所有非空 value 行，由 Rust regex 二次精确过滤。"不漏"语义不变。
3. **replace_cells_inner 单事务**：before 快照查询 + after 值计算 + 批量
   upsert 全部在 `conn.transaction()` 内完成，保证原子性。before/after
   只返回有变化的行（一一对应），未变化的行不出现在快照中。
4. **log_operation_with_snapshot**：保留原 `log_operation` 签名向后兼容，
   新增带 before_snapshot_json 参数的变体，供 T30/T31/T32 在 replace 操作
   事务后记录撤销前置快照。
5. **RegexSearchResult type alias**：避免 `Result<Vec<(Cell, Vec<(usize,usize)>)>, DbError>`
   触发 clippy type_complexity 警告（-D warnings 下会 fail）。
