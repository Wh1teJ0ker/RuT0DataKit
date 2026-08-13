// v1.1.3 脱敏模板参数共享模块。
// 抽出自 MaskPanel.jsx（T49）+ RulesPanel.jsx（T51），避免两处重复定义。
//
// 对齐后端 crates/core/src/processor/masker.rs：
//   - TemplateParams 是 untagged enum：
//     - Simple（7 个 Option 字段，旧 flat 结构向后兼容；T53 新增 reverse）
//     - Segment（T52：delimiter + segments[]，按分隔符拆分后对指定段脱敏）
//   - apply_template 逻辑（keep_prefix/keep_suffix/mask_char/mask_min_len + min_len/max_len guard
//     + T53 reverse 反向：掩码首尾、保留中间）
//   - apply_segment_template 逻辑（split → 对每段调 apply_segment_part → join）
//   - 空模板（Simple 全 None / Segment delimiter 空或 segments 空）= 不脱敏（透传）
// T54：原 general-mask 拆为 simple-mask（整段脱敏，持 Simple 模板）+
//   segment-mask（分段脱敏，持 Segment 模板）。本模块按 shape 分流，不看 rule.id。

/// 默认掩码字符。
export const DEFAULT_MASK_CHAR = "*";

/// 模板类型标识。
export const TEMPLATE_TYPE_SIMPLE = "simple";
export const TEMPLATE_TYPE_SEGMENT = "segment";

