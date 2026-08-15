import { useCallback } from "react";
import { getSheetData, listUndoableOperations } from "../tauri";
import { PAGE_SIZE } from "../constants";
import { ACTION } from "../state/constants";

/**
 * useSheetOps — 封装「操作后刷新」的两种通用模式，消除各面板重复 dispatch 样板。
 *
 * 1) refreshActiveSheet() — 就地操作后刷新当前页数据 + 撤销栈：
 *    getSheetData → SET_SHEET_DATA → listUndoableOperations → SET_UNDO_STACK
 *    用于 CryptoPanel.handleExecute / ColumnOpsPanel.handleTransform /
 *    DataTable.handleReplace / MaskPanel.handleRun
 *
 *    v1.2.1：新增可选 rowStatus 参数——传入则同时 dispatch APPLY_ROW_STATUSES
 *    标记所有刷新行（MaskPanel 用 "masked"）。
 *
 * 2) landNewSheet(parse, name, columnHint) — 新 Tab 落地：
 *    addSheetFromParse → getSheetData(新 Tab 首页) → SET_SHEET_DATA
 *    用于 ExtractPanel.handleExtractValidate / ColumnOpsPanel.handleParse /
 *    ValidatePanel.landSheet（双 Tab）
 *
 * v1.2.0 T94：从各面板重复的 getSheetData + dispatch 样板中抽出。
 * v1.2.1 第三轮：dispatch 改用 ACTION 常量，MaskPanel/DataTable 不再内联重复。
 *
 * @param {Function} dispatch — useAppContext().dispatch
 * @returns {{ refreshActiveSheet, landNewSheet }}
 */
export function useSheetOps(dispatch) {
  // 就地操作后刷新当前 Sheet 的当前页 + 撤销栈。
  // v1.2.1：rowStatus 可选——传入则标记所有刷新行（如 "masked"）。
  const refreshActiveSheet = useCallback(
    async (sheet, rowStatus = null) => {
      if (!sheet) return;
      const page = sheet.page || 1;
      const pageSize = sheet.pageSize || PAGE_SIZE;
      const data = await getSheetData(sheet.id, page, pageSize);
      dispatch({
        type: ACTION.SET_SHEET_DATA,
        payload: { ...data, sheetId: sheet.id },
      });
      if (rowStatus) {
        const rowStatuses = {};
        (data.rows || []).forEach((_, i) => {
          rowStatuses[`${sheet.id}-${page}-${i}`] = rowStatus;
        });
        dispatch({
          type: ACTION.APPLY_ROW_STATUSES,
          payload: { sheetId: sheet.id, rowStatuses },
        });
      }
      const ops = await listUndoableOperations(sheet.id);
      dispatch({ type: ACTION.SET_UNDO_STACK, payload: ops });
    },
    [dispatch]
  );

  // 新 Tab 落地：dispatch ADD_SHEET_FROM_PARSE + 拉首页 + SET_SHEET_DATA。
  // parse = 后端返回的 { newSheetId, headers, rowCount, skipped } 对象。
  const landNewSheet = useCallback(
    async (parse, name, columnHint, sessionId) => {
      dispatch({
        type: ACTION.ADD_SHEET_FROM_PARSE,
        payload: {
          newSheetId: parse.newSheetId,
          headers: parse.headers,
          rowCount: parse.rowCount,
          skipped: parse.skipped ?? 0,
          sessionId,
          column: columnHint,
          name,
        },
      });
      const data = await getSheetData(parse.newSheetId, 1, PAGE_SIZE);
      dispatch({
        type: ACTION.SET_SHEET_DATA,
        payload: { ...data, sheetId: parse.newSheetId },
      });
    },
    [dispatch]
  );

  return { refreshActiveSheet, landNewSheet };
}
