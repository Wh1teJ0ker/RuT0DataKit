//! SQLite schema migration logic.
//!
//! Supported database versions are upgraded incrementally in one transaction:
//! v1 -> v2 -> v3 -> v4 -> v5. Each step persists its own target version so a
//! failed migration rolls back both schema and version changes together.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::error::DbError;
use super::schema::{SCHEMA_DDL, SCHEMA_VERSION};

const V1: i64 = 1;
const V2: i64 = 2;
const V3: i64 = 3;
const V4: i64 = 4;
const V5: i64 = 5;

/// Reads the current database `schema_version`.
///
/// Returns `Ok(None)` only when the `app_settings` table is absent, which means
/// the database is brand new and may be bootstrapped safely. When `app_settings`
/// exists but the `schema_version` row is missing, NULL, or non-integer, the
/// metadata is corrupt/unknown and this returns an error so the caller backs up
/// and rejects startup instead of silently bootstrapping over existing data.
pub fn read_schema_version(conn: &Connection) -> Result<Option<i64>, DbError> {
    let row_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_settings'",
        [],
        |row| row.get(0),
    )?;
    if row_count == 0 {
        return Ok(None);
    }

    // app_settings exists; the schema_version row must be present and parseable.
    // A missing row, NULL value, or non-integer value means the database is not
    // brand new and its real schema is unknown, so it must NOT be bootstrapped.
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='schema_version'",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten();
    match value {
        Some(raw) => raw.parse::<i64>().map(Some).map_err(|_| {
            DbError::Migration(format!(
                "app_settings.schema_version is present but not an integer: {raw:?}"
            ))
        }),
        None => Err(DbError::Migration(
            "app_settings table exists but schema_version row is missing or NULL".into(),
        )),
    }
}

fn write_schema_version(conn: &Connection, version: i64) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![version.to_string()],
    )?;
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, DbError> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in rows {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Creates the current schema for a new database and records the current version.
pub fn bootstrap(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(SCHEMA_DDL)?;
    write_schema_version(conn, SCHEMA_VERSION)
}

/// v1 -> v2: adds the rules table and index without changing existing data.
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
    )?;
    write_schema_version(conn, V2)
}

/// v2 -> v3: adds the operation snapshot column and the column-search index.
pub fn migrate_v2_to_v3(conn: &Connection) -> Result<(), DbError> {
    if !table_has_column(conn, "operations", "before_snapshot_json")? {
        conn.execute(
            "ALTER TABLE operations ADD COLUMN before_snapshot_json TEXT",
            [],
        )?;
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cells_sheet_col ON cells(sheet_id, col_idx)",
        [],
    )?;
    write_schema_version(conn, V3)
}

/// v3 -> v4: adds the optional generic-template rule parameter column.
pub fn migrate_v3_to_v4(conn: &Connection) -> Result<(), DbError> {
    if !table_has_column(conn, "rules", "template")? {
        conn.execute("ALTER TABLE rules ADD COLUMN template TEXT", [])?;
    }
    write_schema_version(conn, V4)
}

/// v4 -> v5: adds the optional extraction rule parameter column.
pub fn migrate_v4_to_v5(conn: &Connection) -> Result<(), DbError> {
    if !table_has_column(conn, "rules", "params")? {
        conn.execute("ALTER TABLE rules ADD COLUMN params TEXT", [])?;
    }
    write_schema_version(conn, V5)
}

fn backup_path(db_path: &Path) -> PathBuf {
    let timestamp = backup_timestamp();
    let filename = db_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ruT0datakit.db");
    db_path.with_file_name(format!("{filename}.bak.{timestamp}"))
}

fn backup_database(db_path: &Path) -> Result<PathBuf, DbError> {
    if !db_path.is_file() {
        return Err(DbError::Migration(format!(
            "cannot back up unsupported schema because database file is missing: {}",
            db_path.display()
        )));
    }

    let backup = backup_path(db_path);
    std::fs::copy(db_path, &backup)?;
    Ok(backup)
}

