// v1.0.0 全局 state 常量（ACTION + initialState）。
// T13 拆分自原 state.js monolith；逻辑/字段保持不变。

export const initialState = {
  // T2 字段（保留，禁止覆盖）
  currentView: "workbench", // 'workbench' | 'settings'
  activeCapability: null, // null | 'mask' | 'validate' | 'extract' | 'rules' | 'columnOps' | 'crypto'
  aiPanel: { visible: false }, // 默认折叠为图标条
  // T3 字段
  sheets: [], // Sheet[]
  activeSheetId: null, // string | null
  // tshark 设置（v1.0.0 全格式扩展）
  tsharkPath: null, // string | null：用户覆盖路径
  tsharkDetected: null, // { path, version } | null
  tsharkLoading: false,
  // v1.1.2 全局每页行数（settings.json 持久化）
  // 初始值 = PAGE_SIZE；PageSizeCard 启动时 loadPageSize 覆盖。
  // 新建/导入 Sheet 的 sheet.pageSize 由 reducer 从此字段继承。
  pageSize: 50,
  // v1.1.1 state（撤销 / 搜索 / 列操作）
  undoStack: [], // Array<{ id: number, kind: string, createdAt: string }>，撤销工具栏数据源
  searchState: { query: "", useRegex: false, colIdx: null, page: 1 }, // 搜索框受控状态（colIdx: null = 全表）
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
  // v1.1.2 全局每页行数（payload = number）
  SET_PAGE_SIZE: "SET_PAGE_SIZE",
  // v1.1.0 action（脱敏/校验/提取 → 行高亮）
  APPLY_ROW_STATUSES: "APPLY_ROW_STATUSES",
  // v1.1.1 action（撤销 / 搜索 / 列操作）
  SET_SEARCH_STATE: "SET_SEARCH_STATE", // 搜索框状态（query/useRegex/colIdx/page）
  APPLY_SEARCH_HITS: "APPLY_SEARCH_HITS", // 命中写入 sheet.searchHits 供 DataTable 高亮（每次先清空）
  APPLY_SEARCH_ROWS: "APPLY_SEARCH_ROWS", // v1.1.1 hotfix 行级搜索结果行写入 sheet.searchRows/searchTotal
  CLEAR_SEARCH: "CLEAR_SEARCH", // 清空搜索状态 + 高亮 + searchRows
  ADD_SHEET_FROM_PARSE: "ADD_SHEET_FROM_PARSE", // parse_column_as_json 返回的新 Sheet 加入 sheets
  SET_UNDO_STACK: "SET_UNDO_STACK", // 可撤销操作列表（撤销工具栏用）
};

// 初始空态：无 Sheet。导入前显示 Workbench 空态文案。
initialState.sheets = [];
initialState.activeSheetId = null;
