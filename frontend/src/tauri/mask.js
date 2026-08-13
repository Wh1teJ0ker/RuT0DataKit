import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `mask_column` IPC：对指定列就地脱敏。
 * v1.1.3 T49：新增 `template` 参数（临时覆盖规则的模板，不写回 DB）。
 *   前端选预设 → 填充 6 个可编辑参数框 → 透传给本参数执行脱敏。
 *   `null` → 用规则自身的 template。空模板（所有字段 null）→ 不脱敏（透传）。
 * v1.1.5 T85：新增 `validateRuleId` / `invalidText` / `paramsOverride` / `phonePrefixes`
 *   四个可选参数，支持「先校验再脱敏」。勾选后先按指定 validate 规则校验该列，
 *   校验通过的行按 mask 规则脱敏；未通过的行写入 `invalidText` 占位（默认 "INVALID"）。
 *   - `validateRuleId`：validate-kind 规则 id（如 "phone-validate" / "idcard-validate"
 *     / "birth-validate" / "generic-validate"），null 表示不校验，直接脱敏。
 *   - `invalidText`：校验未通过时写入单元格的占位文本，null 表示用后端默认 "INVALID"。
 *   - `paramsOverride`：覆盖 validate 规则的默认 params（如 generic-validate 字符类 /
 *     长度限制、birth-validate formats），null 表示沿用 DB 规则默认。
 *   - `phonePrefixes`：手机号三位前缀白名单（仅 phone-validate 生效），null/空 表示
 *     不限制前缀（仅检查 1 开头 + 11 位）。
 * @param {number} sheetId      Sheet ID
 * @param {string} column       列名（headers 中的值）
 * @param {string|null} [ruleId] 规则 ID（可选；指向 DB mask 规则）
 * @param {string|null} [replacement] 掩码字符（取首个字符；空/null → 默认 `*`；不写回 DB）
 * @param {object|null} [template] 通用模板参数（camelCase：keepPrefix/keepSuffix/
 *   maskChar/maskMinLen/minLen/maxLen，全 null = 不脱敏；不写回 DB）
 * @param {string|null} [validateRuleId] T85：先校验再脱敏的 validate 规则 id；null 不校验
 * @param {string|null} [invalidText] T85：校验未通过行的占位文本；null 用默认 "INVALID"
 * @param {object|null} [paramsOverride] T85：覆盖 validate 规则默认 params；null 沿用 DB
 * @param {string[]|null} [phonePrefixes] T85：phone-validate 前缀白名单；null 不限制
 * @returns {Promise<{affected: number}>} 受影响行数
 */
export function maskColumn(
  sheetId,
  column,
  ruleId,
  replacement,
  template,
  validateRuleId,
  invalidText,
  paramsOverride,
  phonePrefixes,
) {
  return invoke("mask_column", {
    sheetId,
    column,
    ruleId,
    replacement,
    template,
    validateRuleId: validateRuleId ?? null,
    invalidText: invalidText ?? null,
    paramsOverride: paramsOverride ?? null,
    phonePrefixes: phonePrefixes ?? null,
  });
}
