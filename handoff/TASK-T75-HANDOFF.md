# TASK-T75-HANDOFF

```yaml
task_id: T75
goal: |
  在 crates/core 新增 DbReader 数据源，实现 Reader trait，支持打开外部
  .db / .sqlite / .sqlite3 二进制 SQLite 数据库文件并读取全部用户表数据
  （跳过 sqlite_% 内部表）。多表时联合输出，每行附 __table 列标识来源表。
  在 detect_format 工厂追加 3 个扩展名分发。无新依赖（rusqlite 已是 crates/core 依赖）。

in_scope:
  - crates/core/src/datasource/db.rs（新建 DbReader）
  - crates/core/src/datasource/mod.rs（mod db + pub use + detect_format 追加 3 个 match arm）
  - crates/core/src/datasource/mod.rs tests（detect_format_routes_by_extension 追加 db/sqlite/sqlite3 断言）

out_of_scope:
  - 不改 src-tauri（DbReader 通过 detect_format 被 import_file 调用，无需改 commands/data.rs）
  - 不改前端（import_file 既有流程自动支持新格式，文件选择器 filter 在前端组件，不在本任务范围）
  - 不改 SqlReader（.sql 脚本读取器保持不变，与 .db 二进制读取器并存）
  - 不改 DB schema（SCHEMA_VERSION 不变）
  - 不改 Cargo.toml（rusqlite 已有，无新依赖）
  - 不创建 tag / 不合并 main

acceptance_criteria:
  - detect_format("/tmp/test.db") 返回 Ok(Box<DbReader>)（不返回 NotImplemented）
  - detect_format("/tmp/test.sqlite") 返回 Ok(Box<DbReader>)
  - detect_format("/tmp/test.sqlite3") 返回 Ok(Box<DbReader>)
  - DbReader 打开含 1 个用户表 users(id,name) + 2 行数据的 .db 文件，read() 返回 Dataset { headers: ["__table","id","name"], rows: 2 }
  - DbReader 打开含 2 个用户表 users + orders 的 .db 文件，read() 返回 Dataset rows 含 __table 列区分来源
  - DbReader 跳过 sqlite_sequence / sqlite_% 内部表（不输出）
  - DbReader 对空 .db 文件（0 用户表）返回 Dataset { headers: [], rows: [] }
  - DbReader 对不存在的路径返回 CoreError::DataSource
  - DbReader 对非 SQLite 文件（如 txt 改名 .db）返回 CoreError::DataSource
  - cargo fmt --all --check exit 0
  - cargo clippy --all-targets --all-features -- -D warnings exit 0
  - cargo test --all 全绿（新增测试不破坏既有 302 passed）

verification_commands:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all

files_likely_to_change:
  - crates/core/src/datasource/db.rs（新建）
  - crates/core/src/datasource/mod.rs

risks:
  - 表名不能参数绑定（SQLite 限制），必须用标识符转义；表名来源是 sqlite_master.name（受信系统表），但仍需 quote_identifier 防御
  - 多表联合 headers 对齐：不同表列名不同，按首次出现顺序扩展 union_headers，缺列补空（与 SqlReader 既有模式一致）
  - BLOB 列用 String::from_utf8_lossy 处理（与 SqlReader 一致）
  - rusqlite Connection::open(path) 对非 SQLite 文件会返回 Err，需 map_err 为 CoreError::DataSource
  - tempfile 测试需用 Connection::open 创建临时 .db 文件

depends_on: []
status: planned
```

## 实现指引

### DbReader 结构

```rust
//! SQLite 二进制数据库文件读取器。
//!
//! 用 `rusqlite::Connection::open(path)` 直接打开 .db / .sqlite / .sqlite3 文件，
//! 查询 `sqlite_master` 获取全部用户表（type='table' AND name NOT LIKE 'sqlite_%'），
//! 逐表 SELECT * 收集数据，多表联合输出，每行附 `__table` 列标识来源。

use rusqlite::types::ValueRef;
use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::{Dataset, Reader};

/// SQLite 二进制数据库文件读取器。
pub struct DbReader {
    path: String,
}

impl DbReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

/// 转义 SQLite 标识符（双引号包裹，内部双引号翻倍）。
/// 表名来源是 sqlite_master.name（受信系统表），但仍做防御性转义。
fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
```

### Reader::read 实现

