//! 盲注数据库结构还原层（v0.5.0 T12-1 拆分自 blind_aggregator.rs）。
//!
//! 负责「一批 [`AggregatedResult`] → [`ReconstructedDatabase`]」的交叉关联：
//! 把 4 类标准 read_target（`database()` /
//! `group_concat(table_name) from information_schema.tables` /
//! `group_concat(column_name) from information_schema.columns where table_name='X'` /
//! `group_concat(col1,0xNN,col2,...) from <table>`）交叉关联成结构化的
//! [`ReconstructedDatabase`]（schema → tables → columns → rows）。
//!
//! 与 [`super::probe`] / [`super::aggregate`] 的边界：
//! - [`super::probe`] 负责探针抽取 + 数据结构定义；
//! - [`super::aggregate`] 负责位置聚类与字符串还原；
//! - 本模块做数据库结构拼装、`from <table>` / `table_name='X'` 谓词解析、
//!   `0xNN` 字面量分隔符解析。

use std::collections::HashMap;

use regex::Regex;

use super::aggregate::BlindAggregator;
use super::probe::AggregatedResult;

// ===== v0.2.4 数据库结构还原（reconstruct_database） =====

/// 重建出的数据库视图（v0.2.4 T4-1）。
///
/// 由 [`BlindAggregator::reconstruct_database`] 对一批 [`AggregatedResult`]
/// 做 4 类 read_target 交叉关联得到：schema（`database()` 还原值）+ 一组
/// [`ReconstructedTable`]（每张表含全量 columns + 已还原行）。未匹配 4 类
/// 模式的 AggregatedResult 原样回填到 [`ReconstructedDatabase::unmatched_results`]，
/// 供前端兜底展示。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReconstructedDatabase {
    /// `database()` 还原值（数据库名）；无 database() 探针时为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// 重建出的表列表。
    pub tables: Vec<ReconstructedTable>,
    /// 未匹配 4 类标准模式的 AggregatedResult（原样回传前端兜底）。
    pub unmatched_results: Vec<AggregatedResult>,
}

/// 重建出的一张表（v0.2.4 T4-1）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReconstructedTable {
    /// 表名，来自 `information_schema.tables` 或行数据查询的 `from <table>`。
    pub name: String,
    /// 全量列名列表，来自 `information_schema.columns` 还原（无列清单时用
    /// `row_data_columns` 兜底）。
    pub columns: Vec<String>,
    /// 重建出的行；每行 `cells` 与 `columns` 等长，未 fetch 列为 `None`。
    pub rows: Vec<ReconstructedRow>,
    /// 行数据查询（`group_concat(col1,0xNN,col2,...)`）实际 fetch 的列名
    /// 列表，用于 GUI 区分「已还原列」与「未 fetch 列」。
    pub row_data_columns: Vec<String>,
    /// 行数据查询的首个 `0xNN` 字面量解码分隔符（如 `Some('~')`）；无
    /// `0xNN` 字面量时为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_separator: Option<char>,
    /// 该表汇集的探针总数（来自 column_list + row_data 两类聚合的
    /// `probe_count` 之和，参考用）。
    pub source_probe_count: u32,
}

/// 重建出的一行（v0.2.4 T4-1）。
///
/// `cells` 与所属 [`ReconstructedTable::columns`] 等长，未 fetch 列为 `None`。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReconstructedRow {
    /// 单元格值列表，与 `columns` 等长，未 fetch 列为 `None`。
    pub cells: Vec<Option<String>>,
}

