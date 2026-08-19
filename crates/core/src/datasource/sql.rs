//! SQL 读取器：用 `rusqlite` in-memory 执行全部语句并收集所有 SELECT 结果。
//!
//! - 用 [`Connection::open_in_memory`] 建临时库。
//! - 用简单文本扫描切分语句（按 `;` 切，跳过引号内 `;`），区分 SELECT 与非 SELECT。
//! - 非 SELECT 语句用 `execute_batch` 执行（CREATE/INSERT/UPDATE/DELETE）。
//! - SELECT 语句逐条 `prepare + query_map`，收集结果。
//! - 多个 SELECT 结果拼接：列对齐到首组 headers，缺列补空。
//! - 若无 SELECT，自动 `SELECT * FROM <最后 CREATE 的表>`。
//!
//! v1.2.2：支持 MySQL dump 导入——剥离块注释 / 版本注释 `/*!nnnnn ... */`，
//! 跳过 MySQL-only 语句（`CREATE DATABASE` / `USE` / `LOCK|UNLOCK TABLES` /
//! `SET`），清理 `CREATE TABLE` 中的 MySQL 列/表选项（`ENGINE=`、
//! `CHARACTER SET`、`COLLATE`、`AUTO_INCREMENT`、`DEFAULT CHARSET` 等），
//! 并将 MySQL backslash escape（`\'`/`\"`/`\\`）转换为 SQLite 兼容格式。

use regex::Regex;
use rusqlite::types::ValueRef;
use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::{Dataset, Reader};

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
        let mut in_block_comment = false;
        // v1.2.2：MySQL backslash escape 支持。
        // MySQL 用 \', \", \\ 转义引号和反斜杠；SQLite 不识别 \ 转义。
        // 先用 prev_was_backslash 在切分时跳过被 \ 转义的引号（防止误判引号状态），
        // 再对每条语句调用 sanitize_backslash_escapes 将转义转换为 SQLite 兼容格式。
        let mut prev_was_backslash = false;

        for ch in content.chars() {
            if in_block_comment {
                if ch == '/' && current.ends_with('*') {
                    in_block_comment = false;
                    current.pop(); // 去掉 '*'
                } else {
                    if ch == '*' {
                        current.push(ch);
                    }
                }
                prev_was_backslash = false;
                continue;
            }
            if in_line_comment {
                if ch == '\n' {
                    in_line_comment = false;
                }
                prev_was_backslash = false;
                continue;
            }
            // 检测块注释开始 /*（含 MySQL 版本注释 /*!nnnnn ... */）。
            if ch == '*' && current.ends_with('/') && !in_single && !in_double {
                in_block_comment = true;
                current.pop(); // 去掉 '/'
                continue;
            }
            // 引号内：处理 MySQL backslash escape。
            // \', \", \\ — 跳过被转义的引号，不切换引号状态。
            if (in_single || in_double) && prev_was_backslash {
                current.push(ch);
                prev_was_backslash = ch == '\\';
                continue;
            }
            if ch == '\\' && (in_single || in_double) {
                current.push(ch);
                prev_was_backslash = true;
                continue;
            }
            if ch == '\'' && !in_double {
                in_single = !in_single;
                current.push(ch);
                prev_was_backslash = false;
                continue;
            }
            if ch == '"' && !in_single {
                in_double = !in_double;
                current.push(ch);
                prev_was_backslash = false;
                continue;
            }
            if ch == '-' && current.ends_with('-') && !in_single && !in_double {
                // 行注释开始
                in_line_comment = true;
                current.pop(); // 去掉前一个 '-'
                prev_was_backslash = false;
                continue;
            }
            if ch == ';' && !in_single && !in_double {
                let stmt = current.trim().to_string();
                current.clear();
                if stmt.is_empty() {
                    prev_was_backslash = false;
                    continue;
                }
                // v1.2.2：跳过 MySQL-only 语句（CREATE DATABASE / USE /
                // LOCK|UNLOCK TABLES / SET），SQLite 无对应语义。
                if is_mysql_only(&stmt) {
                    prev_was_backslash = false;
                    continue;
                }
                let upper = stmt.to_uppercase();
                if upper.trim_start().starts_with("SELECT") {
                    selects.push(stmt);
                } else {
                    // v1.2.2：将 MySQL backslash escape 转换为 SQLite 兼容格式。
                    let stmt = sanitize_backslash_escapes(&stmt);
                    // 记录最后 CREATE TABLE 的表名。
                    if upper.contains("CREATE TABLE") {
                        let stmt = sanitize_create_table(&stmt);
                        if let Some(name) = extract_table_name(&stmt) {
                            last_table = Some(name);
                        }
                        non_select.push(stmt);
                    } else {
                        non_select.push(stmt);
                    }
                }
                prev_was_backslash = false;
                continue;
            }
            current.push(ch);
            prev_was_backslash = false;
        }
        // 末尾无分号的语句。
        let stmt = current.trim().to_string();
        if !stmt.is_empty() {
            if !is_mysql_only(&stmt) {
                let upper = stmt.to_uppercase();
                if upper.trim_start().starts_with("SELECT") {
                    selects.push(stmt);
                } else {
                    let stmt = sanitize_backslash_escapes(&stmt);
                    if upper.contains("CREATE TABLE") {
                        let stmt = sanitize_create_table(&stmt);
                        if let Some(name) = extract_table_name(&stmt) {
                            last_table = Some(name);
                        }
                        non_select.push(stmt);
                    } else {
                        non_select.push(stmt);
                    }
                }
            }
        }
        (non_select, selects, last_table)
    }
}

