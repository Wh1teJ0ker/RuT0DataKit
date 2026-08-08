//! SQLite 持久层入口。
//!
//! v1.1.1: `SCHEMA_VERSION=3`，`operations` 表新增 `before_snapshot_json` 列
//! 存撤销前置快照；新增 `idx_cells_sheet_col` 复合索引供搜索加速；`DbManager`
//! 新增 7 个方法（列范围查询 / 关键字搜索 / 正则搜索 / 计数 / 列内替换 /
//! 全表替换 / 按 id 查 operation），全部参数化 SQL，供 T30/T31/T32 命令层调用。
//!
//! v1.1.0: 新增 `rules` 表 CRUD（`upsert_rule`/`list_rules`/`get_rule`/
//! `set_rule_enabled`/`count_rules`/`update_rule_params`/`seed_builtin_rules`），
//! 6 表 + 4 索引，`SCHEMA_VERSION=2`。
//!
//! v1.0.0: `DbManager` 持有 `Mutex<Connection>`，提供分页查询、批量写 cells、
//! 操作日志记录、键值设置、会话读写等公共方法，供命令层（T5/T7）调用。
//!
//! DB 文件位于 `app_config_dir/ruT0datakit.db`，schema 由 `schema::SCHEMA_DDL`
//! 定义，迁移由 `migrate::migrate` 处理。

pub mod error;
pub mod migrate;
pub mod schema;

pub use error::DbError;

use std::path::Path;
use std::sync::Mutex;

use regex::Regex;
use ruT0_data_kit_core::processor::rules::{Rule, RuleKind, RuleRegistry};
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

/// `operations` 表一行（撤销/重做用）。
///
/// `before_snapshot_json` / `result_snapshot_json` 存撤销/重做所需的 cell 快照
/// JSON（由命令层序列化）；DB 层只做透明存储。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRow {
    pub id: i64,
    pub sheet_id: Option<i64>,
    pub kind: String,
    pub params_json: Option<String>,
    pub before_snapshot_json: Option<String>,
    pub result_snapshot_json: Option<String>,
    pub created_at: String,
}

/// 正则搜索结果：每条命中 cell + 该 cell 内所有匹配区间 `(start, end)`
/// （字节偏移，`end` 为 exclusive 结束位置）。
pub type RegexSearchResult = Vec<(Cell, Vec<(usize, usize)>)>;

/// 可撤销操作摘要（撤销/重做列表用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoableOpRow {
    pub id: i64,
    pub kind: String,
    pub created_at: String,
}

/// `sessions` 摘要（列表用）。
// v1.1+ IPC 将调用；单测已覆盖。
#[allow(
    dead_code,
    reason = "v1.1+ IPC 将接入（list_sessions/get_session 等）；单测已覆盖"
)]
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
#[allow(
    dead_code,
    reason = "v1.1+ IPC 将接入（list_sessions/get_session 等）；单测已覆盖"
)]
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
#[allow(
    dead_code,
    reason = "v1.1+ IPC 将接入（list_sessions/get_session 等）；单测已覆盖"
)]
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

