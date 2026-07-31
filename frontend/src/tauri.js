// Tauri v2 invoke 封装层。
//
// v1.0.0（T7）：AI 占位契约 `aiSuggest` / `invokeAiOp`。
// v1.0.0（T5）：导入流 `importFile` / `getSheetData`。
// v1.0.0（全格式扩展）：tshark 设置 `detectTshark` / `loadTsharkPath` /
// `saveTsharkPath`，导出工具 `exportSheetToCsv`。

import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

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
 * 调用 `detect_tshark` IPC：自动探测本机 tshark。
 * @returns {Promise<{path: string, version: string} | null>} TsharkInfo 或 null
 */
export function detectTshark() {
  return invoke("detect_tshark");
}

/**
 * 调用 `load_tshark_path` IPC：启动时加载 settings.json 中的 tshark 路径。
 * @returns {Promise<string | null>} 已保存的 tshark 路径或 null
 */
export function loadTsharkPath() {
  return invoke("load_tshark_path");
}

/**
 * 调用 `save_tshark_path` IPC：保存 tshark 路径到 settings.json + 注入运行时。
 * @param {string | null} path - tshark 绝对路径；null 清除覆盖
 * @returns {Promise<void>}
 */
export function saveTsharkPath(path) {
  return invoke("save_tshark_path", { path });
}

/**
 * 公共文件保存：优先 Tauri save 对话框 + writeTextFile；回退浏览器 Blob 下载。
 * @param {string} filename  建议文件名（含扩展名）
 * @param {string} content    文本内容
 * @param {string} mimeType   如 "text/csv;charset=utf-8;"
 * @param {[{name: string, extensions: string[]}]} [filters]  save 对话框过滤器
 * @returns {Promise<boolean>} 是否导出成功（用户取消返回 false）
 */
export async function saveTextFile(filename, content, mimeType, filters) {
  const blob = new Blob([content], { type: mimeType || "text/plain;charset=utf-8;" });
  try {
    const target = await save({
      defaultPath: filename,
      filters: filters || [{ name: "File", extensions: ["*"] }],
    });
    if (!target) return false;
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");
    await writeTextFile(target, content);
    return true;
  } catch {
    // 回退：浏览器 Blob 下载（开发态或 Tauri 不可用）。
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    return true;
  }
}

/**
 * 把当前 Sheet 导出为 CSV。
 * @param {{name: string, headers: string[], rows: Array<object>}} sheet
 * @returns {Promise<boolean>}
 */
export async function exportSheetToCsv(sheet) {
  const { headers, rows, name } = sheet;
  const esc = (v) => {
    const s = v == null ? "" : String(v);
    if (/[",\n\r]/.test(s)) {
      return `"${s.replace(/"/g, '""')}"`;
    }
    return s;
  };
  const lines = [headers.map(esc).join(",")];
  for (const row of rows) {
    lines.push(headers.map((h) => esc(row[h])).join(","));
  }
  const csv = "\uFEFF" + lines.join("\r\n");
  return saveTextFile(
    `${name || "export"}.csv`,
    csv,
    "text/csv;charset=utf-8;",
    [{ name: "CSV", extensions: ["csv"] }]
  );
}

/**
 * 把当前 Sheet 导出为 JSON。
 * @param {{name: string, headers: string[], rows: Array<object>}} sheet
 * @returns {Promise<boolean>}
 */
export async function exportSheetToJson(sheet) {
  const { headers, rows, name } = sheet;
  const data = rows.map((row) => {
    const obj = {};
    for (const h of headers) obj[h] = row[h] ?? "";
    return obj;
  });
  const json = JSON.stringify(data, null, 2);
  return saveTextFile(
    `${name || "export"}.json`,
    json,
    "application/json;charset=utf-8;",
    [{ name: "JSON", extensions: ["json"] }]
  );
}

/**
 * 把当前 Sheet 按模板导出为 TXT（一行一条记录）。
 *
 * 模板语法：`{字段名}` → 该字段值；`{其它}` → 原样输出其内容
 * （如 `{_}` → `_`、`{-}` → `-`、`{}` → 空）。模板中不在 `{}` 内的字符
 * 也按字面量输出。无模板时回退为 Tab 分隔全列。
 *
 * @param {{name: string, headers: string[], rows: Array<object>}} sheet
 * @param {string} [template]  模板字符串，如 `{phone}_{name}`
 * @returns {Promise<boolean>}
 */
export async function exportSheetToTxt(sheet, template) {
  const { headers, rows, name } = sheet;
  const renderRow = (row) => {
    if (!template) return headers.map((h) => row[h] ?? "").join("\t");
    return template.replace(/\{([^{}]*)\}/g, (_m, key) => {
      // 字段名命中 → 取该行对应字段值；否则把括号内字面量原样输出。
      return headers.includes(key) ? row[key] ?? "" : key;
    });
  };
  const lines = rows.map(renderRow);
  const txt = "\uFEFF" + lines.join("\r\n");
  return saveTextFile(
    `${name || "export"}.txt`,
    txt,
    "text/plain;charset=utf-8;",
    [{ name: "TXT", extensions: ["txt"] }]
  );
}