/// 将 MySQL backslash escape 转换为 SQLite 兼容格式。
///
/// MySQL 在字符串字面量内用反斜杠转义：`\'` → `'`、`\"` → `"`、`\\` → `\`。
/// SQLite 不识别 `\` 转义，引号需要用双写（`''`）方式转义。
///
/// 本函数逐字符扫描语句，仅在单/双引号内替换：
/// - `\'` → `''`（单引号转义 → SQLite 双写单引号）
/// - `\"` → `"` （双引号转义 → 去掉反斜杠，SQLite 双引号不需转义）
/// - `\\` → `\` （反斜杠转义 → 还原为单个反斜杠）
/// - 其他 `\x` → 原样保留（如 `\n`、`\t` 等不常见于 dump）
fn sanitize_backslash_escapes(stmt: &str) -> String {
    let chars: Vec<char> = stmt.chars().collect();
    let mut out = String::with_capacity(stmt.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' && !in_double {
            in_single = !in_single;
            out.push(ch);
            i += 1;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            out.push(ch);
            i += 1;
            continue;
        }
        // 在引号内处理 backslash escape。
        if ch == '\\' && (in_single || in_double) && i + 1 < chars.len() {
            let next = chars[i + 1];
            match next {
                '\'' => {
                    // \' → ''（SQLite 双写单引号）
                    out.push('\'');
                    out.push('\'');
                    i += 2;
                    continue;
                }
                '"' => {
                    // \" → "（去掉反斜杠）
                    out.push('"');
                    i += 2;
                    continue;
                }
                '\\' => {
                    // \\ → \（还原单个反斜杠）
                    out.push('\\');
                    i += 2;
                    continue;
                }
                _ => {
                    // 其他 \x → 原样保留
                    out.push(ch);
                    out.push(next);
                    i += 2;
                    continue;
                }
            }
        }
        out.push(ch);
        i += 1;
    }
    out
}

