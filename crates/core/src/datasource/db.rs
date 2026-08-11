//! SQLite 二进制数据库文件读取器。
//!
//! 用 `rusqlite::Connection::open(path)` 直接打开 .db / .sqlite / .sqlite3 文件，
//! 查询 `sqlite_master` 获取全部用户表（type='table' AND name NOT LIKE 'sqlite_%'），
//! 逐表 SELECT * 收集数据，多表联合输出，每行附 `__table` 列标识来源。
//!
//! 与 [`super::SqlReader`] 的区别：SqlReader 用 `Connection::open_in_memory`
//! 执行 .sql 文本脚本；DbReader 用 `Connection::open(path)` 打开外部二进制
//! .db 文件。二者共享 union_headers 对齐 + 缺列补空模式。

use std::collections::HashMap;
use std::path::Path;

use rusqlite::types::ValueRef;
use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::{Dataset, Reader};

/// SQLite 二进制数据库文件读取器。
///
/// 打开外部 .db / .sqlite / .sqlite3 文件，读取全部用户表数据。
/// 跳过 `sqlite_%` 内部表；多表联合输出，首列固定 `__table` 标识来源表。
pub struct DbReader {
    path: String,
}

impl DbReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

/// 转义 SQLite 标识符（双引号包裹，内部双引号翻倍）。
///
/// 表名来源是 `sqlite_master.name`（受信系统表，已通过 `type='table' AND
/// name NOT LIKE 'sqlite_%'` 过滤），但仍做防御性转义，避免异常表名破坏 SQL。
fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

