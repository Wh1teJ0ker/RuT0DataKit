// v0.1.0 重构：只保留 3 个通用校验算子（regex / algorithm /
// regex_with_guard），删除所有预置别名（idcard / phone / bankcard / email /
// mac / username / name）。每个算子带 description / paramDocs /
// paramTypes（typed 输入：number/switch/select/regex/text/textarea）/
// exampleInput / exampleOutput，供 RuleDrawer 渲染 typed 表单 +
// ValidateView 映射表展示。
export const VALIDATOR_DEFS = [
  {
    name: "regex",
    params: ["pattern", "message", "empty_message"],
    description:
      "正则校验：单元格必须完整匹配 pattern（Rust regex 语法）。message 为校验失败提示，empty_message 为空值时的提示（空值默认视为合法）。",
    paramDocs: {
      pattern: "正则表达式（Rust regex 语法，必须匹配全文）",
      message: "校验失败提示文案（可空，默认「格式不正确」）",
      empty_message: "空值时的提示（可空，默认视为合法不报错）",
    },
    paramTypes: {
      pattern: { type: "regex", default: "^\\d{11}$" },
      message: { type: "text", default: "格式不正确", optional: true },
      empty_message: { type: "text", default: "", optional: true },
    },
    exampleInput: "13812345678",
    exampleOutput: "合法",
  },
  {
    name: "algorithm",
    params: ["algo", "message"],
    description:
      "算法校验：据 algo 取值调用对应算法——idcard 校验中国大陆 18 位身份证校验位，bankcard 校验银行卡 Luhn。",
    paramDocs: {
      algo: "算法名：idcard（身份证校验位）/ bankcard（银行卡 Luhn）",
      message: "校验失败提示文案（可空）",
    },
    paramTypes: {
      algo: { type: "select", default: "idcard", options: ["idcard", "bankcard"] },
      message: { type: "text", default: "格式或校验位不正确", optional: true },
    },
    exampleInput: "110101199003071234",
    exampleOutput: "合法",
  },
  {
    name: "regex_with_guard",
    params: ["guard", "prefix_set", "prefix", "pattern", "message"],
    description:
      "守卫+正则校验：先按 guard / prefix_set / prefix 判定是否需要校验（phone=号段白名单、mac=前缀匹配），命中后再用 pattern 正则校验；不命中守卫则直接通过。",
    paramDocs: {
      guard: "守卫模式：phone=号段白名单 / mac=前缀匹配（必填）",
      prefix_set: "phone 守卫下的允许前缀集合（逗号分隔，如 13,14,15,17,18,19）",
      prefix: "mac 守卫下的单前缀匹配（如 00:1A）",
      pattern: "正则表达式（Rust regex 语法，匹配全文）",
      message: "校验失败提示文案（可空）",
    },
    paramTypes: {
      guard: { type: "select", default: "phone", options: ["phone", "mac"] },
      prefix_set: { type: "text", default: "13,14,15,16,17,18,19", optional: true },
      prefix: { type: "text", default: "", optional: true },
      pattern: { type: "regex", default: "^1\\d{10}$" },
      message: { type: "text", default: "格式或号段不正确", optional: true },
    },
    exampleInput: "13812345678",
    exampleOutput: "合法",
  },
];

export const VALIDATOR_NAMES = VALIDATOR_DEFS.map((v) => v.name);

export const getValidatorDef = (name) =>
  VALIDATOR_DEFS.find((v) => v.name === name) || { name, params: [], paramTypes: {} };
