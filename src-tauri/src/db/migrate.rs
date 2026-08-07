//! 数据库迁移逻辑。
//!
//! v1.2.0: `schema_version=3`，`operations` 表新增 `before_snapshot_json` 列
//! 存撤销前置快照；新增 `idx_cells_sheet_col` 复合索引（v2→v3 增量迁移）。
//! v1.1.0: `schema_version=2`，新增 `rules` 表（v1→v2 增量迁移）。
//! v1.0.0: `schema_version=1`，无真实历史版本迁移。
//! - 首次启动：建表 + 建索引 + 写 `schema_version`。
//! - 后续启动：读 `app_settings.schema_version`；不存在视为首次；存在且 != 当前版本则备份旧 DB 重建。
//!
//! v1→v2 迁移：仅追加 `rules` 表 + `idx_rules_kind` 索引（`IF NOT EXISTS` 幂等），
//! 不影响历史数据，无需备份重建。
//!
//! v2→v3 迁移：`operations` 表 `ADD COLUMN before_snapshot_json TEXT`（先
//! `PRAGMA table_info` 检查列是否存在再 ALTER，保证幂等）+ `idx_cells_sheet_col`
//! 复合索引（`IF NOT EXISTS` 幂等），不影响历史数据。

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::error::DbError;
use super::schema::{SCHEMA_DDL, SCHEMA_VERSION};

/// 读取当前 DB 的 `schema_version`；不存在（首次启动）返回 `None`。
pub fn read_schema_version(conn: &Connection) -> Result<Option<i64>, DbError> {
    let row_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_settings'",
            [],
            |r| r.get(0),
        )
        .map_err(DbError::Sqlite)?;
    if row_count == 0 {
        return Ok(None);
    }
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='schema_version'",
            [],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten();
    Ok(value.and_then(|v| v.parse::<i64>().ok()))
}

/// 执行首次/幂等建表 + 建索引，写 `schema_version`。
/// 幂等：`IF NOT EXISTS` 保证重复调用安全。
pub fn bootstrap(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(SCHEMA_DDL).map_err(DbError::Sqlite)?;
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![SCHEMA_VERSION.to_string()],
    )
    .map_err(DbError::Sqlite)?;
    Ok(())
}

/// v1→v2 增量迁移：追加 `rules` 表 + `idx_rules_kind` 索引（幂等），
/// 更新 `schema_version=2`。不影响历史数据。
pub fn migrate_v1_to_v2(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rules (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            kind        TEXT NOT NULL,
            field       TEXT,
            pattern     TEXT,
            replacement TEXT,
            enabled     INTEGER NOT NULL DEFAULT 1,
            description TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_rules_kind ON rules(kind);",
    )
    .map_err(DbError::Sqlite)?;
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![SCHEMA_VERSION.to_string()],
    )
    .map_err(DbError::Sqlite)?;
    Ok(())
}

/// v2→v3 增量迁移：`operations` 表 `ADD COLUMN before_snapshot_json TEXT`
/// + `idx_cells_sheet_col` 复合索引（幂等），更新 `schema_version=3`。不影响历史数据。
///
/// 幂等保证：先用 `PRAGMA table_info(operations)` 查 `before_snapshot_json`
/// 列是否存在，不存在才 ALTER；索引用 `IF NOT EXISTS`。
pub fn migrate_v2_to_v3(conn: &Connection) -> Result<(), DbError> {
    // 检查 before_snapshot_json 列是否已存在。
    let col_exists: bool = {
        let mut stmt = conn
            .prepare("PRAGMA table_info(operations)")
            .map_err(DbError::Sqlite)?;
        let mut rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        let mut found = false;
        for row_res in rows.by_ref() {
            match row_res {
                Ok(name) if name == "before_snapshot_json" => {
                    found = true;
                    break;
                }
                Ok(_) => continue,
                Err(e) => return Err(DbError::Sqlite(e)),
            }
        }
        found
    };
    if !col_exists {
        conn.execute(
            "ALTER TABLE operations ADD COLUMN before_snapshot_json TEXT",
            [],
        )
        .map_err(DbError::Sqlite)?;
    }
    // 复合索引（IF NOT EXISTS 幂等）。
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cells_sheet_col ON cells(sheet_id, col_idx)",
        [],
    )
    .map_err(DbError::Sqlite)?;
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![SCHEMA_VERSION.to_string()],
    )
    .map_err(DbError::Sqlite)?;
    Ok(())
}

