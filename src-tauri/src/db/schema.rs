//! SQLite schema DDL。
//!
//! v1.0.0: 5 表 + 3 索引，严格对齐 `docs/02-技术设计文档.md` §3。
//! 全部用 `IF NOT EXISTS`，保证重复启动幂等。

/// 当前 schema 版本。v1.0.0 固定 `1`。
pub const SCHEMA_VERSION: i64 = 1;

/// 5 表 + 3 索引 DDL。`CREATE ... IF NOT EXISTS` 幂等。
pub const SCHEMA_DDL: &str = r#"
-- sessions：导入会话
CREATE TABLE IF NOT EXISTS sessions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    source_path TEXT,
    source_type TEXT    NOT NULL,
    row_count   INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

-- sheets：Tab / 表格
CREATE TABLE IF NOT EXISTS sheets (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id        INTEGER NOT NULL REFERENCES sessions(id),
    name              TEXT    NOT NULL,
    position          INTEGER NOT NULL DEFAULT 0,
    column_order      TEXT,
    column_visibility TEXT,
    created_at        TEXT    NOT NULL
);

-- cells：全量 cell 持久
CREATE TABLE IF NOT EXISTS cells (
    sheet_id INTEGER NOT NULL REFERENCES sheets(id),
    row_idx  INTEGER NOT NULL,
    col_idx  INTEGER NOT NULL,
    value    TEXT,
    PRIMARY KEY (sheet_id, row_idx, col_idx)
);

-- operations：操作日志
CREATE TABLE IF NOT EXISTS operations (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    sheet_id             INTEGER REFERENCES sheets(id),
    kind                 TEXT    NOT NULL,
    params_json          TEXT,
    result_snapshot_json TEXT,
    created_at           TEXT    NOT NULL
);

-- app_settings：键值设置
CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_cells_sheet_row ON cells(sheet_id, row_idx);
CREATE INDEX IF NOT EXISTS idx_operations_sheet ON operations(sheet_id);
CREATE INDEX IF NOT EXISTS idx_sheets_session ON sheets(session_id);
"#;
