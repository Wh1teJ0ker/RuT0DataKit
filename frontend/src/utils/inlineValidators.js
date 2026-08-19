// v1.2.0：前端内联校验器 — 镜像后端 validators/ 的程序化校验逻辑。
// 用于 RulesPanel 内联测试，不写 DB，纯前端预览。
// 每个函数返回 { passed: boolean, message: string, note?: string }。

// 身份证校验码权重（GB 11643-1999）。
const IDCARD_WEIGHTS = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const IDCARD_CHECK_CODES = ["1", "0", "X", "9", "8", "7", "6", "5", "4", "3", "2"];

// 地址关键词（与后端 ADDR_KEYWORDS 对齐）。
const ADDR_KEYWORDS = [
  "省", "市", "区", "县", "镇", "乡", "村", "路", "街", "道", "号", "室",
  "楼", "单元", "栋", "幢", "弄", "巷", "里", "组", "旗", "盟", "社区", "大厦",
  "小区", "花园",
];

/**
 * Luhn 校验：银行卡类。
 * 长度 >= 2，全数字，标准 Luhn 算法。
 */
function validateLuhn(s) {
  const trimmed = (s || "").trim();
  if (trimmed.length < 2) {
    return { passed: false, message: "不通过：长度不足（需至少 2 位数字）" };
  }
  if (!/^\d+$/.test(trimmed)) {
    return { passed: false, message: "不通过：包含非数字字符" };
  }
  let sum = 0;
  const digits = [...trimmed].map(Number);
  for (let i = digits.length - 1, pos = 1; i >= 0; i--, pos++) {
    let d = digits[i];
    if (pos % 2 === 0) {
      d *= 2;
      if (d > 9) d = Math.floor(d / 10) + (d % 10);
    }
    sum += d;
  }
  const passed = sum % 10 === 0;
  return {
    passed,
    message: passed ? "通过：Luhn 校验成功" : "不通过：Luhn 校验码不匹配",
  };
}

/**
 * IPv4 校验：4 段，每段 0-255，无前导零（单独 "0" 允许）。
 */
function validateIpv4(s) {
  const trimmed = (s || "").trim();
  const parts = trimmed.split(".");
  if (parts.length !== 4) {
    return { passed: false, message: "不通过：需 4 段以 . 分隔" };
  }
  for (const part of parts) {
    if (part.length === 0 || !/^\d+$/.test(part)) {
      return { passed: false, message: "不通过：包含非数字段" };
    }
    if (part.length > 1 && part.startsWith("0")) {
      return { passed: false, message: "不通过：前导零" };
    }
    const num = Number(part);
    if (num < 0 || num > 255) {
      return { passed: false, message: "不通过：段值超出 0-255" };
    }
  }
  return { passed: true, message: "通过：合法 IPv4 地址" };
}

/**
 * IPv6 校验：RFC 4291 简化检查。
 * 支持 :: 缩写、完整形式、IPv4 映射形式。
 */
function validateIpv6(s) {
  const trimmed = (s || "").trim();
  if (trimmed.length === 0) {
    return { passed: false, message: "不通过：空输入" };
  }
  // 简化 RFC 4291 校验：8 组十六进制或含 :: 缩写。
  if (/^[0-9a-fA-F]{1,4}(:[0-9a-fA-F]{1,4}){7}$/.test(trimmed)) {
    return { passed: true, message: "通过：合法 IPv6 地址" };
  }
  // 含 :: 缩写
  const halves = trimmed.split("::");
  if (halves.length === 2) {
    const left = halves[0] ? halves[0].split(":") : [];
    const right = halves[1] ? halves[1].split(":") : [];
    const allParts = [...left, ...right];
    if (allParts.every((p) => p === "" || /^[0-9a-fA-F]{1,4}$/.test(p))) {
      if (left.length + right.length <= 7) {
        return { passed: true, message: "通过：合法 IPv6 地址" };
      }
    }
  }
  return { passed: false, message: "不通过：非法 IPv6 格式" };
}

/**
 * 身份证校验：18 位，前 17 位数字，末位数字或 X，校验码验证。
 * 附带性别推断（第 17 位奇=男，偶=女）。
 */
function validateIdcard(s) {
  const trimmed = (s || "").trim();
  if (trimmed.length !== 18) {
    return { passed: false, message: "不通过：长度需 18 位" };
  }
  const first17 = trimmed.slice(0, 17);
  if (!/^\d{17}$/.test(first17)) {
    return { passed: false, message: "不通过：前 17 位须为数字" };
  }
  const lastChar = trimmed[17].toUpperCase();
  if (!/^[\dX]$/.test(lastChar)) {
    return { passed: false, message: "不通过：末位须为数字或 X" };
  }
  let sum = 0;
  for (let i = 0; i < 17; i++) {
    sum += Number(first17[i]) * IDCARD_WEIGHTS[i];
  }
  const expected = IDCARD_CHECK_CODES[sum % 11];
  if (lastChar !== expected) {
    return { passed: false, message: "不通过：校验码不匹配" };
  }
  // 性别推断：第 17 位（索引 16）。
  const genderDigit = Number(first17[16]);
  const note = genderDigit % 2 === 1 ? "男" : "女";
  return { passed: true, message: "通过：合法身份证号", note };
}

