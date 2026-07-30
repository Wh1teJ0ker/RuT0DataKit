// Tauri v2 invoke 封装层。
//
// v1.0.0（T7）：AI 占位契约 `aiSuggest` / `invokeAiOp`。
// v1.0.0（T5）：导入流 `importFile` / `getSheetData`。

import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `ai_suggest` IPC（v1.1+ 释放，v1.0.0 占位）。
 * @param {object} context - AiContext（camelCase），如 { sheetId, selection, prompt }
 * @returns {Promise<{suggestion: string, confidence: number}>} AiSuggestion
 */
export function aiSuggest(context) {
  return invoke("ai_suggest", { context });
}

/**
 * 调用 `invoke_ai_op` IPC（v1.1+ 释放，v1.0.0 占位）。
 * @param {string} op - 操作名
 * @param {object|any} params - 任意 JSON 参数
 * @returns {Promise<any>} serde_json::Value
 */
export function invokeAiOp(op, params) {
  return invoke("invoke_ai_op", { op, params });
}

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
