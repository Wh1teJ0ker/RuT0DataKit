//! SQLite 持久层入口。
//!
//! v1.0.0: `DbManager` 持有 `Mutex<Connection>`，提供分页查询、批量写 cells、
//! 操作日志记录、键值设置、会话读写等公共方法，供命令层（T5/T7）调用。
//!
//! DB 文件位于 `app_config_dir/ruT0datakit.db`，5 表 + 3 索引 schema 由
//! `schema::SCHEMA_DDL` 定义，迁移由 `migrate::migrate` 处理。

pub mod error;
pub mod migrate;
pub mod schema;

pub use error::DbError;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// `cells` 表一行。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub sheet_id: i64,
    pub row_idx: u32,
    pub col_idx: u32,
    pub value: Option<String>,
}

/// `sessions` 摘要（列表用）。
// v1.1+ IPC 将调用；单测已覆盖。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub row_count: u32,
    pub created_at: String,
}

/// `sheets` 摘要（`SessionDetail` 嵌套用）。
// v1.1+ IPC 将调用；单测已覆盖。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetSummary {
    pub id: i64,
    pub session_id: i64,
    pub name: String,
    pub position: i32,
    pub created_at: String,
}

/// 会话详情：摘要 + 关联 sheets。
// v1.1+ IPC 将调用；单测已覆盖。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub session: SessionSummary,
    pub sheets: Vec<SheetSummary>,
}

/// 当前 ISO8601/RFC3339 时间戳（UTC）。
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// SQLite 连接管理器。
///
/// 持有 `std::sync::Mutex<Connection>` 以满足 Tauri State 的
/// `Send + Sync + 'static` 约束（`Connection` 是 `Send` 但非 `Sync`，
/// 用 `Mutex` 包即可）。无需 `Arc`——Tauri State 内部已用 `Arc` 管理。
pub struct DbManager {
    conn: Mutex<Connection>,
}

// v1.1+ IPC 将调用；单测已覆盖。
#[allow(dead_code)]
impl DbManager {
    /// 在 `app_config_dir` 下打开（或创建）`ruT0datakit.db` 并执行初始化迁移。
    pub fn new(app_config_dir: &Path) -> Result<Self, DbError> {
        std::fs::create_dir_all(app_config_dir)?;
        let db_path = app_config_dir.join("ruT0datakit.db");
        let conn = Connection::open(&db_path)?;
        migrate::migrate(&db_path, &conn)?;
        Ok(DbManager {
            conn: Mutex::new(conn),
        })
    }

