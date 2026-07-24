//! SQL 数据源读取器：把 `.sql` 文件读成 [`Records`]。
//!
//! v0.4.4 起支持两种读取模式，按文件内容自动选择：
//!
//! 1. **结构化还原（dump）**：当 SQL 含 `CREATE TABLE` + `INSERT INTO ... VALUES`
//!    时，从 CREATE TABLE 提取列名作为 headers，从 INSERT VALUES 元组列表
//!    还原每一行数据。每个 INSERT 元组变成 `Records` 的一行，cell 按列顺序
//!    对齐，去引号后保留字面值（支持字符串里含 `,` / `;` / `'` 等特殊字符）。
//! 2. **语句切分（fallback）**：若文件不含 CREATE TABLE 或不含 INSERT VALUES，
//!    回退到 v0.4.0 的最小实现——按 `;` 拆语句，每条非空语句一行，
//!    headers = `["sql_text", "statement_type"]`，`statement_type` 按首关键字
//!    小写判定：select/insert/update/delete/create/alter/drop/other。
//!
//! 切分语义（fallback 路径与 dump 路径的 INSERT 元组扫描共用）：
//! - 行首空白后以 `--` 开头 → 整行注释，跳过整行内容（包括其中可能的 `;`）。
//! - 进入引号字符串（`'` 或 `"`）后不再切分；遇到与进入时相同的引号字符且
//!   前一字符不是 `\` 时视为字符串结束（最小处理：不支持 SQL 的 `''` 转义，
//!   fixture 场景足够）。
//! - 在引号外遇到 `;` → 切分点。
//!
//! 限制：不处理 `/* ... */` 块注释；不区分反引号字符串；不解析嵌套分号。
//! 当前实现覆盖 fixture 中的 MySQL dump 与一般 SQL 场景。

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
        // 优先尝试结构化还原（CREATE TABLE + INSERT VALUES）。
        if let Some(rec) = parse_structured_dump(&content) {
            return Ok(rec);
        }
        // 回退：按 ; 切分语句。
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

/// 尝试把 SQL dump 还原成结构化 [`Records`]。
///
/// 当文件含至少一条 `CREATE TABLE` 与至少一条 `INSERT INTO ... VALUES` 时，
/// 从第一条 CREATE TABLE 提取列名作为 headers，扫描所有 INSERT 元组还原行。
/// 若提取不到列名或没有 INSERT VALUES 元组，返回 `None`（caller 回退到
/// 语句切分路径）。
pub fn parse_structured_dump(content: &str) -> Option<Records> {
    // CREATE TABLE 存在性快速判断（小写全文一次，后续匹配在 lowercase 上做
    // 索引切片，保证对原 content 按字节对齐——ASCII 字符在 utf-8 下 lower/
    // upper 字节数不变，所以 lowercase 文本的字节索引可以直接映射回原文）。
    let lower = content.to_ascii_lowercase();
    let create_pos = lower.find("create table")?;
    // 必须有 INSERT INTO ... VALUES 才走结构化路径。
    let _insert_pos = lower.find("insert into")?;
    let _values_pos = lower.find("values")?;

    // 1. 从 CREATE TABLE 提取列名（反引号 `name` 或裸标识符）。
    let headers = extract_create_table_columns(content, create_pos)?;
    if headers.is_empty() {
        return None;
    }

    // 2. 扫描所有 INSERT INTO ... VALUES (...) 元组。
    let rows = extract_all_insert_rows(content, &lower);
    if rows.is_empty() {
        return None;
    }

    Some(Records { headers, rows })
}

