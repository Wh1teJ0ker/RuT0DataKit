import { useReducer, useCallback } from "react";

// v1.0.0 全局 state（useReducer 单一真相源）。
// T2 落 activeCapability / currentView / aiPanel；
// T3 在此基础上扩展 sheets / activeSheetId（Sheet/Tab + antd Table）。
// 后续 updateStatus / sessions 等留给 T6/T8 等下游任务扩展。

// ---- Sheet 工厂 ----
// Sheet 对象结构：{ id, sessionId, name, headers, rows, total, page, pageSize,
//                  columnOrder, columnVisibility, selection, statusHighlights }
// v1.0.0（T5）：真实导入流接管，mock 仅在空态演示用；导入成功路径以
// IMPORT_SUCCESS action 用 ImportResult 填充真实 Sheet。
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

// 由 ImportResult（camelCase）构造真实 Sheet。rows 初始为空，由 SET_SHEET_DATA
// action 在导入后/翻页后填充首页数据。
export function createSheetFromImport(result) {
  const headers = result.headers || [];
  return {
    id: result.sheetId, // Sheet ID 来自 DB（i64），与 getSheetData 入参一致
    sessionId: result.sessionId,
    name: result.name || `Sheet ${result.sheetId}`,
    headers,
    rows: [], // 由 SET_SHEET_DATA 填充首页
    total: result.rowCount, // DB cell 行数（含表头行；前端展示去掉表头行）
    page: 1,
    pageSize: 50,
    columnOrder: [...headers],
    columnVisibility: headers.reduce((acc, h) => ({ ...acc, [h]: true }), {}),
    selection: { selectedRowKeys: [], lastSelectedIndex: null },
    statusHighlights: {},
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
  // T5 action（导入流）
  IMPORT_SUCCESS: "IMPORT_SUCCESS",
  SET_SHEET_DATA: "SET_SHEET_DATA",
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

    // ---- T5（导入流）----
    case ACTION.IMPORT_SUCCESS: {
      // payload = ImportResult { sessionId, sheetId, rowCount, headers } + name
      const sheet = createSheetFromImport(action.payload);
      // 若已存在同 sheetId 的 Sheet，替换之；否则追加。
      const exists = state.sheets.some((s) => s.id === sheet.id);
      const sheets = exists
        ? state.sheets.map((s) => (s.id === sheet.id ? sheet : s))
        : [...state.sheets, sheet];
      return { ...state, sheets, activeSheetId: sheet.id };
    }
    case ACTION.SET_SHEET_DATA:
      // payload = { sheetId, headers, rows, total, page, pageSize }
      return {
        ...state,
        sheets: state.sheets.map((s) => {
          if (s.id !== action.payload.sheetId) return s;
          // PageData.rows 为 Vec<Vec<Option<String>>>；转为 antd 行对象。
          const headers = action.payload.headers || s.headers;
          const rows = action.payload.rows.map((row, i) => {
            const obj = { key: `${action.payload.sheetId}-${action.payload.page}-${i}` };
            headers.forEach((h, col) => {
              obj[h] = row[col] ?? null;
            });
            obj.status = "default";
            return obj;
          });
          return {
            ...s,
            headers,
            rows,
            total: action.payload.total ?? s.total,
            page: action.payload.page ?? s.page,
            pageSize: action.payload.pageSize ?? s.pageSize,
            columnOrder: [...headers],
            columnVisibility: headers.reduce(
              (acc, h) => ({ ...acc, [h]: s.columnVisibility?.[h] !== false }),
              {}
            ),
          };
        }),
      };
    default:
      return state;
  }
}

// 初始空态：无 Sheet。导入前显示 Workbench 空态文案。
initialState.sheets = [];
initialState.activeSheetId = null;

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

  // ---- T5（导入流）----
  const importSuccess = useCallback(
    (payload) => dispatch({ type: ACTION.IMPORT_SUCCESS, payload }),
    []
  );
  const setSheetData = useCallback(
    (payload) => dispatch({ type: ACTION.SET_SHEET_DATA, payload }),
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
    importSuccess,
    setSheetData,
  };
}