/// 把 DB 行（id/name/kind/field/pattern/replacement/enabled/description）映射为 `Rule`。
fn row_to_rule(r: &rusqlite::Row<'_>) -> rusqlite::Result<Rule> {
    let kind_str: String = r.get::<_, String>(2)?;
    let kind = RuleKind::from_str_lowercase(&kind_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown rule kind: {}", kind_str),
            )),
        )
    })?;
    let enabled: i64 = r.get::<_, i64>(6)?;
    Ok(Rule {
        id: r.get::<_, String>(0)?,
        name: r.get::<_, String>(1)?,
        kind,
        field: r.get::<_, Option<String>>(3)?,
        pattern: r.get::<_, Option<String>>(4)?,
        replacement: r.get::<_, Option<String>>(5)?,
        enabled: enabled != 0,
        description: r.get::<_, String>(7)?,
    })
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
#[allow(
    dead_code,
    reason = "v1.1+ IPC 将接入（list_sessions/get_session 等）；单测已覆盖"
)]
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
    pub fn create_sheet(&self, session_id: i64, name: &str, position: i32) -> Result<i64, DbError> {
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
        self.log_operation_with_snapshot(sheet_id, kind, params_json, None, result_json)
    }

    /// 记录操作日志（带撤销前置快照），返回 `id`。
    ///
    /// `before_snapshot_json` 为撤销所需的前置 cell 快照 JSON（由命令层序列化），
    /// `None` 表示该操作不可撤销。`result_json` 为操作结果快照（重做用）。
    pub fn log_operation_with_snapshot(
        &self,
        sheet_id: Option<i64>,
        kind: &str,
        params_json: &str,
        before_snapshot_json: Option<&str>,
        result_json: &str,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO operations (sheet_id, kind, params_json, before_snapshot_json, result_snapshot_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![sheet_id, kind, params_json, before_snapshot_json, result_json, now],
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

    /// 查询某列全部数据行（排除 `row_idx=0` 表头行），按 `row_idx` 升序。
    /// 返回 `(row_idx, value)` 列表，供 processor 命令读取列值。
    pub fn query_column_cells(
        &self,
        sheet_id: i64,
        col_idx: u32,
    ) -> Result<Vec<(u32, Option<String>)>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT row_idx, value FROM cells
             WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
             ORDER BY row_idx ASC",
        )?;
        let rows = stmt.query_map(params![sheet_id, col_idx as i64], |r| {
            Ok((r.get::<_, i64>(0)? as u32, r.get::<_, Option<String>>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 按表头名查找 `col_idx`（查 `row_idx=0` 的 cell value）。
    /// 不存在返回 `Ok(None)`。
    pub fn find_col_idx(&self, sheet_id: i64, header_name: &str) -> Result<Option<u32>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let col_idx: Option<i64> = conn
            .query_row(
                "SELECT col_idx FROM cells
                 WHERE sheet_id = ?1 AND row_idx = 0 AND value = ?2",
                params![sheet_id, header_name],
                |r| r.get::<_, i64>(0),
            )
            .ok();
        Ok(col_idx.map(|c| c as u32))
    }

    // ---- Rule CRUD（v1.1.0）----

    /// upsert 一条规则（按 id 冲突覆盖）。
    pub fn upsert_rule(&self, rule: &Rule) -> Result<(), DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            "INSERT INTO rules (id, name, kind, field, pattern, replacement, enabled, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name,
                kind=excluded.kind,
                field=excluded.field,
                pattern=excluded.pattern,
                replacement=excluded.replacement,
                enabled=excluded.enabled,
                description=excluded.description",
            params![
                rule.id,
                rule.name,
                rule.kind.to_string(),
                rule.field,
                rule.pattern,
                rule.replacement,
                rule.enabled as i64,
                rule.description,
            ],
        )?;
        Ok(())
    }

    /// 列出全部规则（按 id 升序）。
    pub fn list_rules(&self) -> Result<Vec<Rule>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, field, pattern, replacement, enabled, description
             FROM rules
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], row_to_rule)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 按 id 查找规则。
    pub fn get_rule(&self, id: &str) -> Result<Option<Rule>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let rule = conn
            .query_row(
                "SELECT id, name, kind, field, pattern, replacement, enabled, description
                 FROM rules
                 WHERE id = ?1",
                params![id],
                row_to_rule,
            )
            .ok();
        Ok(rule)
    }

    /// 切换规则启用状态。不存在返回 `Ok(false)`。
    pub fn set_rule_enabled(&self, id: &str, enabled: bool) -> Result<bool, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let affected = conn.execute(
            "UPDATE rules SET enabled = ?1 WHERE id = ?2",
            params![enabled as i64, id],
        )?;
        Ok(affected > 0)
    }

    /// 统计规则数。
    pub fn count_rules(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM rules", [], |r| r.get(0))?;
        Ok(count)
    }

    /// 更新规则可填参数（pattern / replacement）。`None` 表示该字段保持不变。
    /// 返回是否命中（id 存在）。
    pub fn update_rule_params(
        &self,
        id: &str,
        pattern: Option<&str>,
        replacement: Option<&str>,
    ) -> Result<bool, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut touched = false;
        if let Some(p) = pattern {
            let n = conn.execute(
                "UPDATE rules SET pattern = ?1 WHERE id = ?2",
                params![p, id],
            )?;
            touched |= n > 0;
        }
        if let Some(r) = replacement {
            let n = conn.execute(
                "UPDATE rules SET replacement = ?1 WHERE id = ?2",
                params![r, id],
            )?;
            touched |= n > 0;
        }
        if !touched {
            // 无字段要更新 → 仅返回存在性。
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM rules WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )?;
            return Ok(count > 0);
        }
        Ok(touched)
    }

    /// 启动时若 DB 无规则，则 seed 三条内置姓名规则。
    /// 已有规则则不动（保留用户参数修改）。
    pub fn seed_builtin_rules(&self) -> Result<(), DbError> {
        if self.count_rules()? > 0 {
            return Ok(());
        }
        // 用 core 的 RuleRegistry::with_defaults() 拿到 3 条内置规则。
        let reg = RuleRegistry::with_defaults();
        for rule in reg.list() {
            self.upsert_rule(rule)?;
        }
        Ok(())
    }

    // ---- 搜索 / 替换 / 操作日志查询（v1.1.1）----

    /// 查询某列指定 row_idx 范围的 cells（搜索分页用，排除 `row_idx=0` 表头）。
    ///
    /// 按 `row_idx` 升序返回，分页用 `OFFSET`/`LIMIT`。`offset`/`limit` 为 0
    /// 时分别视为 0/0（返回空）。
    pub fn query_column_cells_with_row_idx_range(
        &self,
        sheet_id: i64,
        col_idx: u32,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Cell>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT sheet_id, row_idx, col_idx, value FROM cells
             WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
             ORDER BY row_idx ASC
             LIMIT ?3 OFFSET ?4",
        )?;
        let rows = stmt.query_map(
            params![sheet_id, col_idx as i64, limit as i64, offset as i64],
            |r| {
                Ok(Cell {
                    sheet_id: r.get::<_, i64>(0)?,
                    row_idx: r.get::<_, i64>(1)? as u32,
                    col_idx: r.get::<_, i64>(2)? as u32,
                    value: r.get::<_, Option<String>>(3)?,
                })
            },
        )?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 关键字搜索（`LIKE '%kw%'` + `ESCAPE '\'`）。`col_idx=None` 搜全表所有列。
    /// 返回命中 cells（按 `row_idx`、`col_idx` 升序，排除 `row_idx=0` 表头）。
    ///
    /// `keyword` 中的 `%`/`_`/`\` 会被转义为字面字符，避免改变 LIKE 语义。
    pub fn search_cells(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        keyword: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Cell>, DbError> {
        let pattern = format!("%{}%", escape_like(keyword));
        let conn = self.conn.lock().expect("db mutex poisoned");
        let map_cell = |r: &rusqlite::Row<'_>| -> rusqlite::Result<Cell> {
            Ok(Cell {
                sheet_id: r.get::<_, i64>(0)?,
                row_idx: r.get::<_, i64>(1)? as u32,
                col_idx: r.get::<_, i64>(2)? as u32,
                value: r.get::<_, Option<String>>(3)?,
            })
        };
        let mut out = Vec::new();
        if let Some(c) = col_idx {
            // col_idx 限定列搜索。SQL 固定，仅参数绑定。
            let mut stmt = conn.prepare(
                "SELECT sheet_id, row_idx, col_idx, value FROM cells
                 WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                   AND value LIKE ?3 ESCAPE '\\'
                 ORDER BY row_idx ASC, col_idx ASC
                 LIMIT ?4 OFFSET ?5",
            )?;
            let rows = stmt.query_map(
                params![sheet_id, c as i64, pattern, limit as i64, offset as i64],
                map_cell,
            )?;
            for row in rows {
                out.push(row?);
            }
        } else {
            // 全表所有列搜索（不带 col_idx 条件）。
            let mut stmt = conn.prepare(
                "SELECT sheet_id, row_idx, col_idx, value FROM cells
                 WHERE sheet_id = ?1 AND row_idx > 0
                   AND value LIKE ?2 ESCAPE '\\'
                 ORDER BY row_idx ASC, col_idx ASC
                 LIMIT ?3 OFFSET ?4",
            )?;
            let rows = stmt.query_map(
                params![sheet_id, pattern, limit as i64, offset as i64],
                map_cell,
            )?;
            for row in rows {
                out.push(row?);
            }
        }
        Ok(out)
    }

    /// 正则搜索：SQL `LIKE` 预筛（用整个 pattern 做粗筛，`%`/`_`/`\` 转义）
    /// + Rust `regex` 精确匹配。返回命中 cells + 每条命中的所有匹配区间
    ///   `(start, end)`（按字节偏移；`end` 是 exclusive 结束位置）。
    ///
    /// 预筛保证不漏（宁可多筛再由 regex 过滤）；`pattern` 编译失败返回 `Err`，不 panic。
    /// 分页在 Rust 侧做（regex 过滤后再切片），`offset`/`limit` 作用于最终命中结果。
    pub fn search_cells_regex(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        pattern: &str,
        offset: u32,
        limit: u32,
    ) -> Result<RegexSearchResult, DbError> {
        let re = Regex::new(pattern)
            .map_err(|e| DbError::Migration(format!("invalid regex `{}`: {}", pattern, e)))?;
        // LIKE 预筛：不使用 pattern 字面粗筛（regex 元字符如 \d 不会匹配字面值），
        // 而是用 `%` 匹配所有非空 value 行，再由 Rust regex 精确过滤。
        // 这样保证不漏；命中量受 sheet 数据量限制，由 regex 二次精确匹配。
        let like_pattern = "%".to_string();
        let conn = self.conn.lock().expect("db mutex poisoned");
        let map_cell = |r: &rusqlite::Row<'_>| -> rusqlite::Result<Cell> {
            Ok(Cell {
                sheet_id: r.get::<_, i64>(0)?,
                row_idx: r.get::<_, i64>(1)? as u32,
                col_idx: r.get::<_, i64>(2)? as u32,
                value: r.get::<_, Option<String>>(3)?,
            })
        };
        let mut out: RegexSearchResult = Vec::new();
        if let Some(c) = col_idx {
            let mut stmt = conn.prepare(
                "SELECT sheet_id, row_idx, col_idx, value FROM cells
                 WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                   AND value LIKE ?3 ESCAPE '\\'
                 ORDER BY row_idx ASC, col_idx ASC",
            )?;
            let rows = stmt.query_map(params![sheet_id, c as i64, like_pattern], map_cell)?;
            for row in rows {
                let cell = row?;
                if let Some(ref val) = cell.value {
                    let spans: Vec<(usize, usize)> =
                        re.find_iter(val).map(|m| (m.start(), m.end())).collect();
                    if !spans.is_empty() {
                        out.push((cell, spans));
                    }
                }
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT sheet_id, row_idx, col_idx, value FROM cells
                 WHERE sheet_id = ?1 AND row_idx > 0
                   AND value LIKE ?2 ESCAPE '\\'
                 ORDER BY row_idx ASC, col_idx ASC",
            )?;
            let rows = stmt.query_map(params![sheet_id, like_pattern], map_cell)?;
            for row in rows {
                let cell = row?;
                if let Some(ref val) = cell.value {
                    let spans: Vec<(usize, usize)> =
                        re.find_iter(val).map(|m| (m.start(), m.end())).collect();
                    if !spans.is_empty() {
                        out.push((cell, spans));
                    }
                }
            }
        }
        // 在 Rust 侧分页（LIKE 预筛结果可能大于 limit，regex 过滤后再切片）。
        let start = (offset as usize).min(out.len());
        let end = (start + limit as usize).min(out.len());
        Ok(out[start..end].to_vec())
    }

    /// 统计搜索结果总数（分页 total）。语义与 `search_cells` 一致：
    /// `LIKE '%kw%'` + `ESCAPE '\'`，`col_idx=None` 搜全表所有列。
    pub fn count_search_results(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        keyword: &str,
    ) -> Result<u32, DbError> {
        let pattern = format!("%{}%", escape_like(keyword));
        let conn = self.conn.lock().expect("db mutex poisoned");
        let count: i64 = if let Some(c) = col_idx {
            conn.query_row(
                "SELECT COUNT(*) FROM cells
                 WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                   AND value LIKE ?3 ESCAPE '\\'",
                params![sheet_id, c as i64, pattern],
                |r| r.get(0),
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM cells
                 WHERE sheet_id = ?1 AND row_idx > 0
                   AND value LIKE ?2 ESCAPE '\\'",
                params![sheet_id, pattern],
                |r| r.get(0),
            )?
        };
        Ok(count.max(0) as u32)
    }

    /// 取某 sheet 的列数（`row_idx=0` 表头行的 cell 数）。供搜索结果行级展开对齐用。
    pub fn count_columns(&self, sheet_id: i64) -> Result<u32, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM cells WHERE sheet_id = ?1 AND row_idx = 0",
            params![sheet_id],
            |r| r.get(0),
        )?;
        Ok(count.max(0) as u32)
    }

    /// 取某行所有列的 cells（按 `col_idx` 升序）。供搜索结果行级展开用。
    pub fn query_row_cells(&self, sheet_id: i64, row_idx: u32) -> Result<Vec<Cell>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT sheet_id, row_idx, col_idx, value FROM cells
             WHERE sheet_id = ?1 AND row_idx = ?2
             ORDER BY col_idx ASC",
        )?;
        let rows = stmt.query_map(params![sheet_id, row_idx as i64], |r| {
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

    /// 关键字行级搜索：返回有命中 cell 的 **去重 row_idx**（按 row_idx 升序），
    /// 服务端分页（`LIMIT/OFFSET` 作用于 distinct row_idx）。
    ///
    /// 与 `search_cells` 的区别：后者按 cell 分页（一行多列命中各占一条），
    /// 前端需自行拼行；本方法按行分页，配合 `query_row_cells` 取整行数据，
    /// 让前端搜索结果直接以「行」为单位渲染。
    pub fn search_matched_row_ids(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        keyword: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<u32>, DbError> {
        let pattern = format!("%{}%", escape_like(keyword));
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut out = Vec::new();
        if let Some(c) = col_idx {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT row_idx FROM cells
                 WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                   AND value LIKE ?3 ESCAPE '\\'
                 ORDER BY row_idx ASC
                 LIMIT ?4 OFFSET ?5",
            )?;
            let rows = stmt.query_map(
                params![sheet_id, c as i64, pattern, limit as i64, offset as i64],
                |r| r.get::<_, i64>(0).map(|v| v as u32),
            )?;
            for row in rows {
                out.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT row_idx FROM cells
                 WHERE sheet_id = ?1 AND row_idx > 0
                   AND value LIKE ?2 ESCAPE '\\'
                 ORDER BY row_idx ASC
                 LIMIT ?3 OFFSET ?4",
            )?;
            let rows = stmt.query_map(
                params![sheet_id, pattern, limit as i64, offset as i64],
                |r| r.get::<_, i64>(0).map(|v| v as u32),
            )?;
            for row in rows {
                out.push(row?);
            }
        }
        Ok(out)
    }

    /// 统计关键字搜索命中的 **行数**（`COUNT(DISTINCT row_idx)`），
    /// 与 `search_matched_row_ids` 语义一致，供分页 total。
    pub fn count_matched_rows(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        keyword: &str,
    ) -> Result<u32, DbError> {
        let pattern = format!("%{}%", escape_like(keyword));
        let conn = self.conn.lock().expect("db mutex poisoned");
        let count: i64 = if let Some(c) = col_idx {
            conn.query_row(
                "SELECT COUNT(DISTINCT row_idx) FROM cells
                 WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                   AND value LIKE ?3 ESCAPE '\\'",
                params![sheet_id, c as i64, pattern],
                |r| r.get(0),
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(DISTINCT row_idx) FROM cells
                 WHERE sheet_id = ?1 AND row_idx > 0
                   AND value LIKE ?2 ESCAPE '\\'",
                params![sheet_id, pattern],
                |r| r.get(0),
            )?
        };
        Ok(count.max(0) as u32)
    }

    /// 正则行级搜索：返回有命中 cell 的 **去重 row_idx**（按 row_idx 升序），
    /// 服务端分页（`LIMIT/OFFSET` 作用于 distinct row_idx）。
    /// 与 `search_matched_row_ids` 的区别：用 `regex::Regex` 二次精确匹配，
    /// 而非 `LIKE`。语义与 `search_cells_regex` 对齐。
    pub fn search_matched_row_ids_regex(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        pattern: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<u32>, DbError> {
        let re = Regex::new(pattern)
            .map_err(|e| DbError::Migration(format!("invalid regex `{}`: {}", pattern, e)))?;
        let conn = self.conn.lock().expect("db mutex poisoned");
        // 先取 distinct row_idx（按 row_idx 升序），再 regex 过滤，最后分页。
        let sql = if col_idx.is_some() {
            "SELECT DISTINCT row_idx FROM cells
             WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
               AND value LIKE ?3 ESCAPE '\\'
             ORDER BY row_idx ASC"
        } else {
            "SELECT DISTINCT row_idx FROM cells
             WHERE sheet_id = ?1 AND row_idx > 0
               AND value LIKE ?2 ESCAPE '\\'
             ORDER BY row_idx ASC"
        };
        let mut stmt = conn.prepare(sql)?;
        let like_pattern = "%".to_string();
        let rows: rusqlite::Result<Vec<u32>> = if let Some(c) = col_idx {
            let mapped = stmt.query_map(params![sheet_id, c as i64, like_pattern], |r| {
                r.get::<_, i64>(0).map(|v| v as u32)
            })?;
            let mut v = Vec::new();
            for row in mapped {
                v.push(row?);
            }
            Ok(v)
        } else {
            let mapped = stmt.query_map(params![sheet_id, like_pattern], |r| {
                r.get::<_, i64>(0).map(|v| v as u32)
            })?;
            let mut v = Vec::new();
            for row in mapped {
                v.push(row?);
            }
            Ok(v)
        };
        let mut all = rows?;
        // 取每个 row_idx 的 cell values 做 regex 过滤
        let mut filtered: Vec<u32> = Vec::new();
        for rid in all.drain(..) {
            let matched = if let Some(c) = col_idx {
                let val: Option<String> = conn.query_row(
                    "SELECT value FROM cells WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx = ?3",
                    params![sheet_id, c as i64, rid as i64],
                    |r| r.get(0),
                )?;
                val.map(|v| re.is_match(&v)).unwrap_or(false)
            } else {
                // 任意列命中即可
                let mut stmt2 = conn.prepare(
                    "SELECT value FROM cells
                     WHERE sheet_id = ?1 AND row_idx = ?2 AND value IS NOT NULL
                     ORDER BY col_idx ASC",
                )?;
                let mut hit = false;
                let vals = stmt2.query_map(params![sheet_id, rid as i64], |r| {
                    r.get::<_, Option<String>>(0)
                })?;
                for v in vals {
                    if let Some(ref s) = v? {
                        if re.is_match(s) {
                            hit = true;
                            break;
                        }
                    }
                }
                hit
            };
            if matched {
                filtered.push(rid);
            }
        }
        let start = (offset as usize).min(filtered.len());
        let end = (start + limit as usize).min(filtered.len());
        Ok(filtered[start..end].to_vec())
    }

    /// 统计正则搜索命中的 **行数**（`COUNT(DISTINCT row_idx)` 后由 regex 过滤）。
    pub fn count_matched_rows_regex(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        pattern: &str,
    ) -> Result<u32, DbError> {
        // 复用 search_matched_row_ids_regex 逻辑，取全量再数 length。
        // 性能：sheet 内行数有限（<1万），可接受；超大表需后续优化。
        let rows = self.search_matched_row_ids_regex(sheet_id, col_idx, pattern, 0, u32::MAX)?;
        Ok(rows.len() as u32)
    }

    /// 列内批量替换。返回 `(受影响行数, before 快照, after 快照)`。
    ///
    /// `use_regex=true` 时用 `regex::Regex` 替换 `from` → `to`（`from` 为正则
    /// pattern，编译失败返回 `Err`）；`false` 时用 `str::replace` 做字面替换。
    /// 仅返回有变化的行（before/after 一一对应），`before` 含原始值，`after`
    /// 含替换后值。所有写入在单事务内完成，保证原子性。
    pub fn replace_in_column_cells(
        &self,
        sheet_id: i64,
        col_idx: u32,
        from: &str,
        to: &str,
        use_regex: bool,
    ) -> Result<(u32, Vec<Cell>, Vec<Cell>), DbError> {
        self.replace_cells_inner(sheet_id, Some(col_idx), from, to, use_regex)
    }

    /// 全表替换（所有列）。返回 `(受影响行数, before 快照, after 快照)`。
    /// 语义同 `replace_in_column_cells`，但不限定 `col_idx`。
    pub fn replace_all_cells(
        &self,
        sheet_id: i64,
        from: &str,
        to: &str,
        use_regex: bool,
    ) -> Result<(u32, Vec<Cell>, Vec<Cell>), DbError> {
        self.replace_cells_inner(sheet_id, None, from, to, use_regex)
    }

    /// 按 id 查询单条 operation（撤销/重做用）。不存在返回 `Ok(None)`。
    pub fn query_operation_by_id(&self, op_id: i64) -> Result<Option<OperationRow>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let row = conn
            .query_row(
                "SELECT id, sheet_id, kind, params_json, before_snapshot_json,
                        result_snapshot_json, created_at
                 FROM operations
                 WHERE id = ?1",
                params![op_id],
                |r| {
                    Ok(OperationRow {
                        id: r.get::<_, i64>(0)?,
                        sheet_id: r.get::<_, Option<i64>>(1)?,
                        kind: r.get::<_, String>(2)?,
                        params_json: r.get::<_, Option<String>>(3)?,
                        before_snapshot_json: r.get::<_, Option<String>>(4)?,
                        result_snapshot_json: r.get::<_, Option<String>>(5)?,
                        created_at: r.get::<_, String>(6)?,
                    })
                },
            )
            .ok();
        Ok(row)
    }

    /// `replace_in_column_cells` / `replace_all_cells` 的共享实现。
    ///
    /// `col_filter`：`Some(c)` 限定单列，`None` 全表所有列。
    /// 单事务内：查 before 快照 → 计算替换值 → 筛有变化的行 → 批量 upsert
    /// after 值 → 返回 `(affected, before, after)`。
    fn replace_cells_inner(
        &self,
        sheet_id: i64,
        col_filter: Option<u32>,
        from: &str,
        to: &str,
        use_regex: bool,
    ) -> Result<(u32, Vec<Cell>, Vec<Cell>), DbError> {
        let re = if use_regex {
            Some(
                Regex::new(from)
                    .map_err(|e| DbError::Migration(format!("invalid regex `{}`: {}", from, e)))?,
            )
        } else {
            None
        };
        let mut conn = self.conn.lock().expect("db mutex poisoned");
        let tx = conn.transaction()?;
        // 抓 before 快照（限定列 / 全表，排除表头 row_idx=0）。
        let before: Vec<Cell> = {
            let map_cell = |r: &rusqlite::Row<'_>| -> rusqlite::Result<Cell> {
                Ok(Cell {
                    sheet_id: r.get::<_, i64>(0)?,
                    row_idx: r.get::<_, i64>(1)? as u32,
                    col_idx: r.get::<_, i64>(2)? as u32,
                    value: r.get::<_, Option<String>>(3)?,
                })
            };
            let mut out = Vec::new();
            if let Some(c) = col_filter {
                let mut stmt = tx.prepare(
                    "SELECT sheet_id, row_idx, col_idx, value FROM cells
                     WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                     ORDER BY row_idx ASC, col_idx ASC",
                )?;
                let rows = stmt.query_map(params![sheet_id, c as i64], map_cell)?;
                for r in rows {
                    out.push(r?);
                }
            } else {
                let mut stmt = tx.prepare(
                    "SELECT sheet_id, row_idx, col_idx, value FROM cells
                     WHERE sheet_id = ?1 AND row_idx > 0
                     ORDER BY row_idx ASC, col_idx ASC",
                )?;
                let rows = stmt.query_map(params![sheet_id], map_cell)?;
                for r in rows {
                    out.push(r?);
                }
            }
            out
        };
        // 计算替换值，筛有变化的行。
        let mut before_changed: Vec<Cell> = Vec::new();
        let mut after_changed: Vec<Cell> = Vec::new();
        for c in &before {
            if let Some(ref val) = c.value {
                let new_val = if let Some(ref re) = re {
                    re.replace_all(val, to).into_owned()
                } else {
                    val.replace(from, to)
                };
                if new_val != *val {
                    before_changed.push(c.clone());
                    after_changed.push(Cell {
                        sheet_id: c.sheet_id,
                        row_idx: c.row_idx,
                        col_idx: c.col_idx,
                        value: Some(new_val),
                    });
                }
            }
        }
        // 批量写回 after 值（同一事务）。
        if !after_changed.is_empty() {
            let mut stmt = tx.prepare(
                "INSERT INTO cells (sheet_id, row_idx, col_idx, value)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(sheet_id, row_idx, col_idx) DO UPDATE SET value=excluded.value",
            )?;
            for c in &after_changed {
                stmt.execute(params![
                    sheet_id,
                    c.row_idx as i64,
                    c.col_idx as i64,
                    c.value,
                ])?;
            }
        }
        tx.commit()?;
        let affected = after_changed.len() as u32;
        Ok((affected, before_changed, after_changed))
    }

    /// 列出某 sheet 的可撤销操作（最近 `limit` 条，按 `created_at` DESC）。
    ///
    /// 仅返回 kind ∈ {`mask`, `replace_in_column`, `replace_all`} 的操作——
    /// 即「就地变更」类操作；`import`/`validate`/`extract`/`undo`/`redo` 等
    /// 只读或辅助操作不入撤销栈。`kind` 值是硬编码常量（非用户输入），
    /// IN 子句无注入风险；`sheet_id`/`limit` 仍用 `?N` + `params![]` 绑定。
    pub fn list_undoable_operations(
        &self,
        sheet_id: i64,
        limit: u32,
    ) -> Result<Vec<UndoableOpRow>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, kind, created_at FROM operations
             WHERE sheet_id = ?1 AND kind IN ('mask', 'replace_in_column', 'replace_all')
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![sheet_id, limit as i64], |r| {
            Ok(UndoableOpRow {
                id: r.get::<_, i64>(0)?,
                kind: r.get::<_, String>(1)?,
                created_at: r.get::<_, String>(2)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

/// `LIKE` 模式转义：把 `\` → `\\`，`%` → `\%`，`_` → `\_`，
/// 配合 `ESCAPE '\'` 子句让 keyword 中的特殊字符按字面匹配。
fn escape_like(keyword: &str) -> String {
    let mut out = String::with_capacity(keyword.len());
    for ch in keyword.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
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
        // v1.1.1: schema_version=3
        assert_eq!(
            mgr.get_setting("schema_version").unwrap().as_deref(),
            Some("3")
        );
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
        mgr.write_cells(shid, std::slice::from_ref(&c)).unwrap();
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
        let id = mgr.log_operation(Some(shid), "import", "{}", "{}").unwrap();
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

    #[test]
    fn find_col_idx_and_query_column_cells() {
        let (_dir, mgr) = open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        // 3 列 × 3 行（row_idx=0 表头 + 2 数据行）
        let cells: Vec<Cell> = vec![
            Cell {
                sheet_id: shid,
                row_idx: 0,
                col_idx: 0,
                value: Some("name".into()),
            },
            Cell {
                sheet_id: shid,
                row_idx: 0,
                col_idx: 1,
                value: Some("phone".into()),
            },
            Cell {
                sheet_id: shid,
                row_idx: 0,
                col_idx: 2,
                value: Some("email".into()),
            },
            Cell {
                sheet_id: shid,
                row_idx: 1,
                col_idx: 0,
                value: Some("张三".into()),
            },
            Cell {
                sheet_id: shid,
                row_idx: 1,
                col_idx: 1,
                value: Some("13812345678".into()),
            },
            Cell {
                sheet_id: shid,
                row_idx: 2,
                col_idx: 0,
                value: Some("李四".into()),
            },
            Cell {
                sheet_id: shid,
                row_idx: 2,
                col_idx: 1,
                value: Some("13987654321".into()),
            },
        ];
        mgr.write_cells(shid, &cells).unwrap();

        // find_col_idx
        assert_eq!(mgr.find_col_idx(shid, "name").unwrap(), Some(0));
        assert_eq!(mgr.find_col_idx(shid, "phone").unwrap(), Some(1));
        assert_eq!(mgr.find_col_idx(shid, "nope").unwrap(), None);

        // query_column_cells：排除表头，返回 2 行
        let col0 = mgr.query_column_cells(shid, 0).unwrap();
        assert_eq!(col0.len(), 2);
        assert_eq!(col0[0], (1, Some("张三".into())));
        assert_eq!(col0[1], (2, Some("李四".into())));

        // col_idx=2 无数据行 → 空
        let col2 = mgr.query_column_cells(shid, 2).unwrap();
        assert!(col2.is_empty());
    }

    // ---- Rule CRUD（v1.1.0）----

    #[test]
    fn rule_crud_upsert_list_get() {
        let (_dir, mgr) = open();
        assert_eq!(mgr.count_rules().unwrap(), 0);
        let r = RuleRegistry::name_validate_rule();
        mgr.upsert_rule(&r).unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 1);
        // list
        let list = mgr.list_rules().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "name-validate");
        assert_eq!(list[0].kind, RuleKind::Validate);
        assert!(list[0].enabled);
        // get
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.name, "姓名校验");
        assert!(mgr.get_rule("nope").unwrap().is_none());
    }

    #[test]
    fn rule_crud_upsert_overwrites_same_id() {
        let (_dir, mgr) = open();
        let mut r = RuleRegistry::name_validate_rule();
        mgr.upsert_rule(&r).unwrap();
        // 修改 pattern 后再 upsert → 覆盖
        r.pattern = Some(r"^[\u4e00-\u9fa5]{2,8}$".into());
        mgr.upsert_rule(&r).unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 1);
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.pattern.as_deref(), Some(r"^[\u4e00-\u9fa5]{2,8}$"));
    }

    #[test]
    fn rule_crud_toggle_enabled() {
        let (_dir, mgr) = open();
        mgr.upsert_rule(&RuleRegistry::name_validate_rule())
            .unwrap();
        assert!(mgr.get_rule("name-validate").unwrap().unwrap().enabled);
        assert!(mgr.set_rule_enabled("name-validate", false).unwrap());
        assert!(!mgr.get_rule("name-validate").unwrap().unwrap().enabled);
        assert!(!mgr.set_rule_enabled("nope", true).unwrap());
    }

    #[test]
    fn rule_crud_update_params() {
        let (_dir, mgr) = open();
        mgr.upsert_rule(&RuleRegistry::name_validate_rule())
            .unwrap();
        // 更新 pattern
        assert!(mgr
            .update_rule_params("name-validate", Some(r"^[\u4e00-\u9fa5]{2,8}$"), None)
            .unwrap());
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.pattern.as_deref(), Some(r"^[\u4e00-\u9fa5]{2,8}$"));
        // 更新 replacement（mask 规则）
        mgr.upsert_rule(&RuleRegistry::name_mask_rule()).unwrap();
        assert!(mgr
            .update_rule_params("name-mask", None, Some("***"))
            .unwrap());
        let got2 = mgr.get_rule("name-mask").unwrap().unwrap();
        assert_eq!(got2.replacement.as_deref(), Some("***"));
        // 无字段更新 → 仅返回存在性
        assert!(mgr.update_rule_params("name-mask", None, None).unwrap());
        assert!(!mgr.update_rule_params("nope", None, None).unwrap());
    }

    #[test]
    fn seed_builtin_rules_inserts_three_when_empty() {
        let (_dir, mgr) = open();
        assert_eq!(mgr.count_rules().unwrap(), 0);
        mgr.seed_builtin_rules().unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 3);
        let kinds: Vec<RuleKind> = mgr.list_rules().unwrap().iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RuleKind::Mask));
        assert!(kinds.contains(&RuleKind::Validate));
        assert!(kinds.contains(&RuleKind::Extract));
        // 再次 seed 不重复插入
        mgr.seed_builtin_rules().unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 3);
    }

    #[test]
    fn seed_builtin_rules_preserves_user_param_modifications() {
        let (_dir, mgr) = open();
        mgr.seed_builtin_rules().unwrap();
        // 用户修改了 pattern
        mgr.update_rule_params("name-validate", Some(r"^[\u4e00-\u9fa5]{2,8}$"), None)
            .unwrap();
        // 再次 seed 不应覆盖用户修改
        mgr.seed_builtin_rules().unwrap();
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.pattern.as_deref(), Some(r"^[\u4e00-\u9fa5]{2,8}$"));
    }

    #[test]
    fn rule_kind_round_trip_through_db() {
        let (_dir, mgr) = open();
        for r in RuleRegistry::with_defaults().list() {
            mgr.upsert_rule(r).unwrap();
        }
        let list = mgr.list_rules().unwrap();
        // 确保三种 kind 都能正确反序列化
        assert_eq!(list.len(), 3);
        for r in &list {
            // 确认 kind 字符串化 + 反序列化闭环
            let s = r.kind.to_string();
            assert_eq!(RuleKind::from_str_lowercase(&s), Some(r.kind));
        }
    }

    // ---- 搜索 / 替换 / 操作日志查询（v1.1.1）----

    /// 构造一个 3 列 × 4 行的 sheet（row_idx=0 表头）。
    /// col0=name, col1=phone, col2=memo。
    fn build_search_sheet(mgr: &DbManager) -> i64 {
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        let cells: Vec<Cell> = vec![
            cell(shid, 0, 0, "name"),
            cell(shid, 0, 1, "phone"),
            cell(shid, 0, 2, "memo"),
            cell(shid, 1, 0, "张三"),
            cell(shid, 1, 1, "13812345678"),
            cell(shid, 1, 2, "vip"),
            cell(shid, 2, 0, "李四"),
            cell(shid, 2, 1, "13987654321"),
            cell(shid, 2, 2, "普通"),
            cell(shid, 3, 0, "张五"),
            cell(shid, 3, 1, "13700000000"),
            cell(shid, 3, 2, "50%"),
        ];
        mgr.write_cells(shid, &cells).unwrap();
        shid
    }

    fn cell(shid: i64, row: u32, col: u32, val: &str) -> Cell {
        Cell {
            sheet_id: shid,
            row_idx: row,
            col_idx: col,
            value: Some(val.into()),
        }
    }

    #[test]
    fn query_column_cells_with_row_idx_range_paged() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // col=0 数据行 row_idx=1,2,3 → offset=0 limit=2 取前 2 行。
        let p1 = mgr
            .query_column_cells_with_row_idx_range(shid, 0, 0, 2)
            .unwrap();
        assert_eq!(p1.len(), 2);
        assert_eq!(p1[0].row_idx, 1);
        assert_eq!(p1[0].value.as_deref(), Some("张三"));
        assert_eq!(p1[1].row_idx, 2);
        // offset=2 取剩余 1 行。
        let p2 = mgr
            .query_column_cells_with_row_idx_range(shid, 0, 2, 2)
            .unwrap();
        assert_eq!(p2.len(), 1);
        assert_eq!(p2[0].row_idx, 3);
    }

    #[test]
    fn search_cells_keyword_basic() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 全表搜 "张" → 命中 row_idx=1 col0, row_idx=3 col0。
        let hits = mgr.search_cells(shid, None, "张", 0, 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits
            .iter()
            .all(|c| c.value.as_deref().unwrap().contains("张")));
        // 仅 col1 搜 "138" → 命中 row_idx=1。
        let hits_col = mgr.search_cells(shid, Some(1), "138", 0, 10).unwrap();
        assert_eq!(hits_col.len(), 1);
        assert_eq!(hits_col[0].row_idx, 1);
        assert_eq!(hits_col[0].col_idx, 1);
    }

    #[test]
    fn search_cells_excludes_header_row() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 搜 "name"（表头）→ row_idx=0 应被排除，返回空。
        let hits = mgr.search_cells(shid, None, "name", 0, 10).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn search_cells_escapes_like_special_chars() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // memo 列含 "50%"；搜字面 "%" 应只命中 "50%"。
        let hits = mgr.search_cells(shid, Some(2), "%", 0, 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].row_idx, 3);
        assert_eq!(hits[0].value.as_deref(), Some("50%"));
        // 搜 "_" 字面 → memo 列无单下划线值，应 0 命中。
        let hits_u = mgr.search_cells(shid, Some(2), "_", 0, 10).unwrap();
        assert!(hits_u.is_empty());
    }

    #[test]
    fn search_cells_regex_basic() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 正则 \d{3} → phone 列两个值都含 3 位数字段；命中应带区间。
        let hits = mgr
            .search_cells_regex(shid, Some(1), r"\d{3}", 0, 10)
            .unwrap();
        assert_eq!(hits.len(), 3); // 3 个 phone 值都含 3 位数字
        for (c, spans) in &hits {
            assert!(!spans.is_empty());
            // 区间在 value 长度范围内
            let val = c.value.as_ref().unwrap();
            for &(s, e) in spans {
                assert!(s < e && e <= val.len());
            }
        }
    }

    #[test]
    fn search_cells_regex_invalid_pattern_returns_err() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 非法正则（未闭合 `[`）→ 返回 Err，不 panic。
        let res = mgr.search_cells_regex(shid, None, "[unclosed", 0, 10);
        assert!(res.is_err());
    }

    #[test]
    fn search_cells_regex_pagination() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 全表正则 \d → phone 列 3 命中 + memo "50%" 1 命中 = 4。
        let all = mgr.search_cells_regex(shid, None, r"\d", 0, 100).unwrap();
        assert_eq!(all.len(), 4);
        // offset=2 limit=1 → 取第 3 条。
        let p = mgr.search_cells_regex(shid, None, r"\d", 2, 1).unwrap();
        assert_eq!(p.len(), 1);
    }

    #[test]
    fn count_search_results_basic() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 全表搜 "1" → phone 3 行 + memo "50%" 0 = 3。
        let count = mgr.count_search_results(shid, None, "1").unwrap();
        assert_eq!(count, 3);
        // col1 搜 "139" → 1。
        let count_col = mgr.count_search_results(shid, Some(1), "139").unwrap();
        assert_eq!(count_col, 1);
    }

    #[test]
    fn count_search_results_escapes_special_chars() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 搜字面 "%" → 仅 "50%" 命中，count=1。
        let count = mgr.count_search_results(shid, Some(2), "%").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn replace_in_column_cells_returns_before_snapshot() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // col0 把 "张" → "王"：row_idx=1,3 两个值变化。
        let (affected, before, after) = mgr
            .replace_in_column_cells(shid, 0, "张", "王", false)
            .unwrap();
        assert_eq!(affected, 2);
        assert_eq!(before.len(), 2);
        assert_eq!(after.len(), 2);
        // before 含原值 "张三"/"张五"。
        let before_vals: Vec<&str> = before.iter().map(|c| c.value.as_deref().unwrap()).collect();
        assert!(before_vals.contains(&"张三"));
        assert!(before_vals.contains(&"张五"));
        // after 含新值。
        let after_vals: Vec<&str> = after.iter().map(|c| c.value.as_deref().unwrap()).collect();
        assert!(after_vals.contains(&"王三"));
        assert!(after_vals.contains(&"王五"));
        // DB 已更新。
        let col0 = mgr.query_column_cells(shid, 0).unwrap();
        let vals: Vec<&str> = col0.iter().map(|(_, v)| v.as_deref().unwrap()).collect();
        assert!(vals.contains(&"王三"));
        assert!(vals.contains(&"王五"));
        assert!(!vals.contains(&"张三"));
    }

    #[test]
    fn replace_all_cells_across_columns() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 全表正则 \d+ → "#"：phone 3 个纯数字值 → "#"；memo "50%" → "#%"。
        let (affected, _before, after) = mgr.replace_all_cells(shid, r"\d+", "#", true).unwrap();
        assert_eq!(affected, 4);
        // phone 列 3 个值变 "#"；memo "50%" 变 "#%"。
        let phone_hits = after.iter().filter(|c| c.col_idx == 1).collect::<Vec<_>>();
        assert_eq!(phone_hits.len(), 3);
        assert!(phone_hits.iter().all(|c| c.value.as_deref() == Some("#")));
        let memo_hits = after.iter().filter(|c| c.col_idx == 2).collect::<Vec<_>>();
        assert_eq!(memo_hits.len(), 1);
        assert_eq!(memo_hits[0].value.as_deref(), Some("#%"));
    }

    #[test]
    fn replace_in_column_cells_no_change_returns_empty() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 搜不存在的字符串 → 0 受影响，快照空。
        let (affected, before, after) = mgr
            .replace_in_column_cells(shid, 0, "不存在的串", "x", false)
            .unwrap();
        assert_eq!(affected, 0);
        assert!(before.is_empty());
        assert!(after.is_empty());
    }

    #[test]
    fn replace_in_column_cells_invalid_regex_returns_err() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        let res = mgr.replace_in_column_cells(shid, 0, "[bad", "x", true);
        assert!(res.is_err());
    }

    #[test]
    fn replace_all_cells_returns_both_snapshots() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 全表把 "1" → "X"：phone 3 行 + memo "50%" 0（无 1）→ 3 行变化。
        let (affected, before, after) = mgr.replace_all_cells(shid, "1", "X", false).unwrap();
        assert_eq!(affected, 3);
        assert_eq!(before.len(), 3);
        assert_eq!(after.len(), 3);
        // before 中应都含 "1"。
        assert!(before
            .iter()
            .all(|c| c.value.as_deref().unwrap().contains('1')));
        // after 中应都不含 "1"。
        assert!(after
            .iter()
            .all(|c| !c.value.as_deref().unwrap().contains('1')));
    }

    #[test]
    fn query_operation_by_id_basic() {
        let (_dir, mgr) = open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        let id = mgr
            .log_operation_with_snapshot(
                Some(shid),
                "replace",
                r#"{"from":"a","to":"b"}"#,
                Some(r#"[{"row":1,"col":0,"v":"a"}]"#),
                r#"[{"row":1,"col":0,"v":"b"}]"#,
            )
            .unwrap();
        let row = mgr.query_operation_by_id(id).unwrap().unwrap();
        assert_eq!(row.id, id);
        assert_eq!(row.sheet_id, Some(shid));
        assert_eq!(row.kind, "replace");
        assert_eq!(row.params_json.as_deref(), Some(r#"{"from":"a","to":"b"}"#));
        assert_eq!(
            row.before_snapshot_json.as_deref(),
            Some(r#"[{"row":1,"col":0,"v":"a"}]"#)
        );
        assert_eq!(
            row.result_snapshot_json.as_deref(),
            Some(r#"[{"row":1,"col":0,"v":"b"}]"#)
        );
        // 不存在的 id → None。
        assert!(mgr.query_operation_by_id(id + 999).unwrap().is_none());
    }

    #[test]
    fn log_operation_without_snapshot_keeps_before_null() {
        let (_dir, mgr) = open();
        let id = mgr.log_operation(None, "import", "{}", "{}").unwrap();
        let row = mgr.query_operation_by_id(id).unwrap().unwrap();
        assert_eq!(row.before_snapshot_json, None);
        assert_eq!(row.result_snapshot_json.as_deref(), Some("{}"));
    }
}