/// Initializes a database or upgrades a supported older version to the current schema.
///
/// All supported upgrade steps run in one SQLite transaction. A database whose version is
/// newer than this application, or otherwise unsupported, is copied to a backup and left
/// unchanged before startup is rejected.
pub fn migrate(db_path: &Path, conn: &Connection) -> Result<String, DbError> {
    let current = match read_schema_version(conn) {
        Ok(value) => value,
        Err(error) => {
            // app_settings exists but schema_version is missing/NULL/non-integer:
            // the database is not brand new and its real schema is unknown, so
            // back it up unchanged and reject startup (non-destructive).
            let backup = backup_database(db_path)?;
            return Err(DbError::Migration(format!(
                "{error}; backed up unchanged database to {}",
                backup.display()
            )));
        }
    };
    match current {
        None => {
            bootstrap(conn)?;
            Ok(format!("bootstrapped schema_version={SCHEMA_VERSION}"))
        }
        Some(version) if version == SCHEMA_VERSION => {
            conn.execute_batch(SCHEMA_DDL)?;
            Ok(format!("schema_version={version} ok"))
        }
        Some(V1 | V2 | V3 | V4) => {
            let transaction = conn.unchecked_transaction()?;
            let note = match current {
                Some(V1) => {
                    migrate_v1_to_v2(&transaction)?;
                    migrate_v2_to_v3(&transaction)?;
                    migrate_v3_to_v4(&transaction)?;
                    migrate_v4_to_v5(&transaction)?;
                    "migrated v1 -> v5"
                }
                Some(V2) => {
                    migrate_v2_to_v3(&transaction)?;
                    migrate_v3_to_v4(&transaction)?;
                    migrate_v4_to_v5(&transaction)?;
                    "migrated v2 -> v5"
                }
                Some(V3) => {
                    migrate_v3_to_v4(&transaction)?;
                    migrate_v4_to_v5(&transaction)?;
                    "migrated v3 -> v5"
                }
                Some(V4) => {
                    migrate_v4_to_v5(&transaction)?;
                    "migrated v4 -> v5"
                }
                _ => unreachable!("supported versions are matched above"),
            };
            transaction.commit()?;
            Ok(note.into())
        }
        Some(version) => {
            let backup = backup_database(db_path)?;
            Err(DbError::Migration(format!(
                "unsupported schema_version={version} (current {SCHEMA_VERSION}); \
                 backed up unchanged database to {}",
                backup.display()
            )))
        }
    }
}

