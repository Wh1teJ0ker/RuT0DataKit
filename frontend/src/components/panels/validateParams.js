// v1.1.4 续轮 T71：校验规则参数共享模块。
// 提供 Generic 参数默认值/构造/校验提示文案，供 RulesPanel + ValidatePanel 共用。
// T77：allowSpecial: bool → allowSpecialChars: string（自定义特殊字符白名单）。

// Generic 校验参数空对象（与后端 ExtractParams::Generic 对齐，camelCase）。
export const EMPTY_GENERIC_PARAMS = {
  allowDigits: true,
  allowLetters: true,
  allowSpecialChars: "",
  minLen: null,
  maxLen: null,
};

// 防御性默认值填充（后端返回的 params 可能缺字段）。
export function normalizeGenericParams(p) {
  if (!p || typeof p !== "object") return { ...EMPTY_GENERIC_PARAMS };
  return {
    allowDigits: p.allowDigits ?? true,
    allowLetters: p.allowLetters ?? true,
    allowSpecialChars:
      typeof p.allowSpecialChars === "string" ? p.allowSpecialChars : "",
    minLen: p.minLen ?? null,
    maxLen: p.maxLen ?? null,
  };
}

// 组装发送给后端的 ExtractParams::Generic 对象。
export function buildGenericParamsForRun(p) {
  return {
    validator: "generic",
    allowDigits: !!p.allowDigits,
    allowLetters: !!p.allowLetters,
    allowSpecialChars:
      typeof p.allowSpecialChars === "string" ? p.allowSpecialChars : "",
    minLen: p.minLen ?? null,
    maxLen: p.maxLen ?? null,
  };
}

// 各 validate 规则的提示文案（RulesPanel 选中时显示）。
export const VALIDATE_HINTS = {
  "generic-validate":
    "通用校验：勾选允许的字符类 + 填写允许的特殊符号白名单 + 设置长度限制",
  "username-validate": "用户名须为纯字母数字（a-zA-Z0-9）",
  "sex-validate": "性别须为「男」或「女」",
  "birth-validate": "出生日期：接受 20031223 / 2003-12-23 等格式，自动清理分隔符",
  "idcard-validate": "18 位身份证号（GB 11643-1999 校验码）；可勾选跨字段比对性别/出生日期",
  "phone-validate": "11 位手机号（1 开头）；可设置前缀白名单",
  "address-validate": "地址结构化校验：中文 + 地址关键词（省/市/区/路/号/室等）",
  "name-validate": "姓名校验：2-4 位中文字符（正则规则）",
};
