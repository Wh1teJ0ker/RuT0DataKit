//! v1.2.0 搜索 / 全表替换 IPC 命令。
//!
//! - `search_cells`：关键字（`LIKE '%kw%'` + `ESCAPE '\'`）/ 正则
//!   （`LIKE` 预筛 + Rust `regex` 精确匹配）搜索，分页返回命中 cells + 匹配区间。
//! - `replace_all`：全表搜索替换，调用 `replace_all_cells`（单事务，返回 before/after
//!   快照），`log_operation_with_snapshot` 记录操作供撤销。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 所有 SQL 由 `DbManager` 方法处理（参数绑定 `?N` + `params![]`），命令层不直接写 SQL。
//!
//! 测试策略（与 processor 命令一致）：`#[tauri::command]` 函数需要 Tauri State，无法
//! 在单元测试中直接调用。测试通过直接调用 `DbManager` 方法模拟命令内部流程，覆盖
//! 关键字 / 正则 / 分页 / 非法正则 / 全表替换 / 快照写入 / 正则替换等场景。

use serde::Serialize;

// ---------------------------------------------------------------------------
// 序列化数据结构（camelCase，供前端消费）
// ---------------------------------------------------------------------------

/// 搜索命中（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub row_idx: u32,
    pub col_idx: u32,
    pub value: Option<String>,
    /// 匹配区间列表（字节偏移 `start..end`，`end` exclusive）。
    /// 关键字模式：单个或多个区间（每次出现一个）；正则模式：所有 regex 命中。
    pub matches: Vec<MatchSpan>,
}

/// 匹配区间（字节偏移）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchSpan {
    pub start: usize,
    pub end: usize,
}

/// 搜索分页结果（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub rows: Vec<SearchHit>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

/// 替换结果（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceResult {
    pub affected: u32,
}

// ---------------------------------------------------------------------------
// 命令
// ---------------------------------------------------------------------------

