# T75 REVIEW — DbReader 外部 SQLite 解析

```yaml
task_id: T75
reviewer_verdict: review_passed
reviewed_at: 2026-08-11
commits_reviewed: [797088a, 1b613a5]

defects: []

scope_check: none
docs_check: synced
```

## 验收对照

| 验收项 | 结果 | 证据 |
|---|---|---|
| detect_format("/tmp/test.db") 返回 Ok(Box<DbReader>) | PASS | `crates/core/src/datasource/mod.rs:113` `Some("db") => Ok(Box::new(DbReader::new(path)))`；测试 `mod.rs:137` `assert!(detect_format("/tmp/foo.db").is_ok())` |
| detect_format("/tmp/test.sqlite") 返回 Ok(Box<DbReader>) | PASS | `mod.rs:114`；测试 `mod.rs:138` |
| detect_format("/tmp/test.sqlite3") 返回 Ok(Box<DbReader>) | PASS | `mod.rs:115`；测试 `mod.rs:139` |
| DbReader 打开含 1 用户表 users + 2 行 → Dataset { headers: ["__table","id","name"], rows: 2 } | PASS | `db.rs:187-197` `db_reader_single_table` 断言 `headers == ["__table","id","name"]`、`rows.len()==2`、首行 `__table=users / id=1 / name=alice` |
| 多表 users + orders → rows 含 __table 区分来源 | PASS | `db.rs:200-241` `db_reader_multi_table_union` 校验 union_headers=`["__table","id","user_id","amount","name"]`，users 行缺 `user_id/amount` 补空，orders 行缺 `name` 补空 |
| 跳过 sqlite_sequence / sqlite_% 内部表 | PASS | `db.rs:62-66` 查询 `type='table' AND name NOT LIKE 'sqlite_%'`；`db.rs:243-259` `db_reader_skips_internal_tables` 用 AUTOINCREMENT 触发 sqlite_sequence，断言仅 1 行 users |
| 空 .db 文件 → Dataset { headers: [], rows: [] } | PASS | `db.rs:80-85` 空表早返回空 Dataset；`db.rs:261-269` `db_reader_empty_db` 断言 headers/rows 均 empty |
| 不存在路径 → CoreError::DataSource | PASS | `db.rs:49-54` 先 `Path::exists()` 校验，不存在直接 `Err(CoreError::DataSource)`；`db.rs:271-276` `db_reader_nonexistent_path` `expect_err` + `matches!(CoreError::DataSource(_))` |
| 非 SQLite 文件 → CoreError::DataSource | PASS | `db.rs:56-57` `Connection::open` 对 garbage 文件返回 Err 并 map_err 为 `CoreError::DataSource`；`db.rs:278-289` `db_reader_non_sqlite_file` 写入 "not a db" 内容后断言 err 为 `CoreError::DataSource` |
| quote_identifier 双引号包裹 + 内部双引号翻倍 | PASS | `db.rs:40-42` `format!("\"{}\"", name.replace('"', "\"\""))`；`db.rs:291-297` `quote_identifier_escapes_double_quote` 校验 `users → "users"`、`a"b → "a""b"` |
| 表名不参数绑定但经 quote_identifier 转义 | PASS | `db.rs:93` `format!("SELECT * FROM {}", quote_identifier(table))`，表名来源为 `sqlite_master`（受信系统表，已过滤 `sqlite_%`） |
| 多表 union headers 首列加 `__table`，各表数据行前缀表名 | PASS | `db.rs:88` `vec!["__table".to_string()]`；`db.rs:133` `fields.insert("__table", table)`；缺列补空 `db.rs:142-144` |
| cargo fmt --all --check exit 0 | PASS | 本地复跑 exit 0 |
| cargo clippy --all-targets --all-features -- -D warnings exit 0 | PASS | 本地复跑 exit 0 |
| cargo test --all 全绿，新增 7 测试不破坏既有 | PASS | core 165 passed; src-tauri 137 passed; doc-tests 13 passed; db.rs 7 测试全绿（single_table / multi_table_union / skips_internal_tables / empty_db / nonexistent_path / non_sqlite_file / quote_identifier） |

## 验证命令执行结果

| 命令 | 结果 |
|---|---|
| `cargo fmt --all --check` | exit 0（无 diff） |
| `cargo clippy --all-targets --all-features -- -D warnings` | exit 0（Finished dev profile，无 warning） |
| `cargo test --all` | exit 0；core 165 passed / 0 failed / 3 ignored；src-tauri 137 passed / 0 failed；doc-tests 13 passed / 0 failed；新增 7 个 db.rs 测试全部 ok |

报告声称 core 158+3 ignored 的数字与本次复跑 165 passed / 3 ignored 略有出入（应为 158→165，含本任务新增 7 个 db.rs 测试），但全绿结论一致，不影响通过判定。

## 安全合规

