//! 数据库迁移逻辑。
//!
//! v1.0.0: 仅 `schema_version=1`，无真实历史版本迁移。
//! - 首次启动：建表 + 建索引 + 写 `schema_version=1`。
//! - 后续启动：读 `app_settings.schema_version`；不存在视为首次；存在且 != 当前版本则备份旧 DB 重建。
//!
//! v1.1+ 在此扩展版本阶梯式迁移；当前框架占位。

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
    conn.execute_batch(SCHEMA_DDL)
        .map_err(DbError::Sqlite)?;
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![SCHEMA_VERSION.to_string()],
    )
    .map_err(DbError::Sqlite)?;
    Ok(())
}

/// 初始化迁移：首次建表；版本不兼容则备份旧 DB 重建。
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
        Some(v) => {
            // 版本不兼容（低于当前）：备份旧 DB 后重建。
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
                 DROP TABLE IF EXISTS app_settings;",
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
}