/**
 * 用户名校验：纯字母数字（a-zA-Z0-9），非空。
 */
function validateUsername(s) {
  const trimmed = (s || "").trim();
  if (trimmed.length === 0) {
    return { passed: false, message: "不通过：不能为空" };
  }
  const passed = /^[a-zA-Z0-9]+$/.test(trimmed);
  return {
    passed,
    message: passed ? "通过：合法用户名" : "不通过：须为纯字母数字",
  };
}

/**
 * 性别校验：仅「男」或「女」。
 */
function validateSex(s) {
  const trimmed = (s || "").trim();
  const passed = trimmed === "男" || trimmed === "女";
  return {
    passed,
    message: passed ? "通过：合法性别值" : "不通过：须为「男」或「女」",
  };
}

/**
 * 出生日期校验：清理非数字后须为 8 位 yyyymmdd，
 * 年 1900-2100，月 1-12，日 1-31。
 */
function validateBirth(s) {
  const trimmed = (s || "").trim();
  const cleaned = trimmed.replace(/\D/g, "");
  if (cleaned.length !== 8) {
    return { passed: false, message: "不通过：清理后须为 8 位数字（yyyymmdd）" };
  }
  const year = Number(cleaned.slice(0, 4));
  const month = Number(cleaned.slice(4, 6));
  const day = Number(cleaned.slice(6, 8));
  if (year < 1900 || year > 2100) {
    return { passed: false, message: "不通过：年份须在 1900-2100" };
  }
  if (month < 1 || month > 12) {
    return { passed: false, message: "不通过：月份须在 1-12" };
  }
  if (day < 1 || day > 31) {
    return { passed: false, message: "不通过：日期须在 1-31" };
  }
  return { passed: true, message: "通过：合法出生日期" };
}

/**
 * 地址校验：4-200 字符，CJK >= 2，含地址关键词，无英文字母，
 * 可选号/室数字范围（minHao/maxHao/minShi/maxShi 为 null = 不限）。
 */
function validateAddress(s, params) {
  const trimmed = (s || "").trim();
  const charCount = [...trimmed].length;
  if (charCount < 4 || charCount > 200) {
    return { passed: false, message: "不通过：长度须在 4-200 字符" };
  }
  // v1.2.2：全中文（不含英文字母）。
  if (/[a-zA-Z]/.test(trimmed)) {
    return { passed: false, message: "不通过：地址含英文字母（须全中文）" };
  }
  let cjkCount = 0;
  for (const ch of trimmed) {
    const code = ch.codePointAt(0);
    if (code >= 0x4e00 && code <= 0x9fa5) cjkCount++;
  }
  if (cjkCount < 2) {
    return { passed: false, message: "不通过：中文字符不足 2 个" };
  }
  const hasKeyword = ADDR_KEYWORDS.some((kw) => trimmed.includes(kw));
  if (!hasKeyword) {
    return { passed: false, message: "不通过：缺少地址关键词（省/市/区/路/号等）" };
  }
  // v1.2.2：可选号/室范围校验。
  const haoMatch = trimmed.match(/(\d+)号/);
  if (haoMatch) {
    const haoNum = Number(haoMatch[1]);
    const minHao = params?.minHao ?? null;
    const maxHao = params?.maxHao ?? null;
    if (minHao != null && haoNum < minHao) {
      return { passed: false, message: `不通过：「号」${haoNum} < 最小 ${minHao}` };
    }
    if (maxHao != null && haoNum > maxHao) {
      return { passed: false, message: `不通过：「号」${haoNum} > 最大 ${maxHao}` };
    }
  }
  const shiMatch = trimmed.match(/(\d+)室/);
  if (shiMatch) {
    const shiNum = Number(shiMatch[1]);
    const minShi = params?.minShi ?? null;
    const maxShi = params?.maxShi ?? null;
    if (minShi != null && shiNum < minShi) {
      return { passed: false, message: `不通过：「室」${shiNum} < 最小 ${minShi}` };
    }
    if (maxShi != null && shiNum > maxShi) {
      return { passed: false, message: `不通过：「室」${shiNum} > 最大 ${maxShi}` };
    }
  }
  return { passed: true, message: "通过：合法地址" };
}

