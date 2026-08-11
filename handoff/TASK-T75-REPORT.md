# TASK-T75-REPORT

```yaml
implemented_changes:
  - 新建 crates/core/src/datasource/db.rs
    - DbReader { path: String } + new(path) + impl Reader::read()
    - 用 rusqlite::Connection::open(path) 打开外部 .db/.sqlite/.sqlite3 二进制 SQLite 文件
      （与 SqlReader 的 Connection::open_in_memory 执行 .sql 脚本并存但职责不同）
    - 查询 sqlite_master 获取用户表（type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name），
      跳过 sqlite_sequence 等内部表
    - 多表联合：union_headers 首列固定 __table，其余按各表列名首次出现顺序扩展；
      每行 fields 含 __table=来源表 + 各列对齐 union_headers（缺列补空）
    - quote_identifier 双引号转义表名（防御性，表名来源为受信 sqlite_master）
    - SELECT * 全参数绑定 query_map，BLOB 用 from_utf8_lossy
    - 路径不存在（先校验 Path::exists 避免 SQLITE_OPEN_CREATE 静默建库）
      或非 SQLite 文件返回 CoreError::DataSource
    - 空库（无用户表）返回 Dataset { headers: [], rows: [] }
    - 7 个测试：single_table / multi_table_union / skips_internal_tables
      （AUTOINCREMENT 触发 sqlite_sequence）/ empty_db / nonexistent_path
      / non_sqlite_file / quote_identifier 转义
  - crates/core/src/datasource/mod.rs
    - 追加 mod db; pub use db::DbReader;
    - detect_format 追加 3 个 match arm（db/sqlite/sqlite3 → DbReader）
    - 模块文档注释追加 DB 格式说明
    - detect_format_routes_by_extension 测试追加 3 个断言（db/sqlite/sqlite3）
  - docs/02-技术设计文档.md
    - datasource 目录树追加 db.rs
    - core::datasource 模块说明追加 T75 DbReader
    - sessions.source_type 枚举追加 db/sqlite/sqlite3
    - 新增 §4.11 外部 SQLite 数据库文件导入（v1.1.4 / T75）章节

verification_run:
  - cargo fmt --all --check
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all

verification_results:
  - cargo fmt --all --check → exit 0（通过）
  - cargo clippy --all-targets --all-features -- -D warnings → exit 0（通过，
    修复了 2 处 clippy::err_expect 提示，改用 expect_err）
  - cargo test --all → exit 0
    - ruT0-data-kit-core: 165 passed; 0 failed; 3 ignored
    - ruT0-data-kit (src-tauri): 137 passed; 0 failed; 0 ignored
    - doc-tests: 13 passed; 0 failed
    - 新增 7 个 db.rs 测试全绿，既有测试未破坏

docs_updated:
  - docs/02-技术设计文档.md（datasource 目录树 + 模块说明 + source_type 枚举 + §4.11 新章节）

commit_summary:
  - 797088a feat(datasource): T75 DbReader 外部 SQLite .db 文件解析 + detect_format 分发
  - 1b613a5 docs: T75 同步 DbReader 外部 SQLite .db 导入设计说明

reported_status:
  - implemented_and_verified

scope_deviation:
  - 无越界改动。严格遵守 in_scope（仅 db.rs 新建 + mod.rs + mod.rs tests）；
    未改 src-tauri / 前端 / SqlReader / Cargo.toml / DB schema。
    附带更新 docs/02-技术设计文档.md（spec Step 5 要求同步真实行为变化）。
    注意：工作区有 T73 的未提交改动（src-tauri/Cargo.toml + columns.rs 等），
    本任务未 stage 也未触碰这些文件；首次 cargo clippy --all 因 T73 的
    md5 crate 命名问题报错，但 cargo build 后再跑 clippy 全绿——该报错
    属于 T73 范围（src-tauri md-5 依赖），非本任务引入，已在 scope_deviation
    注明供主会话知悉。
```

## 备注

- `reported_status: implemented_and_verified`：本任务要求的三条 verification_commands 全部 exit 0，新增测试全绿，既有 302 条测试（core 158 + src-tauri 137 + doc 13，含 ignored）未破坏。最终 `verified_complete` 判定权在主会话。
- 工作区存在 T73（hash_column）的未提交改动，与本任务隔离；本任务两次 commit 只含 db.rs / mod.rs / docs 三文件。
- 关键文件路径：
  - `/Users/joker/code/RuT0DataKit/crates/core/src/datasource/db.rs`
  - `/Users/joker/code/RuT0DataKit/crates/core/src/datasource/mod.rs`
  - `/Users/joker/code/RuT0DataKit/docs/02-技术设计文档.md`
