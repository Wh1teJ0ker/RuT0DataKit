import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `import_file` IPC：解析 CSV/XLSX → 写 DB cells → 返回 ImportResult。
 * @param {string} path - 文件绝对路径（由 @tauri-apps/plugin-dialog 的 open 选出）
 * @returns {Promise<{sessionId: number, sheetId: number, rowCount: number, headers: string[]}>} ImportResult
 */
export function importFile(path) {
  return invoke("import_file", { path });
}

/**
 * 调用 `get_sheet_data` IPC：分页查询 Sheet cells → PageData。
 * @param {number} sheetId - Sheet ID
 * @param {number} page - 页码，从 1 开始
 * @param {number} pageSize - 每页行数（不含表头行）
 * @returns {Promise<{headers: string[], rows: Array<Array<string|null>>, total: number, page: number, pageSize: number}>} PageData
 */
export function getSheetData(sheetId, page, pageSize) {
  return invoke("get_sheet_data", { sheetId, page, pageSize });
}

/**
 * 调用 `parse_column_as_json` IPC：把指定列解析为 JSON，落成新 Sheet。
 * @param {number} sheetId   源 Sheet ID
 * @param {string} column    列名
 * @param {number} sessionId 当前会话 ID（用于派生新 Sheet 的 sessionId）
 * @returns {Promise<{newSheetId: number, headers: string[], rowCount: number, skipped: number}>}
 */
export function parseColumnAsJson(sheetId, column, sessionId) {
  return invoke("parse_column_as_json", { sheetId, column, sessionId });
}