/// 判断语句是否为 MySQL-only（SQLite 无对应语义），应跳过不执行。
fn is_mysql_only(stmt: &str) -> bool {
    let trimmed = stmt.trim_start();
    let upper = trimmed.to_uppercase();
    let first_word = upper.split_whitespace().next().unwrap_or("");
    match first_word {
        "USE" | "SET" => true,
        _ => {
            // CREATE DATABASE / LOCK TABLES / UNLOCK TABLES
            upper.starts_with("CREATE DATABASE")
                || upper.starts_with("LOCK TABLES")
                || upper.starts_with("UNLOCK TABLES")
        }
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

/// 清理 `CREATE TABLE` 语句中的 MySQL-only 列/表选项，使其兼容 SQLite。
///
/// 剥离内容：
/// 1. `COMMENT '...'`（列/表注释）
/// 2. 列级 `CHARACTER SET <word>` + `COLLATE <word>`
/// 3. 列级 `AUTO_INCREMENT` 关键字
/// 4. 最后 `)` 之后的全部内容（`ENGINE=...`、`AUTO_INCREMENT=...`、
///    `DEFAULT CHARSET=...` 等表选项）
fn sanitize_create_table(stmt: &str) -> String {
    let mut s = stmt.to_string();

    // 1. 剥离 COMMENT '...'（单引号字符串，含转义）。trailing 空格由末尾
    //    split_whitespace 清理；SQLite 容忍逗号前空格。
    let comment_re = Regex::new(r"COMMENT\s+'[^']*'").unwrap();
    s = comment_re.replace_all(&s, "").to_string();

    // 2. 剥离列级 CHARACTER SET <word> 和 COLLATE <word>。
    let charset_re = Regex::new(r"(?i)\s+CHARACTER SET\s+\w+").unwrap();
    s = charset_re.replace_all(&s, "").to_string();
    let collate_re = Regex::new(r"(?i)\s+COLLATE\s+\w+").unwrap();
    s = collate_re.replace_all(&s, "").to_string();

    // 3. 剥离列级 AUTO_INCREMENT 关键字（AUTO_INCREMENT=数字 的表级形式
    //    由下一步截断处理）。
    let ai_re = Regex::new(r"(?i)\s+AUTO_INCREMENT\b").unwrap();
    s = ai_re.replace_all(&s, "").to_string();

    // 4. 截断最后 `)` 之后的所有表选项（ENGINE= / DEFAULT CHARSET= 等）。
    if let Some(end) = s.rfind(')') {
        s.truncate(end + 1);
    }

    // 清理多余空格。
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Reader for SqlReader {
    /// 单次解析：执行 SQL 一次，收集 union headers + 所有 SELECT 数据行。
    fn read(&self) -> CoreResult<Dataset> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| CoreError::DataSource(format!("sql open: {e}")))?;
        let conn = Connection::open_in_memory()
            .map_err(|e| CoreError::DataSource(format!("sqlite open: {e}")))?;

        let (non_select, mut selects, last_table) = Self::split_statements(&content);

        // 执行非 SELECT 语句。
        for stmt in &non_select {
            conn.execute_batch(stmt)
                .map_err(|e| CoreError::DataSource(format!("sqlite execute `{stmt}`: {e}")))?;
        }

        // 若无 SELECT，自动 SELECT * FROM 最后创建的表。
        if selects.is_empty() {
            if let Some(table) = last_table {
                selects.push(format!("SELECT * FROM `{table}`"));
            }
        }

        if selects.is_empty() {
            return Ok(Dataset {
                headers: Vec::new(),
                rows: Vec::new(),
            });
        }

        // 收集所有 SELECT 结果。
        let mut union_headers: Vec<String> = Vec::new();
        let mut all_rows: Vec<(Vec<String>, Vec<String>)> = Vec::new();

        for sql in &selects {
            let mut stmt = conn
                .prepare(sql)
                .map_err(|e| CoreError::DataSource(format!("sqlite prepare `{sql}`: {e}")))?;
            let col_count = stmt.column_count();
            // 收集本组列名。
            let local_headers: Vec<String> = (0..col_count)
                .map(|i| stmt.column_name(i).unwrap_or("col").to_string())
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
                            ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
                            ValueRef::Blob(b) => String::from_utf8_lossy(b).to_string(),
                        };
                        cells.push(cell);
                    }
                    Ok(cells)
                })
                .map_err(|e| CoreError::DataSource(format!("sqlite query `{sql}`: {e}")))?;
            for row_result in rows_iter {
                let cells =
                    row_result.map_err(|e| CoreError::DataSource(format!("sqlite row: {e}")))?;
                all_rows.push((cells, local_headers.clone()));
            }
        }

        // 数据行：按 union_headers 对齐。
        let mut rows: Vec<Record> = Vec::with_capacity(all_rows.len());
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
            rows.push(Record { fields });
        }
        Ok(Dataset {
            headers: union_headers,
            rows,
        })
    }

    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let Dataset { headers, rows } = self.read()?;
        let mut records: Vec<Record> = Vec::with_capacity(rows.len() + 1);
        let mut h_fields = std::collections::HashMap::new();
        for h in &headers {
            h_fields.insert(h.clone(), h.clone());
        }
        records.push(Record { fields: h_fields });
        records.extend(rows);
        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        Ok(self.read()?.headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 块注释和 MySQL 版本注释被整体剥离，不进入语句。
    #[test]
    fn split_strips_block_comments() {
        let sql = "/* plain block comment */ SELECT 1; /*!40101 SET @OLD_X=@@X */; SELECT 2;";
        let (non_select, selects, _table) = SqlReader::split_statements(sql);
        // 版本注释内的 SET 被剥离 → non_select 为空。
        assert!(non_select.is_empty());
        assert_eq!(selects.len(), 2);
        assert!(selects[0].contains("SELECT 1"));
        assert!(selects[1].contains("SELECT 2"));
    }

    /// MySQL-only 语句（CREATE DATABASE / USE / LOCK|UNLOCK TABLES / SET）被跳过。
    #[test]
    fn split_skips_mysql_only_statements() {
        let sql = "\
            CREATE DATABASE `person`;\
            USE `person`;\
            SET NAMES utf8;\
            LOCK TABLES `t` WRITE;\
            UNLOCK TABLES;\
            INSERT INTO t VALUES (1);\
        ";
        let (non_select, _selects, _table) = SqlReader::split_statements(sql);
        // 只有 INSERT 保留，其余被跳过。
        assert_eq!(non_select.len(), 1);
        assert!(non_select[0].to_uppercase().contains("INSERT"));
    }

    /// sanitize_create_table 剥离 MySQL 列/表选项。
    #[test]
    fn sanitize_create_table_strips_mysql_options() {
        let stmt = "CREATE TABLE `person_data` (\
            `id` int(255) NOT NULL AUTO_INCREMENT,\
            `name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,\
            PRIMARY KEY (`id`)\
        ) ENGINE=InnoDB AUTO_INCREMENT=10001 DEFAULT CHARSET=utf8";
        let result = sanitize_create_table(stmt);
        let upper = result.to_uppercase();
        assert!(!upper.contains("CHARACTER SET"));
        assert!(!upper.contains("COLLATE"));
        assert!(!upper.contains("AUTO_INCREMENT"));
        assert!(!upper.contains("ENGINE"));
        assert!(!upper.contains("DEFAULT CHARSET"));
        // 最后一个 ) 截断后不含表选项。
        assert!(result.ends_with(')'));
    }

    /// MySQL backslash escape 被正确转换为 SQLite 兼容格式。
    #[test]
    fn sanitize_backslash_escapes_converts_mysql_escapes() {
        // \' → ''（SQLite 双写单引号）
        let s = sanitize_backslash_escapes("INSERT INTO t VALUES ('It\\'s ok')");
        assert_eq!(s, "INSERT INTO t VALUES ('It''s ok')");
        // \" → "（去掉反斜杠）
        let s = sanitize_backslash_escapes("INSERT INTO t VALUES ('say \\\"hi\\\"')");
        assert_eq!(s, "INSERT INTO t VALUES ('say \"hi\"')");
        // \\ → \（还原单个反斜杠）
        let s = sanitize_backslash_escapes("INSERT INTO t VALUES ('C:\\\\dir')");
        assert_eq!(s, "INSERT INTO t VALUES ('C:\\dir')");
        // 引号外反斜杠不处理。
        let s = sanitize_backslash_escapes("SELECT \\n FROM t");
        assert_eq!(s, "SELECT \\n FROM t");
    }

    /// 端到端：迷你 MySQL dump（版本注释 + CREATE DATABASE + USE +
    /// CREATE TABLE with MySQL options + INSERT + UNLOCK）能被正确解析。
    #[test]
    fn mysql_dump_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let sql_path = dir.path().join("dump.sql");
        let content = "\
-- MySQL dump
/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET NAMES utf8 */;
CREATE DATABASE /*!32312 IF NOT EXISTS*/ `testdb` /*!40100 DEFAULT CHARACTER SET latin1 */;
USE `testdb`;
DROP TABLE IF EXISTS `users`;
CREATE TABLE `users` (
  `id` int(255) NOT NULL AUTO_INCREMENT,
  `name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=10001 DEFAULT CHARSET=utf8;
LOCK TABLES `users` WRITE;
/*!40000 ALTER TABLE `users` DISABLE KEYS */;
INSERT INTO `users` VALUES (1,'Alice'),(2,'Bob'),(3,'It\\'s a test'),(4,'path \\\"C:\\\\x\\\"');
/*!40000 ALTER TABLE `users` ENABLE KEYS */;
UNLOCK TABLES;
/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
";
        std::fs::write(&sql_path, content).unwrap();
        let reader = SqlReader::new(sql_path.to_str().unwrap());
        let dataset = reader.read().unwrap();
        assert_eq!(dataset.headers, vec!["id", "name"]);
        assert_eq!(dataset.rows.len(), 4);
        assert_eq!(dataset.rows[0].fields.get("id"), Some(&"1".to_string()));
        assert_eq!(dataset.rows[0].fields.get("name"), Some(&"Alice".to_string()));
        assert_eq!(dataset.rows[1].fields.get("id"), Some(&"2".to_string()));
        assert_eq!(dataset.rows[1].fields.get("name"), Some(&"Bob".to_string()));
        // MySQL backslash escape 被转换后正确导入。
        assert_eq!(dataset.rows[2].fields.get("name"), Some(&"It's a test".to_string()));
        assert_eq!(dataset.rows[3].fields.get("name"), Some(&"path \"C:\\x\"".to_string()));
    }

    /// 回归测试：标准 SQLite 兼容 SQL 不受影响。
    #[test]
    fn sqlite_compatible_still_works() {
        let dir = tempfile::tempdir().unwrap();
        let sql_path = dir.path().join("dump.sql");
        let content = "\
            CREATE TABLE IF NOT EXISTS users (\
                username TEXT,\
                name TEXT\
            );\
            INSERT INTO users VALUES ('admin','Alice');\
            INSERT INTO users VALUES ('guest','Bob');\
        ";
        std::fs::write(&sql_path, content).unwrap();
        let reader = SqlReader::new(sql_path.to_str().unwrap());
        let dataset = reader.read().unwrap();
        assert_eq!(dataset.headers, vec!["username", "name"]);
        assert_eq!(dataset.rows.len(), 2);
        assert_eq!(dataset.rows[0].fields.get("username"), Some(&"admin".to_string()));
        assert_eq!(dataset.rows[1].fields.get("name"), Some(&"Bob".to_string()));
    }
}