impl BlindAggregator {
    /// 从已聚合的 [`AggregatedResult`] 列表交叉关联，重建结构化数据库
    /// （v0.2.4 T4-1）。
    ///
    /// 算法：
    /// 1. **分类**：对每个 AggregatedResult，按 `read_target` 模式分类：
    ///    - `schema`：`read_target == "database()"`
    ///    - `table_list`：含 `group_concat(table_name)` + `information_schema.tables`
    ///    - `column_list`：含 `group_concat(column_name)` + `information_schema.columns`，
    ///      并从 `table_name='X'` 谓词抽归属表名。
    ///    - `row_data`：含 `group_concat(` 且非上述两类，从 `from <table>` 抽
    ///      表名、从 `group_concat(<cols>)` 抽列清单 + `0xNN` 分隔符。
    ///    - 其他 → `unmatched_results`。
    /// 2. **拼装**：schema 直接填；对每张表（来自 table_list 或 row_data 的
    ///    表名）关联 column_list（按 `table_name='X'` 谓词匹配，无谓词按索引
    ///    对齐）+ row_data；row 按 `,` split 行、按 `column_separator` split
    ///    列，映射到 `columns`，未 fetch 列填 `None`。
    /// 3. **unmatched**：未匹配的 AggregatedResult 原样回传。
    ///
    /// **已知简化**（Risks）：
    /// - 行切分依赖 group_concat 默认行分隔符 `,`；若某列值含 `,` 会切错
    ///   （fixture idcard 为数字无 `,`）。SEPARATOR 子句解析 out_of_scope。
    /// - column_list 谓词缺失时按索引对齐（fixture 单表无影响）。
    pub fn reconstruct_database(results: &[AggregatedResult]) -> ReconstructedDatabase {
        let mut schema: Option<String> = None;
        // (table_name, column_list)
        let mut table_lists: Vec<(String, AggregatedResult)> = Vec::new();
        // (table_name predicate Option, column_list decoded)
        let mut column_lists: Vec<(Option<String>, AggregatedResult)> = Vec::new();
        // (table_name, row_data AggregatedResult)
        let mut row_data: Vec<(String, AggregatedResult)> = Vec::new();
        let mut unmatched: Vec<AggregatedResult> = Vec::new();

        for r in results {
            let rt = r.read_target.trim().to_lowercase();
            if rt == "database()" {
                schema = Some(r.decoded_string.clone());
                continue;
            }
            if rt.contains("group_concat(table_name)") && rt.contains("information_schema.tables") {
                // 表名清单：decoded_string 按 `,` split。优先取首个非空表名
                // 作为关联键（fixture 单表 "person_data"）。
                for t in r.decoded_string.split(',') {
                    let t = t.trim();
                    if !t.is_empty() {
                        table_lists.push((t.to_string(), r.clone()));
                    }
                }
                continue;
            }
            if rt.contains("group_concat(column_name)") && rt.contains("information_schema.columns") {
                // 解析 table_name='X' 谓词。
                let predicate = parse_table_name_predicate(&r.read_target);
                column_lists.push((predicate, r.clone()));
                continue;
            }
            if rt.contains("group_concat(") {
                // 行数据查询：解析 from <table> + group_concat(...) 列清单 + 0xNN 分隔符。
                if let Some(table_name) = parse_from_table(&r.read_target) {
                    row_data.push((table_name, r.clone()));
                } else {
                    unmatched.push(r.clone());
                }
                continue;
            }
            unmatched.push(r.clone());
        }

        // 拼装表：以 table_list 的表名为准，row_data 中无 table_list 的也补上。
        let mut table_names: Vec<String> = table_lists.iter().map(|(t, _)| t.clone()).collect();
        for (t, _) in &row_data {
            if !table_names.iter().any(|n| n == t) {
                table_names.push(t.clone());
            }
        }

        let mut tables: Vec<ReconstructedTable> = Vec::with_capacity(table_names.len());
        for tname in &table_names {
            // 关联 column_list：优先按谓词匹配；无谓词按索引对齐。
            let col_list = column_lists
                .iter()
                .find_map(|(pred, r)| {
                    if pred.as_deref() == Some(tname.as_str()) {
                        Some(r.clone())
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    column_lists
                        .iter()
                        .find(|(pred, _)| pred.is_none())
                        .map(|(_, r)| r.clone())
                });

            // 关联 row_data：按 from <table> 匹配。
            let row_r = row_data.iter().find_map(|(t, r)| {
                if t == tname {
                    Some(r.clone())
                } else {
                    None
                }
            });

            // 全量列：优先 column_list 还原；无则用 row_data_columns 兜底。
            let (columns, row_data_columns, column_separator, rows, source_probe_count) =
                build_table_columns_and_rows(&col_list, &row_r);

            tables.push(ReconstructedTable {
                name: tname.clone(),
                columns,
                rows,
                row_data_columns,
                column_separator,
                source_probe_count,
            });
        }

        // 未匹配 column_list / row_data（无 table_list 关联）补进 unmatched。
        // column_lists 中已被表关联消费的不重复计入；此处简化为不二次追加
        // （unmatched 已在分类阶段收集完非标准 RT）。

        ReconstructedDatabase {
            schema,
            tables,
            unmatched_results: unmatched,
        }
    }
}

/// 解析 `table_name='X'` 谓词，返回表名 `X`（小写 read_target 中保持原值）。
fn parse_table_name_predicate(read_target: &str) -> Option<String> {
    let re = table_name_predicate_regex();
    let caps = re.captures(read_target)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

/// 解析 `from <table>` 子句，返回表名（标识符）。
fn parse_from_table(read_target: &str) -> Option<String> {
    let re = from_table_regex();
    let caps = re.captures(read_target)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

/// 从行数据查询的 read_target 解析列清单与分隔符。
///
/// 返回 (row_data_columns, column_separator)。列清单取自
/// `group_concat(<cols>)` 的捕获组 1，按 `,` split 后过滤 `0xNN` 字面量
/// token（字面量作为分隔符，不作为列）；首个 `0xNN` 字面量解码为
/// `column_separator`。
fn parse_row_data_columns(read_target: &str) -> (Vec<String>, Option<char>) {
    let re = group_concat_args_regex();
    let args = re
        .captures(read_target)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_default();
    let mut cols: Vec<String> = Vec::new();
    let mut separator: Option<char> = None;
    for tok in args.split(',') {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        // 0xNN 字面量 → 分隔符（取首个）。
        if let Some(c) = parse_separator_char(tok) {
            if separator.is_none() {
                separator = Some(c);
            }
            continue;
        }
        cols.push(tok.to_string());
    }
    (cols, separator)
}

/// 为一张表构建全量列、行数据、分隔符、探针总数。
///
/// 返回 (columns, row_data_columns, column_separator, rows, source_probe_count)。
fn build_table_columns_and_rows(
    col_list: &Option<AggregatedResult>,
    row_r: &Option<AggregatedResult>,
) -> (Vec<String>, Vec<String>, Option<char>, Vec<ReconstructedRow>, u32) {
    // 全量列：优先 column_list 的 decoded_string（按 `,` split）；无则用
    // row_data_columns 兜底。
    let (row_data_columns, column_separator) = match row_r {
        Some(r) => parse_row_data_columns(&r.read_target),
        None => (Vec::new(), None),
    };

    let columns: Vec<String> = match col_list {
        Some(r) => r
            .decoded_string
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => row_data_columns.clone(),
    };

    // 行解析：decoded_string 按 `,` split 行（group_concat 默认行分隔符），
    // 每行按 column_separator split 列。
    let mut rows: Vec<ReconstructedRow> = Vec::new();
    let mut source_probe_count: u32 = 0;
    if let Some(r) = row_r {
        source_probe_count = source_probe_count.saturating_add(r.probe_count);
        if !r.decoded_string.is_empty() && !row_data_columns.is_empty() {
            for row_str in r.decoded_string.split(',') {
                let row_str = row_str.trim();
                if row_str.is_empty() {
                    continue;
                }
                let values: Vec<String> = match column_separator {
                    Some(sep) => row_str.split(sep).map(|s| s.to_string()).collect(),
                    None => vec![row_str.to_string()],
                };
                // 建 row_data_columns → value 映射。
                let mut map: HashMap<String, String> = HashMap::new();
                for (i, col) in row_data_columns.iter().enumerate() {
                    if let Some(v) = values.get(i) {
                        map.insert(col.clone(), v.clone());
                    }
                }
                // 按 columns 取值，未 fetch 列 = None。
                let cells: Vec<Option<String>> = columns
                    .iter()
                    .map(|c| map.get(c).cloned())
                    .collect();
                rows.push(ReconstructedRow { cells });
            }
        }
    }
    if let Some(r) = col_list {
        source_probe_count = source_probe_count.saturating_add(r.probe_count);
    }

    (columns, row_data_columns, column_separator, rows, source_probe_count)
}

fn table_name_predicate_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"table_name\s*=\s*'([^']+)'")
            .expect("table_name_predicate regex must compile")
    })
}

