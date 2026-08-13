//! 搜索：关键字 / 正则 cell 搜索、行级搜索、计数与分页。
//!
//! T91 从 `db/mod.rs` 拆出；方法签名 / SQL / 测试逻辑保持不变。

use super::{escape_like, Cell, DbManager, DbError, RegexSearchResult};
use regex::Regex;
use rusqlite::params;

// v1.1+ IPC 将调用；单测已覆盖。
#[allow(
    dead_code,
    reason = "v1.1+ IPC 将接入；单测已覆盖"
)]
impl DbManager {
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
    /// 服务端分页（`offset/limit` 作用于 distinct row_idx）。
    /// 与 `search_matched_row_ids` 的区别：用 `regex::Regex` 二次精确匹配，
    /// 而非 `LIKE`。语义与 `search_cells_regex` 对齐。
    ///
    /// T59：改为复用 `scan_regex_matched_row_ids` 单次候选扫描——一次性取出
    /// `(row_idx, value)` 全部候选 cell（`LIKE '%'` 预筛 + `row_idx>0`），在
    /// Rust 侧逐 cell 跑 `regex::is_match`，按 `row_idx` 升序去重收集命中行，
    /// 最后切片分页。不再对每个 row_idx 单独执行 `SELECT`（消除 N+1）。
    /// NULL value（`None`）按既有语义跳过。
    pub fn search_matched_row_ids_regex(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        pattern: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<u32>, DbError> {
        let all = self.scan_regex_matched_row_ids(sheet_id, col_idx, pattern)?;
        let start = (offset as usize).min(all.len());
        let end = (start + limit as usize).min(all.len());
        Ok(all[start..end].to_vec())
    }

    /// 统计正则搜索命中的 **行数**。
    ///
    /// T59：改为复用 `scan_regex_matched_row_ids` 单次扫描取长度，
    /// 不再通过 `search_matched_row_ids_regex(0, u32::MAX)` 递归复用完整搜索。
    pub fn count_matched_rows_regex(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        pattern: &str,
    ) -> Result<u32, DbError> {
        let all = self.scan_regex_matched_row_ids(sheet_id, col_idx, pattern)?;
        Ok(all.len() as u32)
    }

    /// 正则行级搜索（单次扫描）：返回 `(当前页 row_idx, 命中行总数)`。
    /// 一次候选扫描同时给出分页结果与 total，避免 `count_matched_rows_regex`
    /// 通过 `u32::MAX` 递归复用完整搜索（T59）。
    pub fn search_matched_rows_regex_with_total(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        pattern: &str,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<u32>, u32), DbError> {
        let all = self.scan_regex_matched_row_ids(sheet_id, col_idx, pattern)?;
        let total = all.len() as u32;
        let start = (offset as usize).min(all.len());
        let end = (start + limit as usize).min(all.len());
        Ok((all[start..end].to_vec(), total))
    }

    /// 正则行级搜索的共享单次扫描核心（T59）。
    ///
    /// 一次性以 `LIKE '%' ESCAPE '\'` 预筛 `row_idx>0` 的全部候选 cell，
    /// 取 `(row_idx, value)`，按 `row_idx`、`col_idx` 升序遍历，逐 cell 跑
    /// `regex::is_match`（`None` value 跳过——保留 NULL 跳过语义），首次命中
    /// 的 row_idx 入队（行已按 `row_idx` 排序，故去重只需比较前一行）。
    /// 返回全部命中 row_idx（升序、去重），由调用方切片分页或取长度。
    fn scan_regex_matched_row_ids(
        &self,
        sheet_id: i64,
        col_idx: Option<u32>,
        pattern: &str,
    ) -> Result<Vec<u32>, DbError> {
        let re = Regex::new(pattern)
            .map_err(|e| DbError::Migration(format!("invalid regex `{}`: {}", pattern, e)))?;
        let conn = self.conn.lock().expect("db mutex poisoned");
        let like_pattern = "%".to_string();
        let map_row = |r: &rusqlite::Row<'_>| -> rusqlite::Result<(u32, Option<String>)> {
            Ok((r.get::<_, i64>(0)? as u32, r.get::<_, Option<String>>(1)?))
        };
        // 单次扫描：取 (row_idx, value) 全部候选行。col_idx 限定列时每行一值；
        // 全表搜索时按 (row_idx, col_idx) 升序遍历，任意列命中即计入该行。
        let mut stmt = if col_idx.is_some() {
            conn.prepare(
                "SELECT row_idx, value FROM cells
                 WHERE sheet_id = ?1 AND col_idx = ?2 AND row_idx > 0
                   AND value LIKE ?3 ESCAPE '\\'
                 ORDER BY row_idx ASC, col_idx ASC",
            )?
        } else {
            conn.prepare(
                "SELECT row_idx, value FROM cells
                 WHERE sheet_id = ?1 AND row_idx > 0
                   AND value LIKE ?2 ESCAPE '\\'
                 ORDER BY row_idx ASC, col_idx ASC",
            )?
        };
        let rows = if let Some(c) = col_idx {
            stmt.query_map(params![sheet_id, c as i64, like_pattern], map_row)?
        } else {
            stmt.query_map(params![sheet_id, like_pattern], map_row)?
        };
        let mut matched: Vec<u32> = Vec::new();
        let mut prev: Option<u32> = None;
        for row in rows {
            let (rid, value) = row?;
            // NULL 跳过：None value 不参与匹配。
            if let Some(ref val) = value {
                if re.is_match(val) && prev != Some(rid) {
                    matched.push(rid);
                    prev = Some(rid);
                }
            }
        }
        Ok(matched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support;

    #[test]
    fn query_column_cells_with_row_idx_range_paged() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
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
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
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
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        // 搜 "name"（表头）→ row_idx=0 应被排除，返回空。
        let hits = mgr.search_cells(shid, None, "name", 0, 10).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn search_cells_escapes_like_special_chars() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
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
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
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
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        // 非法正则（未闭合 `[`）→ 返回 Err，不 panic。
        let res = mgr.search_cells_regex(shid, None, "[unclosed", 0, 10);
        assert!(res.is_err());
    }

    #[test]
    fn search_cells_regex_pagination() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        // 全表正则 \d → phone 列 3 命中 + memo "50%" 1 命中 = 4。
        let all = mgr.search_cells_regex(shid, None, r"\d", 0, 100).unwrap();
        assert_eq!(all.len(), 4);
        // offset=2 limit=1 → 取第 3 条。
        let p = mgr.search_cells_regex(shid, None, r"\d", 2, 1).unwrap();
        assert_eq!(p.len(), 1);
    }

