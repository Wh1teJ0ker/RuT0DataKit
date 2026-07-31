// v1.0.0 全局 state 常量（ACTION + initialState）。
// T13 拆分自原 state.js monolith；逻辑/字段保持不变。

export const initialState = {
  // T2 字段（保留，禁止覆盖）
  currentView: "workbench", // 'workbench' | 'settings'
  activeCapability: null, // null | 'mask' | 'validate' | 'extract' | 'rules'
  aiPanel: { visible: false }, // 默认折叠为图标条
  // T3 字段
  sheets: [], // Sheet[]
  activeSheetId: null, // string | null
  // tshark 设置（v1.0.0 全格式扩展）
  tsharkPath: null, // string | null：用户覆盖路径
  tsharkDetected: null, // { path, version } | null
  tsharkLoading: false,
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
  // tshark 设置 action（v1.0.0 全格式扩展）
  SET_TSHARK_PATH: "SET_TSHARK_PATH",
  SET_TSHARK_DETECTED: "SET_TSHARK_DETECTED",
  SET_TSHARK_LOADING: "SET_TSHARK_LOADING",
};

// 初始空态：无 Sheet。导入前显示 Workbench 空态文案。
initialState.sheets = [];
initialState.activeSheetId = null;
