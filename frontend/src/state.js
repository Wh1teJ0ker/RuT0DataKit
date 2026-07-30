import { useReducer, useCallback } from "react";

// v1.0.0 全局 state（useReducer 单一真相源）。
// T2 落 activeCapability / currentView / aiPanel；
// T3 在此基础上扩展 sheets / activeSheetId（Sheet/Tab + antd Table）。
// 后续 updateStatus / sessions 等留给 T6/T8 等下游任务扩展。

// ---- Sheet 工厂 ----
// Sheet 对象结构：{ id, sessionId, name, headers, rows, total, page, pageSize,
//                  columnOrder, columnVisibility, selection, statusHighlights }
// TODO(T5): replace mock with real import —— mock 数据生成与初始 sheet 均为交互演示，
// T5 接管真实导入流后移除 mock。
let mockSheetSeq = 0;
function createMockSheet(name) {
  mockSheetSeq += 1;
  const id = `sheet-${Date.now()}-${mockSheetSeq}`;
  const headers = ["id", "name", "value"];
  const rows = Array.from({ length: 100 }, (_, i) => ({
    key: i,
    id: i + 1,
    name: `行 ${i + 1}`,
    value: Math.round(Math.random() * 1000),
    status: "default", // v1.0.0 mock 全 default；invalid/masked/hit 触发逻辑是 v1.2+
  }));
  return {
    id,
    sessionId: null, // T5 接管后由真实导入流填充
    name,
    headers,
    rows,
    total: rows.length,
    page: 1,
    pageSize: 50,
    columnOrder: [...headers],
    columnVisibility: headers.reduce((acc, h) => ({ ...acc, [h]: true }), {}),
    selection: { selectedRowKeys: [], lastSelectedIndex: null },
    statusHighlights: {}, // v1.2+ 触发逻辑使用，v1.0.0 占位
  };
}

export const initialState = {
  // T2 字段（保留，禁止覆盖）
  currentView: "workbench", // 'workbench' | 'settings'
  activeCapability: null, // null | 'mask' | 'validate' | 'extract' | 'rules'
  aiPanel: { visible: true }, // lastSuggestion 由 T7 扩展
  // T3 字段
  sheets: [], // Sheet[]
  activeSheetId: null, // string | null
};

export const ACTION = {
  // T2 action（保留）
  SET_VIEW: "SET_VIEW",
  SET_ACTIVE_CAPABILITY: "SET_ACTIVE_CAPABILITY",
  SET_AI_PANEL_VISIBLE: "SET_AI_PANEL_VISIBLE",
  // T3 action
  ADD_SHEET: "ADD_SHEET",
  CLOSE_SHEET: "CLOSE_SHEET",
  SET_ACTIVE_SHEET: "SET_ACTIVE_SHEET",
  RENAME_SHEET: "RENAME_SHEET",
  SET_SELECTION: "SET_SELECTION",
  REORDER_COLUMNS: "REORDER_COLUMNS",
  SET_COLUMN_VISIBILITY: "SET_COLUMN_VISIBILITY",
  SET_PAGE: "SET_PAGE",
};

function patchActiveSheet(state, patch) {
  if (!state.activeSheetId) return state;
  return {
    ...state,
    sheets: state.sheets.map((s) =>
      s.id === state.activeSheetId ? { ...s, ...patch(s) } : s
    ),
  };
}

export function reducer(state, action) {
  switch (action.type) {
    // ---- T2 ----
    case ACTION.SET_VIEW:
      return { ...state, currentView: action.payload };
    case ACTION.SET_ACTIVE_CAPABILITY: {
      const next =
        action.payload === state.activeCapability ? null : action.payload;
      return { ...state, activeCapability: next };
    }
    case ACTION.SET_AI_PANEL_VISIBLE:
      return {
        ...state,
        aiPanel: { ...state.aiPanel, visible: Boolean(action.payload) },
      };

    // ---- T3 ----
    case ACTION.ADD_SHEET: {
      // payload = { sheet } | { name }（缺省时自建 mock）
      const sheet =
        action.payload?.sheet ||
        createMockSheet(action.payload?.name || `Sheet ${mockSheetSeq + 1}`);
      return {
        ...state,
        sheets: [...state.sheets, sheet],
        activeSheetId: sheet.id,
      };
    }
    case ACTION.CLOSE_SHEET: {
      const id = action.payload;
      const idx = state.sheets.findIndex((s) => s.id === id);
      if (idx < 0) return state;
      const nextSheets = state.sheets.filter((s) => s.id !== id);
      let nextActive = state.activeSheetId;
      if (state.activeSheetId === id) {
        // 关闭后激活相邻 Tab：优先右邻，否则左邻。
        nextActive = nextSheets.length
          ? nextSheets[Math.min(idx, nextSheets.length - 1)].id
          : null;
      }
      return { ...state, sheets: nextSheets, activeSheetId: nextActive };
    }
    case ACTION.SET_ACTIVE_SHEET:
      return { ...state, activeSheetId: action.payload };
    case ACTION.RENAME_SHEET:
      return {
        ...state,
        sheets: state.sheets.map((s) =>
          s.id === action.payload.id ? { ...s, name: action.payload.name } : s
        ),
      };
    case ACTION.SET_SELECTION:
      // payload = { selectedRowKeys, lastSelectedIndex }
      return patchActiveSheet(state, () => ({
        selection: {
          selectedRowKeys: action.payload.selectedRowKeys,
          lastSelectedIndex:
            action.payload.lastSelectedIndex !== undefined
              ? action.payload.lastSelectedIndex
              : null,
        },
      }));
    case ACTION.REORDER_COLUMNS:
      // payload = columnOrder (string[])
      return patchActiveSheet(state, () => ({
        columnOrder: action.payload,
      }));
    case ACTION.SET_COLUMN_VISIBILITY:
      // payload = { [header]: boolean } | columnVisibility 全量对象
      return patchActiveSheet(state, (s) => ({
        columnVisibility: { ...s.columnVisibility, ...action.payload },
      }));
    case ACTION.SET_PAGE:
      // payload = page
      return patchActiveSheet(state, () => ({ page: action.payload }));
    default:
      return state;
  }
}

// 初始注入 1 个 mock Sheet，便于交互演示。
// TODO(T5): replace mock with real import
initialState.sheets = [createMockSheet("Sheet 1")];
initialState.activeSheetId = initialState.sheets[0].id;

export function useAppState() {
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

  return {
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
  };
}
