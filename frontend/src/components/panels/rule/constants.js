// v1.2.1：RulesPanel 共享常量。从 RulesPanel.jsx 拆出，供 RuleDetail 及
// params 子组件引用。

// 内置提取规则的出厂正则（与 crates/core BuiltinRules 保持一致）。
// 重置按钮恢复出厂值时使用，确保"重置"= 回到初始状态而非 DB 当前值。
export const BUILTIN_PATTERNS = {
  "phone-extract": "\\b[1-9]\\d{10}\\b",
  "idcard-extract": "\\b[1-9]\\d{16}[\\dXx]\\b",
  "bankcard-extract": "\\b[1-9]\\d{12,18}\\b",
  "ip4-extract": "\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b",
  "ip6-extract": "(?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}",
  "name-extract": "[\\u4e00-\\u9fff]{2,4}",
  "name-validate": "^[\\u4e00-\\u9fa5]{2,4}$",
};

// validate 规则校验类型只读 Tag 文案映射。
export const VALIDATE_LABELS = {
  generic: "通用校验",
  phonePrefix: "手机前缀",
  luhn: "Luhn",
  ipv4: "IPv4",
  ipv6: "IPv6",
  idcard: "身份证",
  username: "用户名",
  sex: "性别",
  birth: "出生日期",
  address: "地址",
};

export const KIND_LABEL = { mask: "脱敏", validate: "校验", extract: "提取" };
export const KIND_COLOR = { mask: "orange", validate: "red", extract: "blue" };
export const KIND_ORDER = ["mask", "validate", "extract"];

// v1.2.2：出生日期格式常量（与后端 crates/core datetime.rs 支持的 4 种格式对齐）。
// RulesPanel BirthParams + ValidatePanel RuleRowParams 共用。
// 全不选 = 接受所有格式（向后兼容）。
export const BIRTH_FORMATS = [
  { label: "yyyymmdd", value: "yyyymmdd" },
  { label: "yyyy-mm-dd", value: "yyyy-mm-dd" },
  { label: "yyyy/mm/dd", value: "yyyy/mm/dd" },
  { label: "yyyy.mm.dd", value: "yyyy.mm.dd" },
];
