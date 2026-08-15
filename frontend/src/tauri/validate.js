import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `validate_multi_rules_to_two_sheets` IPC：多规则列式校验 → 通过 / 失败
 * 两行分流到两个新 Tab（保留原列，不新增列）。v1.1.4 T69 新增。
 *
 * 允许用户自由组合多条「目标列 + 校验规则」记录，每条记录可独立选择列与
 * validate 规则。身份证规则（idcard-validate）行可通过 crossField 勾选
 * 「对比性别一致性」与「对比出生日期一致性」，并指定对应列。
 *
 * @param {number} sheetId    源 Sheet ID
 * @param {number} sessionId  当前会话 ID（两个新 Tab 挂到本会话）
 * @param {Array<{column: string, ruleId: string, crossField?: {checkSex: boolean, sexColumn: string|null, checkBirth: boolean, birthColumn: string|null} | null, paramsOverride?: object | null}>} rules
 *   多条「列 + 校验规则」组合；crossField 仅在 ruleId 为 idcard-validate 时生效；
 *   paramsOverride 仅在 ruleId 为 generic-validate 时携带，覆盖 DB 默认 params
 * @param {string[]} [phonePrefixes=[]]  手机号三位前缀白名单；空数组 = 不过滤前缀
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