    /// 分页查询 `cells`，按**行**分页（每页返回 `page_size` 个 `row_idx`
    /// 对应的全部列），结果按 `row_idx` 升序、`col_idx` 升序返回。
    ///
    /// `page` 从 1 开始；`page=0` 按 1 处理（`saturating_sub`）。
    /// 行级分页：先用子查询取本页的 `row_idx` 集合（`DISTINCT` +
    /// `LIMIT/OFFSET`），再取这些行的全部 cells——确保多列 Sheet
    /// 每页返回 `page_size` 行而非 `page_size` 个 cell。
    pub fn query_cells(
        &self,
        sheet_id: i64,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Cell>, DbError> {
        let offset = (page.saturating_sub(1).saturating_mul(page_size)) as i64;
        let limit = page_size as i64;
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT sheet_id, row_idx, col_idx, value FROM cells
             WHERE sheet_id = ?1 AND row_idx IN (
                 SELECT DISTINCT row_idx FROM cells
                 WHERE sheet_id = ?1
                 ORDER BY row_idx ASC
                 LIMIT ?2 OFFSET ?3
             )
             ORDER BY row_idx ASC, col_idx ASC",
        )?;
        let rows = stmt.query_map(params![sheet_id, limit, offset], |r| {
            Ok(Cell {
                sheet_id: r.get::<_, i64>(0)?,
                row_idx: r.get::<_, i64>(1)? as u32,
                col_idx: r.get::<_, i64>(2)? as u32,
                value: r.get::<_, Option<String>>(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 事务批量写 `cells`（`sheet_id` 取参数，忽略 `Cell.sheet_id` 字段）。
    pub fn write_cells(&self, sheet_id: i64, cells: &[Cell]) -> Result<(), DbError> {
        let mut conn = self.conn.lock().expect("db mutex poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO cells (sheet_id, row_idx, col_idx, value)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(sheet_id, row_idx, col_idx) DO UPDATE SET value=excluded.value",
            )?;
            for c in cells {
            stmt.execute(params![
                sheet_id,
                c.row_idx as i64,
                c.col_idx as i64,
                c.value,
            ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// 统计某 sheet 的数据**行数**（`DISTINCT row_idx`，含表头行）。
    pub fn count_rows(&self, sheet_id: i64) -> Result<u32, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT row_idx) FROM cells WHERE sheet_id = ?1",
            params![sheet_id],
            |r| r.get(0),
        )?;
        Ok(count.max(0) as u32)
    }

    /// 创建导入会话，返回 `id`。
    pub fn create_session(
        &self,
        name: &str,
        source_path: Option<&str>,
        source_type: &str,
        row_count: u32,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO sessions (name, source_path, source_type, row_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![name, source_path, source_type, row_count as i64, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 创建 sheet，返回 `id`。
    pub fn create_sheet(
        &self,
        session_id: i64,
        name: &str,
        position: i32,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO sheets (session_id, name, position, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, name, position, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 记录操作日志，返回 `id`。
    pub fn log_operation(
        &self,
        sheet_id: Option<i64>,
        kind: &str,
        params_json: &str,
        result_json: &str,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO operations (sheet_id, kind, params_json, result_snapshot_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sheet_id, kind, params_json, result_json, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 读键值设置。
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        Ok(value)
    }

    /// 写键值设置（upsert）。
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// 列出全部会话摘要（按 `created_at` 降序）。
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, name, source_type, row_count, created_at
             FROM sessions
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SessionSummary {
                id: r.get::<_, i64>(0)?,
                name: r.get::<_, String>(1)?,
                source_type: r.get::<_, String>(2)?,
                row_count: r.get::<_, i64>(3)? as u32,
                created_at: r.get::<_, String>(4)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 取会话详情：摘要 + 关联 sheets（按 `position` 升序）。
    pub fn get_session(&self, session_id: i64) -> Result<SessionDetail, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let session = conn.query_row(
            "SELECT id, name, source_type, row_count, created_at
             FROM sessions
             WHERE id = ?1",
            params![session_id],
            |r| {
                Ok(SessionSummary {
                    id: r.get::<_, i64>(0)?,
                    name: r.get::<_, String>(1)?,
                    source_type: r.get::<_, String>(2)?,
                    row_count: r.get::<_, i64>(3)? as u32,
                    created_at: r.get::<_, String>(4)?,
                })
            },
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, name, position, created_at
             FROM sheets
             WHERE session_id = ?1
             ORDER BY position ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |r| {
            Ok(SheetSummary {
                id: r.get::<_, i64>(0)?,
                session_id: r.get::<_, i64>(1)?,
                name: r.get::<_, String>(2)?,
                position: r.get::<_, i32>(3)?,
                created_at: r.get::<_, String>(4)?,
            })
        })?;
        let mut sheets = Vec::new();
        for row in rows {
            sheets.push(row?);
        }
        Ok(SessionDetail { session, sheets })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> (tempfile::TempDir, DbManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        (dir, mgr)
    }

    #[test]
    fn new_creates_tables_and_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        // schema_version 应为 1
        assert_eq!(mgr.get_setting("schema_version").unwrap().as_deref(), Some("1"));
        // DB 文件已生成
        assert!(dir.path().join("ruT0datakit.db").exists());
        // 重复 new 幂等
        let _ = DbManager::new(dir.path()).unwrap();
    }

    #[test]
    fn write_and_query_cells_paginated() {
        let (_dir, mgr) = open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        let cells: Vec<Cell> = (0..5u32)
            .map(|i| Cell {
                sheet_id: shid,
                row_idx: i,
                col_idx: 0,
                value: Some(format!("v{}", i)),
            })
            .collect();
        mgr.write_cells(shid, &cells).unwrap();
        assert_eq!(mgr.count_rows(shid).unwrap(), 5);
        // 第 1 页 2 行
        let p1 = mgr.query_cells(shid, 1, 2).unwrap();
        assert_eq!(p1.len(), 2);
        assert_eq!(p1[0].row_idx, 0);
        assert_eq!(p1[1].row_idx, 1);
        // 第 3 页 2 行（只剩 1 行）
        let p3 = mgr.query_cells(shid, 3, 2).unwrap();
        assert_eq!(p3.len(), 1);
        assert_eq!(p3[0].row_idx, 4);
        // page=0 当作 1
        let p0 = mgr.query_cells(shid, 0, 2).unwrap();
        assert_eq!(p0.len(), 2);
    }

    #[test]
    fn write_and_query_cells_paginated_multi_column() {
        // 多列 Sheet：3 列 × 5 行（含表头 row_idx=0），验证行级分页语义。
        let (_dir, mgr) = open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        let mut cells: Vec<Cell> = Vec::new();
        for row in 0..5u32 {
            for col in 0..3u32 {
                cells.push(Cell {
                    sheet_id: shid,
                    row_idx: row,
                    col_idx: col,
                    value: Some(format!("r{}c{}", row, col)),
                });
            }
        }
        mgr.write_cells(shid, &cells).unwrap();
        // count_rows 应返回行数 5（而非 cell 数 15）。
        assert_eq!(mgr.count_rows(shid).unwrap(), 5);
        // 第 1 页 2 行 → 6 个 cell（2 行 × 3 列）。
        let p1 = mgr.query_cells(shid, 1, 2).unwrap();
        assert_eq!(p1.len(), 6);
        // 全部属于 row_idx ∈ {0, 1}。
        assert!(p1.iter().all(|c| c.row_idx <= 1));
        // 第 2 页 2 行 → row_idx ∈ {2, 3}，6 个 cell。
        let p2 = mgr.query_cells(shid, 2, 2).unwrap();
        assert_eq!(p2.len(), 6);
        assert!(p2.iter().all(|c| c.row_idx >= 2 && c.row_idx <= 3));
        // 第 3 页 1 行 → row_idx = 4，3 个 cell。
        let p3 = mgr.query_cells(shid, 3, 2).unwrap();
        assert_eq!(p3.len(), 3);
        assert!(p3.iter().all(|c| c.row_idx == 4));
        // 第 4 页 0 行。
        let p4 = mgr.query_cells(shid, 4, 2).unwrap();
        assert!(p4.is_empty());
    }

    #[test]
    fn write_cells_upsert_on_conflict() {
        let (_dir, mgr) = open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        let c = Cell {
            sheet_id: shid,
            row_idx: 0,
            col_idx: 0,
            value: Some("a".into()),
        };
        mgr.write_cells(shid, &[c.clone()]).unwrap();
        // 覆盖同一主键
        let c2 = Cell {
            sheet_id: shid,
            row_idx: 0,
            col_idx: 0,
            value: Some("b".into()),
        };
        mgr.write_cells(shid, &[c2]).unwrap();
        assert_eq!(mgr.count_rows(shid).unwrap(), 1);
        let rows = mgr.query_cells(shid, 1, 10).unwrap();
        assert_eq!(rows[0].value.as_deref(), Some("b"));
    }

    #[test]
    fn log_operation_inserts_row() {
        let (_dir, mgr) = open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        let id = mgr
            .log_operation(Some(shid), "import", "{}", "{}")
            .unwrap();
        assert!(id > 0);
    }

    #[test]
    fn settings_roundtrip() {
        let (_dir, mgr) = open();
        assert_eq!(mgr.get_setting("k").unwrap(), None);
        mgr.set_setting("k", "v1").unwrap();
        assert_eq!(mgr.get_setting("k").unwrap().as_deref(), Some("v1"));
        mgr.set_setting("k", "v2").unwrap();
        assert_eq!(mgr.get_setting("k").unwrap().as_deref(), Some("v2"));
    }

    #[test]
    fn list_and_get_session() {
        let (_dir, mgr) = open();
        let sid = mgr
            .create_session("s1", Some("/a/b.csv"), "csv", 100)
            .unwrap();
        let sh1 = mgr.create_sheet(sid, "Sheet1", 1).unwrap();
        let sh2 = mgr.create_sheet(sid, "Sheet2", 0).unwrap();
        // list
        let list = mgr.list_sessions().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, sid);
        assert_eq!(list[0].name, "s1");
        assert_eq!(list[0].source_type, "csv");
        assert_eq!(list[0].row_count, 100);
        // get（按 position 升序）
        let detail = mgr.get_session(sid).unwrap();
        assert_eq!(detail.session.id, sid);
        assert_eq!(detail.sheets.len(), 2);
        assert_eq!(detail.sheets[0].id, sh2); // position=0 在前
        assert_eq!(detail.sheets[1].id, sh1); // position=1 在后
    }
}
