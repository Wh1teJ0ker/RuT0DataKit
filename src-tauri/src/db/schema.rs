//! SQLite schema DDL。
//!
//! v1.1.3: `rules` 表新增 `template TEXT` 列存通用模板脱敏参数（JSON），
//! `SCHEMA_VERSION=4`。
//! v1.1.1: 6 表 + 5 索引（`operations` 表新增 `before_snapshot_json` 列存撤销前置快照，
//! 新增 `idx_cells_sheet_col` 复合索引供搜索加速），`SCHEMA_VERSION=3`。
//! v1.1.0: 6 表 + 4 索引（新增 `rules` 表 + `idx_rules_kind` 索引）。
//! v1.0.0: 5 表 + 3 索引，严格对齐 `docs/02-技术设计文档.md` §3。
//! 全部用 `IF NOT EXISTS`，保证重复启动幂等。

/// 当前 schema 版本。v1.1.3 起为 `4`（`rules` 表加 `template TEXT` 列存
/// 通用模板脱敏参数 JSON）。
pub const SCHEMA_VERSION: i64 = 4;

/// 6 表 + 5 索引 DDL。`CREATE ... IF NOT EXISTS` 幂等。
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
    before_snapshot_json TEXT,
    result_snapshot_json TEXT,
    created_at           TEXT    NOT NULL
);

-- app_settings：键值设置
CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT
);

-- rules：规则定义（v1.1.0 新增，持久化脱敏/校验/提取规则）
-- v1.1.3：新增 `template TEXT` 列存通用模板脱敏参数（TemplateParams JSON）
CREATE TABLE IF NOT EXISTS rules (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL,
    field       TEXT,
    pattern     TEXT,
    replacement TEXT,
    template    TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1,
    description TEXT NOT NULL DEFAULT ''
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_cells_sheet_row ON cells(sheet_id, row_idx);
CREATE INDEX IF NOT EXISTS idx_cells_sheet_col ON cells(sheet_id, col_idx);
CREATE INDEX IF NOT EXISTS idx_operations_sheet ON operations(sheet_id);
CREATE INDEX IF NOT EXISTS idx_sheets_session ON sheets(session_id);
CREATE INDEX IF NOT EXISTS idx_rules_kind ON rules(kind);
"#;
