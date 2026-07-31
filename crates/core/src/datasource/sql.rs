//! SQL 读取器：用 `rusqlite` in-memory 执行全部语句并收集所有 SELECT 结果。
//!
//! - 用 [`Connection::open_in_memory`] 建临时库。
//! - 用简单文本扫描切分语句（按 `;` 切，跳过引号内 `;`），区分 SELECT 与非 SELECT。
//! - 非 SELECT 语句用 `execute_batch` 执行（CREATE/INSERT/UPDATE/DELETE）。
//! - SELECT 语句逐条 `prepare + query_map`，收集结果。
//! - 多个 SELECT 结果拼接：列对齐到首组 headers，缺列补空。
//! - 若无 SELECT，自动 `SELECT * FROM <最后 CREATE 的表>`。
//!
//! 现状：`split_statements` 不处理块注释 / BEGIN END，这是已知现状，
//! 不在本任务修复范围。

use rusqlite::types::ValueRef;
use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::Reader;

/// SQL 读取器：用 `rusqlite` in-memory 执行全部语句并收集所有 SELECT 结果。
pub struct SqlReader {
    path: String,
}

impl SqlReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// 切分 SQL 文本为语句，区分 SELECT 与非 SELECT。
    /// 返回 (non_select_stmts, select_stmts, last_created_table)。
    fn split_statements(content: &str) -> (Vec<String>, Vec<String>, Option<String>) {
        let mut non_select: Vec<String> = Vec::new();
        let mut selects: Vec<String> = Vec::new();
        let mut last_table: Option<String> = None;

        let mut current = String::new();
        let mut in_single = false;
        let mut in_double = false;
        let mut in_line_comment = false;

        for ch in content.chars() {
            if in_line_comment {
                if ch == '\n' {
                    in_line_comment = false;
                }
                continue;
            }
            if ch == '\'' && !in_double {
                in_single = !in_single;
                current.push(ch);
                continue;
            }
            if ch == '"' && !in_single {
                in_double = !in_double;
                current.push(ch);
                continue;
            }
            if ch == '-' && current.ends_with('-') && !in_single && !in_double {
                // 行注释开始
                in_line_comment = true;
                current.pop(); // 去掉前一个 '-'
                continue;
            }
            if ch == ';' && !in_single && !in_double {
                let stmt = current.trim().to_string();
                current.clear();
                if stmt.is_empty() {
                    continue;
                }
                let upper = stmt.to_uppercase();
                if upper.trim_start().starts_with("SELECT") {
                    selects.push(stmt);
                } else {
                    // 记录最后 CREATE TABLE 的表名。
                    if upper.contains("CREATE TABLE") {
                        if let Some(name) = extract_table_name(&stmt) {
                            last_table = Some(name);
                        }
                    }
                    non_select.push(stmt);
                }
                continue;
            }
            current.push(ch);
        }
        // 末尾无分号的语句。
        let stmt = current.trim().to_string();
        if !stmt.is_empty() {
            let upper = stmt.to_uppercase();
            if upper.trim_start().starts_with("SELECT") {
                selects.push(stmt);
            } else {
                if upper.contains("CREATE TABLE") {
                    if let Some(name) = extract_table_name(&stmt) {
                        last_table = Some(name);
                    }
                }
                non_select.push(stmt);
            }
        }
        (non_select, selects, last_table)
    }
}

/// 从 `CREATE TABLE <name> (...)` 提取表名。
fn extract_table_name(stmt: &str) -> Option<String> {
    let upper = stmt.to_uppercase();
    let pos = upper.find("CREATE TABLE")?;
    let after = &stmt[pos + "CREATE TABLE".len()..];
    // 跳过 IF NOT EXISTS。
    let after = after.trim_start();
    let after = after
        .strip_prefix("IF NOT EXISTS")
        .unwrap_or(after)
        .trim_start();
    // 去反引号 / 双引号 / 方括号。
    let after = after.trim_start_matches('`').trim_start_matches('"');
    let name: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

impl Reader for SqlReader {
    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| CoreError::DataSource(format!("sql open: {e}")))?;
        let conn = Connection::open_in_memory()
            .map_err(|e| CoreError::DataSource(format!("sqlite open: {e}")))?;

        let (non_select, mut selects, last_table) = Self::split_statements(&content);

        // 执行非 SELECT 语句。
        for stmt in &non_select {
            conn.execute_batch(stmt).map_err(|e| {
                CoreError::DataSource(format!("sqlite execute `{stmt}`: {e}"))
            })?;
        }

        // 若无 SELECT，自动 SELECT * FROM 最后创建的表。
        if selects.is_empty() {
            if let Some(table) = last_table {
                selects.push(format!("SELECT * FROM `{table}`"));
            }
        }

        if selects.is_empty() {
            return Ok(Vec::new());
        }

        // 收集所有 SELECT 结果。
        let mut union_headers: Vec<String> = Vec::new();
        let mut all_rows: Vec<(Vec<String>, Vec<String>)> = Vec::new();

        for sql in &selects {
            let mut stmt = conn.prepare(sql).map_err(|e| {
                CoreError::DataSource(format!("sqlite prepare `{sql}`: {e}"))
            })?;
            let col_count = stmt.column_count();
            // 收集本组列名。
            let local_headers: Vec<String> = (0..col_count)
                .map(|i| {
                    stmt.column_name(i)
                        .unwrap_or("col")
                        .to_string()
                })
                .collect();
            // 扩展 union_headers。
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
                            ValueRef::Real(f) => format!("{}", f),
                            ValueRef::Text(t) => {
                                String::from_utf8_lossy(t).to_string()
                            }
                            ValueRef::Blob(b) => {
                                String::from_utf8_lossy(b).to_string()
                            }
                        };
                        cells.push(cell);
                    }
                    Ok(cells)
                })
                .map_err(|e| {
                    CoreError::DataSource(format!("sqlite query `{sql}`: {e}"))
                })?;
            for row_result in rows_iter {
                let cells = row_result
                    .map_err(|e| CoreError::DataSource(format!("sqlite row: {e}")))?;
                all_rows.push((cells, local_headers.clone()));
            }
        }

        let mut records: Vec<Record> = Vec::new();
        // 表头行。
        let mut h_fields = std::collections::HashMap::new();
        for h in &union_headers {
            h_fields.insert(h.clone(), h.clone());
        }
        records.push(Record { fields: h_fields });

        // 数据行：按 union_headers 对齐。
        for (cells, local_headers) in all_rows {
            let mut fields = std::collections::HashMap::new();
            for (i, cell) in cells.iter().enumerate() {
                let key = local_headers
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col{i}"));
                fields.insert(key, cell.clone());
            }
            // 缺列补空。
            for h in &union_headers {
                fields.entry(h.clone()).or_insert_with(String::new);
            }
            records.push(Record { fields });
        }
        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        let recs = self.read_all()?;
        Ok(recs
            .into_iter()
            .next()
            .map(|r| r.fields.into_iter().map(|(k, _)| k).collect())
            .unwrap_or_default())
    }
}
