import { save } from "@tauri-apps/plugin-dialog";
import { PAGE_SIZE } from "../constants";
import { getSheetData } from "./sheet";

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
  } catch (e) {
    // writeTextFile 失败（常见于 fs scope 未授权该路径）→ 回退浏览器 Blob 下载。
    // 保留日志便于诊断 scope 缺失，避免静默吞错。
    // eslint-disable-next-line no-console
    console.error("[export] saveTextFile writeTextFile failed, fallback to Blob:", e);
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
 * 把后端 PageData.rows（`Array<Array<string|null>>`）转成 antd 行对象。
 * 与 state/reducer.js 的 SET_SHEET_DATA 构造一致：{ key, [header]: value|null, status }。
 * 抽成纯函数供 reducer 与导出共用，避免重复实现。
 * @param {Array<Array<string|null>>} rawRows  后端 PageData.rows
 * @param {string[]} headers  字段名顺序
 * @param {number} sheetId  用于生成稳定 key
 * @param {number} [page=1]  当前页码（仅用于 key 区分）
 * @param {number} [pageSize=PAGE_SIZE]  每页行数（v1.1.2：用于 _rowIdx 全局行号计算）
 * @returns {Array<object>} antd 行对象数组
 */
export function toRowObjects(rawRows, headers, sheetId, page = 1, pageSize = PAGE_SIZE) {
  const base = (page - 1) * pageSize;
  return rawRows.map((row, i) => {
    const obj = { key: `${sheetId}-${page}-${i}`, _rowIdx: base + i + 1 };
    headers.forEach((h, col) => {
      obj[h] = row[col] ?? null;
    });
    obj.status = "default";
    return obj;
  });
}

/**
 * 导出前一次性拉取 Sheet 全表数据（不依赖当前页 sheet.rows）。
 *
 * 修复 BUG：原导出只读 sheet.rows（仅当前页 ≤50 行），>50 行数据缺失表现为空白。
 * 后端 get_sheet_data 的 SQL `LIMIT page_size OFFSET offset` 接受任意 page_size。
 *
 * T62：count_rows / query_cells 已统一排除 row_idx=0 表头行，sheet.total 即
 * 数据行数；传 page_size = sheet.total 恰好覆盖全部数据行（表头由 get_sheet_data
 * 通过 query_row_cells(sheet_id, 0) 单独返回，不占用数据页行槽）。无需新增 IPC
 * 或改 DB schema。
 *
 * @param {{id: number, total?: number, rows?: Array<object>}} sheet
 * @returns {Promise<{headers: string[], rows: Array<object>}>} 全表 antd 行对象
 */
async function fetchAllRowsForExport(sheet) {
  const total = sheet.total ?? (sheet.rows ? sheet.rows.length : 0) ?? 0;
  // total 为 0（空表）直接返回空，避免传 page_size=0 触发后端分页边界。
  if (total <= 0) {
    return { headers: [], rows: [] };
  }
  const data = await getSheetData(sheet.id, 1, total);
  return {
    headers: data.headers || [],
    rows: toRowObjects(data.rows, data.headers || [], sheet.id, 1),
  };
}

/**
 * 解析用户分隔符选项为实际字符。Tab/逗号/分号/竖线 选项 → 对应字符；
 * "custom" → 用 custom 值；custom 为空回退 Tab。
 * @param {string} sep  "tab" | "comma" | "semicolon" | "pipe" | "custom"
 * @param {string} [custom]  自定义分隔符原始字符串
 * @returns {string}
 */
function resolveSeparator(sep, custom) {
  switch (sep) {
    case "comma":
      return ",";
    case "semicolon":
      return ";";
    case "pipe":
      return "|";
    case "custom":
      return custom && custom.length > 0 ? custom : "\t";
    case "tab":
    default:
      return "\t";
  }
}

/**
 * 解析用户行尾选项为实际字符。
 * @param {"crlf"|"lf"} le  行尾模式
 * @returns {string}
 */
function resolveLineEnding(le) {
  return le === "lf" ? "\n" : "\r\n";
}

/**
 * 把当前 Sheet 导出为 CSV。
 *
 * 修复 BUG：内部 fetchAllRowsForExport 拉全表，不再依赖 sheet.rows（当前页）。
 *
 * @param {{id: number, name?: string}} sheet
 * @param {{separator?: string, customSeparator?: string, withHeader?: boolean, headers?: string[]}} [opts]
 *   - separator: "comma"|"semicolon"|"tab"|"custom"（默认 comma）
 *   - customSeparator: separator==="custom" 时的自定义字符
 *   - withHeader: 是否写表头行（默认 true）
 *   - headers: 选中导出的列名数组（默认全列）
 * @returns {Promise<boolean>}
 */
export async function exportSheetToCsv(sheet, opts = {}) {
  const { headers: allHeaders, rows } = await fetchAllRowsForExport(sheet);
  const selHeaders = opts.headers && opts.headers.length ? opts.headers : allHeaders;
  const sep = resolveSeparator(opts.separator ?? "comma", opts.customSeparator);
  const withHeader = opts.withHeader !== false;
  // CSV 转义：含 " / 换行 / 分隔符 时加引号并把 " 双写。
  const esc = (v) => {
    const s = v == null ? "" : String(v);
    if (s.includes('"') || s.includes('\n') || s.includes('\r') || s.includes(sep)) {
      return `"${s.replace(/"/g, '""')}"`;
    }
    return s;
  };
  const lines = [];
  if (withHeader) {
    lines.push(selHeaders.map(esc).join(sep));
  }
  for (const row of rows) {
    lines.push(selHeaders.map((h) => esc(row[h])).join(sep));
  }
  const csv = "\uFEFF" + lines.join("\r\n");
  return saveTextFile(
    `${sheet.name || "export"}.csv`,
    csv,
    "text/csv;charset=utf-8;",
    [{ name: "CSV", extensions: ["csv"] }]
  );
}

/**
 * 把当前 Sheet 导出为 JSON。
 *
 * 修复 BUG：内部 fetchAllRowsForExport 拉全表。
 *
 * @param {{id: number, name?: string}} sheet
 * @param {{indent?: number, ndjson?: boolean, headers?: string[]}} [opts]
 *   - indent: 缩进空格数，0 = 紧凑单行（默认 2）
 *   - ndjson: true → 每行一对象（NDJSON）；false → 标准数组（默认 false）
 *   - headers: 选中导出的列名数组
 * @returns {Promise<boolean>}
 */
export async function exportSheetToJson(sheet, opts = {}) {
  const { headers: allHeaders, rows } = await fetchAllRowsForExport(sheet);
  const selHeaders = opts.headers && opts.headers.length ? opts.headers : allHeaders;
  const indent = opts.indent ?? 2;
  const ndjson = opts.ndjson === true;
  const data = rows.map((row) => {
    const obj = {};
    for (const h of selHeaders) obj[h] = row[h] ?? "";
    return obj;
  });
  const json =
    ndjson
      ? data.map((o) => JSON.stringify(o)).join("\n")
      : JSON.stringify(data, null, indent);
  return saveTextFile(
    `${sheet.name || "export"}.json`,
    json,
    "application/json;charset=utf-8;",
    [{ name: "JSON", extensions: ["json"] }]
  );
}

/**
 * 把当前 Sheet 按模板导出为 TXT（每行一条）。
 *
 * 模板语法：`{字段名}` → 该列名；`{值}` → 该单元格值。
 * 其余字符（_、-、: 等）按字面输出，可自由填写作为连接符。
 *
 * 渲染规则：对每行数据的每个选中列各渲染一行。
 * 默认模板 `{字段名}_{值}` → 形如 `username_zhangsan`。
 *
 * 修复 BUG：内部 fetchAllRowsForExport 拉全表，不再只导当前页。
 *
 * @param {{id: number, name?: string}} sheet
 * @param {{template?: string, lineEnding?: "crlf"|"lf", headers?: string[]}} [opts]
 * @returns {Promise<boolean>}
 */
export async function exportSheetToTxt(sheet, opts = {}) {
  const { headers: allHeaders, rows } = await fetchAllRowsForExport(sheet);
  const selHeaders = opts.headers && opts.headers.length ? opts.headers : allHeaders;
  const eol = resolveLineEnding(opts.lineEnding ?? "crlf");
  const template = opts.template && opts.template.trim() ? opts.template : "{字段名}_{值}";
  const lines = [];
  for (const row of rows) {
    for (const h of selHeaders) {
      lines.push(
        template
          .replace(/\{字段名\}/g, h)
          .replace(/\{name\}/g, h)
          .replace(/\{值\}/g, row[h] ?? "")
          .replace(/\{value\}/g, row[h] ?? "")
      );
    }
  }
  const txt = "\uFEFF" + lines.join(eol);
  return saveTextFile(
    `${sheet.name || "export"}.txt`,
    txt,
    "text/plain;charset=utf-8;",
    [{ name: "TXT", extensions: ["txt"] }]
  );
}
