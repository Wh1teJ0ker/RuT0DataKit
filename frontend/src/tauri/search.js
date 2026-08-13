import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `search_cells` IPC：分页搜索匹配单元格。
 * @param {number} sheetId   Sheet ID
 * @param {string} query     搜索文本（useRegex=true 时为正则）
 * @param {boolean} useRegex 是否正则模式
 * @param {number|null} colIdx 0-based 列号；null 表示搜全表所有列
 * @param {number} page       页码，从 1 开始
 * @param {number} pageSize   每页命中条数
 * @returns {Promise<{rows: Array<{rowIdx: number, colIdx: number, value: string|null, matches: Array<{start: number, end: number}>}>, total: number, page: number, pageSize: number}>}
 */
export function searchCells(sheetId, query, useRegex, colIdx, page, pageSize) {
  return invoke("search_cells", {
    sheetId,
    query,
    useRegex,
    colIdx,
    page,
    pageSize,
  });
}

/**
 * 调用 `search_rows` IPC（v1.1.1 hotfix）：**行级**分页搜索，返回整行数据 +
 * 命中区间。前端用其结果「只保留搜索结果」渲染整张过滤后的表，并高亮匹配单元格。
 * @param {number} sheetId   Sheet ID
 * @param {string} query     搜索文本（useRegex=true 时为正则）
 * @param {boolean} useRegex 是否正则模式
 * @param {number|null} colIdx 0-based 列号；null 表示搜全表所有列
 * @param {number} page       页码，从 1 开始
 * @param {number} pageSize   每页命中行数
 * @returns {Promise<{rows: Array<{rowIdx: number, cells: Array<string|null>, hits: Array<{colIdx: number, value: string|null, matches: Array<{start: number, end: number}>}>, total: number, page: number, pageSize: number}>}
 */
export function searchRows(sheetId, query, useRegex, colIdx, page, pageSize) {
  return invoke("search_rows", {
    sheetId,
    query,
    useRegex,
    colIdx,
    page,
    pageSize,
  });
}

/**
 * 调用 `replace_all` IPC：全表替换。
 * @param {number} sheetId  Sheet ID
 * @param {string} from    搜索文本（useRegex=true 时为正则）
 * @param {string} to      替换文本
 * @param {boolean} useRegex 是否正则模式
 * @returns {Promise<{affected: number}>} 受影响单元格数
 */
export function replaceAll(sheetId, from, to, useRegex) {
  return invoke("replace_all", { sheetId, from, to, useRegex });
}

/**
 * 调用 `replace_in_column` IPC：在指定列内替换匹配项。
 * @param {number} sheetId  Sheet ID
 * @param {string} column   列名
 * @param {string} from     搜索文本（useRegex=true 时为正则）
 * @param {string} to       替换文本
 * @param {boolean} useRegex 是否正则模式
 * @returns {Promise<{affected: number}>} 受影响单元格数
 */
export function replaceInColumn(sheetId, column, from, to, useRegex) {
  return invoke("replace_in_column", {
    sheetId,
    column,
    from,
    to,
    useRegex,
  });
}
