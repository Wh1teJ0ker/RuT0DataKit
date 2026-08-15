//! `cells` 表读写：分页查询、批量写入、行/列计数、列查询、行查询等。
//!
//! T91 从 `db/mod.rs` 拆出；方法签名 / SQL / 测试逻辑保持不变。

use super::{map_row_to_cell, Cell, DbManager, DbError};
use rusqlite::{params, params_from_iter, types::Value as SqlValue};

impl DbManager {
    /// 分页查询 `cells`，按**行**分页（每页返回 `page_size` 个 `row_idx`
    /// 对应的全部列），结果按 `row_idx` 升序、`col_idx` 升序返回。
    ///
    /// `page` 从 1 开始；`page=0` 按 1 处理（`saturating_sub`）。
    /// 行级分页：先用子查询取本页的 `row_idx` 集合（`DISTINCT` +
    /// `LIMIT/OFFSET`），再取这些行的全部 cells——确保多列 Sheet
    /// 每页返回 `page_size` 行而非 `page_size` 个 cell。
    ///
    /// T62：分页与 total 语义统一为排除 `row_idx=0` 表头行——本方法只对
    /// 数据行（`row_idx > 0`）分页，表头行不占用任何页的行槽，也不计入
    /// `count_rows` 的 total。表头请用 `query_row_cells(sheet_id, 0)` 单独读取。
    pub fn query_cells(
        &self,
        sheet_id: i64,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Cell>, DbError> {
        let offset = (page.saturating_sub(1).saturating_mul(page_size)) as i64;
        let limit = page_size as i64;
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT sheet_id, row_idx, col_idx, value FROM cells
             WHERE sheet_id = ?1 AND row_idx > 0 AND row_idx IN (
                 SELECT DISTINCT row_idx FROM cells
                 WHERE sheet_id = ?1 AND row_idx > 0
                 ORDER BY row_idx ASC
                 LIMIT ?2 OFFSET ?3
             )
             ORDER BY row_idx ASC, col_idx ASC",
        )?;
        let rows = stmt.query_map(params![sheet_id, limit, offset], map_row_to_cell)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 事务批量写 `cells`（`sheet_id` 取参数，忽略 `Cell.sheet_id` 字段）。
    pub fn write_cells(&self, sheet_id: i64, cells: &[Cell]) -> Result<(), DbError> {
        let mut conn = self.conn();
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

    /// 统计某 sheet 的数据**行数**（`DISTINCT row_idx`，排除 `row_idx=0` 表头行）。
    ///
    /// T62：与 `query_cells` 分页语义统一——表头行不计入 total，分页 total
    /// 就是数据行数。表头请用 `query_row_cells(sheet_id, 0)` 单独读取。
    pub fn count_rows(&self, sheet_id: i64) -> Result<u32, DbError> {
        let conn = self.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT row_idx) FROM cells WHERE sheet_id = ?1 AND row_idx > 0",
            params![sheet_id],
            |r| r.get(0),
        )?;
        Ok(count.max(0) as u32)
    }

    /// 按 `sheet_id` 查 sheet 名（T57：用于双 Tab 命名 `{name}_校验通过`）。
    /// sheet 不存在返回 `Ok(None)`。
    pub fn get_sheet_name(&self, sheet_id: i64) -> Result<Option<String>, DbError> {
        let conn = self.conn();
        let name: Option<String> = conn
            .query_row(
                "SELECT name FROM sheets WHERE id = ?1",
                params![sheet_id],
                |r| r.get(0),
            )
            .ok();
        Ok(name)
    }

    /// 查询某列全部数据行（排除 `row_idx=0` 表头行），按 `row_idx` 升序。
    /// 返回 `(row_idx, value)` 列表，供 processor 命令读取列值。
    pub fn query_column_cells(
        &self,
        sheet_id: i64,
        col_idx: u32,
    ) -> Result<Vec<(u32, Option<String>)>, DbError> {
        let conn = self.conn();
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
        let conn = self.conn();
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

    /// 取某 sheet 的列数（`row_idx=0` 表头行的 cell 数）。供搜索结果行级展开对齐用。
    pub fn count_columns(&self, sheet_id: i64) -> Result<u32, DbError> {
        let conn = self.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM cells WHERE sheet_id = ?1 AND row_idx = 0",
            params![sheet_id],
            |r| r.get(0),
        )?;
        Ok(count.max(0) as u32)
    }