/// 从 `CREATE TABLE \`tbl\` ( ... )` 块提取列名。
///
/// 列名取自列定义行首的 `` `name` `` 或裸标识符；跳过 PRIMARY KEY / KEY /
/// INDEX / CONSTRAINT / FOREIGN / UNIQUE / CHECK / FULLTEXT / SPATIAL 等非列项。
/// 跳过反引号字符串外由 `--` 行注释引入的内容（CREATE TABLE 块通常在 dump
/// 头部，不含 `--`，但兜底）。
fn extract_create_table_columns(content: &str, create_pos: usize) -> Option<Vec<String>> {
    // 找到 CREATE TABLE 后第一个 `(`，再找匹配的 `)`（不处理嵌套，CREATE
    // TABLE 列定义不会嵌套括号；PRIMARY KEY (`id`) 里的括号在反引号外但属于
    // 非列项，会被识别跳过）。
    let after_kw = &content[create_pos..];
    let paren_open = after_kw.find('(')?;
    let abs_open = create_pos + paren_open;

    // 找匹配的右括号：在引号外扫描第一个 `)`。CREATE TABLE 列定义里反引号
    // 字符串不含 `)`，且列定义本身不嵌套括号，所以第一个引号外 `)` 即闭合。
    let mut depth: i32 = 0;
    let bytes = content.as_bytes();
    let mut i = abs_open;
    let mut quote: Option<u8> = None;
    let mut close_abs: Option<usize> = None;
    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(q) => {
                if b == q {
                    quote = None;
                }
            }
            None => {
                if b == b'`' || b == b'\'' || b == b'"' {
                    quote = Some(b);
                } else if b == b'(' {
                    depth += 1;
                } else if b == b')' {
                    depth -= 1;
                    if depth == 0 {
                        close_abs = Some(i);
                        break;
                    }
                }
            }
        }
        i += 1;
    }
    let close_abs = close_abs?;
    let body = &content[abs_open + 1..close_abs];

    // 按顶层逗号切列定义（引号外 / 括号深度 0 的逗号）。
    // 用字节缓冲区累积，避免 byte→char 对多字节 UTF-8 产生乱码。
    let mut cols: Vec<String> = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    let mut q: Option<u8> = None;
    let mut d: i32 = 0;
    for b in body.bytes() {
        match q {
            Some(qb) => {
                cur.push(b);
                if b == qb {
                    q = None;
                }
            }
            None => {
                if b == b'`' || b == b'\'' || b == b'"' {
                    q = Some(b);
                    cur.push(b);
                } else if b == b'(' {
                    d += 1;
                    cur.push(b);
                } else if b == b')' {
                    d -= 1;
                    cur.push(b);
                } else if b == b',' && d == 0 {
                    cols.push(extract_col_name_from_def_bytes(&cur));
                    cur.clear();
                } else {
                    cur.push(b);
                }
            }
        }
    }
    if cur.iter().any(|&b| b != b' ' && b != b'\n' && b != b'\r' && b != b'\t') {
        cols.push(extract_col_name_from_def_bytes(&cur));
    }

    // 过滤非列项关键字。
    let non_col = [
        "primary", "key", "index", "constraint", "foreign", "unique", "check",
        "fulltext", "spatial",
    ];
    let headers: Vec<String> = cols
        .into_iter()
        .filter(|c| {
            let lc = c.trim().to_ascii_lowercase();
            !non_col.iter().any(|k| lc == *k || lc.starts_with(k))
        })
        .collect();
    Some(headers)
}

/// 从单个列定义行（如 `` `编号` int(255) NOT NULL AUTO_INCREMENT ``）
/// 提取列名（去反引号）。若首 token 不是反引号字符串也不是裸标识符，
/// 返回原字符串 trim。接收字节切片以正确处理 UTF-8 多字节列名。
fn extract_col_name_from_def(def: &str) -> String {
    extract_col_name_from_def_bytes(def.as_bytes())
}

/// `extract_col_name_from_def` 的字节版：直接处理 `[u8]`，避免 `b as char`
/// 把 UTF-8 多字节序列拆散导致乱码。
fn extract_col_name_from_def_bytes(def: &[u8]) -> String {
    // trim 前后 ASCII 空白。
    let mut start = 0;
    while start < def.len() && (def[start] == b' ' || def[start] == b'\n' || def[start] == b'\r' || def[start] == b'\t') {
        start += 1;
    }
    let mut end = def.len();
    while end > start && (def[end-1] == b' ' || def[end-1] == b'\n' || def[end-1] == b'\r' || def[end-1] == b'\t') {
        end -= 1;
    }
    let t = &def[start..end];
    if t.is_empty() {
        return String::new();
    }
    if t[0] == b'`' {
        // 反引号字符串：找下一个 `。
        if let Some(pos) = t[1..].iter().position(|&b| b == b'`') {
            return String::from_utf8_lossy(&t[1..1+pos]).into_owned();
        }
    }
    // 裸标识符：取首个非 [A-Za-z0-9_$] 字符前的部分。
    let id_end = t
        .iter()
        .position(|&b| !(b.is_ascii_alphanumeric() || b == b'_' || b == b'$'))
        .unwrap_or(t.len());
    String::from_utf8_lossy(&t[..id_end]).into_owned()
}

