//! 撤销 / 重做 / 可撤销列表命令（v1.1.1）。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（操作历史 + 单元格读写）。

use serde::Serialize;

use crate::db::Cell;

// ---------------------------------------------------------------------------
// 撤销 / 重做 / 可撤销列表命令（v1.1.1）
// ---------------------------------------------------------------------------

/// 撤销结果（camelCase）。`restored` = 回写的 cell 数。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoResult {
    pub restored: u32,
}

/// 重做结果（camelCase）。`restored` = 回写的 cell 数。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedoResult {
    pub restored: u32,
}

/// 可撤销操作摘要（camelCase）。撤销工具栏列表用。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoableOp {
    pub id: i64,
    pub kind: String,
    pub created_at: String,
}

/// 撤销某次就地变更操作：读 `before_snapshot_json` → 回写 cells
/// → `log_operation("undo")`（undo 自身不可再撤销）。返回回写 cell 数。
///
/// - 操作不存在 → `Err("操作 {op_id} 不存在")`
/// - `before_snapshot_json` 为 `None` → `Err("操作 {op_id} 不可撤销（无 before 快照）")`
#[tauri::command]
pub fn undo_operation(
    op_id: i64,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<UndoResult, String> {
    let op = db
        .query_operation_by_id(op_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("操作 {op_id} 不存在"))?;
    let before_json = op
        .before_snapshot_json
        .as_ref()
        .ok_or_else(|| format!("操作 {op_id} 不可撤销（无 before 快照）"))?;
    let before_cells: Vec<Cell> =
        serde_json::from_str(before_json).map_err(|e| format!("解析 before 快照失败: {e}"))?;
    let sheet_id = op
        .sheet_id
        .ok_or_else(|| format!("操作 {op_id} 无 sheet_id"))?;
    let restored = before_cells.len() as u32;
    if !before_cells.is_empty() {
        db.write_cells(sheet_id, &before_cells)
            .map_err(|e| e.to_string())?;
    }
    // undo 自身 log_operation(kind="undo")，before_snapshot=None → 不可再撤销。
    db.log_operation(
        Some(sheet_id),
        "undo",
        &serde_json::json!({ "opId": op_id }).to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;
    Ok(UndoResult { restored })
}

/// 重做某次就地变更操作：读 `result_snapshot_json`（after）→ 回写 cells
/// → `log_operation("redo")`（redo 自身不可再撤销）。返回回写 cell 数。
///
/// - 操作不存在 → `Err("操作 {op_id} 不存在")`
/// - `result_snapshot_json` 为 `None` 或 `"{}"` → `Err("操作 {op_id} 不可重做")`
#[tauri::command]
pub fn redo_operation(
    op_id: i64,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<RedoResult, String> {
    let op = db
        .query_operation_by_id(op_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("操作 {op_id} 不存在"))?;
    let after_json = op
        .result_snapshot_json
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "{}")
        .ok_or_else(|| format!("操作 {op_id} 不可重做（无 result 快照）"))?;
    let after_cells: Vec<Cell> =
        serde_json::from_str(after_json).map_err(|e| format!("解析 result 快照失败: {e}"))?;
    let sheet_id = op
        .sheet_id
        .ok_or_else(|| format!("操作 {op_id} 无 sheet_id"))?;
    let restored = after_cells.len() as u32;
    if !after_cells.is_empty() {
        db.write_cells(sheet_id, &after_cells)
            .map_err(|e| e.to_string())?;
    }
    db.log_operation(
        Some(sheet_id),
        "redo",
        &serde_json::json!({ "opId": op_id }).to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;
    Ok(RedoResult { restored })
}

/// 列出某 sheet 最近 50 条可撤销操作（kind ∈ {mask, replace_in_column,
/// replace_all}，按 `created_at` DESC）。供前端撤销工具栏展示。
#[tauri::command]
pub fn list_undoable_operations(
    sheet_id: i64,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<Vec<UndoableOp>, String> {
    let rows = db
        .list_undoable_operations(sheet_id, 50)
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| UndoableOp {
            id: r.id,
            kind: r.kind,
            created_at: r.created_at,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// commands 层测试（v1.1.1 新增）
// ---------------------------------------------------------------------------

/// 撤销/重做核心路径不依赖 Tauri runtime（`#[tauri::command]` 函数签名带
/// `tauri::State<'_, DbManager>` 在单元测试中无法直接调用）。测试策略：
/// 直接调用 `DbManager` 方法模拟 mask 流程
/// （`query_column_cells` → `SimpleMasker::mask` → `write_cells`
/// → `log_operation_with_snapshot`），再测 undo / redo / list_undoable_operations
/// 的底层逻辑，覆盖撤销核心路径。
#[cfg(test)]
mod tests {
    use crate::db::{Cell, DbManager, OperationRow, UndoableOpRow};
    use ruT0_data_kit_core::processor::{Masker, SimpleMasker};


    /// 构造一个 tempdir + 空 DbManager。返回 TempDir 以保活（TempDir drop 会
    /// 删除目录与 db 文件，必须跨测试函数持有）。
    fn setup_db() -> (tempfile::TempDir, DbManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        (dir, mgr)
    }

    /// 构造一个 sheet：1 列（col0=name），表头 + 2 行数据（张三 / 李四）。
    fn setup_sheet_with_data(db: &DbManager) -> i64 {
        let session_id = db.create_session("test", None, "csv", 3).unwrap();
        let sheet_id = db.create_sheet(session_id, "test", 0).unwrap();
        let cells: Vec<Cell> = vec![
            Cell {
                sheet_id,
                row_idx: 0,
                col_idx: 0,
                value: Some("name".into()),
            },
            Cell {
                sheet_id,
                row_idx: 1,
                col_idx: 0,
                value: Some("张三".into()),
            },
            Cell {
                sheet_id,
                row_idx: 2,
                col_idx: 0,
                value: Some("李四".into()),
            },
        ];
        db.write_cells(sheet_id, &cells).unwrap();
        sheet_id
    }

    /// 模拟 `mask_column` 的底层流程（无 Tauri State）：
    /// 读列 cells → SimpleMasker::mask → write_cells → log_operation_with_snapshot。
    /// 返回 mask operation 的 id。
    fn simulate_mask(db: &DbManager, sheet_id: i64, col_idx: u32) -> i64 {
        let rows = db.query_column_cells(sheet_id, col_idx).unwrap();
        // before 快照：列原始值。
        let before_cells: Vec<Cell> = rows
            .iter()
            .map(|(row_idx, value)| Cell {
                sheet_id,
                row_idx: *row_idx,
                col_idx,
                value: value.clone(),
            })
            .collect();
        let before_json = serde_json::to_string(&before_cells).unwrap();
        // mask：用通用脱敏（无规则），保留首尾 + 中间 `*`。
        let masker = SimpleMasker;
        let mut cells: Vec<Cell> = Vec::with_capacity(rows.len());
        for (row_idx, value) in &rows {
            let input = value.as_deref().unwrap_or("");
            let result = masker.mask(input, None).unwrap();
            cells.push(Cell {
                sheet_id,
                row_idx: *row_idx,
                col_idx,
                value: Some(result.output),
            });
        }
        if !cells.is_empty() {
            db.write_cells(sheet_id, &cells).unwrap();
        }
        let after_json = serde_json::to_string(&cells).unwrap();
        db.log_operation_with_snapshot(
            Some(sheet_id),
            "mask",
            r#"{"column":"name","affected":2}"#,
            Some(&before_json),
            &after_json,
        )
        .unwrap()
    }

    /// 模拟 `undo_operation` 的底层流程：读 op → 解析 before → write_cells
    /// → log_operation("undo")。返回回写 cell 数。
    fn simulate_undo(db: &DbManager, op_id: i64) -> Result<u32, String> {
        let op: OperationRow = db
            .query_operation_by_id(op_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("操作 {op_id} 不存在"))?;
        let before_json = op
            .before_snapshot_json
            .as_ref()
            .filter(|s| !s.is_empty() && *s != "{}")
            .ok_or_else(|| format!("操作 {op_id} 不可撤销（无 before 快照）"))?;
        let before_cells: Vec<Cell> =
            serde_json::from_str(before_json).map_err(|e| format!("解析 before 快照失败: {e}"))?;
        let sheet_id = op
            .sheet_id
            .ok_or_else(|| format!("操作 {op_id} 无 sheet_id"))?;
        let restored = before_cells.len() as u32;
        if !before_cells.is_empty() {
            db.write_cells(sheet_id, &before_cells)
                .map_err(|e| e.to_string())?;
        }
        db.log_operation(
            Some(sheet_id),
            "undo",
            &serde_json::json!({ "opId": op_id }).to_string(),
            "{}",
        )
        .map_err(|e| e.to_string())?;
        Ok(restored)
    }

    /// 模拟 `redo_operation` 的底层流程：读 op → 解析 result_snapshot →
    /// write_cells → log_operation("redo")。返回回写 cell 数。
    fn simulate_redo(db: &DbManager, op_id: i64) -> Result<u32, String> {
        let op = db
            .query_operation_by_id(op_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("操作 {op_id} 不存在"))?;
        let after_json = op
            .result_snapshot_json
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "{}")
            .ok_or_else(|| format!("操作 {op_id} 不可重做（无 result 快照）"))?;
        let after_cells: Vec<Cell> =
            serde_json::from_str(after_json).map_err(|e| format!("解析 result 快照失败: {e}"))?;
        let sheet_id = op
            .sheet_id
            .ok_or_else(|| format!("操作 {op_id} 无 sheet_id"))?;
        let restored = after_cells.len() as u32;
        if !after_cells.is_empty() {
            db.write_cells(sheet_id, &after_cells)
                .map_err(|e| e.to_string())?;
        }
        db.log_operation(
            Some(sheet_id),
            "redo",
            &serde_json::json!({ "opId": op_id }).to_string(),
            "{}",
        )
        .map_err(|e| e.to_string())?;
        Ok(restored)
    }

    /// 读列全部数据行的值（按 row_idx 升序），断言用。
    fn column_values(db: &DbManager, sheet_id: i64, col_idx: u32) -> Vec<String> {
        db.query_column_cells(sheet_id, col_idx)
            .unwrap()
            .into_iter()
            .map(|(_, v)| v.unwrap_or_default())
            .collect()
    }

    #[test]
    fn mask_then_undo_restores_cells() {
        // 用例 1：mask 后 undo，cells 恢复为 mask 前的原始值。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);
        let original = column_values(&db, sheet_id, 0);
        assert_eq!(original, vec!["张三".to_string(), "李四".to_string()]);

        let mask_op_id = simulate_mask(&db, sheet_id, 0);
        // mask 后值已脱敏（"张三"→"张*", "李四"→"李*"，通用脱敏保留首尾+中间*）
        let masked = column_values(&db, sheet_id, 0);
        assert_ne!(masked, original);
        assert!(masked.iter().all(|v| v.contains('*')));

        // undo：before 快照回写 → 恢复原始值。
        let restored = simulate_undo(&db, mask_op_id).unwrap();
        assert_eq!(restored, 2);
        let after_undo = column_values(&db, sheet_id, 0);
        assert_eq!(after_undo, original);
    }

    #[test]
    fn mask_then_redo_reapplies_mask() {
        // 用例 2：mask 后 undo 后 redo，cells 重新脱敏。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);
        let original = column_values(&db, sheet_id, 0);

        let mask_op_id = simulate_mask(&db, sheet_id, 0);
        let masked = column_values(&db, sheet_id, 0);
        assert_ne!(masked, original);

        // undo 后恢复原始。
        simulate_undo(&db, mask_op_id).unwrap();
        assert_eq!(column_values(&db, sheet_id, 0), original);

        // redo：result 快照回写 → 重新脱敏。
        let restored = simulate_redo(&db, mask_op_id).unwrap();
        assert_eq!(restored, 2);
        assert_eq!(column_values(&db, sheet_id, 0), masked);
    }

    #[test]
    fn undo_non_undoable_op_returns_err() {
        // 用例 3：对 before_snapshot=None 的操作 undo 返回错误。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);
        // 插入一个不可撤销操作（before_snapshot=None，如 import）。
        let non_undoable_id = db
            .log_operation(Some(sheet_id), "import", "{}", "{}")
            .unwrap();
        let err = simulate_undo(&db, non_undoable_id).unwrap_err();
        assert!(err.contains("不可撤销"));
    }

    #[test]
    fn list_undoable_operations_filters_kind() {
        // 用例 4：只返回 mask/replace_in_column/replace_all，不返回 undo/redo/import。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);

        let mask_id = simulate_mask(&db, sheet_id, 0);
        // undo mask → 产生一条 undo 操作（不应出现在 list 中）。
        simulate_undo(&db, mask_id).unwrap();
        // 再插一条 import（不应出现）。
        db.log_operation(Some(sheet_id), "import", "{}", "{}")
            .unwrap();

        let rows: Vec<UndoableOpRow> = db.list_undoable_operations(sheet_id, 50).unwrap();
        assert!(rows.iter().all(|r| {
            matches!(
                r.kind.as_str(),
                "mask" | "replace_in_column" | "replace_all"
            )
        }));
        // 仅 mask 一条可撤销操作（undo/import 被过滤）。
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "mask");
    }

    #[test]
    fn list_undoable_operations_order_desc() {
        // 用例 5：按 created_at DESC 排序。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);

        // 连续 3 次 mask（每次都覆盖同列值，但各自记录独立 op）。
        let id1 = simulate_mask(&db, sheet_id, 0);
        let id2 = simulate_mask(&db, sheet_id, 0);
        let id3 = simulate_mask(&db, sheet_id, 0);

        let rows = db.list_undoable_operations(sheet_id, 50).unwrap();
        assert_eq!(rows.len(), 3);
        // DESC：最新（id3）在最前。
        assert_eq!(rows[0].id, id3);
        assert_eq!(rows[1].id, id2);
        assert_eq!(rows[2].id, id1);
    }
}