    /// 取某行所有列的 cells（按 `col_idx` 升序）。供搜索结果行级展开用。
    pub fn query_row_cells(&self, sheet_id: i64, row_idx: u32) -> Result<Vec<Cell>, DbError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT sheet_id, row_idx, col_idx, value FROM cells
             WHERE sheet_id = ?1 AND row_idx = ?2
             ORDER BY col_idx ASC",
        )?;
        let rows = stmt.query_map(params![sheet_id, row_idx as i64], map_row_to_cell)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 批量取多行的全部 cells（按 `row_idx IN (...)` 查询）。
    ///
    /// 替代逐行 `query_row_cells` 的 N+1 查询。结果按 `row_idx ASC, col_idx ASC`
    /// 排序。空入参返回空 Vec。
    ///
    /// 按 500 一批分块查询，规避 SQLITE_MAX_VARIABLE_NUMBER 限制。
    pub fn query_row_cells_batch(
        &self,
        sheet_id: i64,
        row_idxs: &[u32],
    ) -> Result<Vec<Cell>, DbError> {
        if row_idxs.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn();
        let mut out: Vec<Cell> = Vec::new();
        for chunk in row_idxs.chunks(500) {
            // 构造 IN 子句占位符：?, ?, ?
            let placeholders: Vec<&str> = std::iter::repeat_n("?", chunk.len()).collect();
            let in_clause = placeholders.join(", ");
            let sql = format!(
                "SELECT sheet_id, row_idx, col_idx, value FROM cells
                 WHERE sheet_id = ?1 AND row_idx IN ({in_clause})
                 ORDER BY row_idx ASC, col_idx ASC"
            );
            // 绑定参数：sheet_id + chunk 各 row_idx（i64 形式，与 schema 列类型一致）。
            let mut bind_args: Vec<SqlValue> = Vec::with_capacity(1 + chunk.len());
            bind_args.push(SqlValue::Integer(sheet_id));
            for &r in chunk {
                bind_args.push(SqlValue::Integer(r as i64));
            }
            let mut stmt = conn.prepare(&sql)?;
            let mapped = stmt.query_map(params_from_iter(bind_args.iter()), map_row_to_cell)?;
            for row in mapped {
                out.push(row?);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support;

    #[test]
    fn write_and_query_cells_paginated() {
        let (_dir, mgr) = test_support::open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Sheet1", 0).unwrap();
        // row_idx 0..5（5 行：0=表头，1..5=4 数据行）。
        let cells: Vec<Cell> = (0..5u32)
            .map(|i| Cell {
                sheet_id: shid,
                row_idx: i,
                col_idx: 0,
                value: Some(format!("v{}", i)),
            })
            .collect();
        mgr.write_cells(shid, &cells).unwrap();
        // T62：count_rows 排除 row_idx=0 表头 → 4 数据行。
        assert_eq!(mgr.count_rows(shid).unwrap(), 4);
        // 表头行单独读取（query_row_cells 不受 T62 排除影响）。
        let header = mgr.query_row_cells(shid, 0).unwrap();
        assert_eq!(header.len(), 1);
        assert_eq!(header[0].row_idx, 0);
        assert_eq!(header[0].value.as_deref(), Some("v0"));
        // 第 1 页 2 行：row_idx ∈ {1, 2}（表头已排除）。
        let p1 = mgr.query_cells(shid, 1, 2).unwrap();
        assert_eq!(p1.len(), 2);
        assert_eq!(p1[0].row_idx, 1);
        assert_eq!(p1[1].row_idx, 2);
        // 第 2 页 2 行：row_idx ∈ {3, 4}。
        let p2 = mgr.query_cells(shid, 2, 2).unwrap();
        assert_eq!(p2.len(), 2);
        assert_eq!(p2[0].row_idx, 3);
        assert_eq!(p2[1].row_idx, 4);
        // 第 3 页 0 行（数据行已取完）。
        let p3 = mgr.query_cells(shid, 3, 2).unwrap();
        assert!(p3.is_empty());
        // page=0 当作 1。
        let p0 = mgr.query_cells(shid, 0, 2).unwrap();
        assert_eq!(p0.len(), 2);
    }

    #[test]
    fn write_and_query_cells_paginated_multi_column() {
        // 多列 Sheet：3 列 × 5 行（0=表头，1..5=4 数据行），验证行级分页语义。
        let (_dir, mgr) = test_support::open();
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
        // T62：count_rows 排除表头 → 4 数据行（而非 cell 数 15）。
        assert_eq!(mgr.count_rows(shid).unwrap(), 4);
        // 第 1 页 2 行 → 6 个 cell（2 行 × 3 列），row_idx ∈ {1, 2}。
        let p1 = mgr.query_cells(shid, 1, 2).unwrap();
        assert_eq!(p1.len(), 6);
        assert!(p1.iter().all(|c| c.row_idx == 1 || c.row_idx == 2));
        // 第 2 页 2 行 → row_idx ∈ {3, 4}，6 个 cell。
        let p2 = mgr.query_cells(shid, 2, 2).unwrap();
        assert_eq!(p2.len(), 6);
        assert!(p2.iter().all(|c| c.row_idx == 3 || c.row_idx == 4));
        // 第 3 页 0 行（数据行已取完）。
        let p3 = mgr.query_cells(shid, 3, 2).unwrap();
        assert!(p3.is_empty());
        // 第 4 页 0 行。
        let p4 = mgr.query_cells(shid, 4, 2).unwrap();
        assert!(p4.is_empty());
    }

    #[test]
    fn write_cells_upsert_on_conflict() {
        let (_dir, mgr) = test_support::open();
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
        // T62：row_idx=0 是表头，count_rows 排除 → 0 数据行。
        assert_eq!(mgr.count_rows(shid).unwrap(), 0);
        // query_cells 也排除表头 → 空 Vec。
        let rows = mgr.query_cells(shid, 1, 10).unwrap();
        assert!(rows.is_empty());
        // upsert 仍生效：用 query_row_cells 取 row_idx=0 验证值为 "b"。
        let header = mgr.query_row_cells(shid, 0).unwrap();
        assert_eq!(header[0].value.as_deref(), Some("b"));
    }

    #[test]
    fn find_col_idx_and_query_column_cells() {
        let (_dir, mgr) = test_support::open();
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

    /// `query_row_cells_batch` 批量取行结果与逐行 `query_row_cells` 等价。
    #[test]
    fn query_row_cells_batch_returns_all_rows() {
        let (_dir, mgr) = test_support::open();
        let shid = test_support::build_search_sheet(&mgr);
        let row_idxs = vec![1u32, 2, 3];
        let batch = mgr.query_row_cells_batch(shid, &row_idxs).unwrap();
        // 与逐行查询拼接结果比对。
        let mut legacy: Vec<Cell> = Vec::new();
        for &rid in &row_idxs {
            let mut row_cells = mgr.query_row_cells(shid, rid).unwrap();
            legacy.append(&mut row_cells);
        }
        assert_eq!(batch.len(), legacy.len());
        // 逐 cell 比对（row_idx, col_idx, value）。
        for (b, l) in batch.iter().zip(legacy.iter()) {
            assert_eq!(b.row_idx, l.row_idx);
            assert_eq!(b.col_idx, l.col_idx);
            assert_eq!(b.value, l.value);
        }
        // 排序检查：按 row_idx ASC, col_idx ASC（逐字段比对，不依赖 Cell: PartialEq）。
        let mut sorted = batch.clone();
        sorted.sort_by(|a, b| a.row_idx.cmp(&b.row_idx).then(a.col_idx.cmp(&b.col_idx)));
        assert_eq!(batch.len(), sorted.len());
        for (b, s) in batch.iter().zip(sorted.iter()) {
            assert_eq!(b.row_idx, s.row_idx);
            assert_eq!(b.col_idx, s.col_idx);
            assert_eq!(b.value, s.value);
        }
    }

    /// `query_row_cells_batch` 处理 600 行（>500 分块阈值），结果完整且有序。
    #[test]
    fn query_row_cells_batch_handles_large_input() {
        let (_dir, mgr) = test_support::open();
        let sid = mgr.create_session("s", None, "csv", 0).unwrap();
        let shid = mgr.create_sheet(sid, "Big", 0).unwrap();
        // 表头 + 600 数据行，每行 1 列。
        let mut cells: Vec<Cell> = Vec::with_capacity(601);
        cells.push(test_support::cell(shid, 0, 0, "h"));
        for r in 1..=600u32 {
            cells.push(test_support::cell(shid, r, 0, &format!("v{r}")));
        }
        mgr.write_cells(shid, &cells).unwrap();
        // 取所有数据行。
        let row_idxs: Vec<u32> = (1..=600).collect();
        let batch = mgr.query_row_cells_batch(shid, &row_idxs).unwrap();
        assert_eq!(batch.len(), 600);
        // 顺序检查：row_idx 单调递增。
        for (i, c) in batch.iter().enumerate() {
            assert_eq!(c.row_idx, (i + 1) as u32);
            assert_eq!(c.value.as_deref(), Some(format!("v{}", i + 1).as_str()));
        }
        // 空入参返回空 Vec。
        let empty = mgr.query_row_cells_batch(shid, &[]).unwrap();
        assert!(empty.is_empty());
    }
}
