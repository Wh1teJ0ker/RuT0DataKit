import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `undo_operation` IPC：按 op_id 撤销单个操作（就地回滚 DB cells）。
 * 后端签名 `undo_operation(op_id: i64, db: State<...>)` 只接 op_id（db 自动注入），
 * 因此 JS 侧不传 sheetId。
 * @param {number} opId  操作 ID（来自 listUndoableOperations 返回的 id）
 * @returns {Promise<{restored: number}>} 还原的行数
 */
export function undoOperation(opId) {
  return invoke("undo_operation", { opId });
}

/**
 * 调用 `redo_operation` IPC：按 op_id 重做单个操作。
 * 与 undo_operation 对称，只接 op_id。
 * @param {number} opId  操作 ID
 * @returns {Promise<{restored: number}>} 还原的行数
 */
export function redoOperation(opId) {
  return invoke("redo_operation", { opId });
}

/**
 * 调用 `list_undoable_operations` IPC：列出可撤销操作（撤销工具栏数据源）。
 * @param {number} sheetId  Sheet ID
 * @returns {Promise<Array<{id: number, kind: string, createdAt: string}>>}
 */
export function listUndoableOperations(sheetId) {
  return invoke("list_undoable_operations", { sheetId });
}