/// 搜索 cells。
///
/// - `use_regex=false`（关键字模式）：`LIKE '%kw%'` + `ESCAPE '\'`，`count_search_results`
///   取总数；匹配区间由 `compute_keyword_matches` 按字面子串出现位置计算。
/// - `use_regex=true`（正则模式）：`search_cells_regex`（`LIKE` 预筛 + Rust `regex`
///   精确匹配）。正则模式无法用 SQL COUNT，取全量命中后切片分页，`total` 为全量命中数。
///
/// `page` 从 1 开始，`page_size` 为每页条数（均下界为 1，避免 0 除）。`col_idx=None`
/// 搜全表所有列，`Some(c)` 只搜指定列。空 `query` 返回空结果（不搜，不报错）。
#[tauri::command]
pub fn search_cells(
    sheet_id: i64,
    query: String,
    use_regex: bool,
    col_idx: Option<u32>,
    page: u32,
    page_size: u32,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<SearchPage, String> {
    let page = page.max(1);
    let page_size = page_size.max(1);
    let offset = (page - 1) * page_size;

    // 空 query：返回空结果，不报错（不搜）。
    if query.is_empty() {
        return Ok(SearchPage {
            rows: Vec::new(),
            total: 0,
            page,
            page_size,
        });
    }

    if use_regex {
        // 正则模式：单次取全量命中（search_cells_regex 内部已做 LIKE 预筛 + regex
        // 精确匹配，并在 Rust 侧分页切片）。此处取全量后自行切片分页 + 计数，避免
        // 跑两次 regex（HANDOFF 原方案跑两次，此处优化为一次）。
        let all = db
            .search_cells_regex(sheet_id, col_idx, &query, 0, u32::MAX)
            .map_err(|e| e.to_string())?;
        let total = all.len() as u32;
        let start = (offset as usize).min(all.len());
        let end = (start + page_size as usize).min(all.len());
        let rows = all[start..end]
            .iter()
            .map(|(cell, spans)| SearchHit {
                row_idx: cell.row_idx,
                col_idx: cell.col_idx,
                value: cell.value.clone(),
                matches: spans
                    .iter()
                    .map(|&(s, e)| MatchSpan { start: s, end: e })
                    .collect(),
            })
            .collect();
        Ok(SearchPage {
            rows,
            total,
            page,
            page_size,
        })
    } else {
        // 关键字模式：SQL LIKE 分页取数 + COUNT 取总数。
        let cells = db
            .search_cells(sheet_id, col_idx, &query, offset, page_size)
            .map_err(|e| e.to_string())?;
        let total = db
            .count_search_results(sheet_id, col_idx, &query)
            .map_err(|e| e.to_string())?;
        let rows = cells
            .into_iter()
            .map(|cell| SearchHit {
                row_idx: cell.row_idx,
                col_idx: cell.col_idx,
                value: cell.value.clone(),
                matches: compute_keyword_matches(&cell.value, &query),
            })
            .collect();
        Ok(SearchPage {
            rows,
            total,
            page,
            page_size,
        })
    }
}

/// 全表搜索替换。调用 `replace_all_cells`（单事务，返回 `(affected, before, after)`
/// 快照），`log_operation_with_snapshot` 记录操作供撤销。返回受影响行数。
///
/// `use_regex=true` 时 `from` 为正则 pattern（编译失败由 DB 方法返回 `Err`，不 panic）；
/// `false` 时 `from` 为字面子串。before/after 快照序列化后存入 operations 表。
#[tauri::command]
pub fn replace_all(
    sheet_id: i64,
    from: String,
    to: String,
    use_regex: bool,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<ReplaceResult, String> {
    let (affected, before_cells, after_cells) = db
        .replace_all_cells(sheet_id, &from, &to, use_regex)
        .map_err(|e| e.to_string())?;
    // 序列化 before/after 快照供撤销 / 重做。
    let before_json = serde_json::to_string(&before_cells).map_err(|e| e.to_string())?;
    let after_json = serde_json::to_string(&after_cells).map_err(|e| e.to_string())?;
    db.log_operation_with_snapshot(
        Some(sheet_id),
        "replace_all",
        &serde_json::json!({
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

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 计算关键字在 value 中的所有出现位置（字节偏移）。
///
/// `value` 为 `None` 返回空。`query` 为空返回空。按 `str::find` 字面查找，
/// 不重叠。返回的 `start..end` 区间 `end` 为 exclusive 结束位置。
fn compute_keyword_matches(value: &Option<String>, query: &str) -> Vec<MatchSpan> {
    let mut spans = Vec::new();
    if let Some(val) = value {
        if query.is_empty() {
            return spans;
        }
        let mut start = 0;
        while let Some(pos) = val[start..].find(query) {
            let abs_start = start + pos;
            let abs_end = abs_start + query.len();
            spans.push(MatchSpan {
                start: abs_start,
                end: abs_end,
            });
            start = abs_end;
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Cell, DbManager};

    /// 构造一个 3 列 × 4 行的 sheet（row_idx=0 表头）。
    /// col0=name, col1=phone, col2=memo。
    fn build_search_sheet(mgr: &DbManager) -> i64 {
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

    fn cell(shid: i64, row: u32, col: u32, val: &str) -> Cell {
        Cell {
            sheet_id: shid,
            row_idx: row,
            col_idx: col,
            value: Some(val.into()),
        }
    }

    fn open() -> (tempfile::TempDir, DbManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        (dir, mgr)
    }

    // ---- compute_keyword_matches 单元测试 ----

    #[test]
    fn compute_keyword_matches_finds_all_occurrences() {
        let val = Some("abcabcabc".to_string());
        let spans = compute_keyword_matches(&val, "abc");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 3);
        assert_eq!(spans[1].start, 3);
        assert_eq!(spans[1].end, 6);
        assert_eq!(spans[2].start, 6);
        assert_eq!(spans[2].end, 9);
    }

    #[test]
    fn compute_keyword_matches_none_value_returns_empty() {
        let spans = compute_keyword_matches(&None, "x");
        assert!(spans.is_empty());
    }

    #[test]
    fn compute_keyword_matches_empty_query_returns_empty() {
        let val = Some("abc".to_string());
        let spans = compute_keyword_matches(&val, "");
        assert!(spans.is_empty());
    }

    #[test]
    fn compute_keyword_matches_no_match_returns_empty() {
        let val = Some("abc".to_string());
        let spans = compute_keyword_matches(&val, "xyz");
        assert!(spans.is_empty());
    }

    // ---- 模拟 search_cells 关键字模式流程 ----

    /// 验收点 1：关键字模式返回所有命中 cell（row_idx/col_idx/value/匹配区间）。
    #[test]
    fn search_keyword_finds_matches() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 模拟 search_cells(use_regex=false, col_idx=None, page=1, page_size=50)。
        let cells = mgr.search_cells(shid, None, "张", 0, 50).unwrap();
        let total = mgr.count_search_results(shid, None, "张").unwrap();
        let rows: Vec<SearchHit> = cells
            .into_iter()
            .map(|cell| SearchHit {
                row_idx: cell.row_idx,
                col_idx: cell.col_idx,
                value: cell.value.clone(),
                matches: compute_keyword_matches(&cell.value, "张"),
            })
            .collect();
        assert_eq!(total, 2);
        assert_eq!(rows.len(), 2);
        // 每条命中应有 1 个匹配区间。
        for hit in &rows {
            assert_eq!(hit.matches.len(), 1);
            let val = hit.value.as_ref().unwrap();
            let span = &hit.matches[0];
            assert_eq!(&val[span.start..span.end], "张");
        }
        // row_idx 应为 1 和 3。
        let row_idxs: Vec<u32> = rows.iter().map(|h| h.row_idx).collect();
        assert!(row_idxs.contains(&1));
        assert!(row_idxs.contains(&3));
    }

    /// 验收点 4：col_idx=Some(c) 只搜指定列。
    #[test]
    fn search_keyword_col_filtered() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // col1 搜 "138" → 只命中 row_idx=1。
        let cells = mgr.search_cells(shid, Some(1), "138", 0, 50).unwrap();
        let total = mgr.count_search_results(shid, Some(1), "138").unwrap();
        assert_eq!(total, 1);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].row_idx, 1);
        assert_eq!(cells[0].col_idx, 1);
    }

    /// 验收点 3：分页正确（page=1 page_size=2 返回前 2 条，total 为总数）。
    #[test]
    fn search_pagination() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 全表搜 "1" → phone 3 行 + memo "50%" 0（无 1）= 3 条。
        let total = mgr.count_search_results(shid, None, "1").unwrap();
        assert_eq!(total, 3);
        // page_size=2 offset=0 → 前 2 条。
        let p1 = mgr.search_cells(shid, None, "1", 0, 2).unwrap();
        assert_eq!(p1.len(), 2);
        // page_size=2 offset=2 → 剩余 1 条。
        let p2 = mgr.search_cells(shid, None, "1", 2, 2).unwrap();
        assert_eq!(p2.len(), 1);
    }

    /// 验收点 4 验收：空 query 返回空结果。
    #[test]
    fn search_empty_query_returns_empty() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 模拟 search_cells(query="")：应直接返回空，不调用 DB。
        let query = "";
        let rows: Vec<SearchHit> = if query.is_empty() {
            Vec::new()
        } else {
            mgr.search_cells(shid, None, query, 0, 50)
                .unwrap()
                .into_iter()
                .map(|cell| SearchHit {
                    row_idx: cell.row_idx,
                    col_idx: cell.col_idx,
                    value: cell.value.clone(),
                    matches: compute_keyword_matches(&cell.value, query),
                })
                .collect()
        };
        assert!(rows.is_empty());
    }

    // ---- 模拟 search_cells 正则模式流程 ----

    /// 验收点 2：正则模式用 LIKE 预筛 + regex 精确匹配。
    #[test]
    fn search_regex_finds_matches() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 正则 \d{3} → phone 列 3 个值都命中。
        let all = mgr
            .search_cells_regex(shid, None, r"\d{3}", 0, u32::MAX)
            .unwrap();
        let total = all.len() as u32;
        // 模拟命令分页：page=1 page_size=50。
        let page_size = 50u32;
        let start = 0usize.min(all.len());
        let end = (start + page_size as usize).min(all.len());
        let rows: Vec<SearchHit> = all[start..end]
            .iter()
            .map(|(cell, spans)| SearchHit {
                row_idx: cell.row_idx,
                col_idx: cell.col_idx,
                value: cell.value.clone(),
                matches: spans
                    .iter()
                    .map(|&(s, e)| MatchSpan { start: s, end: e })
                    .collect(),
            })
            .collect();
        assert_eq!(total, 3);
        assert_eq!(rows.len(), 3);
        for hit in &rows {
            assert!(!hit.matches.is_empty());
            let val = hit.value.as_ref().unwrap();
            for span in &hit.matches {
                assert!(span.start < span.end && span.end <= val.len());
            }
        }
    }

    /// 验收点 5：正则非法模式返回明确错误。
    #[test]
    fn search_invalid_regex_returns_err() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 非法正则（未闭合 `[`）→ Err。
        let res = mgr.search_cells_regex(shid, None, "[unclosed", 0, 10);
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(
            err.contains("invalid regex"),
            "错误信息应包含 invalid regex，实际: {err}"
        );
    }

    /// 正则模式分页正确。
    #[test]
    fn search_regex_pagination() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 全表正则 \d → phone 3 + memo "50%" 1 = 4 命中。
        let all = mgr
            .search_cells_regex(shid, None, r"\d", 0, u32::MAX)
            .unwrap();
        assert_eq!(all.len(), 4);
        let total = all.len() as u32;
        // 模拟命令分页：page=3 page_size=1 → offset=2，取第 3 条。
        let page = 3u32;
        let page_size = 1u32;
        let offset = (page - 1) * page_size;
        let start = (offset as usize).min(all.len());
        let end = (start + page_size as usize).min(all.len());
        let rows: Vec<SearchHit> = all[start..end]
            .iter()
            .map(|(cell, spans)| SearchHit {
                row_idx: cell.row_idx,
                col_idx: cell.col_idx,
                value: cell.value.clone(),
                matches: spans
                    .iter()
                    .map(|&(s, e)| MatchSpan { start: s, end: e })
                    .collect(),
            })
            .collect();
        assert_eq!(total, 4);
        assert_eq!(rows.len(), 1);
    }

    // ---- 模拟 replace_all 流程 ----

    /// 验收点 6：replace_all 后所有命中 cell 被替换。
    #[test]
    fn replace_all_replaces_all_columns() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 全表正则 \d+ → "#"：phone 3 个纯数字值 + memo "50%" → "#%"。
        let (affected, _before, after) = mgr.replace_all_cells(shid, r"\d+", "#", true).unwrap();
        assert_eq!(affected, 4);
        // phone 列 3 个值变 "#"。
        let phone_hits: Vec<&Cell> = after.iter().filter(|c| c.col_idx == 1).collect();
        assert_eq!(phone_hits.len(), 3);
        assert!(phone_hits.iter().all(|c| c.value.as_deref() == Some("#")));
        // memo "50%" → "#%"。
        let memo_hits: Vec<&Cell> = after.iter().filter(|c| c.col_idx == 2).collect();
        assert_eq!(memo_hits.len(), 1);
        assert_eq!(memo_hits[0].value.as_deref(), Some("#%"));
        // DB 已更新：再搜 "\d" 应 0 命中。
        let after_search = mgr.search_cells_regex(shid, None, r"\d", 0, 10).unwrap();
        assert!(after_search.is_empty());
    }

    /// 验收点 7：replace_all 的 operations 行有 before_snapshot_json（供撤销）。
    #[test]
    fn replace_all_logs_before_snapshot() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 模拟 replace_all 命令内部流程。
        let (affected, before_cells, after_cells) =
            mgr.replace_all_cells(shid, "1", "X", false).unwrap();
        let before_json = serde_json::to_string(&before_cells).unwrap();
        let after_json = serde_json::to_string(&after_cells).unwrap();
        let op_id = mgr
            .log_operation_with_snapshot(
                Some(shid),
                "replace_all",
                &serde_json::json!({
                    "from": "1",
                    "to": "X",
                    "useRegex": false,
                    "affected": affected
                })
                .to_string(),
                Some(&before_json),
                &after_json,
            )
            .unwrap();
        // 读回 operation，验证 before_snapshot_json 非空。
        let row = mgr.query_operation_by_id(op_id).unwrap().unwrap();
        assert_eq!(row.kind, "replace_all");
        assert_eq!(row.sheet_id, Some(shid));
        assert!(
            row.before_snapshot_json.is_some(),
            "before_snapshot_json 必须存在供撤销"
        );
        assert!(!row.before_snapshot_json.as_deref().unwrap().is_empty());
        assert_eq!(
            row.result_snapshot_json.as_deref(),
            Some(after_json.as_str())
        );
        // affected 应 = before/after 长度（每变化行 1 条）。
        assert_eq!(affected as usize, before_cells.len());
        assert_eq!(before_cells.len(), after_cells.len());
    }

    /// 验收点 8：use_regex=true 时用正则替换，编译失败返回错误。
    #[test]
    fn replace_all_regex_mode() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 正则替换：\d+ → "#" 命中 4 行。
        let (affected, _before, after) = mgr.replace_all_cells(shid, r"\d+", "#", true).unwrap();
        assert_eq!(affected, 4);
        assert!(after
            .iter()
            .all(|c| c.value.as_deref().unwrap().contains('#')));

        // 非法正则 → Err。
        let res = mgr.replace_all_cells(shid, "[bad", "x", true);
        assert!(res.is_err());
    }

    /// 验收点 9：命令层不直接写 SQL，全部由 DB 方法参数绑定（此处仅验证调用链通畅）。
    #[test]
    fn replace_all_keyword_mode() {
        let (_dir, mgr) = open();
        let shid = build_search_sheet(&mgr);
        // 字面替换 "1" → "X"：phone 3 行（都含 1）变化。
        let (affected, before, after) = mgr.replace_all_cells(shid, "1", "X", false).unwrap();
        assert_eq!(affected, 3);
        assert_eq!(before.len(), 3);
        assert_eq!(after.len(), 3);
        // before 中应都含 "1"，after 中应都不含 "1"。
        assert!(before
            .iter()
            .all(|c| c.value.as_deref().unwrap().contains('1')));
        assert!(after
            .iter()
            .all(|c| !c.value.as_deref().unwrap().contains('1')));
    }
}
