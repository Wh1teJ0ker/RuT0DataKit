// Tauri v2 invoke 封装层。
//
// v1.0.0：AI 占位契约 `aiSuggest` / `invokeAiOp`；导入流 `importFile` /
// `getSheetData`；tshark 设置 `detectTshark` / `loadTsharkPath` /
// `saveTsharkPath`；更新检查 `checkUpdate` / `installUpdate`。
// v1.0.0（导出面板重构）：`toRowObjects` 纯函数 + `fetchAllRowsForExport`
// 全表拉取（修复只导当前页的 BUG），CSV/JSON/TXT 导出函数接收 options
// （分隔符/表头/模板/行尾/NDJSON 等）+ 内部拉全表。
// v1.1.0：新增数据处理原型 IPC（脱敏 / 校验 / 提取 / 规则管理）。

import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { PAGE_SIZE } from "./constants";

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
 * 调用 `load_page_size` IPC：启动时加载 settings.json 中的全局每页行数。
 * v1.1.2 新增。
 * @returns {Promise<number | null>} 已保存的每页行数或 null（未配置时回退默认 50）
 */
export function loadPageSize() {
  return invoke("load_page_size");
}

/**
 * 调用 `save_page_size` IPC：保存全局每页行数到 settings.json。
 * v1.1.2 新增。
 * @param {number | null} pageSize - 每页行数；null 清除（回退默认 50）
 * @returns {Promise<void>}
 */
export function savePageSize(pageSize) {
  return invoke("save_page_size", { pageSize });
}

/**
 * 调用 `check_update` IPC：检查应用更新（无网络/无新版本统一降级 available=false）。
 * @returns {Promise<{available: boolean, version: string|null, notes: string|null}>} UpdateStatus（camelCase）
 */
export function checkUpdate() {
  return invoke("check_update");
}

/**
 * 调用 `install_update` IPC：安装已下载的更新（重启后生效）。
 * @returns {Promise<void>}
 */
export function installUpdate() {
  return invoke("install_update");
}

// v1.1.0 数据处理原型 IPC 封装（脱敏 / 校验 / 提取 / 规则管理）。

/**
 * 调用 `mask_column` IPC：对指定列就地脱敏。
 * v1.1.3 T49：新增 `template` 参数（临时覆盖规则的模板，不写回 DB）。
 *   前端选预设 → 填充 6 个可编辑参数框 → 透传给本参数执行脱敏。
 *   `null` → 用规则自身的 template。空模板（所有字段 null）→ 不脱敏（透传）。
 * @param {number} sheetId      Sheet ID
 * @param {string} column       列名（headers 中的值）
 * @param {string|null} [ruleId] 规则 ID（可选；指向 DB mask 规则）
 * @param {string|null} [replacement] 掩码字符（取首个字符；空/null → 默认 `*`；不写回 DB）
 * @param {object|null} [template] 通用模板参数（camelCase：keepPrefix/keepSuffix/
 *   maskChar/maskMinLen/minLen/maxLen，全 null = 不脱敏；不写回 DB）
 * @returns {Promise<{affected: number}>} 受影响行数
 */
export function maskColumn(sheetId, column, ruleId, replacement, template) {
  return invoke("mask_column", { sheetId, column, ruleId, replacement, template });
}

/**
 * 调用 `validate_column` IPC：按规则校验指定列。
 * @param {number} sheetId  Sheet ID
 * @param {string} column   列名
 * @param {string} ruleId   规则 ID（RuleKind=Validate）
 * @returns {Promise<{results: Array<{rowIdx: number, passed: boolean, message: string}>}>}
 */
export function validateColumn(sheetId, column, ruleId) {
  return invoke("validate_column", { sheetId, column, ruleId });
}

/**
 * 调用 `extract_column` IPC：从指定列提取 PII（手机/邮箱/身份证）。
 * v1.1.0 保留命令；前端提取面板改走内部正则预览，不再调用本函数。
 * @param {number} sheetId  Sheet ID
 * @param {string} column   列名
 * @returns {Promise<{results: Array<{rowIdx: number, hits: Array<{kind: string, value: string, start: number, end: number}>}>}>}
 */
export function extractColumn(sheetId, column) {
  return invoke("extract_column", { sheetId, column });
}

