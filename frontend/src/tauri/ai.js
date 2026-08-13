import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `ai_suggest` IPC（业务能力开发中，v1.0.0 占位）。
 * @param {object} context - AiContext（camelCase），如 { sheetId, selection, prompt }
 * @returns {Promise<{suggestion: string, confidence: number}>} AiSuggestion
 */
export function aiSuggest(context) {
  return invoke("ai_suggest", { context });
}

/**
 * 调用 `invoke_ai_op` IPC（业务能力开发中，v1.0.0 占位）。
 * @param {string} op - 操作名
 * @param {object|any} params - 任意 JSON 参数
 * @returns {Promise<any>} serde_json::Value
 */
export function invokeAiOp(op, params) {
  return invoke("invoke_ai_op", { op, params });
}