/// 扫描整段 SQL，提取所有 `INSERT INTO ... VALUES (...)` 元组，
/// 每个 (...) 还原成一行 cells。
///
/// 支持单条 INSERT 含多个 value 元组（MySQL dump 常见形态）：
/// `INSERT INTO t VALUES (1,'a'),(2,'b'),(3,'c');`
/// 支持列名清单 `INSERT INTO t (a,b) VALUES (1,2);`（按 VALUES 元组顺序对齐，
/// 列清单被忽略——行顺序与 CREATE TABLE 一致即可）。
fn extract_all_insert_rows(content: &str, lower_content: &str) -> Vec<Vec<String>> {
    let bytes = content.as_bytes();
    let lower_bytes = lower_content.as_bytes();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut search_from = 0usize;
    while search_from < lower_bytes.len() {
        let Some(rel) = lower_content[search_from..].find("insert into") else {
            break;
        };
        let abs_kw = search_from + rel;
        // 找这条 INSERT 的 VALUES 关键字（必须在本语句内；用最近一个 ; 之前界定）。
        let Some(values_rel) = lower_content[abs_kw..].find("values") else {
            break;
        };
        let abs_values = abs_kw + values_rel;
        // 找本条 INSERT 的结尾 `;`（引号外），用于界定本条 INSERT 的 VALUES
        // 元组扫描范围。
        let stmt_end = find_statement_terminator(bytes, abs_values);

        // 从 abs_values 之后扫描元组：每个 `(` 开始、配对 `)` 结束。
        // 用字节缓冲区累积 cell，避免 UTF-8 多字节字符被 byte→char 拆散。
        // 字符串字面量走 MySQL 转义规则：`\'` / `\\` 反斜杠转义 + `''` 双引号
        // 转义，避免字符串内的 `'` 提前结束字符串、把后续数据误判为元组边界。
        let scan_end = stmt_end.unwrap_or(bytes.len());
        let mut i = abs_values + "values".len();
        let mut depth: i32 = 0;
        let mut cell_buf: Vec<u8> = Vec::new();
        let mut row: Vec<String> = Vec::new();
        while i < scan_end {
            let b = bytes[i];
            if b == b'\'' || b == b'"' {
                // 字符串字面量：用 scan_string_literal 跳到闭合引号，把解码后
                // 的内容（去引号、解码 \' / \\ / '' 转义）追加到 cell_buf。
                let after = scan_string_literal(bytes, i, b, scan_end);
                decode_string_into(bytes, i + 1, after.saturating_sub(1), b, &mut cell_buf);
                i = after;
                continue;
            }
            if b == b'(' {
                depth += 1;
                if depth == 1 {
                    // 新元组开始，重置 row。
                    row.clear();
                    cell_buf.clear();
                } else {
                    cell_buf.push(b);
                }
                i += 1;
            } else if b == b')' {
                depth -= 1;
                if depth == 0 {
                    // 元组结束：把最后一个 cell 入栈（元组末尾无逗号）。
                    push_cell(&mut row, &cell_buf);
                    cell_buf.clear();
                    if !row.is_empty() {
                        rows.push(std::mem::take(&mut row));
                    }
                } else {
                    cell_buf.push(b);
                }
                i += 1;
            } else if b == b',' && depth == 1 {
                push_cell(&mut row, &cell_buf);
                cell_buf.clear();
                i += 1;
            } else {
                // 仅在元组内累积 cell；元组外的空白/换行忽略。
                if depth >= 1 {
                    cell_buf.push(b);
                }
                i += 1;
            }
        }
        search_from = scan_end.max(abs_values + 1);
    }
    rows
}

/// 把一个 cell 文本入栈：trim 前后空白。空串保持空（SQL NULL 在 dump 里是
/// 裸 `NULL` 字面量，这里原样保留为字符串 "NULL"，下游 pipeline 校验会自然
/// 判它 invalid；与 CSV reader 把空 cell 转 "" 的策略不同，但 SQL dump 里
/// NULL 是显式语义，保留字面更诚实）。接收字节切片以正确处理 UTF-8。
fn push_cell(row: &mut Vec<String>, buf: &[u8]) {
    // trim 前后 ASCII 空白。
    let mut start = 0;
    while start < buf.len() && (buf[start] == b' ' || buf[start] == b'\n' || buf[start] == b'\r' || buf[start] == b'\t') {
        start += 1;
    }
    let mut end = buf.len();
    while end > start && (buf[end-1] == b' ' || buf[end-1] == b'\n' || buf[end-1] == b'\r' || buf[end-1] == b'\t') {
        end -= 1;
    }
    row.push(String::from_utf8_lossy(&buf[start..end]).into_owned());
}