/**
 * 调用 `list_rules` IPC：列出全部规则（v1.1.0 三条姓名相关内置规则）。
 * @returns {Promise<Array<{id: string, name: string, kind: string, field: string|null, pattern: string|null, replacement: string|null, enabled: boolean, description: string}>>}
 */
export function listRules() {
  return invoke("list_rules");
}

/**
 * 调用 `toggle_rule` IPC：启用/禁用规则。
 * @param {string} ruleId   规则 ID
 * @param {boolean} enabled 是否启用
 * @returns {Promise<void>}
 */
export function toggleRule(ruleId, enabled) {
  return invoke("toggle_rule", { ruleId, enabled });
}

/**
 * 调用 `update_rule_params` IPC：更新规则可填参数（pattern / replacement）。
 * v1.1.0：仅允许改参数，不可新增规则。`null` 字段表示不变。
 * @param {string} ruleId      规则 ID
 * @param {string|null} pattern       正则模式（null 不变）
 * @param {string|null} replacement   脱敏替换模板（null 不变）
 * @returns {Promise<void>}
 */
export function updateRuleParams(ruleId, pattern, replacement) {
  return invoke("update_rule_params", { ruleId, pattern, replacement });
}

/**
 * 调用 `update_rule_template` IPC：更新规则的模板脱敏参数（`rules.template` 列）。
 * v1.1.3 T49 新增。T54 拆分：前端选预设 → 填充 7 个可编辑参数框（含 T53 反向
 * 脱敏开关）→ 调本命令持久化到 `simple-mask`（整段脱敏）或 `segment-mask`
 * （分段脱敏）规则。
 * @param {string} ruleId      规则 ID（`simple-mask` 或 `segment-mask`）
 * @param {object|null} template 模板参数（Simple：keepPrefix/keepSuffix/maskChar/
 *   maskMinLen/minLen/maxLen/reverse；Segment：maskChar/delimiter/segments）。
 *   `null` → 清空模板（写 NULL）。空模板（所有字段 null / 空）→ 不脱敏。
 * @returns {Promise<void>}
 */
export function updateRuleTemplate(ruleId, template) {
  return invoke("update_rule_template", { ruleId, template });
}

/**
 * 调用 `update_rule_extract_config` IPC：更新提取规则的 pattern + params。
 * v1.1.3 T55 新增。
 * @param {string} ruleId   规则 id（如 "phone-extract"）
 * @param {string|null} pattern  提取正则；null 表示不变，"" 表示清空
 * @param {object|null} params   ExtractParams（{validator:"luhn"} / {validator:"phonePrefix",allowedPrefixes:[...]} / {validator:"ipv4"} / {validator:"ipv6"} / {validator:"idcard"}）；null 表示清空
 * @returns {Promise<void>}
 */
export function updateRuleExtractConfig(ruleId, pattern, params) {
  return invoke("update_rule_extract_config", { ruleId, pattern, params });
}

/**
 * 调用 `extract_validate_to_new_sheet` IPC：提取指定列候选 → 函数式严格校验
 * （Luhn / IPv4 / IPv6 / 手机前缀 / 身份证校验码 + 性别推断）→ 结果落到新 Tab
 * （T56：2 列 [类型, 数据值]，只写有效候选）。v1.1.3 T55 新增，T55c 追加性别联合校验，
 * T56 改为批量多规则 + 两列输出。
 * @param {number} sheetId   源 Sheet ID
 * @param {string} column    源列表头名
 * @param {string[]} ruleIds  提取规则 id 数组（如 ["phone-extract", "idcard-extract"]），
 *   T56：支持批量多规则，一次提取到同一个新 Tab
 * @param {number} sessionId 当前会话 ID（新 sheet 挂到本会话）
 * @param {string|null} [genderCol]  T55c：可选性别列表头名，仅当选中的规则含
 *   idcard-extract 时使用。指定后对有效身份证候选比对第 17 位推断性别
 *   （奇=男/偶=女）与该列值，矛盾判无效（不写入新 Tab）。非 idcard 规则不受影响。
 * @returns {Promise<{newSheetId: number, headers: string[], rowCount: number, skipped: number}>}
 */