    #[test]
    fn count_search_results_basic() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        // 全表搜 "1" → phone 3 行 + memo "50%" 0 = 3。
        let count = mgr.count_search_results(shid, None, "1").unwrap();
        assert_eq!(count, 3);
        // col1 搜 "139" → 1。
        let count_col = mgr.count_search_results(shid, Some(1), "139").unwrap();
        assert_eq!(count_col, 1);
    }

    #[test]
    fn count_search_results_escapes_special_chars() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        // 搜字面 "%" → 仅 "50%" 命中，count=1。
        let count = mgr.count_search_results(shid, Some(2), "%").unwrap();
        assert_eq!(count, 1);
    }

    // ---- T59：搜索行级 N+1 消除 + 正则重扫消除 ----

    /// 新方法 `search_matched_rows_regex_with_total` 的结果应与旧
    /// `search_matched_row_ids_regex` + `count_matched_rows_regex` 组合完全等价。
    #[test]
    fn search_rows_regex_with_total_matches_legacy() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        let pattern = r"\d{3}";
        // 全表正则：phone 列 3 行命中。
        let (ids_new, total_new) = mgr
            .search_matched_rows_regex_with_total(shid, None, pattern, 0, 50)
            .unwrap();
        let ids_old = mgr
            .search_matched_row_ids_regex(shid, None, pattern, 0, 50)
            .unwrap();
        let total_old = mgr.count_matched_rows_regex(shid, None, pattern).unwrap();
        assert_eq!(total_new, total_old);
        assert_eq!(total_new, 3);
        assert_eq!(ids_new, ids_old);
        assert_eq!(ids_new, vec![1, 2, 3]);
    }

    /// 新方法分页正确：offset/limit 作用于命中 row_idx 列表。
    #[test]
    fn search_rows_regex_with_total_pagination() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        let pattern = r"\d{3}";
        // total=3, page_size=1 offset=1 → 第 2 行（row_idx=2）。
        let (p1, total) = mgr
            .search_matched_rows_regex_with_total(shid, None, pattern, 1, 1)
            .unwrap();
        assert_eq!(total, 3);
        assert_eq!(p1, vec![2]);
        // offset=2 limit=2 → 剩余 1 行（row_idx=3）。
        let (p2, total2) = mgr
            .search_matched_rows_regex_with_total(shid, None, pattern, 2, 2)
            .unwrap();
        assert_eq!(total2, 3);
        assert_eq!(p2, vec![3]);
        // offset 越界 → 空页，total 仍为 3。
        let (p3, total3) = mgr
            .search_matched_rows_regex_with_total(shid, None, pattern, 10, 1)
            .unwrap();
        assert_eq!(total3, 3);
        assert!(p3.is_empty());
    }

    /// 新方法支持 col_idx 限定列过滤。
    #[test]
    fn search_rows_regex_with_total_col_filtered() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        let pattern = r"\d{3}";
        // col1 (phone) 3 行命中。
        let (ids, total) = mgr
            .search_matched_rows_regex_with_total(shid, Some(1), pattern, 0, 50)
            .unwrap();
        assert_eq!(total, 3);
        assert_eq!(ids, vec![1, 2, 3]);
        // col0 (name) 0 命中（无数字）。
        let (ids0, total0) = mgr
            .search_matched_rows_regex_with_total(shid, Some(0), pattern, 0, 50)
            .unwrap();
        assert_eq!(total0, 0);
        assert!(ids0.is_empty());
    }
}