fn backup_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1_SCHEMA: &str = r#"
        CREATE TABLE sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            source_path TEXT,
            source_type TEXT NOT NULL,
            row_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE sheets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            column_order TEXT,
            column_visibility TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE cells (
            sheet_id INTEGER NOT NULL,
            row_idx INTEGER NOT NULL,
            col_idx INTEGER NOT NULL,
            value TEXT,
            PRIMARY KEY (sheet_id, row_idx, col_idx)
        );
        CREATE TABLE operations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sheet_id INTEGER,
            kind TEXT NOT NULL,
            params_json TEXT,
            result_snapshot_json TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT);
        CREATE INDEX idx_cells_sheet_row ON cells(sheet_id, row_idx);
        CREATE INDEX idx_operations_sheet ON operations(sheet_id);
        CREATE INDEX idx_sheets_session ON sheets(session_id);
    "#;

    fn set_fixture_version(conn: &Connection, version: i64) {
        write_schema_version(conn, version).unwrap();
    }

    fn build_v1_db(conn: &Connection) {
        conn.execute_batch(V1_SCHEMA).unwrap();
        conn.execute_batch(
            "INSERT INTO sessions(id, name, source_path, source_type, row_count, created_at, updated_at)
             VALUES(1, 'legacy session', 'legacy.csv', 'csv', 1, '2024-01-01', '2024-01-01');
             INSERT INTO sheets(id, session_id, name, position, created_at)
             VALUES(1, 1, 'legacy sheet', 0, '2024-01-01');
             INSERT INTO cells(sheet_id, row_idx, col_idx, value) VALUES(1, 0, 0, 'legacy cell');
             INSERT INTO operations(id, sheet_id, kind, params_json, result_snapshot_json, created_at)
             VALUES(1, 1, 'import', '{}', '{}', '2024-01-01');",
        )
        .unwrap();
        set_fixture_version(conn, V1);
    }

    fn build_v2_db(conn: &Connection) {
        build_v1_db(conn);
        conn.execute_batch(
            "CREATE TABLE rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                field TEXT,
                pattern TEXT,
                replacement TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                description TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX idx_rules_kind ON rules(kind);",
        )
        .unwrap();
        set_fixture_version(conn, V2);
    }

    fn build_v3_db(conn: &Connection) {
        build_v2_db(conn);
        conn.execute_batch(
            "ALTER TABLE operations ADD COLUMN before_snapshot_json TEXT;
             CREATE INDEX idx_cells_sheet_col ON cells(sheet_id, col_idx);",
        )
        .unwrap();
        set_fixture_version(conn, V3);
    }

    fn build_v4_db(conn: &Connection) {
        build_v3_db(conn);
        conn.execute("ALTER TABLE rules ADD COLUMN template TEXT", [])
            .unwrap();
        set_fixture_version(conn, V4);
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        table_has_column(conn, table, column).unwrap()
    }

    fn index_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name=?1)",
            rusqlite::params![name],
            |row| row.get::<_, bool>(0),
        )
        .unwrap()
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            rusqlite::params![name],
            |row| row.get::<_, bool>(0),
        )
        .unwrap()
    }

    fn assert_legacy_data_is_readable(conn: &Connection) {
        let session: String = conn
            .query_row("SELECT name FROM sessions WHERE id=1", [], |row| row.get(0))
            .unwrap();
        let sheet: String = conn
            .query_row("SELECT name FROM sheets WHERE id=1", [], |row| row.get(0))
            .unwrap();
        let cell: String = conn
            .query_row(
                "SELECT value FROM cells WHERE sheet_id=1 AND row_idx=0 AND col_idx=0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let operation: String = conn
            .query_row("SELECT kind FROM operations WHERE id=1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(session, "legacy session");
        assert_eq!(sheet, "legacy sheet");
        assert_eq!(cell, "legacy cell");
        assert_eq!(operation, "import");
    }

    fn assert_v5_schema(conn: &Connection) {
        assert_eq!(read_schema_version(conn).unwrap(), Some(V5));
        assert!(column_exists(conn, "operations", "before_snapshot_json"));
        assert!(column_exists(conn, "rules", "template"));
        assert!(column_exists(conn, "rules", "params"));
        assert!(index_exists(conn, "idx_cells_sheet_col"));
    }

    #[test]
    fn migrate_first_time_bootstraps() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();

        let note = migrate(&db_path, &conn).unwrap();

        assert!(note.contains("bootstrapped"));
        assert_v5_schema(&conn);
    }

    #[test]
    fn absent_schema_version_row_is_backed_up_and_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        // app_settings table exists but the schema_version row is absent.
        conn.execute_batch("CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT);")
            .unwrap();
        conn.execute(
            "INSERT INTO app_settings(key, value) VALUES('unrelated', 'value')",
            [],
        )
        .unwrap();

        let error = migrate(&db_path, &conn).unwrap_err();

        assert!(
            matches!(error, DbError::Migration(ref msg) if msg.contains("schema_version row is missing or NULL"))
        );
        // schema_version row still absent -> read_schema_version keeps rejecting.
        assert!(read_schema_version(&conn).is_err());

        let backups: Vec<PathBuf> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("ruT0datakit.db.bak."))
            })
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn null_schema_version_is_backed_up_and_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT);")
            .unwrap();
        conn.execute(
            "INSERT INTO app_settings(key, value) VALUES('schema_version', NULL)",
            [],
        )
        .unwrap();

        let error = migrate(&db_path, &conn).unwrap_err();

        assert!(
            matches!(error, DbError::Migration(ref msg) if msg.contains("schema_version row is missing or NULL"))
        );
        assert!(read_schema_version(&conn).is_err());

        let backups: Vec<PathBuf> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("ruT0datakit.db.bak."))
            })
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn non_integer_schema_version_is_backed_up_and_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT);")
            .unwrap();
        conn.execute(
            "INSERT INTO app_settings(key, value) VALUES('schema_version', 'not-a-number')",
            [],
        )
        .unwrap();

        let error = migrate(&db_path, &conn).unwrap_err();

        assert!(matches!(error, DbError::Migration(ref msg) if msg.contains("not an integer")));
        assert!(read_schema_version(&conn).is_err());

        let backups: Vec<PathBuf> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("ruT0datakit.db.bak."))
            })
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn genuinely_empty_database_is_bootstrapped_not_backed_up() {
        // A file with no app_settings table at all is brand new -> bootstrap,
        // not the backup-and-reject path.
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();

        let note = migrate(&db_path, &conn).unwrap();

        assert!(note.contains("bootstrapped"));
        assert_eq!(read_schema_version(&conn).unwrap(), Some(SCHEMA_VERSION));

        let backups: Vec<PathBuf> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("ruT0datakit.db.bak."))
            })
            .collect();
        assert!(backups.is_empty());
    }

    #[test]
    fn each_step_records_its_explicit_target_version() {
        let conn = Connection::open_in_memory().unwrap();
        build_v1_db(&conn);

        migrate_v1_to_v2(&conn).unwrap();
        assert_eq!(read_schema_version(&conn).unwrap(), Some(V2));
        migrate_v2_to_v3(&conn).unwrap();
        assert_eq!(read_schema_version(&conn).unwrap(), Some(V3));
        migrate_v3_to_v4(&conn).unwrap();
        assert_eq!(read_schema_version(&conn).unwrap(), Some(V4));
        migrate_v4_to_v5(&conn).unwrap();
        assert_eq!(read_schema_version(&conn).unwrap(), Some(V5));
    }

    #[test]
    fn migrate_supported_versions_to_v5_in_order_without_data_loss() {
        type MigFixture = (i64, fn(&Connection));
        let fixtures: [MigFixture; 4] = [
            (V1, build_v1_db),
            (V2, build_v2_db),
            (V3, build_v3_db),
            (V4, build_v4_db),
        ];

        for (source_version, build_fixture) in fixtures {
            let dir = tempfile::tempdir().unwrap();
            let db_path = dir.path().join(format!("v{source_version}.db"));
            let conn = Connection::open(&db_path).unwrap();
            build_fixture(&conn);

            let note = migrate(&db_path, &conn).unwrap();

            assert!(note.contains(&format!("v{source_version} -> v5")), "{note}");
            assert_legacy_data_is_readable(&conn);
            assert_v5_schema(&conn);

            conn.execute(
                "INSERT INTO rules(id, name, kind, enabled, description)
                 VALUES('legacy-rule', 'legacy rule', 'validate', 1, '')",
                [],
            )
            .unwrap();
            let (template, params): (Option<String>, Option<String>) = conn
                .query_row(
                    "SELECT template, params FROM rules WHERE id='legacy-rule'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(template, None);
            assert_eq!(params, None);
        }
    }

    #[test]
    fn repeat_migration_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        build_v1_db(&conn);

        migrate(&db_path, &conn).unwrap();
        let note = migrate(&db_path, &conn).unwrap();

        assert_eq!(note, "schema_version=5 ok");
        assert_legacy_data_is_readable(&conn);
        assert_v5_schema(&conn);
    }

    #[test]
    fn failed_supported_migration_rolls_back_schema_and_version() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        build_v1_db(&conn);
        // Block v1->v2 by pre-creating an incompatible `rules` view. The very
        // first migration step fails, so this asserts rollback of a failed
        // first step. The test below covers rollback after a successful step.
        conn.execute_batch("CREATE VIEW rules AS SELECT 'placeholder' AS kind;")
            .unwrap();

        let error = migrate(&db_path, &conn).unwrap_err();

        assert!(matches!(error, DbError::Sqlite(_)));
        assert_eq!(read_schema_version(&conn).unwrap(), Some(V1));
        assert!(!table_exists(&conn, "rules"));
        assert_legacy_data_is_readable(&conn);
    }

    /// A failing step *after* an earlier successful step must roll back both the
    /// schema and the version changes introduced by the successful step.
    ///
    /// Setup: a v1 fixture plus a pre-created table named `idx_cells_sheet_col`.
    /// v1->v2 (rules table + `idx_rules_kind` index) succeeds, then v2->v3 fails
    /// because its `CREATE INDEX IF NOT EXISTS idx_cells_sheet_col ON cells(...)`
    /// statement hits SQLite's "there is already a table named
    /// idx_cells_sheet_col" error. The transaction rolls back; the v1 schema,
    /// version, and fixture data must all be restored.
    #[test]
    fn rollback_restores_prior_state_after_partial_success() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        build_v1_db(&conn);
        // Pre-create a table whose name collides with the index that v2->v3
        // tries to create. v1->v2 does not touch this name, so it succeeds
        // inside the migration transaction; v2->v3 then fails.
        conn.execute_batch("CREATE TABLE idx_cells_sheet_col(x INTEGER);")
            .unwrap();
        // Snapshot the v1 state we expect to be restored after rollback.
        assert_eq!(read_schema_version(&conn).unwrap(), Some(V1));
        assert!(!table_exists(&conn, "rules"));

        let error = migrate(&db_path, &conn).unwrap_err();

        // v2->v3 fails on the name collision; the whole transaction rolls back.
        assert!(matches!(error, DbError::Sqlite(_)));

        // Version restored to v1 (not the intermediate v2 the successful step wrote).
        assert_eq!(read_schema_version(&conn).unwrap(), Some(V1));
        // The rules table added by v1->v2 is rolled back (absent).
        assert!(!table_exists(&conn, "rules"));
        // The pre-existing collision table is still there (created outside the
        // migration transaction).
        assert!(table_exists(&conn, "idx_cells_sheet_col"));
        // Original fixture data fully restored.
        assert_legacy_data_is_readable(&conn);
    }

    #[test]
    fn future_schema_is_backed_up_and_left_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ruT0datakit.db");
        let conn = Connection::open(&db_path).unwrap();
        build_v1_db(&conn);
        set_fixture_version(&conn, SCHEMA_VERSION + 1);

        let error = migrate(&db_path, &conn).unwrap_err();

        assert!(matches!(error, DbError::Migration(_)));
        assert_eq!(
            read_schema_version(&conn).unwrap(),
            Some(SCHEMA_VERSION + 1)
        );
        assert!(!table_exists(&conn, "rules"));
        assert_legacy_data_is_readable(&conn);

        let backups: Vec<PathBuf> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("ruT0datakit.db.bak."))
            })
            .collect();
        assert_eq!(backups.len(), 1);
        let backup = Connection::open(&backups[0]).unwrap();
        assert_eq!(
            read_schema_version(&backup).unwrap(),
            Some(SCHEMA_VERSION + 1)
        );
        assert_legacy_data_is_readable(&backup);
    }
}
