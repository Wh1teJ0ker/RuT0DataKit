import { invoke } from "@tauri-apps/api/core";

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
 * @param {string[]} [phonePrefixes]  T78：可选手机号前缀白名单（如 ["134","159"]），
 *   仅当选中的规则含 phone-extract 时使用。非空时覆盖 DB 规则 params.allowed_prefixes，
 *   仅前 3 位在列表内的候选才算有效；空数组则沿用 DB 规则默认。非 phone 规则不受影响。
 * @returns {Promise<{newSheetId: number, headers: string[], rowCount: number, skipped: number}>}
 */
export function extractValidateToNewSheet(
  sheetId,
  column,
  ruleIds,
  sessionId,
  genderCol = null,
  phonePrefixes = [],
) {
  return invoke("extract_validate_to_new_sheet", {
    sheetId,
    column,
    ruleIds,
    sessionId,
    genderCol,
    phonePrefixes,
  });
}
