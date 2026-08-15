//! SQLite 持久层入口。
//!
//! `DbManager` 持有 `Mutex<Connection>`，供命令层调用。DB 文件位于
//! `app_config_dir/ruT0datakit.db`，schema 由 `schema::SCHEMA_DDL` 定义，迁移由
//! `migrate::migrate` 处理。
//!
//! T91：按领域拆分为 `cells`/`sheets`/`rules`/`search`/`operations` 子模块，
//! `impl DbManager` 跨文件分布；本文件仅保留结构体、辅助函数、构造与设置。

pub mod error;
pub mod migrate;
pub mod schema;

mod cells;
mod operations;
mod rules;
mod search;
mod sheets;

pub use error::DbError;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use ruT0_data_kit_core::processor::rules::{
    ExtractParams, Rule, RuleKind, TemplateParams,
};
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
pub(super) fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 把 DB 行（id/name/kind/field/pattern/replacement/template/enabled/description）映射为 `Rule`。
/// `template` 列存 `TemplateParams` JSON（v1.1.3 新增，旧库迁移后为 NULL）。
pub(super) fn row_to_rule(r: &rusqlite::Row<'_>) -> rusqlite::Result<Rule> {
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
    let enabled: i64 = r.get::<_, i64>(7)?;
    // template 列（index 6）存 TemplateParams JSON；NULL/空 → None。
    let template_json: Option<String> = r.get::<_, Option<String>>(6)?;
    let template = template_json
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str::<TemplateParams>(s).ok());
    // params 列（index 9）存 ExtractParams JSON；NULL/空 → None。
    let params_json: Option<String> = r.get::<_, Option<String>>(9)?;
    let params = params_json
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str::<ExtractParams>(s).ok());
    Ok(Rule {
        id: r.get::<_, String>(0)?,
        name: r.get::<_, String>(1)?,
        kind,
        field: r.get::<_, Option<String>>(3)?,
        pattern: r.get::<_, Option<String>>(4)?,
        replacement: r.get::<_, Option<String>>(5)?,
        template,
        enabled: enabled != 0,
        description: r.get::<_, String>(8)?,
        params,
    })
}

/// `LIKE` 模式转义：把 `\` → `\\`，`%` → `\%`，`_` → `\_`，
/// 配合 `ESCAPE '\'` 子句让 keyword 中的特殊字符按字面匹配。
pub(super) fn escape_like(keyword: &str) -> String {
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

/// 把 DB 行（sheet_id / row_idx / col_idx / value）映射为 [`Cell`]。
///
/// 供 `cells` / `search` / `operations` 子模块的 `query_map` 闭包复用，
/// 消除重复的 `|r| Ok(Cell { ... })` 闭包。
pub(super) fn map_row_to_cell(r: &rusqlite::Row<'_>) -> rusqlite::Result<Cell> {
    Ok(Cell {
        sheet_id: r.get::<_, i64>(0)?,
        row_idx: r.get::<_, i64>(1)? as u32,
        col_idx: r.get::<_, i64>(2)? as u32,
        value: r.get::<_, Option<String>>(3)?,
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
    /// 获取 DB 连接的 MutexGuard（统一 `expect("db mutex poisoned")`）。
    ///
    /// 供各子模块的 `self.conn()` 调用，消除重复的
    /// `self.conn.lock().expect("db mutex poisoned")` 语句。
    pub(super) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().expect("db mutex poisoned")
    }

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

    /// 读键值设置。
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, DbError> {
        let conn = self.conn();
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
        let conn = self.conn();
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

/// 跨子模块测试辅助（T91 供各子文件 `#[cfg(test)]` 引用）。
#[cfg(test)]
pub(super) mod test_support {
    use super::*;

    pub fn open() -> (tempfile::TempDir, DbManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        (dir, mgr)
    }

    pub fn cell(shid: i64, row: u32, col: u32, val: &str) -> Cell {
        Cell { sheet_id: shid, row_idx: row, col_idx: col, value: Some(val.into()) }
    }

    /// 3 列 × 4 行 sheet（row_idx=0 表头）：col0=name, col1=phone, col2=memo。
    pub fn build_search_sheet(mgr: &DbManager) -> i64 {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_tables_and_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        // v1.1.3: schema_version=5（T55: rules.params 列）
        assert_eq!(
            mgr.get_setting("schema_version").unwrap().as_deref(),
            Some("5")
        );
        // DB 文件已生成
        assert!(dir.path().join("ruT0datakit.db").exists());
        // 重复 new 幂等
        let _ = DbManager::new(dir.path()).unwrap();
    }

    #[test]
    fn settings_roundtrip() {
        let (_dir, mgr) = super::test_support::open();
        assert_eq!(mgr.get_setting("k").unwrap(), None);
        mgr.set_setting("k", "v1").unwrap();
        assert_eq!(mgr.get_setting("k").unwrap().as_deref(), Some("v1"));
        mgr.set_setting("k", "v2").unwrap();
        assert_eq!(mgr.get_setting("k").unwrap().as_deref(), Some("v2"));
    }
}