impl Reader for DbReader {
    /// 单次解析：打开 .db 文件，遍历全部用户表，收集 union headers + 所有数据行。
    fn read(&self) -> CoreResult<Dataset> {
        // 打开前校验文件存在：Connection::open 默认带 SQLITE_OPEN_CREATE 会
        // 静默创建空库，与「打开已有外部 .db」的语义不符；不存在则报错。
        if !Path::new(&self.path).exists() {
            return Err(CoreError::DataSource(format!(
                "db file not found: {}",
                self.path
            )));
        }

        let conn = Connection::open(&self.path)
            .map_err(|e| CoreError::DataSource(format!("db open: {e}")))?;

        // 查询全部用户表（跳过 sqlite_% 内部表），按名排序保证输出稳定。
        let table_names: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT name FROM sqlite_master \
                     WHERE type='table' AND name NOT LIKE 'sqlite_%' \
                     ORDER BY name",
                )
                .map_err(|e| CoreError::DataSource(format!("db list tables: {e}")))?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| CoreError::DataSource(format!("db query tables: {e}")))?;
            let mut names: Vec<String> = Vec::new();
            for row_result in rows {
                let name =
                    row_result.map_err(|e| CoreError::DataSource(format!("db table row: {e}")))?;
                names.push(name);
            }
            names
        };

        if table_names.is_empty() {
            return Ok(Dataset {
                headers: Vec::new(),
                rows: Vec::new(),
            });
        }

        // union_headers 首列固定 __table，其余按各表列名首次出现顺序扩展。
        let mut union_headers: Vec<String> = vec!["__table".to_string()];
        // (table, cells, local_headers)：暂存每行原始数据 + 所属表的列名。
        let mut all_rows: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();

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
                            ValueRef::Real(f) => f.to_string(),
                            ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
                            ValueRef::Blob(b) => String::from_utf8_lossy(b).to_string(),
                        };
                        cells.push(cell);
                    }
                    Ok(cells)
                })
                .map_err(|e| CoreError::DataSource(format!("db query `{sql}`: {e}")))?;
            for row_result in rows_iter {
                let cells =
                    row_result.map_err(|e| CoreError::DataSource(format!("db row: {e}")))?;
                all_rows.push((table.clone(), cells, local_headers.clone()));
            }
        }

        // 组装 rows：每行 fields 含 __table + 各列对齐 union_headers（缺列补空）。
        let mut rows: Vec<Record> = Vec::with_capacity(all_rows.len());
        for (table, cells, local_headers) in all_rows {
            let mut fields: HashMap<String, String> = HashMap::new();
            fields.insert("__table".to_string(), table);
            for (i, cell) in cells.iter().enumerate() {
                let key = local_headers
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col{i}"));
                fields.insert(key, cell.clone());
            }
            // 缺列补空（其它表的列在此行不存在）。
            for h in &union_headers {
                fields.entry(h.clone()).or_default();
            }
            rows.push(Record { fields });
        }
        Ok(Dataset {
            headers: union_headers,
            rows,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;
    use rusqlite::Connection;
    use std::io::Write;

    /// 创建一个临时 .db 文件，用 `setup` 闭包建表插数据，返回路径字符串。
    /// NamedTempFile 由调用方持有以保活。
    fn make_db(setup: impl FnOnce(&Connection)) -> tempfile::NamedTempFile {
        let tmp = tempfile::Builder::new()
            .suffix(".db")
            .tempfile()
            .expect("create tmp db");
        let conn = Connection::open(tmp.path()).expect("open tmp db");
        setup(&conn);
        // 显式 close：确保 SQLite 把 schema 写入文件。
        conn.close().expect("close tmp db");
        tmp
    }

    #[test]
    fn db_reader_single_table() {
        let tmp = make_db(|conn| {
            conn.execute_batch(
                "CREATE TABLE users (id INTEGER, name TEXT); \
                 INSERT INTO users (id, name) VALUES (1, 'alice'); \
                 INSERT INTO users (id, name) VALUES (2, 'bob');",
            )
            .expect("setup users");
        });
        let reader = DbReader::new(tmp.path().to_str().unwrap());
        let ds = reader.read().expect("read single table");
        assert_eq!(ds.headers, vec!["__table", "id", "name"]);
        assert_eq!(ds.rows.len(), 2);
        // 首行 __table=users, id=1, name=alice。
        let r0 = &ds.rows[0].fields;
        assert_eq!(r0.get("__table").unwrap(), "users");
        assert_eq!(r0.get("id").unwrap(), "1");
        assert_eq!(r0.get("name").unwrap(), "alice");
        let r1 = &ds.rows[1].fields;
        assert_eq!(r1.get("__table").unwrap(), "users");
        assert_eq!(r1.get("id").unwrap(), "2");
        assert_eq!(r1.get("name").unwrap(), "bob");
    }

    #[test]
    fn db_reader_multi_table_union() {
        let tmp = make_db(|conn| {
            conn.execute_batch(
                "CREATE TABLE users (id INTEGER, name TEXT); \
                 INSERT INTO users VALUES (1, 'alice'); \
                 CREATE TABLE orders (id INTEGER, user_id INTEGER, amount REAL); \
                 INSERT INTO orders VALUES (100, 1, 9.9);",
            )
            .expect("setup users+orders");
        });
        let reader = DbReader::new(tmp.path().to_str().unwrap());
        let ds = reader.read().expect("read multi table");
        // 表按 name 升序遍历：orders 在 users 前；union_headers 首列 __table，
        // 后跟 orders 的列 (id, user_id, amount)，再 users 新增的列 (name)。
        assert_eq!(
            ds.headers,
            vec!["__table", "id", "user_id", "amount", "name"]
        );
        assert_eq!(ds.rows.len(), 2);
        // 找到来自 users 与 orders 的行。
        let users_row = ds
            .rows
            .iter()
            .find(|r| r.fields.get("__table").map(|v| v.as_str()) == Some("users"))
            .expect("has users row");
        assert_eq!(users_row.fields.get("id").unwrap(), "1");
        assert_eq!(users_row.fields.get("name").unwrap(), "alice");
        // orders 表的列在 users 行补空。
        assert_eq!(users_row.fields.get("user_id").unwrap(), "");
        assert_eq!(users_row.fields.get("amount").unwrap(), "");
        let orders_row = ds
            .rows
            .iter()
            .find(|r| r.fields.get("__table").map(|v| v.as_str()) == Some("orders"))
            .expect("has orders row");
        assert_eq!(orders_row.fields.get("id").unwrap(), "100");
        assert_eq!(orders_row.fields.get("user_id").unwrap(), "1");
        assert_eq!(orders_row.fields.get("amount").unwrap(), "9.9");
        // users 表的 name 列在 orders 行补空。
        assert_eq!(orders_row.fields.get("name").unwrap(), "");
    }

    #[test]
    fn db_reader_skips_internal_tables() {
        let tmp = make_db(|conn| {
            // AUTOINCREMENT 会触发 SQLite 创建 sqlite_sequence 内部表。
            conn.execute_batch(
                "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT); \
                 INSERT INTO users (name) VALUES ('alice');",
            )
            .expect("setup autoincrement");
        });
        let reader = DbReader::new(tmp.path().to_str().unwrap());
        let ds = reader.read().expect("read skip internal");
        // 仅输出 users 行，sqlite_sequence 不出现。
        assert_eq!(ds.rows.len(), 1);
        assert_eq!(ds.rows[0].fields.get("__table").unwrap(), "users");
        assert_eq!(ds.rows[0].fields.get("name").unwrap(), "alice");
    }

    #[test]
    fn db_reader_empty_db() {
        // 创建一个无任何用户表的空 .db 文件。
        let tmp = make_db(|_conn| {});
        let reader = DbReader::new(tmp.path().to_str().unwrap());
        let ds = reader.read().expect("read empty db");
        assert!(ds.headers.is_empty());
        assert!(ds.rows.is_empty());
    }

    #[test]
    fn db_reader_nonexistent_path() {
        let reader = DbReader::new("/tmp/ruT0_nonexistent_db_T75_does_not_exist.db");
        let err = reader.read().expect_err("should error on missing path");
        assert!(matches!(err, CoreError::DataSource(_)), "got: {err:?}");
    }

    #[test]
    fn db_reader_non_sqlite_file() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".db")
            .tempfile()
            .expect("create tmp non-sqlite");
        tmp.write_all(b"not a db").expect("write garbage");
        tmp.flush().expect("flush");
        let reader = DbReader::new(tmp.path().to_str().unwrap());
        let err = reader.read().expect_err("should error on non-sqlite file");
        assert!(matches!(err, CoreError::DataSource(_)), "got: {err:?}");
    }

    #[test]
    fn quote_identifier_escapes_double_quote() {
        // 正常表名：双引号包裹。
        assert_eq!(quote_identifier("users"), "\"users\"");
        // 含双引号的表名：内部双引号翻倍。
        assert_eq!(quote_identifier("a\"b"), "\"a\"\"b\"");
    }
}
