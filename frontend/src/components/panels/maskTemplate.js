// v1.1.3 T51 通用脱敏模板参数共享模块。
// 抽出自 MaskPanel.jsx（T49）+ RulesPanel.jsx（T51），避免两处重复定义。
//
// 对齐后端 crates/core/src/processor/masker.rs：
//   - TemplateParams（camelCase，6 个 Option 字段）
//   - apply_template 逻辑（keep_prefix/keep_suffix/mask_char/mask_min_len + min_len/max_len guard）
//   - 空模板（全 None）= 不脱敏（透传）

/// 默认掩码字符。
export const DEFAULT_MASK_CHAR = "*";

/// 通用脱敏预设（general-mask 的子规则）。选预设 → 填充 6 个可编辑参数框。
// 参数对齐后端 TemplateParams（camelCase）。
export const MASK_PRESETS = [
  {
    key: "idcard",
    label: "身份证号",
    params: {
      keepPrefix: 6,
      keepSuffix: 4,
      maskChar: "*",
      maskMinLen: 8,
      minLen: 18,
      maxLen: 18,
    },
  },
  {
    key: "phone",
    label: "手机号",
    params: {
      keepPrefix: 3,
      keepSuffix: 4,
      maskChar: "*",
      maskMinLen: 4,
      minLen: 11,
      maxLen: 11,
    },
  },
  {
    key: "birthdate",
    label: "出生日期",
    params: {
      keepPrefix: 8,
      keepSuffix: 0,
      maskChar: "*",
      maskMinLen: 2,
      minLen: 10,
      maxLen: 10,
    },
  },
  {
    key: "bankcard",
    label: "银行卡号",
    params: {
      keepPrefix: 4,
      keepSuffix: 4,
      maskChar: "*",
      maskMinLen: 1,
      minLen: null,
      maxLen: null,
    },
  },
  { key: "custom", label: "自定义", params: null },
  { key: "empty", label: "不脱敏（空模板）", params: {} },
];

/// 空模板（全 None = 透传，不脱敏）。
export const EMPTY_TEMPLATE = {
  keepPrefix: null,
  keepSuffix: null,
  maskChar: null,
  maskMinLen: null,
  minLen: null,
  maxLen: null,
};

/// 从规则 template 对象初始化参数框（补全缺失字段为 null）。
export function normalizeTemplate(tpl) {
  if (!tpl) return { ...EMPTY_TEMPLATE };
  return {
    keepPrefix: tpl.keepPrefix ?? null,
    keepSuffix: tpl.keepSuffix ?? null,
    maskChar: tpl.maskChar ?? null,
    maskMinLen: tpl.maskMinLen ?? null,
    minLen: tpl.minLen ?? null,
    maxLen: tpl.maxLen ?? null,
  };
}

/// 去掉 null/空值字段，用于预设匹配比较。
export function stripNulls(t) {
  const o = {};
  for (const [k, v] of Object.entries(t)) {
    if (v != null && v !== "") o[k] = v;
  }
  return o;
}

/// 检测当前模板参数匹配哪个预设（用于回显子规则下拉）。
export function detectPreset(t) {
  const current = JSON.stringify(stripNulls(t));
  for (const p of MASK_PRESETS) {
    if (!p.params) continue; // custom 跳过
    if (JSON.stringify(stripNulls(p.params)) === current) return p.key;
  }
  if (Object.values(t).every((v) => v == null || v === "")) return "empty";
  return "custom";
}

/// 按预设 key 填充 6 参数框。custom 不填充（保留当前用户输入）。
export function templateFromPreset(key) {
  const preset = MASK_PRESETS.find((p) => p.key === key);
  if (!preset) return { ...EMPTY_TEMPLATE };
  if (preset.params === null) return null; // custom → 调用方保留当前
  if (Object.keys(preset.params).length === 0) return { ...EMPTY_TEMPLATE };
  return normalizeTemplate(preset.params);
}

/// 组装 template（null 字段表示不设）。全空 → null（透传，用规则自身空模板）。
/// 用于 mask_column 临时覆盖 / updateRuleTemplate 持久化。
export function buildTemplateForRun(template) {
  const tpl = {};
  let hasAny = false;
  if (template.keepPrefix != null && template.keepPrefix !== "") {
    tpl.keepPrefix = Number(template.keepPrefix);
    hasAny = true;
  }
  if (template.keepSuffix != null && template.keepSuffix !== "") {
    tpl.keepSuffix = Number(template.keepSuffix);
    hasAny = true;
  }
  if (template.maskChar && template.maskChar.length > 0) {
    tpl.maskChar = template.maskChar[0];
    hasAny = true;
  }
  if (template.maskMinLen != null && template.maskMinLen !== "") {
    tpl.maskMinLen = Number(template.maskMinLen);
    hasAny = true;
  }
  if (template.minLen != null && template.minLen !== "") {
    tpl.minLen = Number(template.minLen);
    hasAny = true;
  }
  if (template.maxLen != null && template.maxLen !== "") {
    tpl.maxLen = Number(template.maxLen);
    hasAny = true;
  }
  return hasAny ? tpl : null;
}

/// 前端模板脱敏预览（移植自后端 apply_template，纯逻辑不写 DB）。
/// 空模板（全 null）→ 原样返回（透传）。
///
/// 返回 { output, skipped }：skipped=true 表示 guard 命中（长度不在区间内），
/// output 为原值。
export function previewMask(template, input, fallbackMaskChar) {
  const value = input ?? "";
  const chars = [...value];
  const n = chars.length;

  // 空模板 → 透传（不脱敏）。
  const built = buildTemplateForRun(template);
  if (!built) {
    return { output: value, skipped: false, passthrough: true };
  }

  const kp = built.keepPrefix ?? 0;
  const ks = built.keepSuffix ?? 0;
  const mml = built.maskMinLen ?? 1;
  const maskChar =
    (built.maskChar && built.maskChar.length > 0
      ? built.maskChar[0]
      : null) || fallbackMaskChar || DEFAULT_MASK_CHAR;

  // guard：长度不在 [minLen, maxLen] 区间原样返回。
  if (built.minLen != null && n < built.minLen) {
    return { output: value, skipped: true };
  }
  if (built.maxLen != null && n > built.maxLen) {
    return { output: value, skipped: true };
  }

  const headEnd = Math.min(kp, n);
  const tailStart = Math.max(0, n - ks);
  // 保留段重叠（含 kp+ks==n 边界：中间段长度为 0）→ 仅输出 maskMinLen 个 maskChar。
  if (tailStart <= headEnd) {
    return { output: maskChar.repeat(mml), skipped: false };
  }
  const midLen = tailStart - headEnd;
  const maskLen = Math.max(midLen, mml);
  const head = chars.slice(0, headEnd).join("");
  const tail = chars.slice(tailStart).join("");
  const mask = maskChar.repeat(maskLen);
  return { output: `${head}${mask}${tail}`, skipped: false };
}

/// 解析规则的默认掩码字符：template.maskChar > replacement[0] > 默认 *。
export function resolveMaskChar(rule) {
  if (rule?.template?.maskChar) return rule.template.maskChar;
  if (rule?.replacement && rule.replacement.length > 0)
    return rule.replacement[0];
  return DEFAULT_MASK_CHAR;
}
