//! 操作日志 / 替换 / Base64 变换 / 可撤销操作列表。
//!
//! T91 从 `db/mod.rs` 拆出；方法签名 / SQL / 测试逻辑保持不变。

use super::{map_row_to_cell, now_rfc3339, Cell, DbManager, DbError, OperationRow, UndoableOpRow};
use regex::Regex;
use rusqlite::params;

impl DbManager {
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
        let conn = self.conn();
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO operations (sheet_id, kind, params_json, before_snapshot_json, result_snapshot_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![sheet_id, kind, params_json, before_snapshot_json, result_json, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 按 id 查询单条 operation（撤销/重做用）。不存在返回 `Ok(None)`。
    pub fn query_operation_by_id(&self, op_id: i64) -> Result<Option<OperationRow>, DbError> {
        let conn = self.conn();
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
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        // 抓 before 快照（限定列 / 全表，排除表头 row_idx=0）。
        let before: Vec<Cell> = {
            let mut out = Vec::new();
            if let Some(c) = col_filter {
                let mut stmt = tx.prepare(
                    "SELECT sheet_id, row_idx, col_idx, value FROM cells
                     WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                     ORDER BY row_idx ASC, col_idx ASC",
                )?;
                let rows = stmt.query_map(params![sheet_id, c as i64], map_row_to_cell)?;
                for r in rows {
                    out.push(r?);
                }
            } else {
                let mut stmt = tx.prepare(
                    "SELECT sheet_id, row_idx, col_idx, value FROM cells
                     WHERE sheet_id = ?1 AND row_idx > 0
                     ORDER BY row_idx ASC, col_idx ASC",
                )?;
                let rows = stmt.query_map(params![sheet_id], map_row_to_cell)?;
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

    /// Base64 编/解码列数据（就地变更）。单事务：查 before 快照 → 对每行
    /// 调用 `transform` → 筛变化行 → 批量 upsert after 值 → 返回
    /// `(affected, skipped, before_changed, after_changed)`。
    ///
    /// `transform` 接受当前值 `&str`，返回：
    /// - `Some(new_val)`：转换成功；与原值不同则计入 `affected`，相同则忽略。
    /// - `None`：该行跳过（如 Base64 解码失败 / 非 UTF-8 字节），计入 `skipped`。
    ///
    /// 空值（`None`）行不参与变换，不计入 `affected` 也不计入 `skipped`
    /// （与 `replace_cells_inner` 语义一致：`None` 不入变换）。
    ///
    /// 用闭包而非 `Base64Mode` 入参，使 DB 层不依赖 commands 层；转换逻辑
    /// （`base64` crate 编/解码）由调用方在闭包内实现。SQL 全部用 `?N` +
    /// `params![]` 绑定，禁止字符串拼接。
    pub fn base64_transform_column_cells<F>(
        &self,
        sheet_id: i64,
        col_idx: u32,
        transform: F,
    ) -> Result<(u32, u32, Vec<Cell>, Vec<Cell>), DbError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        // 抓 before 快照（限定列，排除表头 row_idx=0）。
        let before: Vec<Cell> = {
            let mut out = Vec::new();
            let mut stmt = tx.prepare(
                "SELECT sheet_id, row_idx, col_idx, value FROM cells
                 WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                 ORDER BY row_idx ASC",
            )?;
            let rows = stmt.query_map(params![sheet_id, col_idx as i64], map_row_to_cell)?;
            for r in rows {
                out.push(r?);
            }
            out
        };
        // 计算转换值，筛有变化的行；transform 返回 None 的行计入 skipped。
        let mut before_changed: Vec<Cell> = Vec::new();
        let mut after_changed: Vec<Cell> = Vec::new();
        let mut skipped: u32 = 0;
        for c in &before {
            match c.value.as_deref() {
                Some(val) => match transform(val) {
                    Some(new_val) if new_val != val => {
                        before_changed.push(c.clone());
                        after_changed.push(Cell {
                            sheet_id: c.sheet_id,
                            row_idx: c.row_idx,
                            col_idx: c.col_idx,
                            value: Some(new_val),
                        });
                    }
                    Some(_) => { /* 转换后与原值相同，不变 */ }
                    None => skipped += 1, // transform 返回 None（如 decode 失败）
                },
                None => { /* 空值行不参与变换 */ }
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
        Ok((affected, skipped, before_changed, after_changed))
    }

    /// 列出某 sheet 的可撤销操作（最近 `limit` 条，按 `created_at` DESC）。
    ///
    /// 仅返回 kind ∈ {`mask`, `replace_in_column`, `replace_all`,
    /// `base64_column`, `hash_column`} 的操作——即「就地变更」类操作；
    /// `import`/`validate`/`extract`/`undo`/`redo` 等只读或辅助操作不入撤销栈。
    /// `kind` 值是硬编码常量（非用户输入），IN 子句无注入风险；
    /// `sheet_id`/`limit` 仍用 `?N` + `params![]` 绑定。
    pub fn list_undoable_operations(
        &self,
        sheet_id: i64,
        limit: u32,
    ) -> Result<Vec<UndoableOpRow>, DbError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, kind, created_at FROM operations
             WHERE sheet_id = ?1 AND kind IN ('mask', 'replace_in_column', 'replace_all', 'base64_column', 'hash_column', 'transform_column')
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

#[cfg(test)]
mod tests {
    use crate::db::test_support;

    #[test]
    fn log_operation_inserts_row() {
        let (_dir, mgr) = test_support::open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        let id = mgr.log_operation(Some(shid), "import", "{}", "{}").unwrap();
        assert!(id > 0);
    }

    #[test]
    fn query_operation_by_id_basic() {
        let (_dir, mgr) = test_support::open();
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
        let (_dir, mgr) = test_support::open();
        let id = mgr.log_operation(None, "import", "{}", "{}").unwrap();
        let row = mgr.query_operation_by_id(id).unwrap().unwrap();
        assert_eq!(row.before_snapshot_json, None);
        assert_eq!(row.result_snapshot_json.as_deref(), Some("{}"));
    }

    #[test]
    fn replace_in_column_cells_returns_before_snapshot() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
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
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
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
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
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
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        let res = mgr.replace_in_column_cells(shid, 0, "[bad", "x", true);
        assert!(res.is_err());
    }

    #[test]
    fn replace_all_cells_returns_both_snapshots() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
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

    /// 端到端等价性：新流程（单次扫描 + 批量取行）vs 旧逐行流程
    /// （`search_matched_row_ids_regex` + `count_matched_rows_regex` + 逐行
    /// `query_row_cells`）的结果应完全一致，含 cells 与 hits。
    ///
    /// 注：`compute_regex_matches` / `RowCellMatch` / `SearchRow` 在命令层是
    /// 私有辅助，本测试通过 `super::super::commands::search` 路径访问仅用于
    /// 跨模块等价性验证——不构成公开 API。
    #[test]
    fn search_rows_regex_single_scan_matches_legacy_flow() {
        // compute_regex_matches 在 commands::search 是私有函数；测试用 inline
        // 等价实现复刻命令层逻辑，避免暴露内部 API。
        fn compute_regex_spans(value: &Option<String>, pattern: &str) -> Vec<(usize, usize)> {
            let mut out = Vec::new();
            if let Some(val) = value {
                if pattern.is_empty() {
                    return out;
                }
                if let Ok(re) = regex::Regex::new(pattern) {
                    for m in re.find_iter(val) {
                        out.push((m.start(), m.end()));
                    }
                }
            }
            out
        }

        // 复刻 commands::search 的 RowCellMatch / SearchRow 形状用于比对。
        #[derive(Debug, Clone, PartialEq)]
        struct RowHit {
            col_idx: u32,
            value: Option<String>,
            matches: Vec<(usize, usize)>,
        }
        #[derive(Debug, Clone, PartialEq)]
        struct RowResult {
            row_idx: u32,
            cells: Vec<Option<String>>,
            hits: Vec<RowHit>,
        }

        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        let col_count = mgr.count_columns(shid).unwrap() as usize;
        let pattern = r"\d{3}";
        let offset = 0u32;
        let page_size = 50u32;

        // 新流程：search_matched_rows_regex_with_total + query_row_cells_batch。
        let (row_ids_new, total_new) = mgr
            .search_matched_rows_regex_with_total(shid, None, pattern, offset, page_size)
            .unwrap();
        let cells_raw_new = mgr.query_row_cells_batch(shid, &row_ids_new).unwrap();
        let mut cells_iter = cells_raw_new.into_iter().peekable();
        let mut rows_new: Vec<RowResult> = Vec::with_capacity(row_ids_new.len());
        for &rid in &row_ids_new {
            let mut cells: Vec<Option<String>> = vec![None; col_count];
            let mut hits: Vec<RowHit> = Vec::new();
            while let Some(c) = cells_iter.peek() {
                if c.row_idx != rid {
                    break;
                }
                let c = cells_iter.next().unwrap();
                let ci = c.col_idx as usize;
                if ci < col_count {
                    cells[ci] = c.value.clone();
                }
                let spans = compute_regex_spans(&c.value, pattern);
                if !spans.is_empty() {
                    hits.push(RowHit {
                        col_idx: c.col_idx,
                        value: c.value.clone(),
                        matches: spans,
                    });
                }
            }
            rows_new.push(RowResult {
                row_idx: rid,
                cells,
                hits,
            });
        }

        // 旧流程：search_matched_row_ids_regex + count_matched_rows_regex + 逐行 query_row_cells。
        let row_ids_old = mgr
            .search_matched_row_ids_regex(shid, None, pattern, offset, page_size)
            .unwrap();
        let total_old = mgr.count_matched_rows_regex(shid, None, pattern).unwrap();
        let mut rows_old: Vec<RowResult> = Vec::with_capacity(row_ids_old.len());
        for rid in row_ids_old {
            let cells_raw = mgr.query_row_cells(shid, rid).unwrap();
            let mut cells: Vec<Option<String>> = vec![None; col_count];
            let mut hits: Vec<RowHit> = Vec::new();
            for c in &cells_raw {
                let ci = c.col_idx as usize;
                if ci < col_count {
                    cells[ci] = c.value.clone();
                }
                let spans = compute_regex_spans(&c.value, pattern);
                if !spans.is_empty() {
                    hits.push(RowHit {
                        col_idx: c.col_idx,
                        value: c.value.clone(),
                        matches: spans,
                    });
                }
            }
            rows_old.push(RowResult {
                row_idx: rid,
                cells,
                hits,
            });
        }

        // 等价性断言：total、行数、每行的 cells 和 hits。
        assert_eq!(total_new, total_old);
        assert_eq!(rows_new, rows_old);
    }

    /// UTF-8 命中区间在批量 cells 路径下保持正确（关键字模式 byte-offset）。
    #[test]
    fn search_rows_keyword_utf8_byte_offsets_preserved() {
        // compute_keyword_matches 在 commands::search 是私有；inline 等价实现。
        fn compute_keyword_spans(value: &Option<String>, query: &str) -> Vec<(usize, usize)> {
            let mut out = Vec::new();
            if let Some(val) = value {
                if query.is_empty() {
                    return out;
                }
                let mut start = 0;
                while let Some(pos) = val[start..].find(query) {
                    let abs_start = start + pos;
                    let abs_end = abs_start + query.len();
                    out.push((abs_start, abs_end));
                    start = abs_end;
                }
            }
            out
        }

        #[derive(Debug, Clone, PartialEq)]
        struct RowHit {
            col_idx: u32,
            value: Option<String>,
            matches: Vec<(usize, usize)>,
        }
        #[derive(Debug, Clone, PartialEq)]
        struct RowResult {
            row_idx: u32,
            cells: Vec<Option<String>>,
            hits: Vec<RowHit>,
        }

        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        let col_count = mgr.count_columns(shid).unwrap() as usize;
        let query = "张";
        let offset = 0u32;
        let page_size = 50u32;

        // 关键字路径：search_matched_row_ids + count_matched_rows + query_row_cells_batch。
        let row_ids = mgr
            .search_matched_row_ids(shid, None, query, offset, page_size)
            .unwrap();
        let total = mgr.count_matched_rows(shid, None, query).unwrap();
        assert_eq!(total, 2);
        assert_eq!(row_ids, vec![1, 3]);

        let cells_raw = mgr.query_row_cells_batch(shid, &row_ids).unwrap();
        let mut cells_iter = cells_raw.into_iter().peekable();
        let mut rows: Vec<RowResult> = Vec::with_capacity(row_ids.len());
        for &rid in &row_ids {
            let mut cells: Vec<Option<String>> = vec![None; col_count];
            let mut hits: Vec<RowHit> = Vec::new();
            while let Some(c) = cells_iter.peek() {
                if c.row_idx != rid {
                    break;
                }
                let c = cells_iter.next().unwrap();
                let ci = c.col_idx as usize;
                if ci < col_count {
                    cells[ci] = c.value.clone();
                }
                let spans = compute_keyword_spans(&c.value, query);
                if !spans.is_empty() {
                    hits.push(RowHit {
                        col_idx: c.col_idx,
                        value: c.value.clone(),
                        matches: spans,
                    });
                }
            }
            rows.push(RowResult {
                row_idx: rid,
                cells,
                hits,
            });
        }

        // 每行 col0 应命中 "张"，区间为 byte-offset 0..3（UTF-8 三字节）。
        assert_eq!(rows.len(), 2);
        for r in &rows {
            assert!(r.cells[0].as_deref().unwrap().contains('张'));
            let hit = r.hits.iter().find(|h| h.col_idx == 0).unwrap();
            assert_eq!(hit.matches.len(), 1);
            let span = &hit.matches[0];
            assert_eq!(span.0, 0);
            assert_eq!(span.1, 3); // "张" UTF-8 占 3 字节
            let val = hit.value.as_ref().unwrap();
            assert_eq!(&val[span.0..span.1], "张");
        }
    }
}