/**
 * 手机号校验：11 位、1 开头、全数字、可选前缀白名单。
 */
function validatePhone(s, allowedPrefixes) {
  const trimmed = (s || "").trim();
  if (trimmed.length !== 11) {
    return { passed: false, message: "不通过：须为 11 位" };
  }
  if (!trimmed.startsWith("1")) {
    return { passed: false, message: "不通过：须以 1 开头" };
  }
  if (!/^\d{11}$/.test(trimmed)) {
    return { passed: false, message: "不通过：包含非数字字符" };
  }
  if (Array.isArray(allowedPrefixes) && allowedPrefixes.length > 0) {
    const matched = allowedPrefixes.some((p) => trimmed.startsWith(p));
    if (!matched) {
      return { passed: false, message: `不通过：前缀不在白名单（${allowedPrefixes.join(", ")}）` };
    }
  }
  return { passed: true, message: "通过：合法手机号" };
}

/**
 * 通用校验：字符类白名单 + 长度限制。
 */
function validateGeneric(s, params) {
  const input = s || "";
  const allowDigits = !!params?.allowDigits;
  const allowLetters = !!params?.allowLetters;
  const allowSpecial = params?.allowSpecialChars ?? "";
  const minLen = params?.minLen ?? null;
  const maxLen = params?.maxLen ?? null;

  if (!allowDigits && !allowLetters && allowSpecial.length === 0) {
    return { passed: false, message: "不通过：未勾选任何允许的字符类" };
  }
  if (input.length === 0) {
    return { passed: false, message: "不通过：输入为空" };
  }
  const count = [...input].length;
  if (minLen != null && count < minLen) {
    return { passed: false, message: `不通过：长度不足（需 >= ${minLen}）` };
  }
  if (maxLen != null && count > maxLen) {
    return { passed: false, message: `不通过：长度超出（需 <= ${maxLen}）` };
  }
  for (const ch of input) {
    const isDigit = /[0-9]/.test(ch);
    const isLetter = /[a-zA-Z]/.test(ch);
    if (isDigit && allowDigits) continue;
    if (isLetter && allowLetters) continue;
    if (allowSpecial.includes(ch)) continue;
    return { passed: false, message: `不通过：字符「${ch}」不在允许范围内` };
  }
  return { passed: true, message: "通过：符合通用校验规则" };
}

/**
 * 邮箱校验：local@domain 格式，local <= 64，总长 <= 254。
 */
function validateEmail(s) {
  const trimmed = (s || "").trim();
  if (trimmed.length === 0 || trimmed.length > 254) {
    return { passed: false, message: "不通过：长度不合法" };
  }
  const atIdx = trimmed.indexOf("@");
  const lastAtIdx = trimmed.lastIndexOf("@");
  if (atIdx === -1 || atIdx !== lastAtIdx) {
    return { passed: false, message: "不通过：须有且仅有一个 @" };
  }
  const local = trimmed.slice(0, atIdx);
  const domain = trimmed.slice(atIdx + 1);
  if (local.length === 0 || local.length > 64) {
    return { passed: false, message: "不通过：本地部分长度不合法" };
  }
  if (!/^[a-zA-Z0-9._%+-]+$/.test(local)) {
    return { passed: false, message: "不通过：本地部分含非法字符" };
  }
  if (domain.length === 0 || !domain.includes(".")) {
    return { passed: false, message: "不通过：域名部分不合法" };
  }
  const domainParts = domain.split(".");
  for (const part of domainParts) {
    if (part.length === 0 || !/^[a-zA-Z0-9.-]+$/.test(part)) {
      return { passed: false, message: "不通过：域名段不合法" };
    }
  }
  return { passed: true, message: "通过：合法邮箱地址" };
}

// 校验器分发表。
const VALIDATORS = {
  luhn: validateLuhn,
  ipv4: validateIpv4,
  ipv6: validateIpv6,
  idcard: validateIdcard,
  username: validateUsername,
  sex: validateSex,
  birth: validateBirth,
  address: (s, params) => validateAddress(s, params),
  generic: validateGeneric,
  email: validateEmail,
  // phonePrefix 和 phone 都走 validatePhone。
  phonePrefix: (s, params) =>
    validatePhone(s, params?.allowedPrefixes),
};

/**
 * 按 validator 类型分派执行校验。
 * @param {string} input — 用户测试输入
 * @param {string} validator — params.validator 字段
 * @param {object} params — 完整 params 对象
 * @returns {{ passed: boolean, message: string, note?: string }}
 */
export function runInlineValidator(input, validator, params) {
  const fn = VALIDATORS[validator];
  if (!fn) {
    return { passed: false, message: `未知校验类型：${validator}` };
  }
  return fn(input, params);
}