/// 初始化迁移：首次建表；版本不兼容则备份旧 DB 重建；v1→v2 / v2→v3 增量迁移。
///
/// - `db_path`: DB 文件路径（`app_config_dir/ruT0datakit.db`）。
/// - `conn`: 已打开的连接。
/// - 返回迁移说明（用于日志/测试断言）。
pub fn migrate(db_path: &Path, conn: &Connection) -> Result<String, DbError> {
    let current = read_schema_version(conn)?;
    match current {
        None => {
            bootstrap(conn)?;
            Ok(format!("bootstrapped schema_version={}", SCHEMA_VERSION))
        }
        Some(v) if v == SCHEMA_VERSION => {
            // 版本一致，幂等补建（保证 DB 文件存在但表缺的边缘情况）。
            conn.execute_batch(SCHEMA_DDL).map_err(DbError::Sqlite)?;
            Ok(format!("schema_version={} ok", v))
        }
        Some(1) if SCHEMA_VERSION == 2 => {
            // v1→v2 增量迁移：追加 rules 表，不备份重建。
            migrate_v1_to_v2(conn)?;
            Ok("migrated v1→v2 (rules table added)".into())
        }
        Some(2) if SCHEMA_VERSION == 3 => {
            // v2→v3 增量迁移：operations 表加 before_snapshot_json 列 + 复合索引。
            migrate_v2_to_v3(conn)?;
            Ok("migrated v2→v3 (before_snapshot_json column + idx_cells_sheet_col added)".into())
        }
        Some(v) => {
            // 版本不兼容（高于当前或无法增量迁移）：备份旧 DB 后重建。
            let ts = backup_timestamp();
            let bak: PathBuf = db_path
                .with_extension(format!("db.bak.{}", ts))
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(format!("ruT0datakit.db.bak.{}", ts));
            // 关闭连接前先落盘备份：复制当前文件（若存在）。
            if db_path.exists() {
                std::fs::copy(db_path, &bak).map_err(DbError::Io)?;
            }
            conn.execute_batch(
                "DROP TABLE IF EXISTS cells; \
                 DROP TABLE IF EXISTS operations; \
                 DROP TABLE IF EXISTS sheets; \
                 DROP TABLE IF EXISTS sessions; \
                 DROP TABLE IF EXISTS app_settings; \
                 DROP TABLE IF EXISTS rules;",
            )
            .map_err(DbError::Sqlite)?;
            bootstrap(conn)?;
            Err(DbError::Migration(format!(
                "incompatible schema_version={} (expected {}), rebuilt from backup",
                v, SCHEMA_VERSION
            )))
        }
    }
}

