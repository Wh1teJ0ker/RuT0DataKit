// 共享工具：提取/脱敏/校验面板共用的纯函数。

/**
 * 归一化手机号前缀白名单：trim + 只保留恰好三位数字的条目。
 *
 * MaskPanel / ValidatePanel / ExtractPanel 三处共用，避免重复 `/^\d{3}$/` 过滤逻辑。
 *
 * @param {Array<string|number>} input  原始前缀数组（可能含非数字 / 长度不对的条目）
 * @returns {string[]}  形如 ["134", "178"] 的纯数字前缀数组
 */
export function normalizePhonePrefixes(input) {
  if (!Array.isArray(input)) return [];
  return input
    .map((p) => String(p).trim())
    .filter((p) => /^\d{3}$/.test(p));
}