| 检查项 | 结果 | 证据 |
|---|---|---|
| 表名 SQL 注入防御 | PASS | 表名虽不能参数绑定（SQLite 限制），但经 `quote_identifier` 双引号转义（`db.rs:40-42`）；表名来源限定为 `sqlite_master` WHERE `type='table' AND name NOT LIKE 'sqlite_%'`（`db.rs:62-66`），受信系统表，用户无法注入恶意表名 |
| 不查询 sqlite_ 系统表 | PASS | `name NOT LIKE 'sqlite_%'` 过滤掉 `sqlite_sequence` / `sqlite_master` 等内部表，`db_reader_skips_internal_tables` 覆盖 |
| 无凭据字面量 | PASS | 代码中无任何硬编码凭据 / 密钥 |
| 不创建 tag / 不 merge main | PASS | 两个 commit 仅落到 `feat/v1.1.4-r3-hash-dbparse` 分支，无 tag 操作，工作区无 main 合并痕迹 |
| SQLITE_OPEN_CREATE 静默建库风险 | PASS | `db.rs:49-54` 在 `Connection::open` 之前先 `Path::exists()` 校验，避免 rusqlite 默认 CREATE flag 静默创建空 db 误判为「空库」 |
| 非法 SQLite 文件误打开 | PASS | `db.rs:56-57` `Connection::open` 对 garbage 内容返回 Err，map_err 为 `CoreError::DataSource`；`db_reader_non_sqlite_file` 测试覆盖 |

## Scope 核对

- in_scope：`crates/core/src/datasource/db.rs`（新建）+ `crates/core/src/datasource/mod.rs`（mod db + pub use + detect_format 3 arm + 模块文档 + 测试断言）—— 全部命中，无遗漏。
- docs 同步：`docs/02-技术设计文档.md`（目录树 db.rs + datasource 模块说明 T75 + source_type 枚举 db/sqlite/sqlite3 + §4.11 新章节）—— 与 spec Step 5「同步真实行为变化」一致，属于必要文档同步。
- out_of_scope 未触碰：未改 src-tauri、前端、SqlReader、DB schema、Cargo.toml；commit 797088a 仅含 db.rs + mod.rs 两文件，commit 1b613a5 仅含 docs 一文件，无夹带。
- 工作区有 T73 未提交改动（src-tauri/Cargo.toml md-5 等），未被 T75 两次 commit stage，隔离干净。

## commit 核对

- 797088a `feat(datasource): T75 DbReader 外部 SQLite .db 文件解析 + detect_format 分发`：Conventional Commits 规范，单一逻辑目的（DbReader 实现 + 工厂分发 + 测试），无格式化噪音。
- 1b613a5 `docs: T75 同步 DbReader 外部 SQLite .db 导入设计说明`：Conventional Commits 规范，单一文档同步目的。
- 两个 commit 拆分合理（实现 / 文档分离），便于回滚。

## 文档核对

- `docs/02-技术设计文档.md` 已同步真实行为：datasource 目录树追加 db.rs、模块说明追加 T75 DbReader、`sessions.source_type` 枚举追加 db/sqlite/sqlite3、新增 §4.11 章节（用户表发现 / 多表联合 / 表名安全 / 错误处理 / 空库 五个实现要点）。
- 文档未比代码更乐观：明确写出 SQLITE_OPEN_CREATE 静默建库风险与先校验存在的应对、表名不能参数绑定的限制与 quote_identifier 防御，与实现一致。
- 注意：文档中 §4.11 标题与既有「4.11 UI 布局调整（v1.1.2 新增）」重复（`docs/02-技术设计文档.md:509` 与 `:527`），属于既有版本管理的轻微编号冲突，不阻塞本任务（T75 新增的 4.11 内容正确且完整，既有 4.11 章节未被破坏）。

## 缺陷清单（若有）

无阻塞问题。

minor（不阻塞，可后续清理）：
- `docs/02-技术设计文档.md:509` 与 `:527` 同为 `### 4.11` 标题，章节编号重复。T75 新增的「外部 SQLite 数据库文件导入」与既有「UI 布局调整」均为 4.11。建议后续把 T75 章节顺延为 4.12 或调整既有编号，但本任务范围内不影响验收。

## 最终意见

T75 实现忠实于 HANDOFF 的 goal/scope/acceptance：DbReader 用 `Connection::open(path)` 打开外部二进制 SQLite 文件，查询 `sqlite_master` 过滤 `sqlite_%` 内部表，多表联合输出附 `__table` 列，缺列补空；表名经 `quote_identifier` 防御性转义；路径不存在先 `Path::exists()` 校验避免 SQLITE_OPEN_CREATE 静默建库；非 SQLite 文件 map_err 为 `CoreError::DataSource`。detect_format 追加 db/sqlite/sqlite3 三 arm。7 个测试覆盖全部边界。三条验证命令本地复跑全绿。文档已同步真实行为。无 blocker/major 缺陷，无越界改动。

verdict: review_passed