fn from_table_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"from\s+([a-zA-Z_][a-zA-Z0-9_]*)")
            .expect("from_table regex must compile")
    })
}

fn group_concat_args_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"group_concat\s*\(([^)]*)\)")
            .expect("group_concat_args regex must compile")
    })
}

/// 从 read_target 解析首个 `0x([0-9a-fA-F]{2})` 十六进制字面量分隔符。
///
/// 用于 fixture `group_concat(id,0x7e,username,0x7e,idcard)` 中 0x7e → '~'
/// 的展示层高亮。仅返回字面量解码后的首个 ASCII 字符；无字面量返回 None。
/// 字面量值 > 127（非 ASCII）返回 None。
///
/// v0.5.0 T12-1：拆分后由 [`super::aggregate`] 跨模块调用，故标 `pub(super)`。
pub(super) fn parse_separator_char(read_target: &str) -> Option<char> {
    let re = hex_literal_regex();
    let caps = re.captures(read_target)?;
    let hex = caps.get(1)?.as_str();
    let val = u32::from_str_radix(hex, 16).ok()?;
    if val > 0x7f {
        return None;
    }
    char::from_u32(val)
}

fn hex_literal_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"0x([0-9a-fA-F]{2})").expect("hex_literal regex must compile")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogEntry;

    fn mk_entry(
        line_no: usize,
        ip: &str,
        decoded_query: Option<&str>,
        size: Option<u64>,
    ) -> LogEntry {
        LogEntry {
            line_no,
            ip: ip.to_string(),
            timestamp: "17/Nov/2023:03:45:42 +0000".to_string(),
            method: "GET".to_string(),
            path: "/".to_string(),
            query: decoded_query.map(|s| s.to_string()),
            status: 200,
            size,
            user_agent: "curl".to_string(),
            raw: String::new(),
            decoded_path: String::new(),
            decoded_query: decoded_query.map(|s| s.to_string()),
            decoded_ua: String::new(),
        }
    }

    // ---- v0.2.4 reconstruct_database 数据库结构还原 ----

    fn mk_result(read_target: &str, decoded_string: &str, probe_count: u32) -> AggregatedResult {
        AggregatedResult {
            read_target: read_target.to_string(),
            decoded_string: decoded_string.to_string(),
            resolved_chars: decoded_string.chars().count() as u32,
            unresolved_chars: 0,
            beyond_end_positions: 0,
            probe_count,
            source_ips: vec!["1.1.1.1".to_string()],
            position_details: Vec::new(),
            separator_char: parse_separator_char(read_target),
            kind: Some("ascii_binary".to_string()),
        }
    }

    #[test]
    fn reconstruct_full_fixture_database_view() {
        // 合成 4 个 AggregatedResult，模拟 fixture access.log 的盲注聚合结果。
        let results = vec![
            mk_result("database()", "person", 100),
            mk_result(
                "select group_concat(table_name) from information_schema.tables where table_schema=database()",
                "person_data",
                200,
            ),
            mk_result(
                "select group_concat(column_name) from information_schema.columns where table_name='person_data'",
                "id,username,password,sex,birth,idcard,phone",
                300,
            ),
            mk_result(
                "select group_concat(id,0x7e,username,0x7e,idcard) from person_data",
                "1~zhangsan~IDCARD1,2~lisi~IDCARD2",
                400,
            ),
        ];
        let db = BlindAggregator::reconstruct_database(&results);
        assert_eq!(db.schema.as_deref(), Some("person"));
        assert_eq!(db.tables.len(), 1, "expect 1 table");
        assert!(db.unmatched_results.is_empty(), "no unmatched");

        let table = &db.tables[0];
        assert_eq!(table.name, "person_data");
        assert_eq!(
            table.columns,
            vec!["id", "username", "password", "sex", "birth", "idcard", "phone"],
            "full columns from column_list",
        );
        assert_eq!(
            table.row_data_columns,
            vec!["id", "username", "idcard"],
            "row_data_columns parsed from group_concat args",
        );
        assert_eq!(table.column_separator, Some('~'));
        assert_eq!(table.rows.len(), 2, "2 rows");
        // row 0: 1~zhangsan~IDCARD1
        assert_eq!(table.rows[0].cells.len(), 7);
        assert_eq!(table.rows[0].cells[0].as_deref(), Some("1"));
        assert_eq!(table.rows[0].cells[1].as_deref(), Some("zhangsan"));
        assert_eq!(table.rows[0].cells[2], None, "password not fetched");
        assert_eq!(table.rows[0].cells[3], None, "sex not fetched");
        assert_eq!(table.rows[0].cells[4], None, "birth not fetched");
        assert_eq!(table.rows[0].cells[5].as_deref(), Some("IDCARD1"));
        assert_eq!(table.rows[0].cells[6], None, "phone not fetched");
        // row 1: 2~lisi~IDCARD2
        assert_eq!(table.rows[1].cells[0].as_deref(), Some("2"));
        assert_eq!(table.rows[1].cells[1].as_deref(), Some("lisi"));
        assert_eq!(table.rows[1].cells[5].as_deref(), Some("IDCARD2"));
        assert_eq!(table.source_probe_count, 300 + 400);
    }

    #[test]
    fn reconstruct_unmatched_results_collected() {
        // 非标准 RT → unmatched_results。
        let results = vec![
            mk_result("database()", "person", 100),
            mk_result("some weird payload", "xyz", 50),
        ];
        let db = BlindAggregator::reconstruct_database(&results);
        assert_eq!(db.schema.as_deref(), Some("person"));
        assert_eq!(db.tables.len(), 0, "no tables");
        assert_eq!(db.unmatched_results.len(), 1, "1 unmatched");
        assert_eq!(db.unmatched_results[0].read_target, "some weird payload");
    }

    #[test]
    fn reconstruct_partial_no_column_list_fallback() {
        // 只有 database() + row_data（无 table_name/column_name）→ columns 用
        // row_data_columns 兜底。
        let results = vec![
            mk_result("database()", "person", 100),
            mk_result(
                "select group_concat(id,0x7e,username,0x7e,idcard) from person_data",
                "1~zhangsan~IDCARD1",
                400,
            ),
        ];
        let db = BlindAggregator::reconstruct_database(&results);
        assert_eq!(db.schema.as_deref(), Some("person"));
        assert_eq!(db.tables.len(), 1);
        let table = &db.tables[0];
        assert_eq!(table.name, "person_data");
        // 无 column_list → columns 用 row_data_columns 兜底。
        assert_eq!(table.columns, vec!["id", "username", "idcard"]);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells[0].as_deref(), Some("1"));
        assert_eq!(table.rows[0].cells[1].as_deref(), Some("zhangsan"));
        assert_eq!(table.rows[0].cells[2].as_deref(), Some("IDCARD1"));
    }

    #[test]
    fn reconstruct_empty_results() {
        let db = BlindAggregator::reconstruct_database(&[]);
        assert!(db.schema.is_none());
        assert!(db.tables.is_empty());
        assert!(db.unmatched_results.is_empty());
    }

    #[test]
    fn reconstruct_no_separator_single_column_row() {
        // 行数据查询无 0xNN 分隔符，单列 group_concat(username) → 每行整体为单元格。
        let results = vec![mk_result(
            "select group_concat(username) from person_data",
            "zhangsan,lisi",
            50,
        )];
        let db = BlindAggregator::reconstruct_database(&results);
        assert_eq!(db.tables.len(), 1);
        let table = &db.tables[0];
        assert_eq!(table.name, "person_data");
        assert_eq!(table.columns, vec!["username"], "columns fallback to row_data_columns");
        assert_eq!(table.column_separator, None);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[0].as_deref(), Some("zhangsan"));
        assert_eq!(table.rows[1].cells[0].as_deref(), Some("lisi"));
    }

    #[test]
    fn parse_table_name_predicate_extracts_name() {
        assert_eq!(
            parse_table_name_predicate(
                "select group_concat(column_name) from information_schema.columns where table_name='person_data'"
            ),
            Some("person_data".to_string()),
        );
        assert_eq!(
            parse_table_name_predicate("select group_concat(column_name) from information_schema.columns"),
            None,
        );
    }

    #[test]
    fn parse_from_table_extracts_name() {
        assert_eq!(
            parse_from_table("select group_concat(id,0x7e,username,0x7e,idcard) from person_data"),
            Some("person_data".to_string()),
        );
        assert_eq!(
            parse_from_table("select group_concat(table_name) from information_schema.tables where table_schema=database()"),
            Some("information_schema".to_string()),
        );
    }

    #[test]
    fn parse_row_data_columns_filters_hex_literals() {
        let (cols, sep) = parse_row_data_columns(
            "select group_concat(id,0x7e,username,0x7e,idcard) from person_data",
        );
        assert_eq!(cols, vec!["id", "username", "idcard"]);
        assert_eq!(sep, Some('~'));
    }

    // 保留一个 mk_entry 引用以避免未使用警告（mk_entry 在子模块测试中暂未直接调用，
    // 但保留它以便未来扩展时与原文件测试结构一致）。
    #[test]
    #[allow(unused_variables)]
    fn mk_entry_compiles() {
        let e = mk_entry(1, "1.1.1.1", Some("ascii(substr((database()),1,1))>79"), Some(875));
        // 仅验证 helper 可用，不执行断言。
    }
}
