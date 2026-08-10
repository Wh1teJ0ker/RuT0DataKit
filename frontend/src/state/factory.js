// Sheet 工厂。
// Sheet 对象结构：{ id, sessionId, name, headers, rows, total, page, pageSize,
//                  columnOrder, columnVisibility, selection, statusHighlights,
//                  searchHits, searchRows, searchTotal }
// 「新建」Tab 产生空表（无列、无行），由用户在导入或后续列编辑流程中填充；
// IMPORT_SUCCESS action 用 ImportResult 填充真实 Sheet。
// ADD_SHEET_FROM_PARSE action 用 ParseResult（parse_column_as_json 返回）填充新 Sheet。
// sessionId 由真实导入流填充（importFile → ImportResult.sessionId）。
//
// v1.1.1 hotfix：新增 `searchRows` / `searchTotal` 两个字段。
// - `searchRows`：行级搜索命中行（toRowObjects 结果），非 null 时 DataTable 切换为
//   「只保留搜索结果」渲染；`null` 表示不在搜索态。
// - `searchTotal`：搜索命中行数（供分页 total）。
// - `searchHits`：仍保留单元格高亮区间（rowKey → colHeader → [[start, end]]）。
//   APPLY_SEARCH_HITS 现在每次先清空再写入，避免 stale highlight。
//
// v1.1.2：3 个工厂新增可选 `pageSize` 参数（默认 PAGE_SIZE）。
// reducer 在 ADD_SHEET / IMPORT_SUCCESS / ADD_SHEET_FROM_PARSE 时传入
// `state.pageSize`（全局每页行数），使新建/导入 Sheet 继承当前全局设置。

import { PAGE_SIZE } from "../constants";

let sheetSeq = 0;

/// 默认 Sheet 名（递增序号，与 createEmptySheet 共享计数器）。
/// reducer 在 payload 未指定 name 时调用此函数，避免跨模块读取私有 sheetSeq。
export function defaultSheetName() {
  sheetSeq += 1;
  return `Sheet ${sheetSeq}`;
}

function createEmptySheet(name, pageSize = PAGE_SIZE) {
  sheetSeq += 1;
  const id = `sheet-${Date.now()}-${sheetSeq}`;
  const headers = [];
  return {
    id,
    sessionId: null, // 真实导入流接管后填充（importFile → ImportResult.sessionId）
    name,
    headers,
    rows: [],
    total: 0,
    page: 1,
    pageSize,
    columnOrder: [],
    columnVisibility: {},
    selection: { selectedRowKeys: [], lastSelectedIndex: null },
    statusHighlights: {}, // v1.1.0 ValidatePanel/MaskPanel/ExtractPanel 触发行高亮
    searchHits: {}, // v1.1.1 搜索命中高亮：{ [rowKey]: { [colHeader]: [[start, end], ...] } }
    searchRows: null, // v1.1.1 hotfix 行级搜索结果行（toRowObjects），null = 不在搜索态
    searchTotal: 0, // v1.1.1 hotfix 行级搜索命中行数
  };
}

// 由 ImportResult（camelCase）构造真实 Sheet。rows 初始为空，由 SET_SHEET_DATA
// action 在导入后/翻页后填充首页数据。
export function createSheetFromImport(result, pageSize = PAGE_SIZE) {
  const headers = result.headers || [];
  return {
    id: result.sheetId, // Sheet ID 来自 DB（i64），与 getSheetData 入参一致
    sessionId: result.sessionId,
    name: result.name || `Sheet ${result.sheetId}`,
    headers,
    rows: [], // 由 SET_SHEET_DATA 填充首页
    total: result.rowCount, // DB cell 行数（含表头行；前端展示去掉表头行）
    page: 1,
    pageSize,
    columnOrder: [...headers],
    // 单遍构造（Object.fromEntries），避免 reduce + spread 的 O(C²) 复制；
    // 重复 header 后写覆盖前写，语义与原 reduce 一致。
    columnVisibility: Object.fromEntries(headers.map((h) => [h, true])),
    selection: { selectedRowKeys: [], lastSelectedIndex: null },
    statusHighlights: {},
    searchHits: {}, // v1.1.1 搜索命中高亮（默认空）
    searchRows: null, // v1.1.1 hotfix 行级搜索结果行
    searchTotal: 0, // v1.1.1 hotfix 行级搜索命中行数
  };
}

// 由 ParseResult（parse_column_as_json 返回，camelCase）构造新 Sheet。
// 与 createSheetFromImport 几乎一致，但 id 取 newSheetId（而非 sheetId），
// sessionId / name / column 由调用方在 payload 传入。rows 初始为空，
// 由 SET_SHEET_DATA action 在 ADD_SHEET_FROM_PARSE 后填充。
export function createSheetFromParse(result, pageSize = PAGE_SIZE) {
  const headers = result.headers || [];
  return {
    id: result.newSheetId,
    sessionId: result.sessionId,
    name: result.name || (result.column ? `${result.column}_json` : `Sheet ${result.newSheetId}`),
    headers,
    rows: [], // 由 SET_SHEET_DATA 填充首页
    total: result.rowCount,
    page: 1,
    pageSize,
    columnOrder: [...headers],
    // 单遍构造（Object.fromEntries），避免 reduce + spread 的 O(C²) 复制；
    // 重复 header 后写覆盖前写，语义与原 reduce 一致。
    columnVisibility: Object.fromEntries(headers.map((h) => [h, true])),
    selection: { selectedRowKeys: [], lastSelectedIndex: null },
    statusHighlights: {},
    searchHits: {},
    searchRows: null, // v1.1.1 hotfix 行级搜索结果行
    searchTotal: 0, // v1.1.1 hotfix 行级搜索命中行数
  };
}

export { createEmptySheet };
