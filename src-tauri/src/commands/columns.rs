//! v1.1.1 列操作类 IPC 命令（JSON 展开解析 / 列内批量替换）。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（`find_col_idx` / `query_column_cells` / `replace_in_column_cells`
//! / `create_sheet` / `write_cells` / `log_operation` / `log_operation_with_snapshot`）。
//!
//! - `parse_column_as_json`：把某列每行当 JSON 解析，展开成多列写入新 sheet。
//!   新增 sheet（非就地变更），不纳入撤销栈（`before_snapshot=None`）；撤销 = 关闭 Tab。
//! - `replace_in_column`：列内批量字段替换，单事务 + before/after 快照，可撤销。
//!
//! 命令实现拆为 `_inner` 核心逻辑（接受 `&DbManager`，便于单测直接调用）+
//! 薄 `#[tauri::command]` 包装（仅做 `tauri::State` → `&DbManager` 解包）。

use std::collections::BTreeMap;

use serde::Serialize;

use crate::db::{Cell, DbManager};

// ---------------------------------------------------------------------------
// 结果结构体（camelCase 序列化）
// ---------------------------------------------------------------------------

/// JSON 解析结果（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub new_sheet_id: i64,
    pub headers: Vec<String>,
    pub row_count: u32,
    pub skipped: u32,
}

/// 替换结果（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceResult {
    pub affected: u32,
}

// ---------------------------------------------------------------------------
// parse_column_as_json
// ---------------------------------------------------------------------------

/// 把某列每行当 JSON 解析，展开成多列写入新 sheet。
///
/// - 读列全部数据行（排除表头 `row_idx=0`）→ 每行 `serde_json::from_str` 为
///   JSON object；解析失败的行跳过并计入 `skipped`，不 panic。
/// - 收集所有 key 作新表头（按首次出现顺序去重）。
/// - `create_sheet`（名为 `{column}_json`，position=0）+ `write_cells`
///   （表头行 `row_idx=0` + 数据行 `row_idx>=1`）。
/// - `log_operation("parse_json")`，`before_snapshot=None`（不可撤销，撤销=关闭 Tab）。
///
/// 嵌套对象的 value 用 `to_string()` 序列化（v1.1.1 仅展平一层）。
pub fn parse_column_as_json_inner(
    db: &DbManager,
    sheet_id: i64,
    column: &str,
    session_id: i64,
) -> Result<ParseResult, String> {
    let col_idx = db
        .find_col_idx(sheet_id, column)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("列 `{column}` 不存在"))?;

    let rows = db
        .query_column_cells(sheet_id, col_idx)
        .map_err(|e| e.to_string())?;

    // 解析每行为 JSON object，收集所有 key 作表头（按首次出现顺序去重）。
    let mut parsed: Vec<BTreeMap<String, serde_json::Value>> = Vec::new();
    let mut headers: Vec<String> = Vec::new(); // 保持插入顺序的去重
    let mut skipped: u32 = 0;

    for (_row_idx, value) in &rows {
        let val = value.as_deref().unwrap_or("");
        match serde_json::from_str::<serde_json::Value>(val) {
            Ok(serde_json::Value::Object(map)) => {
                let mut row_map = BTreeMap::new();
                for (k, v) in map.into_iter() {
                    if !headers.contains(&k) {
                        headers.push(k.clone());
                    }
                    row_map.insert(k, v);
                }
                parsed.push(row_map);
            }
            _ => {
                skipped += 1;
            }
        }
    }

    let row_count = parsed.len() as u32;

    // 创建新 sheet（position=0；前端通过 ADD_SHEET_FROM_PARSE action 追加到 sheets 数组）。
    let new_sheet_name = format!("{column}_json");
    let new_sheet_id = db
        .create_sheet(session_id, &new_sheet_name, 0)
        .map_err(|e| e.to_string())?;

    // 写表头行（row_idx=0）+ 数据行。
    let mut cells: Vec<Cell> = Vec::new();
    for (col, header) in headers.iter().enumerate() {
        cells.push(Cell {
            sheet_id: new_sheet_id,
            row_idx: 0,
            col_idx: col as u32,
            value: Some(header.clone()),
        });
    }
    for (row, row_map) in parsed.iter().enumerate() {
        for (col, header) in headers.iter().enumerate() {
            let cell_value = row_map.get(header).map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            });
            // 空串视为 None（与 write_cells 语义一致，避免存大量空值）。
            let cell_value = cell_value.filter(|s| !s.is_empty());
            cells.push(Cell {
                sheet_id: new_sheet_id,
                row_idx: (row + 1) as u32, // row_idx=0 是表头
                col_idx: col as u32,
                value: cell_value,
            });
        }
    }

    if !cells.is_empty() {
        db.write_cells(new_sheet_id, &cells)
            .map_err(|e| e.to_string())?;
    }

    // parse_json 不纳入撤销栈（新增 sheet，撤销 = 关闭 Tab）。
    db.log_operation(
        Some(new_sheet_id),
        "parse_json",
        &serde_json::json!({
            "sourceSheetId": sheet_id,
            "column": column,
            "rowCount": row_count,
            "skipped": skipped
        })
        .to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;

    Ok(ParseResult {
        new_sheet_id,
        headers,
        row_count,
        skipped,
    })
}