export function extractValidateToNewSheet(
  sheetId,
  column,
  ruleIds,
  sessionId,
  genderCol = null,
) {
  return invoke("extract_validate_to_new_sheet", {
    sheetId,
    column,
    ruleIds,
    sessionId,
    genderCol,
  });
}

/**
 * 调用 `validate_rows_to_two_sheets` IPC：行级多字段校验 → 通过 / 失败两行分流到
 * 两个新 Tab（保留原列，不新增列）。v1.1.3 T57 新增。
 *
 * 7 个字段（username/name/sex/birth/idcard/phone/address）各自映射到源 sheet 的
 * 某个列表头；未映射的字段（值为 null 或空串）不校验。跨字段联合校验（sex vs
 * idcard 第 17 位性别推断、birth vs idcard 第 7-14 位出生日期码）仅当相关字段都
 * 映射且 idcard 本身有效时生效。
 *
 * @param {number} sheetId    源 Sheet ID
 * @param {number} sessionId  当前会话 ID（两个新 Tab 挂到本会话）
 * @param {object} fieldColumns  字段→列名映射（camelCase；未映射字段传 null 或省略）：
 *   `{ username?: string, name?: string, sex?: string, birth?: string,
 *      idcard?: string, phone?: string, address?: string }`
 * @param {string[]} [phonePrefixes=[]]  手机号三位前缀白名单；空数组 = 仅检查 1 开头 + 11 位
 * @returns {Promise<{validSheet: {newSheetId: number, headers: string[], rowCount: number, skipped: number}, invalidSheet: {newSheetId: number, headers: string[], rowCount: number, skipped: number}, invalidReasons: Array<{sourceRow: number, field: string, reason: string}>}>}
 */
export function validateRowsToTwoSheets(
  sheetId,
  sessionId,
  fieldColumns,
  phonePrefixes = [],
) {
  return invoke("validate_rows_to_two_sheets", {
    sheetId,
    sessionId,
    fieldColumns,
    phonePrefixes,
  });
}

// v1.1.1 撤销 / 搜索 / 列操作 IPC 封装。

/**
 * 调用 `undo_operation` IPC：按 op_id 撤销单个操作（就地回滚 DB cells）。
 * 后端签名 `undo_operation(op_id: i64, db: State<...>)` 只接 op_id（db 自动注入），
 * 因此 JS 侧不传 sheetId。
 * @param {number} opId  操作 ID（来自 listUndoableOperations 返回的 id）
 * @returns {Promise<{restored: number}>} 还原的行数
 */
export function undoOperation(opId) {
  return invoke("undo_operation", { opId });
}

/**
 * 调用 `redo_operation` IPC：按 op_id 重做单个操作。
 * 与 undo_operation 对称，只接 op_id。
 * @param {number} opId  操作 ID
 * @returns {Promise<{restored: number}>} 还原的行数
 */
export function redoOperation(opId) {
  return invoke("redo_operation", { opId });
}

/**
 * 调用 `list_undoable_operations` IPC：列出可撤销操作（撤销工具栏数据源）。
 * @param {number} sheetId  Sheet ID
 * @returns {Promise<Array<{id: number, kind: string, createdAt: string}>>}
 */
export function listUndoableOperations(sheetId) {
  return invoke("list_undoable_operations", { sheetId });
}

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
 * @returns {Promise<{rows: Array<{rowIdx: number, cells: Array<string|null>, hits: Array<{colIdx: number, value: string|null, matches: Array<{start: number, end: number}>}>}>, total: number, page: number, pageSize: number}>}
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
 * 调用 `parse_column_as_json` IPC：把指定列解析为 JSON，落成新 Sheet。
 * @param {number} sheetId   源 Sheet ID
 * @param {string} column    列名
 * @param {number} sessionId 当前会话 ID（用于派生新 Sheet 的 sessionId）
 * @returns {Promise<{newSheetId: number, headers: string[], rowCount: number, skipped: number}>}
 */
export function parseColumnAsJson(sheetId, column, sessionId) {
  return invoke("parse_column_as_json", { sheetId, column, sessionId });
}

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
 * 后端 get_sheet_data 的 SQL `LIMIT page_size OFFSET offset` 接受任意 page_size，
 * 传 page_size = sheet.total（含表头行）即可一次取回所有数据行（后端 data.rs 跳过
 * row_idx=0 表头行）。无需新增 IPC 或改 DB schema。
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
