import { useCallback } from "react";
import { getSheetData, listUndoableOperations } from "../tauri";
import { PAGE_SIZE } from "../constants";

/**
 * useSheetOps — 封装「操作后刷新」的两种通用模式，消除各面板重复 dispatch 样板。
 *
 * 1) refreshActiveSheet() — 就地操作后刷新当前页数据 + 撤销栈：
 *    getSheetData → SET_SHEET_DATA → listUndoableOperations → SET_UNDO_STACK
 *    用于 CryptoPanel.handleExecute / ColumnOpsPanel.handleTransform / MaskPanel.handleRun
 *
 * 2) landNewSheet(parse, name, columnHint) — 新 Tab 落地：
 *    addSheetFromParse → getSheetData(新 Tab 首页) → SET_SHEET_DATA
 *    用于 ExtractPanel.handleExtractValidate / ColumnOpsPanel.handleParse /
 *    ValidatePanel.landSheet（双 Tab）
 *
 * v1.2.0 T94：从各面板重复的 getSheetData + dispatch 样板中抽出。
 *
 * @param {Function} dispatch — useAppContext().dispatch
 * @param {Function} addSheetFromParse — useAppContext().addSheetFromParse
 * @returns {{ refreshActiveSheet, landNewSheet }}
 */
export function useSheetOps(dispatch, addSheetFromParse) {
  // 就地操作后刷新当前 Sheet 的当前页 + 撤销栈。
  const refreshActiveSheet = useCallback(
    async (sheet) => {
      if (!sheet) return;
      const page = sheet.page || 1;
      const pageSize = sheet.pageSize || PAGE_SIZE;
      const data = await getSheetData(sheet.id, page, pageSize);
      dispatch({
        type: "SET_SHEET_DATA",
        payload: { ...data, sheetId: sheet.id },
      });
      const ops = await listUndoableOperations(sheet.id);
      dispatch({ type: "SET_UNDO_STACK", payload: ops });
    },
    [dispatch]
  );

  // 新 Tab 落地：addSheetFromParse + 拉首页 + SET_SHEET_DATA。
  // parse = 后端返回的 { newSheetId, headers, rowCount, skipped } 对象。
  const landNewSheet = useCallback(
    async (parse, name, columnHint, sessionId) => {
      addSheetFromParse({
        newSheetId: parse.newSheetId,
        headers: parse.headers,
        rowCount: parse.rowCount,
        skipped: parse.skipped ?? 0,
        sessionId,
        column: columnHint,
        name,
      });
      const data = await getSheetData(parse.newSheetId, 1, PAGE_SIZE);
      dispatch({
        type: "SET_SHEET_DATA",
        payload: { ...data, sheetId: parse.newSheetId },
      });
    },
    [dispatch, addSheetFromParse]
  );

  return { refreshActiveSheet, landNewSheet };
}
