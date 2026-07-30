// Tauri v2 invoke 封装层。
//
// v1.0.0（T7）：仅落地 AI 占位契约 `aiSuggest` / `invokeAiOp`。
// 其它封装（导入/更新等）留给后续任务，此处仅留 TODO，不写 invoke 调用。

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

// TODO(T5): importFile(path) / getSheetData(...) 封装
// TODO(T6): checkUpdate() / installUpdate() 封装
