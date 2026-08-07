//! SQLite schema DDL。
//!
//! v1.1.0: 6 表 + 4 索引（新增 `rules` 表 + `idx_rules_kind` 索引）。
//! v1.0.0: 5 表 + 3 索引，严格对齐 `docs/02-技术设计文档.md` §3。
//! 全部用 `IF NOT EXISTS`，保证重复启动幂等。

/// 当前 schema 版本。v1.1.0 起为 `2`（新增 `rules` 表）。
pub const SCHEMA_VERSION: i64 = 2;

/// 6 表 + 4 索引 DDL。`CREATE ... IF NOT EXISTS` 幂等。
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

-- rules：规则定义（v1.1.0 新增，持久化脱敏/校验/提取规则）
CREATE TABLE IF NOT EXISTS rules (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL,
    field       TEXT,
    pattern     TEXT,
    replacement TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1,
    description TEXT NOT NULL DEFAULT ''
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_cells_sheet_row ON cells(sheet_id, row_idx);
CREATE INDEX IF NOT EXISTS idx_operations_sheet ON operations(sheet_id);
CREATE INDEX IF NOT EXISTS idx_sheets_session ON sheets(session_id);
CREATE INDEX IF NOT EXISTS idx_rules_kind ON rules(kind);
"#;
