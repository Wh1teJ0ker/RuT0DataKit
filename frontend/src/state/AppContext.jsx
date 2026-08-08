// AppContext：Context + Provider + useAppContext hook（T13 引入）。
// 替代原 useAppState hook，让深层组件直接消费 state / dispatch / dispatcher，
// 消除 prop drilling。Provider value 用 useMemo 包裹，依赖稳定，避免整树重渲染。
import { createContext, useContext, useMemo, useReducer, useCallback } from "react";
import { ACTION, initialState } from "./constants";
import { reducer } from "./reducer";

// Context 值：{ state, dispatch, ...19 个 dispatcher }
export const AppContext = createContext(null);

export function AppProvider({ children }) {
  const [state, dispatch] = useReducer(reducer, initialState);

  // ---- T2 ----
  const setView = useCallback(
    (view) => dispatch({ type: ACTION.SET_VIEW, payload: view }),
    []
  );
  const setActiveCapability = useCallback(
    (capability) =>
      dispatch({ type: ACTION.SET_ACTIVE_CAPABILITY, payload: capability }),
    []
  );
  const setAiPanelVisible = useCallback(
    (visible) => dispatch({ type: ACTION.SET_AI_PANEL_VISIBLE, payload: visible }),
    []
  );

  // ---- T3 ----
  const addSheet = useCallback(
    (payload = {}) => dispatch({ type: ACTION.ADD_SHEET, payload }),
    []
  );
  const closeSheet = useCallback(
    (id) => dispatch({ type: ACTION.CLOSE_SHEET, payload: id }),
    []
  );
  const setActiveSheet = useCallback(
    (id) => dispatch({ type: ACTION.SET_ACTIVE_SHEET, payload: id }),
    []
  );
  const renameSheet = useCallback(
    (payload) => dispatch({ type: ACTION.RENAME_SHEET, payload }),
    []
  );
  const setSelection = useCallback(
    (payload) => dispatch({ type: ACTION.SET_SELECTION, payload }),
    []
  );
  const reorderColumns = useCallback(
    (columnOrder) => dispatch({ type: ACTION.REORDER_COLUMNS, payload: columnOrder }),
    []
  );
  const setColumnVisibility = useCallback(
    (payload) => dispatch({ type: ACTION.SET_COLUMN_VISIBILITY, payload }),
    []
  );
  const setPage = useCallback(
    (page) => dispatch({ type: ACTION.SET_PAGE, payload: page }),
    []
  );

  // ---- T5（导入流）----
  const importSuccess = useCallback(
    (payload) => dispatch({ type: ACTION.IMPORT_SUCCESS, payload }),
    []
  );
  const setSheetData = useCallback(
    (payload) => dispatch({ type: ACTION.SET_SHEET_DATA, payload }),
    []
  );

  // ---- v1.1.0 行状态高亮（脱敏/校验/提取）----
  const applyRowStatuses = useCallback(
    (payload) => dispatch({ type: ACTION.APPLY_ROW_STATUSES, payload }),
    []
  );

  // ---- v1.1.1 撤销 / 搜索 / 列操作 ----
  const setSearchState = useCallback(
    (payload) => dispatch({ type: ACTION.SET_SEARCH_STATE, payload }),
    []
  );
  const applySearchHits = useCallback(
    (payload) => dispatch({ type: ACTION.APPLY_SEARCH_HITS, payload }),
    []
  );
  const applySearchRows = useCallback(
    (payload) => dispatch({ type: ACTION.APPLY_SEARCH_ROWS, payload }),
    []
  );
  const clearSearch = useCallback(
    () => dispatch({ type: ACTION.CLEAR_SEARCH }),
    []
  );
  const addSheetFromParse = useCallback(
    (payload) => dispatch({ type: ACTION.ADD_SHEET_FROM_PARSE, payload }),
    []
  );
  const setUndoStack = useCallback(
    (payload) => dispatch({ type: ACTION.SET_UNDO_STACK, payload }),
    []
  );

  const value = useMemo(
    () => ({
      state,
      dispatch,
      setView,
      setActiveCapability,
      setAiPanelVisible,
      addSheet,
      closeSheet,
      setActiveSheet,
      renameSheet,
      setSelection,
      reorderColumns,
      setColumnVisibility,
      setPage,
      importSuccess,
      setSheetData,
      applyRowStatuses,
      setSearchState,
      applySearchHits,
      applySearchRows,
      clearSearch,
      addSheetFromParse,
      setUndoStack,
    }),
    [
      state,
      dispatch,
      setView,
      setActiveCapability,
      setAiPanelVisible,
      addSheet,
      closeSheet,
      setActiveSheet,
      renameSheet,
      setSelection,
      reorderColumns,
      setColumnVisibility,
      setPage,
      importSuccess,
      setSheetData,
      applyRowStatuses,
      setSearchState,
      applySearchHits,
      applySearchRows,
      clearSearch,
      addSheetFromParse,
      setUndoStack,
    ]
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

// 供深层组件消费。必须在组件顶层调用（Rules of Hooks）。
export function useAppContext() {
  const ctx = useContext(AppContext);
  if (!ctx) {
    throw new Error("useAppContext must be used within <AppProvider>");
  }
  return ctx;
}
