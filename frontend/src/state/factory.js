// Sheet 工厂（T13 拆分自原 state.js monolith）。
// Sheet 对象结构：{ id, sessionId, name, headers, rows, total, page, pageSize,
//                  columnOrder, columnVisibility, selection, statusHighlights }
// v1.0.0（T5）：真实导入流接管。「新建」Tab 产生空表（无列、无行），由用户
// 在导入或后续列编辑流程中填充；IMPORT_SUCCESS action 用 ImportResult 填充真实 Sheet。

let sheetSeq = 0;

/// 默认 Sheet 名（递增序号，与 createEmptySheet 共享计数器）。
/// reducer 在 payload 未指定 name 时调用此函数，避免跨模块读取私有 sheetSeq。
export function defaultSheetName() {
  sheetSeq += 1;
  return `Sheet ${sheetSeq}`;
}

function createEmptySheet(name) {
  sheetSeq += 1;
  const id = `sheet-${Date.now()}-${sheetSeq}`;
  const headers = [];
  return {
    id,
    sessionId: null, // T5 接管后由真实导入流填充
    name,
    headers,
    rows: [],
    total: 0,
    page: 1,
    pageSize: 50,
    columnOrder: [],
    columnVisibility: {},
    selection: { selectedRowKeys: [], lastSelectedIndex: null },
    statusHighlights: {}, // 状态高亮触发逻辑开发中，v1.0.0 占位
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

export { createEmptySheet };