/// `parse_column_as_json` 的 Tauri 命令包装。
#[tauri::command]
pub fn parse_column_as_json(
    sheet_id: i64,
    column: String,
    session_id: i64,
    db: tauri::State<'_, DbManager>,
) -> Result<ParseResult, String> {
    parse_column_as_json_inner(&db, sheet_id, &column, session_id)
}

// ---------------------------------------------------------------------------
// replace_in_column
// ---------------------------------------------------------------------------

/// 列内批量替换。调用 `replace_in_column_cells`（单事务，返回 before/after 快照）
/// → `log_operation_with_snapshot` 供撤销。返回受影响行数。
///
/// - `use_regex=false`：字面替换（`str::replace`）。
/// - `use_regex=true`：正则替换（`Regex::replace_all`）；编译失败返回错误。
/// - 仅返回有变化的行（before/after 一一对应），快照存变更列 cells。
pub fn replace_in_column_inner(
    db: &DbManager,
    sheet_id: i64,
    column: &str,
    from: &str,
    to: &str,
    use_regex: bool,
) -> Result<ReplaceResult, String> {
    let col_idx = db
        .find_col_idx(sheet_id, column)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("列 `{column}` 不存在"))?;

    let (affected, before_cells, after_cells) = db
        .replace_in_column_cells(sheet_id, col_idx, from, to, use_regex)
        .map_err(|e| e.to_string())?;

    let before_json = serde_json::to_string(&before_cells).map_err(|e| e.to_string())?;
    let after_json = serde_json::to_string(&after_cells).map_err(|e| e.to_string())?;

    db.log_operation_with_snapshot(
        Some(sheet_id),
        "replace_in_column",
        &serde_json::json!({
            "column": column,
            "from": from,
            "to": to,
            "useRegex": use_regex,
            "affected": affected
        })
        .to_string(),
        Some(&before_json),
        &after_json,
    )
    .map_err(|e| e.to_string())?;

    Ok(ReplaceResult { affected })
}