/// 整段脱敏预设（simple-mask 的预设）。选预设 → 填充 7 个可编辑参数框。
// 参数对齐后端 TemplateParams::Simple（camelCase）。
// 仅 Simple 模板内置预设；Segment 模板不内置预设（用户自行配置分段）。
export const MASK_PRESETS = [
  {
    key: "idcard",
    label: "身份证号",
    params: {
      keepPrefix: 6,
      keepSuffix: 4,
      maskChar: "*",
      maskMinLen: 8,
      minLen: null,
      maxLen: null,
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

/// 空模板（Simple 全 None = 透传，不脱敏）。
/// T53：reverse=null（默认正向）。reverse=true 单独存在（其他全 null）= kp=ks=0
/// → 不脱码任何位 = 透传，故 is_empty 语义不变。
export const EMPTY_TEMPLATE = {
  keepPrefix: null,
  keepSuffix: null,
  maskChar: null,
  maskMinLen: null,
  minLen: null,
  maxLen: null,
  reverse: null,
};

/// 空分段模板（Segment 空配置 = 透传）。
export const EMPTY_SEGMENT_TEMPLATE = {
  maskChar: null,
  delimiter: "",
  segments: [],
};

/// 规整单个 SegmentMask 配置（补全缺失字段为 null）。
function normalizeSegment(seg) {
  if (!seg) return null;
  return {
    index: seg.index ?? 0,
    keepPrefix: seg.keepPrefix ?? null,
    keepSuffix: seg.keepSuffix ?? null,
    maskMinLen: seg.maskMinLen ?? null,
  };
}

/// 从规则 template 对象初始化参数框（补全缺失字段为 null）。
/// 兼容后端 untagged enum：含 delimiter 字段 → Segment，否则 → Simple。
export function normalizeTemplate(tpl) {
  if (!tpl) return { ...EMPTY_TEMPLATE };
  // Segment 变体：后端存 {maskChar, delimiter, segments}（无 type 字段，untagged）
  if (typeof tpl.delimiter === "string" || Array.isArray(tpl.segments)) {
    return {
      maskChar: tpl.maskChar ?? null,
      delimiter: tpl.delimiter ?? "",
      segments: Array.isArray(tpl.segments)
        ? tpl.segments.map(normalizeSegment).filter(Boolean)
        : [],
    };
  }
  // Simple 变体（旧 flat 6 字段 + T53 reverse）
  return {
    keepPrefix: tpl.keepPrefix ?? null,
    keepSuffix: tpl.keepSuffix ?? null,
    maskChar: tpl.maskChar ?? null,
    maskMinLen: tpl.maskMinLen ?? null,
    minLen: tpl.minLen ?? null,
    maxLen: tpl.maxLen ?? null,
    reverse: tpl.reverse === true ? true : null,
  };
}

/// 判断模板是否为 Segment 类型（用于 UI 切换 + build/preview 分支）。
export function isSegmentTemplate(tpl) {
  return !!tpl && typeof tpl.delimiter === "string";
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
/// Segment 模板 → 永远 "custom"（不内置分段预设）。
export function detectPreset(t) {
  if (isSegmentTemplate(t)) return "custom";
  const current = JSON.stringify(stripNulls(t));
  for (const p of MASK_PRESETS) {
    if (!p.params) continue; // custom 跳过
    if (JSON.stringify(stripNulls(p.params)) === current) return p.key;
  }
  if (Object.values(t).every((v) => v == null || v === "")) return "empty";
  return "custom";
}

/// 按预设 key 填充 6 参数框。custom 不填充（保留当前用户输入）。
/// 预设都是 Simple 模板，切换到预设时自动用 Simple 空模板初始化。
export function templateFromPreset(key) {
  const preset = MASK_PRESETS.find((p) => p.key === key);
  if (!preset) return { ...EMPTY_TEMPLATE };
  if (preset.params === null) return null; // custom → 调用方保留当前
  if (Object.keys(preset.params).length === 0) return { ...EMPTY_TEMPLATE };
  return normalizeTemplate(preset.params);
}

/// 组装 Simple 模板（null 字段表示不设）。全空 → null（透传，用规则自身空模板）。
/// T53：reverse=true 时输出 reverse:true（false/null 不输出，省字段 + 保持旧 JSON 形态）。
function buildSimpleForRun(template) {
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
  if (template.reverse === true) {
    tpl.reverse = true;
    hasAny = true;
  }
  return hasAny ? tpl : null;
}

/// 组装 Segment 模板。delimiter 空 或 segments 空 → null（透传）。
function buildSegmentForRun(template) {
  const delimiter = template.delimiter ?? "";
  const segments = Array.isArray(template.segments) ? template.segments : [];
  if (!delimiter || segments.length === 0) return null;
  const tpl = {
    delimiter,
    segments: segments.map((s) => ({
      index: Number(s.index ?? 0),
      keepPrefix:
        s.keepPrefix != null && s.keepPrefix !== ""
          ? Number(s.keepPrefix)
          : null,
      keepSuffix:
        s.keepSuffix != null && s.keepSuffix !== ""
          ? Number(s.keepSuffix)
          : null,
      maskMinLen:
        s.maskMinLen != null && s.maskMinLen !== ""
          ? Number(s.maskMinLen)
          : null,
    })),
  };
  if (template.maskChar && template.maskChar.length > 0) {
    tpl.maskChar = template.maskChar[0];
  }
  return tpl;
}

/// 组装 template（null 字段表示不设）。全空 → null（透传，用规则自身空模板）。
/// 用于 mask_column 临时覆盖 / updateRuleTemplate 持久化。
/// 自动按模板类型走 Simple / Segment 分支。
export function buildTemplateForRun(template) {
  if (!template) return null;
  if (isSegmentTemplate(template)) return buildSegmentForRun(template);
  return buildSimpleForRun(template);
}

/// 前端单段脱敏预览（移植自后端 apply_segment_part，纯逻辑不写 DB）。
/// 保留前 kp + 后 ks 字符，中间替换为 maskChar（至少 mml 个）。无 min/max guard。
function previewSegmentPart(part, cfg, maskChar) {
  const chars = [...part];
  const n = chars.length;
  const kp = cfg.keepPrefix ?? 0;
  const ks = cfg.keepSuffix ?? 0;
  const mml = cfg.maskMinLen ?? 1;
  if (n === 0) return maskChar.repeat(mml);
  const headEnd = Math.min(kp, n);
  const tailStart = Math.max(0, n - ks);
  if (tailStart <= headEnd) return maskChar.repeat(mml);
  const midLen = tailStart - headEnd;
  const maskLen = Math.max(midLen, mml);
  const head = chars.slice(0, headEnd).join("");
  const tail = chars.slice(tailStart).join("");
  return `${head}${maskChar.repeat(maskLen)}${tail}`;
}

/// 前端分段脱敏预览（移植自后端 apply_segment_template）。
function previewSegmentMask(template, input, fallbackMaskChar) {
  const value = input ?? "";
  const built = buildSegmentForRun(template);
  if (!built) return { output: value, skipped: false, passthrough: true };
  const maskChar =
    (built.maskChar && built.maskChar.length > 0 ? built.maskChar[0] : null) ||
    fallbackMaskChar ||
    DEFAULT_MASK_CHAR;
  const parts = value.split(built.delimiter);
  const out = parts.map((part, i) => {
    const cfg = built.segments.find((c) => Number(c.index) === i);
    return cfg ? previewSegmentPart(part, cfg, maskChar) : part;
  });
  return { output: out.join(built.delimiter), skipped: false };
}

/// 前端 Simple 模板脱敏预览（移植自后端 apply_template）。
/// T53：reverse=true → 反向（掩码首尾、保留中间）；reverse=null/false → 正向。
function previewSimpleMask(template, input, fallbackMaskChar) {
  const value = input ?? "";
  const chars = [...value];
  const n = chars.length;

  const built = buildSimpleForRun(template);
  if (!built) return { output: value, skipped: false, passthrough: true };

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

  // T53：反向脱敏——掩码首尾，保留中间。
  if (built.reverse === true) {
    if (n === 0) return { output: "", skipped: false };
    const headMaskEnd = Math.min(kp, n);
    const tailMaskStart = Math.max(0, n - ks);
    if (tailMaskStart <= headMaskEnd) {
      // kp+ks >= n → 整段脱敏，至少 mml 个
      return { output: maskChar.repeat(Math.max(n, mml)), skipped: false };
    }
    const headMask = maskChar.repeat(headMaskEnd);
    const middle = chars.slice(headMaskEnd, tailMaskStart).join("");
    const tailMask = maskChar.repeat(n - tailMaskStart);
    return { output: `${headMask}${middle}${tailMask}`, skipped: false };
  }

  // 正向：保留首尾，掩码中间。
  const headEnd = Math.min(kp, n);
  const tailStart = Math.max(0, n - ks);
  if (tailStart <= headEnd) {
    return { output: maskChar.repeat(mml), skipped: false };
  }
  const midLen = tailStart - headEnd;
  const maskLen = Math.max(midLen, mml);
  const head = chars.slice(0, headEnd).join("");
  const tail = chars.slice(tailStart).join("");
  return { output: `${head}${maskChar.repeat(maskLen)}${tail}`, skipped: false };
}

/// 前端模板脱敏预览（移植自后端 apply_template / apply_segment_template）。
/// 空模板（全 null / segment 空）→ 原样返回（透传）。
///
/// 返回 { output, skipped }：skipped=true 表示 guard 命中（长度不在区间内），
/// output 为原值。
export function previewMask(template, input, fallbackMaskChar) {
  if (isSegmentTemplate(template)) {
    return previewSegmentMask(template, input, fallbackMaskChar);
  }
  return previewSimpleMask(template, input, fallbackMaskChar);
}

/// 解析规则的默认掩码字符：template.maskChar > replacement[0] > 默认 *。
/// 兼容 Simple / Segment 两种模板（都有 maskChar 字段）。
export function resolveMaskChar(rule) {
  if (rule?.template?.maskChar) return rule.template.maskChar;
  if (rule?.replacement && rule.replacement.length > 0)
    return rule.replacement[0];
  return DEFAULT_MASK_CHAR;
}