fn backup_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_first_time_bootstraps() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        let note = migrate(&db_path, &conn).unwrap();
        assert!(note.contains("bootstrapped"));
        assert_eq!(read_schema_version(&conn).unwrap(), Some(SCHEMA_VERSION));
    }

    #[test]
    fn migrate_repeat_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        migrate(&db_path, &conn).unwrap();
        let note = migrate(&db_path, &conn).unwrap();
        assert!(note.contains("ok"));
        assert_eq!(read_schema_version(&conn).unwrap(), Some(SCHEMA_VERSION));
    }

    #[test]
    fn migrate_v1_to_v2_adds_rules_table() {
        // v1.2.0 起 SCHEMA_VERSION=3：v1→v2 增量迁移已被 v1→v3 全量重建取代。
        // 此测试用 v1.1.0 时代的固定 v1→v2 路径验证（直接调用 migrate_v1_to_v2）。
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open(dir.path().join("ruT0datakit.db")).unwrap();
        // 模拟 v1：建 app_settings + 写 schema_version=1。
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO app_settings(key, value) VALUES('schema_version', '1');",
        )
        .unwrap();
        // 直接走 v1→v2 增量迁移。
        migrate_v1_to_v2(&conn).unwrap();
        // rules 表已存在。
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='rules'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        // migrate_v1_to_v2 把 schema_version 写为 SCHEMA_VERSION（当前为 3）。
        assert_eq!(read_schema_version(&conn).unwrap(), Some(SCHEMA_VERSION));
    }

    #[test]
    fn migrate_v1_to_v2_preserves_existing_data() {
        // 同上：v1.2.0 起 v1 DB 走重建路径，此测试改用直接 migrate_v1_to_v2
        // 验证「增量迁移不破坏历史数据」这一原语义。
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open(dir.path().join("ruT0datakit.db")).unwrap();
        conn.execute_batch(
            "CREATE TABLE sessions (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO app_settings(key, value) VALUES('schema_version', '1');
             INSERT INTO sessions(id, name) VALUES(1, 'preserved');",
        )
        .unwrap();
        migrate_v1_to_v2(&conn).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM sessions WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "preserved");
    }

    /// v2 DB（不含 before_snapshot_json 列、不含 idx_cells_sheet_col）。
    fn build_v2_db(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL,
                source_path TEXT, source_type TEXT NOT NULL,
                row_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS sheets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL, name TEXT NOT NULL,
                position INTEGER NOT NULL DEFAULT 0,
                column_order TEXT, column_visibility TEXT,
                created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS cells (
                sheet_id INTEGER NOT NULL, row_idx INTEGER NOT NULL,
                col_idx INTEGER NOT NULL, value TEXT,
                PRIMARY KEY (sheet_id, row_idx, col_idx));
             CREATE TABLE IF NOT EXISTS operations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sheet_id INTEGER, kind TEXT NOT NULL,
                params_json TEXT, result_snapshot_json TEXT,
                created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT);
             CREATE TABLE IF NOT EXISTS rules (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL,
                field TEXT, pattern TEXT, replacement TEXT,
                enabled INTEGER NOT NULL DEFAULT 1, description TEXT NOT NULL DEFAULT '');
             CREATE INDEX IF NOT EXISTS idx_cells_sheet_row ON cells(sheet_id, row_idx);
             CREATE INDEX IF NOT EXISTS idx_operations_sheet ON operations(sheet_id);
             CREATE INDEX IF NOT EXISTS idx_sheets_session ON sheets(session_id);
             CREATE INDEX IF NOT EXISTS idx_rules_kind ON rules(kind);
             INSERT INTO app_settings(key, value) VALUES('schema_version', '2');",
        )
        .unwrap();
    }

    /// `operations.before_snapshot_json` 列是否存在。
    fn column_exists(conn: &Connection, table: &str, col: &str) -> bool {
        let sql = format!("PRAGMA table_info({})", table);
        let mut stmt = match conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let mut rows = match stmt.query_map([], |r| r.get::<_, String>(1)) {
            Ok(r) => r,
            Err(_) => return false,
        };
        while let Some(Ok(name)) = rows.next() {
            if name == col {
                return true;
            }
        }
        false
    }

    /// `idx_cells_sheet_col` 索引是否存在。
    fn index_exists(conn: &Connection, name: &str) -> bool {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                rusqlite::params![name],
                |r| r.get(0),
            )
            .unwrap();
        n > 0
    }

    #[test]
    fn migrate_v2_to_v3_adds_before_snapshot_column() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        build_v2_db(&conn);
        assert!(!column_exists(&conn, "operations", "before_snapshot_json"));
        assert_eq!(read_schema_version(&conn).unwrap(), Some(2));
        let note = migrate(&db_path, &conn).unwrap();
        assert!(note.contains("v2→v3"), "note={}", note);
        assert_eq!(read_schema_version(&conn).unwrap(), Some(3));
        assert!(column_exists(&conn, "operations", "before_snapshot_json"));
    }

    #[test]
    fn migrate_v2_to_v3_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        build_v2_db(&conn);
        migrate(&db_path, &conn).unwrap();
        // 第二次：版本已是 3，走幂等补建分支（execute_batch SCHEMA_DDL）。
        let note = migrate(&db_path, &conn).unwrap();
        assert!(note.contains("ok"));
        assert_eq!(read_schema_version(&conn).unwrap(), Some(3));
        assert!(column_exists(&conn, "operations", "before_snapshot_json"));
        // 直接调 migrate_v2_to_v3 也应幂等（不报 duplicate column）。
        migrate_v2_to_v3(&conn).unwrap();
        assert!(column_exists(&conn, "operations", "before_snapshot_json"));
    }

    #[test]
    fn idx_cells_sheet_col_exists_after_migrate() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        build_v2_db(&conn);
        assert!(!index_exists(&conn, "idx_cells_sheet_col"));
        migrate(&db_path, &conn).unwrap();
        assert!(index_exists(&conn, "idx_cells_sheet_col"));
    }

    #[test]
    fn migrate_v2_to_v3_preserves_existing_data() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        build_v2_db(&conn);
        conn.execute(
            "INSERT INTO sessions(id, name, source_type, row_count, created_at, updated_at)
             VALUES(1, 'keep', 'csv', 0, 't', 't')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO operations(id, sheet_id, kind, params_json, result_snapshot_json, created_at)
             VALUES(1, NULL, 'import', '{}', '{}', 't')",
            [],
        )
        .unwrap();
        migrate(&db_path, &conn).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM sessions WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "keep");
        // 旧 operation 行的 before_snapshot_json 应为 NULL。
        let bs: Option<String> = conn
            .query_row(
                "SELECT before_snapshot_json FROM operations WHERE id=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bs, None);
    }
}