/// `replace_in_column` 的 Tauri 命令包装。
#[tauri::command]
pub fn replace_in_column(
    sheet_id: i64,
    column: String,
    from: String,
    to: String,
    use_regex: bool,
    db: tauri::State<'_, DbManager>,
) -> Result<ReplaceResult, String> {
    replace_in_column_inner(&db, sheet_id, &column, &from, &to, use_regex)
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Cell, DbManager};

    /// 构造 tempdir + 空 DbManager。返回 TempDir 跨测试保活。
    fn setup_db() -> (tempfile::TempDir, DbManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        (dir, mgr)
    }

    /// 构造一个 sheet：表头行 + 数据行（cells 由调用方提供完整结构，含表头）。
    /// 返回 (session_id, sheet_id)。
    fn setup_sheet(db: &DbManager, cells: &[Cell]) -> (i64, i64) {
        // cells[0].sheet_id 可能是占位 0，按需重写为真实 sheet_id。
        let session_id = db
            .create_session("test", None, "csv", cells.len() as u32)
            .unwrap();
        let sheet_id = db.create_sheet(session_id, "test", 0).unwrap();
        let rewritten: Vec<Cell> = cells
            .iter()
            .map(|c| Cell {
                sheet_id,
                ..c.clone()
            })
            .collect();
        db.write_cells(sheet_id, &rewritten).unwrap();
        (session_id, sheet_id)
    }

    /// 构造一个 cell（sheet_id 占位，setup_sheet 会重写）。
    fn cell(row: u32, col: u32, val: &str) -> Cell {
        Cell {
            sheet_id: 0,
            row_idx: row,
            col_idx: col,
            value: Some(val.into()),
        }
    }

    /// 读列全部数据行（排除表头），按 row_idx 升序返回字符串值（None→空串）。
    fn column_values(db: &DbManager, sheet_id: i64, col_idx: u32) -> Vec<String> {
        db.query_column_cells(sheet_id, col_idx)
            .unwrap()
            .into_iter()
            .map(|(_, v)| v.unwrap_or_default())
            .collect()
    }

    // 1. parse_column_as_json 对 {"a":1,"b":2} 列生成新 sheet，headers=[a,b]
    #[test]
    fn parse_column_as_json_expands_keys() {
        let (_dir, db) = setup_db();
        let (session_id, sheet_id) = setup_sheet(
            &db,
            &[
                cell(0, 0, "payload"),
                cell(1, 0, r#"{"a":1,"b":2}"#),
                cell(2, 0, r#"{"a":3,"b":4}"#),
            ],
        );

        let result = parse_column_as_json_inner(&db, sheet_id, "payload", session_id).unwrap();
        assert_eq!(result.headers, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(result.row_count, 2);
        assert_eq!(result.skipped, 0);

        // 新 sheet 表头行：row_idx=0, col_idx=0..2。
        let headers = db
            .query_cells(result.new_sheet_id, 1, 100)
            .unwrap()
            .into_iter()
            .filter(|c| c.row_idx == 0)
            .collect::<Vec<_>>();
        assert_eq!(headers.len(), 2);
        let header_vals: Vec<String> = headers
            .iter()
            .map(|c| c.value.clone().unwrap_or_default())
            .collect();
        assert!(header_vals.contains(&"a".to_string()));
        assert!(header_vals.contains(&"b".to_string()));

        // 数据行：a 列 1/3，b 列 2/4。
        let a_vals = column_values(&db, result.new_sheet_id, 0);
        let b_vals = column_values(&db, result.new_sheet_id, 1);
        assert_eq!(a_vals, vec!["1".to_string(), "3".to_string()]);
        assert_eq!(b_vals, vec!["2".to_string(), "4".to_string()]);
    }

    // 2. JSON 解析失败的行被跳过，skipped 正确
    #[test]
    fn parse_column_as_json_skips_invalid() {
        let (_dir, db) = setup_db();
        let (session_id, sheet_id) = setup_sheet(
            &db,
            &[
                cell(0, 0, "payload"),
                cell(1, 0, r#"{"a":1}"#),
                cell(2, 0, "not-json"),
                cell(3, 0, r#"{"a":2}"#),
                // 非对象 JSON（数组）也算解析失败跳过。
                cell(4, 0, r#"[1,2,3]"#),
            ],
        );

        let result = parse_column_as_json_inner(&db, sheet_id, "payload", session_id).unwrap();
        assert_eq!(result.headers, vec!["a".to_string()]);
        assert_eq!(result.row_count, 2); // 仅 2 个对象行
        assert_eq!(result.skipped, 2); // "not-json" + 数组
    }

    // 3. 不同行的 key 取并集作为表头
    #[test]
    fn parse_column_as_json_union_headers() {
        let (_dir, db) = setup_db();
        let (session_id, sheet_id) = setup_sheet(
            &db,
            &[
                cell(0, 0, "payload"),
                cell(1, 0, r#"{"a":1}"#),
                cell(2, 0, r#"{"b":2}"#),
                cell(3, 0, r#"{"a":3,"c":4}"#),
            ],
        );

        let result = parse_column_as_json_inner(&db, sheet_id, "payload", session_id).unwrap();
        // 按首次出现顺序：a（行1）→ b（行2）→ c（行3）。
        assert_eq!(result.headers, vec!["a", "b", "c"]);
        assert_eq!(result.row_count, 3);

        // 行2 无 a → None → 空串；行3 有 c。
        let a_vals = column_values(&db, result.new_sheet_id, 0);
        let b_vals = column_values(&db, result.new_sheet_id, 1);
        let c_vals = column_values(&db, result.new_sheet_id, 2);
        assert_eq!(
            a_vals,
            vec!["1".to_string(), String::new(), "3".to_string()]
        );
        assert_eq!(b_vals, vec![String::new(), "2".to_string(), String::new()]);
        assert_eq!(c_vals, vec![String::new(), String::new(), "4".to_string()]);
    }

    // 4. replace_in_column 该列命中值替换，其他列不变
    #[test]
    fn replace_in_column_replaces_only_target_column() {
        let (_dir, db) = setup_db();
        let (_session_id, sheet_id) = setup_sheet(
            &db,
            &[
                cell(0, 0, "name"),
                cell(0, 1, "phone"),
                cell(1, 0, "张三"),
                cell(1, 1, "13800000000"),
                cell(2, 0, "李四"),
                cell(2, 1, "13900000000"),
            ],
        );

        // name 列把 "张" → "王"。
        let result = replace_in_column_inner(&db, sheet_id, "name", "张", "王", false).unwrap();
        assert_eq!(result.affected, 1); // 仅 "张三" 命中

        // name 列已替换。
        let names = column_values(&db, sheet_id, 0);
        assert!(names.contains(&"王三".to_string()));
        assert!(names.contains(&"李四".to_string()));
        assert!(!names.contains(&"张三".to_string()));

        // phone 列不变。
        let phones = column_values(&db, sheet_id, 1);
        assert_eq!(
            phones,
            vec!["13800000000".to_string(), "13900000000".to_string()]
        );
    }

    // 5. replace_in_column 的 operations 行有 before_snapshot_json（供撤销）
    #[test]
    fn replace_in_column_logs_before_snapshot() {
        let (_dir, db) = setup_db();
        let (_session_id, sheet_id) = setup_sheet(
            &db,
            &[cell(0, 0, "name"), cell(1, 0, "张三"), cell(2, 0, "李四")],
        );

        let result = replace_in_column_inner(&db, sheet_id, "name", "张", "王", false).unwrap();
        assert_eq!(result.affected, 1);

        // 查最近一条 replace_in_column 操作，断言 before_snapshot_json 非空。
        let undoable = db.list_undoable_operations(sheet_id, 50).unwrap();
        let replace_op = undoable
            .iter()
            .find(|r| r.kind == "replace_in_column")
            .expect("应有 replace_in_column 操作记录");
        let op = db.query_operation_by_id(replace_op.id).unwrap().unwrap();
        assert!(
            op.before_snapshot_json.is_some(),
            "before_snapshot_json 必须存在供撤销"
        );
        let before_json = op.before_snapshot_json.unwrap();
        assert!(before_json.contains("张三"), "before 快照应含原始值 张三");

        // 撤销：把 before 快照回写 → 恢复原始值。
        let before_cells: Vec<Cell> = serde_json::from_str(&before_json).unwrap();
        db.write_cells(sheet_id, &before_cells).unwrap();
        let restored = column_values(&db, sheet_id, 0);
        assert!(restored.contains(&"张三".to_string()));
    }

    // 6. replace_in_column use_regex=true 时用正则替换
    #[test]
    fn replace_in_column_regex_mode() {
        let (_dir, db) = setup_db();
        let (_session_id, sheet_id) = setup_sheet(
            &db,
            &[
                cell(0, 0, "phone"),
                cell(1, 0, "13812345678"),
                cell(2, 0, "13987654321"),
                cell(3, 0, "not-a-phone"),
            ],
        );

        // 正则 \d+ → "#"：2 个纯数字行命中（"13812345678" / "13987654321"），
        // "not-a-phone" 0 命中（无数字）→ 2 行变化。
        let result = replace_in_column_inner(&db, sheet_id, "phone", r"\d+", "#", true).unwrap();
        assert_eq!(result.affected, 2);

        let phones = column_values(&db, sheet_id, 0);
        // 2 个数字串变 "#"，"not-a-phone" 不变。
        let masked_count = phones.iter().filter(|v| *v == "#").count();
        assert_eq!(masked_count, 2);
        assert!(phones.contains(&"not-a-phone".to_string()));
    }

    // 7. replace_in_column 非法正则返回错误
    #[test]
    fn replace_in_column_invalid_regex_returns_err() {
        let (_dir, db) = setup_db();
        let (_session_id, sheet_id) = setup_sheet(&db, &[cell(0, 0, "name"), cell(1, 0, "张三")]);

        let res = replace_in_column_inner(&db, sheet_id, "name", "[bad", "x", true);
        assert!(res.is_err(), "非法正则应返回错误");
        let err = res.unwrap_err();
        assert!(
            err.contains("regex") || err.contains("invalid"),
            "错误信息应提示正则问题: {err}"
        );

        // 列数据不应被修改（事务回滚 / 未写入）。
        let names = column_values(&db, sheet_id, 0);
        assert_eq!(names, vec!["张三".to_string()]);
    }
}