/// 从 `start` 起在引号外查找第一个 `;`，返回其在 `bytes` 中的绝对索引。
/// 若找不到，返回 `None`（扫描到文件尾）。引号内（`'` / `"` / `` ` ``）的 `;`
/// 不算语句终止符；引号内遵循 MySQL 反斜杠转义（`\'` 不结束字符串）。
fn find_statement_terminator(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\'' || b == b'"' || b == b'`' {
            i = scan_string_literal(bytes, i, b, bytes.len());
            continue;
        }
        if b == b';' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// 从 `bytes[start]`（必须是引号字符 `quote`）起扫描一个 SQL 字符串字面量，
/// 返回闭合引号**之后**的索引（即下一个待处理字符位置）。
///
/// MySQL 转义规则：反斜杠 `\` 转义下一个字符（`\'` → `'`，`\\` → `\`）；
/// 连续两个相同引号 `''` / `""` 表示字面引号字符。若到 `limit` 仍未闭合，
/// 返回 `limit`（容错，不 panic）。
fn scan_string_literal(bytes: &[u8], start: usize, quote: u8, limit: usize) -> usize {
    let mut i = start + 1;
    while i < limit {
        let b = bytes[i];
        if b == b'\\' {
            // 反斜杠转义：跳过下一字节。
            i += 2;
            continue;
        }
        if b == quote {
            // 可能是结束，也可能是双引号转义（''/""）。
            if i + 1 < limit && bytes[i + 1] == quote {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    limit
}

/// 把字符串字面量内部字节（不含首尾引号）按 MySQL 转义规则解码，追加到 `out`。
///
/// - `\\X` → `X`（保留任意被反斜杠转义的字符，如 `\'` → `'`、`\\` → `\`、
///   `\n` → `n`——注意这里不解码 `\n` 为换行，因为 MySQL dump 里 `\n` 字面
///   就是两字符，下游按字面处理即可；与 CSV reader 行为一致）。
/// - `''` / `""` → 单个 `'` / `"`。
/// - 其他字节原样拷贝（UTF-8 多字节序列自然保留）。
fn decode_string_into(
    bytes: &[u8],
    inner_start: usize,
    inner_end: usize,
    _quote: u8,
    out: &mut Vec<u8>,
) {
    let mut i = inner_start;
    while i < inner_end {
        let b = bytes[i];
        if b == b'\\' && i + 1 < inner_end {
            // 反斜杠转义：保留下一字节字面。
            out.push(bytes[i + 1]);
            i += 2;
            continue;
        }
        if b == _quote && i + 1 < inner_end && bytes[i + 1] == _quote {
            // 双引号转义：输出单个引号。
            out.push(_quote);
            i += 2;
            continue;
        }
        out.push(b);
        i += 1;
    }
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

    // -------- 结构化还原测试 --------

    #[test]
    fn structured_dump_restores_create_table_columns_and_insert_rows() {
        let sql = "\
CREATE TABLE `person_data` (
  `编号` int(255) NOT NULL AUTO_INCREMENT,
  `用户名` varchar(255) DEFAULT NULL,
  `姓名` varchar(255) DEFAULT NULL,
  `身份证号` varchar(255) DEFAULT NULL,
  `手机号码` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`编号`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;
INSERT INTO `person_data` VALUES (1,'AdminTonyzhangxiulan','殷水舞','647982200105121710','78067665942'),(2,'lalajun','屠岸立诚','980334198905011016','77135529660');
";
        let p = write_tmp("rut0_sql_dump_basic.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(
            rec.headers,
            vec!["编号", "用户名", "姓名", "身份证号", "手机号码"],
        );
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0], vec!["1", "AdminTonyzhangxiulan", "殷水舞", "647982200105121710", "78067665942"]);
        assert_eq!(rec.rows[1], vec!["2", "lalajun", "屠岸立诚", "980334198905011016", "77135529660"]);
    }

    #[test]
    fn structured_dump_handles_comma_inside_quoted_string() {
        let sql = "\
CREATE TABLE `t` (`id` int, `name` varchar(255));
INSERT INTO `t` VALUES (1,'张,三'),(2,'a;b;c');
";
        let p = write_tmp("rut0_sql_dump_quoted.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(rec.headers, vec!["id", "name"]);
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0], vec!["1", "张,三"]);
        assert_eq!(rec.rows[1], vec!["2", "a;b;c"]);
    }

    #[test]
    fn structured_dump_supports_insert_with_column_list() {
        let sql = "\
CREATE TABLE `t` (`id` int, `name` varchar(255));
INSERT INTO `t` (id, name) VALUES (1,'alice'),(2,'bob');
";
        let p = write_tmp("rut0_sql_dump_collist.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(rec.headers, vec!["id", "name"]);
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0], vec!["1", "alice"]);
        assert_eq!(rec.rows[1], vec!["2", "bob"]);
    }

    #[test]
    fn structured_dump_multiple_insert_statements_concatenate() {
        let sql = "\
CREATE TABLE `t` (`id` int, `v` varchar(255));
INSERT INTO `t` VALUES (1,'a');
INSERT INTO `t` VALUES (2,'b'),(3,'c');
";
        let p = write_tmp("rut0_sql_dump_multi.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(rec.rows.len(), 3);
        assert_eq!(rec.rows[0], vec!["1", "a"]);
        assert_eq!(rec.rows[1], vec!["2", "b"]);
        assert_eq!(rec.rows[2], vec!["3", "c"]);
    }

    #[test]
    fn structured_dump_handles_null_and_empty_string() {
        let sql = "\
CREATE TABLE `t` (`id` int, `name` varchar(255));
INSERT INTO `t` VALUES (1,NULL),(2,''),(3,'x');
";
        let p = write_tmp("rut0_sql_dump_null.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(rec.rows.len(), 3);
        assert_eq!(rec.rows[0], vec!["1", "NULL"]);
        assert_eq!(rec.rows[1], vec!["2", ""]);
        assert_eq!(rec.rows[2], vec!["3", "x"]);
    }

    #[test]
    fn fallback_when_no_create_table() {
        // 只有 INSERT，没有 CREATE TABLE → 回退语句切分路径。
        let sql = "INSERT INTO t VALUES (1,'a');\nSELECT 1;\n";
        let p = write_tmp("rut0_sql_no_create.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        assert_eq!(rec.headers, vec!["sql_text", "statement_type"]);
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0][1], "insert");
        assert_eq!(rec.rows[1][1], "select");
    }

    #[test]
    fn fallback_when_create_table_but_no_insert_values() {
        let sql = "CREATE TABLE `t` (`id` int);\nSELECT 1;\n";
        let p = write_tmp("rut0_sql_no_insert.sql", sql);
        let rec = SqlReader::new().read(&p).expect("read sql");
        // 有 CREATE TABLE 但无 INSERT VALUES → 回退语句切分。
        assert_eq!(rec.headers, vec!["sql_text", "statement_type"]);
        assert!(rec.rows.iter().any(|r| r[1] == "create"));
        assert!(rec.rows.iter().any(|r| r[1] == "select"));
    }

    #[test]
    fn structured_dump_parses_person_data_fixture() {
        // 真实 fixture：10000 行 person_data（AUTO_INCREMENT=10001），
        // 列 = 编号/用户名/姓名/身份证号/手机号码。INSERT 含 `\'` 反斜杠转义
        // 与字符串内 `,` / `;` 等特殊字符，验证结构化还原正确。
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/samples/tips/附件/person_data.sql");
        if !path.exists() {
            eprintln!("person_data.sql fixture not found, skip: {}", path.display());
            return;
        }
        let rec = SqlReader::new().read(&path).expect("read person_data.sql");
        assert_eq!(
            rec.headers,
            vec!["编号", "用户名", "姓名", "身份证号", "手机号码"],
            "headers mismatch: {:?}",
            rec.headers,
        );
        assert_eq!(rec.rows.len(), 10000, "expected 10000 rows, got {}", rec.rows.len());
        // 第一行 sanity check。
        assert_eq!(rec.rows[0][0], "1");
        assert_eq!(rec.rows[0][1], "AdminTonyzhangxiulan");
        assert_eq!(rec.rows[0][2], "殷水舞");
        assert_eq!(rec.rows[0][3], "647982200105121710");
        assert_eq!(rec.rows[0][4], "78067665942");
        // 末行 sanity（编号 10000）。
        let last = rec.rows.last().unwrap();
        assert_eq!(last[0], "10000");
        assert_eq!(last.len(), 5);
        // 所有行 5 列对齐。
        assert!(rec.rows.iter().all(|r| r.len() == 5), "some row != 5 cols");
        // 抽样：row index 9（编号 10）含规范矛盾的 818 前缀手机号，确认
        // 原样还原（不做语义校验，只做结构化还原）。
        assert_eq!(rec.rows[9][0], "10");
        assert_eq!(rec.rows[9][4], "81825660184");
    }
}
