// v0.1.0 重构：只保留 4 个通用脱敏算子（template / split_template /
// regex_replace / const_replace），删除所有预置别名（idcard_mask / phone_mask /
// bankcard_mask / email_mask / name_mask / customer_id_mask / custom /
// regex_extract / delete / replace）。每个算子带 description / paramDocs /
// paramTypes（typed 输入：number/switch/select/regex/text/textarea）/exampleInput /
// exampleOutput，供 RuleDrawer 渲染 typed 表单 + MaskView 映射表展示。
export const MASKER_DEFS = [
  {
    name: "template",
    params: ["keep_prefix", "keep_suffix", "mask_char", "mask_min_len", "min_len", "max_len", "cjk"],
    description:
      "通用模板脱敏：保留首尾字符，中间用 mask_char 替换；mask_min_len 控制中间段最小脱敏长度，min_len/max_len 限定整体长度 guard，cjk 启用中文字符宽度处理。",
    paramDocs: {
      keep_prefix: "保留前缀字符数（整数）",
      keep_suffix: "保留后缀字符数（整数）",
      mask_char: "替换字符，默认 *",
      mask_min_len: "中间段最小脱敏长度（整数）",
      min_len: "整体最小长度 guard（整数，可空）",
      max_len: "整体最大长度 guard（整数，可空）",
      cjk: "是否启用 CJK 字符宽度处理（true/false，可空）",
    },
    paramTypes: {
      keep_prefix: { type: "number", default: 0, min: 0 },
      keep_suffix: { type: "number", default: 0, min: 0 },
      mask_char: { type: "text", default: "*", maxLength: 1 },
      mask_min_len: { type: "number", default: 1, min: 1 },
      min_len: { type: "number", default: null, min: 0, optional: true },
      max_len: { type: "number", default: null, min: 0, optional: true },
      cjk: { type: "switch", default: false },
    },
    exampleInput: "13812345678",
    exampleOutput: "138****5678",
  },
  {
    name: "split_template",
    params: ["separator", "segment_index", "keep_prefix", "keep_suffix", "mask_char", "mask_min_len", "min_len", "max_len", "cjk"],
    description:
      "切分模板脱敏：先按 separator 切分，对 segment_index 指定段套用 keep_prefix/keep_suffix 模板脱敏；常用于邮箱、域名等结构化字段。",
    paramDocs: {
      separator: "切分分隔符字符串（如 @ 或 .）",
      segment_index: "需脱敏的段下标（整数，从 0 起）",
      keep_prefix: "该段保留前缀字符数（整数）",
      keep_suffix: "该段保留后缀字符数（整数）",
      mask_char: "替换字符，默认 *",
      mask_min_len: "中间段最小脱敏长度（整数）",
      min_len: "该段整体最小长度 guard（整数，可空）",
      max_len: "该段整体最大长度 guard（整数，可空）",
      cjk: "是否启用 CJK 字符宽度处理（true/false，可空）",
    },
    paramTypes: {
      separator: { type: "text", default: "@" },
      segment_index: { type: "number", default: 0, min: 0 },
      keep_prefix: { type: "number", default: 1, min: 0 },
      keep_suffix: { type: "number", default: 1, min: 0 },
      mask_char: { type: "text", default: "*", maxLength: 1 },
      mask_min_len: { type: "number", default: 1, min: 1 },
      min_len: { type: "number", default: null, min: 0, optional: true },
      max_len: { type: "number", default: null, min: 0, optional: true },
      cjk: { type: "switch", default: false },
    },
    exampleInput: "alice@example.com",
    exampleOutput: "a***@example.com",
  },
  {
    name: "regex_replace",
    params: ["pattern", "replacement", "match_mode"],
    description:
      "正则替换脱敏：用正则 pattern 匹配内容，按 replacement 替换（支持 $0/$1/$2 捕获组）；match_mode=first 仅替换首个，all 替换全部。",
    paramDocs: {
      pattern: "正则表达式（Rust regex 语法）",
      replacement: "替换串，支持 $0/$1/$2 捕获组；留空表示删除匹配",
      match_mode: "替换模式：first=仅替换首处 / all=替换全部",
    },
    paramTypes: {
      pattern: { type: "regex", default: "\\d{11}" },
      replacement: { type: "text", default: "$0" },
      match_mode: { type: "select", default: "all", options: ["all", "first"] },
    },
    exampleInput: "13812345678",
    exampleOutput: "138****5678",
  },
  {
    name: "const_replace",
    params: ["with"],
    description:
      "常量替换：整列替换为固定字符串 with；with 留空等价于 delete（清空单元格）。",
    paramDocs: {
      with: "替换为的固定字符串（留空等价 delete）",
    },
    paramTypes: {
      with: { type: "text", default: "" },
    },
    exampleInput: "原值",
    exampleOutput: "REDACTED",
  },
];

export const MASKER_NAMES = MASKER_DEFS.map((m) => m.name);

export const getMaskerDef = (name) =>
  MASKER_DEFS.find((m) => m.name === name) || { name, params: [], paramTypes: {} };