```rust
fn read(&self) -> CoreResult<Dataset> {
    let conn = Connection::open(&self.path)
        .map_err(|e| CoreError::DataSource(format!("db open: {e}")))?;

    // 查询全部用户表（跳过 sqlite_% 内部表）。
    let table_names: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .map_err(|e| CoreError::DataSource(format!("db list tables: {e}")))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| CoreError::DataSource(format!("db query tables: {e}")))?;
        rows.filter_map(Result::ok).collect()
    };

    if table_names.is_empty() {
        return Ok(Dataset { headers: Vec::new(), rows: Vec::new() });
    }

    let mut union_headers: Vec<String> = vec!["__table".to_string()];
    let mut all_rows: Vec<(String, Vec<String>, Vec<String>)> = Vec::new(); // (table, cells, local_headers)

    for table in &table_names {
        let sql = format!("SELECT * FROM {}", quote_identifier(table));
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| CoreError::DataSource(format!("db prepare `{sql}`: {e}")))?;
        let col_count = stmt.column_count();
        let local_headers: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("col").to_string())
            .collect();
        for h in &local_headers {
            if !union_headers.contains(h) {
                union_headers.push(h.clone());
            }
        }
        let rows_iter = stmt
            .query_map([], |row| {
                let mut cells: Vec<String> = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let cell = match row.get_ref(i)? {
                        ValueRef::Null => String::new(),
                        ValueRef::Integer(n) => n.to_string(),
                        ValueRef::Real(f) => format!("{}", f),
                        ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
                        ValueRef::Blob(b) => String::from_utf8_lossy(b).to_string(),
                    };
                    cells.push(cell);
                }
                Ok(cells)
            })
            .map_err(|e| CoreError::DataSource(format!("db query `{sql}`: {e}")))?;
        for row_result in rows_iter {
            let cells = row_result.map_err(|e| CoreError::DataSource(format!("db row: {e}")))?;
            all_rows.push((table.clone(), cells, local_headers.clone()));
        }
    }

    // 组装 rows：每行 fields 含 __table + 各列对齐 union_headers。
    let mut rows: Vec<Record> = Vec::with_capacity(all_rows.len());
    for (table, cells, local_headers) in all_rows {
        let mut fields = std::collections::HashMap::new();
        fields.insert("__table".to_string(), table);
        for (i, cell) in cells.iter().enumerate() {
            let key = local_headers.get(i).cloned().unwrap_or_else(|| format!("col{i}"));
            fields.insert(key, cell.clone());
        }
        for h in &union_headers {
            fields.entry(h.clone()).or_insert_with(String::new);
        }
        rows.push(Record { fields });
    }
    Ok(Dataset { headers: union_headers, rows })
}
```

### detect_format 追加（mod.rs:96-110）

```rust
Some("db") => Ok(Box::new(DbReader::new(path))),
Some("sqlite") => Ok(Box::new(DbReader::new(path))),
Some("sqlite3") => Ok(Box::new(DbReader::new(path))),
```

mod 声明 + pub use：
```rust
mod db;
pub use db::DbReader;
```

### 测试（db.rs 内或 mod.rs tests）

用 `tempfile` + `Connection::open` 创建临时 .db 文件：
- `db_reader_single_table`：建 1 表 users(id INTEGER, name TEXT) + INSERT 2 行 → read() headers=["__table","id","name"] rows=2
- `db_reader_multi_table_union`：建 users + orders 两表 → read() rows 含 __table 列区分
- `db_reader_skips_internal_tables`：建 users + 触发 sqlite_sequence（AUTOINCREMENT）→ rows 只含 users 行
- `db_reader_empty_db`：空 .db（无用户表）→ headers=[] rows=[]
- `db_reader_nonexistent_path` → CoreError::DataSource
- `db_reader_non_sqlite_file`：tempfile 写 "not a db" 内容改后缀 .db → CoreError::DataSource

mod.rs `detect_format_routes_by_extension` 追加：
```rust
assert!(detect_format("/tmp/foo.db").is_ok());
assert!(detect_format("/tmp/foo.sqlite").is_ok());
assert!(detect_format("/tmp/foo.sqlite3").is_ok());
```

### 参考既有 SqlReader

`crates/core/src/datasource/sql.rs` — in-memory SQL 脚本读取器，union_headers 对齐 + 缺列补空逻辑一致。DbReader 是其「打开外部二进制 .db 文件」的对应物。

### 安全约束

- 表名 SQL 不能参数绑定 → 用 `quote_identifier`（双引号转义）
- 表名来源限定为 `sqlite_master` WHERE type='table' AND name NOT LIKE 'sqlite_%'（受信系统表，用户无法注入恶意表名）
- SELECT * FROM 后跟转义标识符，无拼接注入风险
