import { invoke } from "@tauri-apps/api/core";

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
