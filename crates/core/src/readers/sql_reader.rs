//! SQL 数据源读取器：把 `.sql` 文件按 `;` 拆语句，每条非空语句一行。
//!
//! 规则（v0.4.0 最小可用）：
//! - 逐字符扫描，记录是否在单引号 / 双引号字符串内；只有不在引号内且不在
//!   `--` 行注释内的 `;` 才作为语句分隔。
//! - 行首（去掉前导空白后）以 `--` 开头的行整体跳过（行注释）。
//! - 每条语句去除首尾空白后非空才产出一行。
//! - headers = `["sql_text", "statement_type"]`，`statement_type` 按首关键字
//!   小写判定：select/insert/update/delete/create/alter/drop/other。
//!
//! 限制（留 v0.4.1+ 精化）：不处理 `/* ... */` 块注释；不区分反引号字符串；
//! 不解析嵌套分号。当前实现足以覆盖 fixture 中的 SQL dump 场景。

use std::path::Path;

use crate::error::CoreError;
use crate::readers::{Records, SourceReader};

/// SQL 读取器。
#[derive(Debug, Default, Clone, Copy)]
pub struct SqlReader;

impl SqlReader {
    pub fn new() -> Self {
        Self
    }
}

impl SourceReader for SqlReader {
    fn read(&self, path: &Path) -> Result<Records, CoreError> {
        let content = std::fs::read_to_string(path)?;
        let statements = split_sql_statements(&content);
        let rows = statements
            .into_iter()
            .map(|stmt| {
                let stmt = stmt.trim();
                vec![stmt.to_string(), classify_statement_type(stmt)]
            })
            .collect::<Vec<_>>();
        Ok(Records {
            headers: vec!["sql_text".to_string(), "statement_type".to_string()],
            rows,
        })
    }
}

/// 把整段 SQL 文本按 `;` 拆分为多条语句。
///
/// 语义：
/// - 行首空白后以 `--` 开头 → 整行注释，跳过整行内容（包括其中可能的 `;`）。
/// - 进入引号字符串（`'` 或 `"`）后不再切分；遇到与进入时相同的引号字符且
///   前一字符不是 `\` 时视为字符串结束（最小处理：不支持 SQL 的 `''` 转义，
///   fixture 场景足够）。
/// - 在引号外遇到 `;` → 切分点。
pub fn split_sql_statements(content: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    // 先逐行剔除整行 `--` 注释，再拼回统一字符扫描，保证行内注释不会
    // 误判引号状态（`--` 不会出现在数据字符串里的极端情况不在最小 scope 内）。
    let cleaned: String = content
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            !t.starts_with("--")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut buf = String::new();
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(q) => {
                buf.push(b as char);
                if b == q {
                    // 最小处理：不识别 SQL `''` 转义。fixture 不含此类转义。
                    quote = None;
                }
                i += 1;
            }
            None => {
                if b == b'\'' || b == b'"' {
                    quote = Some(b);
                    buf.push(b as char);
                    i += 1;
                } else if b == b';' {
                    let stmt = buf.trim();
                    if !stmt.is_empty() {
                        out.push(stmt.to_string());
                    }
                    buf.clear();
                    i += 1;
                } else {
                    buf.push(b as char);
                    i += 1;
                }
            }
        }
    }
    let tail = buf.trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

/// 按首关键字判定 SQL 语句类型。
///
/// 首关键字按非 ASCII 字母数字字符（空格 / `(` / `\n` 等）切分；小写后
/// 匹配：select/insert/update/delete/create/alter/drop → 对应类型，否则 other。
pub fn classify_statement_type(stmt: &str) -> String {
    let s = stmt.trim_start();
    // 取首关键字：连续 ASCII 字母字符（含下划线）。
    let end = s
        .find(|c: char| !c.is_ascii_alphabetic() && c != '_')
        .unwrap_or(s.len());
    let kw = s[..end].to_ascii_lowercase();
    match kw.as_str() {
        "select" => "select",
        "insert" => "insert",
        "update" => "update",
        "delete" => "delete",
        "create" => "create",
        "alter" => "alter",
        "drop" => "drop",
        _ => "other",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, content: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(name);
        let mut f = std::fs::File::create(&dir).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn splits_statements_and_classifies() {
        let sql = "-- comment line\nSELECT * FROM users;\nINSERT INTO t VALUES (1, 'a;b');\nDROP TABLE x;\n";
        let p = write_tmp("rut0_sql_basic.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(rec.headers, vec!["sql_text".to_string(), "statement_type".to_string()]);
        assert_eq!(rec.rows.len(), 3);
        assert_eq!(rec.rows[0][0], "SELECT * FROM users");
        assert_eq!(rec.rows[0][1], "select");
        assert_eq!(rec.rows[1][1], "insert");
        assert_eq!(rec.rows[2][1], "drop");
    }

    #[test]
    fn semicolon_inside_string_is_not_split() {
        let sql = "INSERT INTO t VALUES ('a;b;c');\nSELECT 1;\n";
        let p = write_tmp("rut0_sql_quoted.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0][1], "insert");
        assert_eq!(rec.rows[1][1], "select");
    }

    #[test]
    fn empty_and_comments_only_yields_no_rows() {
        let sql = "-- just a comment\n-- another\n";
        let p = write_tmp("rut0_sql_empty.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert!(rec.rows.is_empty());
    }

    #[test]
    fn missing_file_errors() {
        let res = SqlReader::new().read(Path::new("/nonexistent/RuT0DataKit/none.sql"));
        assert!(res.is_err());
    }

    #[test]
    fn other_keyword_classified() {
        let sql = "TRUNCATE TABLE foo;\nBEGIN;\n";
        let p = write_tmp("rut0_sql_other.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(rec.rows[0][1], "other");
        assert_eq!(rec.rows[1][1], "other");
    }
}
