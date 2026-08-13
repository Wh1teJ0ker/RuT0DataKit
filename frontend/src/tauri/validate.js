import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `validate_column` IPC：按规则校验指定列。
 * @param {number} sheetId  Sheet ID
 * @param {string} column   列名
 * @param {string} ruleId   规则 ID（RuleKind=Validate）
 * @returns {Promise<Array<{rowIdx: number, passed: boolean, message: string}>>}
 *   RowValidation[]（直接返回数组，不是 { results: [...] }）。
 *   rowIdx 是 DB 绝对行号（row_idx=0 表头行，数据行从 1 开始）。
 */
export function validateColumn(sheetId, column, ruleId) {
  return invoke("validate_column", { sheetId, column, ruleId });
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

/**
 * 调用 `validate_multi_rules_to_two_sheets` IPC：多规则列式校验 → 通过 / 失败
 * 两行分流到两个新 Tab（保留原列，不新增列）。v1.1.4 T69 新增。
 *
 * 与 `validateRowsToTwoSheets`（固定 7 字段映射）不同，本命令允许用户自由组合
 * 多条「目标列 + 校验规则」记录，每条记录可独立选择列与 validate 规则。
 * 身份证规则（idcard-validate）行可通过 crossField 勾选「对比性别一致性」
 * 与「对比出生日期一致性」，并指定对应列。
 *
 * @param {number} sheetId    源 Sheet ID
 * @param {number} sessionId  当前会话 ID（两个新 Tab 挂到本会话）
 * @param {Array<{column: string, ruleId: string, crossField?: {checkSex: boolean, sexColumn: string|null, checkBirth: boolean, birthColumn: string|null} | null, paramsOverride?: object | null}>} rules
 *   多条「列 + 校验规则」组合；crossField 仅在 ruleId 为 idcard-validate 时生效；
 *   paramsOverride 仅在 ruleId 为 generic-validate 时携带，覆盖 DB 默认 params
 * @param {string[]} [phonePrefixes=[]]  手机号三位前缀白名单；空数组 = 仅检查 1 开头 + 11 位
 * @returns {Promise<{validSheet: {newSheetId: number, headers: string[], rowCount: number, skipped: number}, invalidSheet: {newSheetId: number, headers: string[], rowCount: number, skipped: number}, invalidReasons: Array<{sourceRow: number, field: string, reason: string}>}>}
 */
export function validateMultiRulesToTwoSheets(
  sheetId,
  sessionId,
  rules,
  phonePrefixes = [],
) {
  return invoke("validate_multi_rules_to_two_sheets", {
    sheetId,
    sessionId,
    rules,
    phonePrefixes,
  });
}
