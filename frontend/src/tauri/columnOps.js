import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `base64_column` IPC：对指定列就地 Base64 编/解码（可撤销，已入撤销栈）。
 * @param {number} sheetId  Sheet ID
 * @param {string} column   列名（headers 中的值）
 * @param {string} mode     "encode" | "decode"（后端 Base64Mode serde lowercase）
 * @returns {Promise<{affected: number, skipped: number}>} Base64Result（camelCase）
 */
export function base64Column(sheetId, column, mode) {
  return invoke("base64_column", { sheetId, column, mode });
}

/**
 * 调用 `hash_column` IPC：对指定列就地计算哈希（MD5/SHA1/SHA256，可撤销，已入撤销栈）。
 * @param {number} sheetId    Sheet ID
 * @param {string} column     列名（headers 中的值）
 * @param {string} algorithm  "md5" | "sha1" | "sha256"（后端 HashAlgorithm serde lowercase）
 * @param {string} case_      "lower" | "upper"（后端 HashCase serde lowercase，v1.1.5 T83）
 * @returns {Promise<{affected: number, skipped: number}>} HashResult（camelCase）
 */
export function hashColumn(sheetId, column, algorithm, case_) {
  return invoke("hash_column", { sheetId, column, algorithm, case: case_ });
}

/**
 * 调用 `transform_column` IPC：对指定列就地做大小写归一化（可撤销，已入撤销栈）。
 * @param {number} sheetId  Sheet ID
 * @param {string} column   列名（headers 中的值）
 * @param {string} op       "uppercase" | "lowercase"（后端 TransformOp serde lowercase）
 * @returns {Promise<{affected: number, skipped: number}>} Base64Result 形（camelCase）
 */
export function transformColumn(sheetId, column, op) {
  return invoke("transform_column", { sheetId, column, op });
}
