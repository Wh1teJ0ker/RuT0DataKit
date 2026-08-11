//! 函数式校验器（v1.1.3 T55 新增）。
//!
//! 提取规则的「严格校验」层：提取正则宽松（召回优先），命中候选后用本模块的
//! 纯函数做严格校验。五条提取规则分别对应：
//! - `phone-extract` → [`check_phone_prefix`]（前 3 位白名单；空列表 = 默认通过）
//! - `bankcard-extract` → [`luhn_check`]（Luhn 算法）
//! - `ip4-extract` → [`is_valid_ipv4`]（段范围 + 前导零）
//! - `ip6-extract` → [`is_valid_ipv6`]（RFC 4291 严格）
//! - `idcard-extract` → [`is_valid_idcard`]（GB 11643-1999 校验码）
//!
//! T55b：原 `ip-extract` 拆为 `ip4-extract` + `ip6-extract` 两条独立规则，
//! `IpFamily` 枚举已删除。
//!
//! T55c：新增 `idcard-extract` 规则，`is_valid_idcard` 校验 18 位身份证号
//! 校验码（前 17 位加权求和 mod 11 查表），[`idcard_gender`] 推断性别
//! （第 17 位奇=男/偶=女）。性别联合校验（比对指定性别列）在
//! `extract_validate_to_new_sheet_inner` 中进行，不在此模块。
//!
//! T57：新增 5 个行级校验独立函数 [`is_valid_username`] / [`is_valid_sex`] /
//! [`is_valid_birth`] / [`is_valid_phone`] / [`is_valid_address`]，服务于
//! `validate_rows_to_two_sheets` 命令（行级多字段校验 → 双 Tab 输出）。
//! 这些函数不依赖 `ExtractParams` / DB rule 记录，直接对字段原值做严格校验；
//! 跨字段联合校验（sex vs idcard 性别、birth vs idcard 出生日期码）在
//! `validate_rows_to_two_sheets_inner` 中进行。
//!
//! [`validate_extracted`] 按 `rule.params` 分发到对应函数；`params=None` →
//! `(true, "")`（仅正则提取，不额外校验，向后兼容 name-extract 等老规则）。

use std::str::FromStr;

use crate::processor::rules::{ExtractParams, Rule};

/// Luhn 算法校验（银行卡号）。
///
/// 标准实现：从右往左，对偶数位（1-based，右起第 2/4/... 位）数字 ×2，
/// 若乘积 > 9 则数位相加，最后所有数字求和，总和能被 10 整除则有效。
///
/// 非 ASCII 数字字符直接判否（提取正则已保证纯数字，此处兜底）。
/// 空串 / 单字符 → `false`（Luhn 至少需要 2 位）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::luhn_check;
/// assert!(luhn_check("6222021234567890128"));   // 通过 Luhn（校验位 8）
/// assert!(!luhn_check("6222021234567890123"));  // 未通过 Luhn（校验位 3，总和 75）
/// ```
///
/// 注：需求原始示例 `6222021234567890123`（声称通过）与标准 Luhn 不符
/// （总和 75，%10≠0），故有效样例改为校验位 8（`...0128`，总和 80）。
pub fn luhn_check(s: &str) -> bool {
    if s.len() < 2 {
        return false;
    }
    let digits: Vec<u8> = s.bytes().filter(|b| b.is_ascii_digit()).collect();
    if digits.len() != s.len() {
        return false;
    }
    let mut sum: u32 = 0;
    let mut odd = true; // 右起第 1 位 = 奇数位（不 ×2）
    for &b in digits.iter().rev() {
        let mut d = (b - b'0') as u32;
        if !odd {
            d *= 2;
            if d > 9 {
                d = d / 10 + d % 10;
            }
        }
        sum += d;
        odd = !odd;
    }
    sum.is_multiple_of(10)
}

/// IPv4 严格校验：4 段点号分隔，每段 0-255，禁前导零（`"0"` 单独允许，
/// `"01"` / `"00"` 拒）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::is_valid_ipv4;
/// assert!(is_valid_ipv4("192.168.1.1"));
/// assert!(is_valid_ipv4("0.0.0.0"));
/// assert!(is_valid_ipv4("255.255.255.255"));
/// assert!(!is_valid_ipv4("256.1.1.1"));       // 超范围
/// assert!(!is_valid_ipv4("192.168.1"));       // 段数不足
/// assert!(!is_valid_ipv4("192.168.01.1"));    // 前导零
/// ```
pub fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for p in parts {
        if p.is_empty() {
            return false;
        }
        // 前导零：长度 > 1 且首字符为 '0' → 非法（"0" 单独允许）
        if p.len() > 1 && p.starts_with('0') {
            return false;
        }
        // 必须全 ASCII 数字
        if !p.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        let n: u32 = match p.parse() {
            Ok(n) => n,
            Err(_) => return false,
        };
        if n > 255 {
            return false;
        }
    }
    true
}

/// IPv6 严格校验：委托 `std::net::Ipv6Addr::from_str`（RFC 4291，支持 `::`
/// 缩写、全写、IPv4-mapped 等所有合法形式）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::is_valid_ipv6;
/// assert!(is_valid_ipv6("::1"));
/// assert!(is_valid_ipv6("2001:db8::1"));
/// assert!(is_valid_ipv6("fe80::1"));
/// assert!(!is_valid_ipv6("::g"));              // 非法十六进制
/// assert!(!is_valid_ipv6("2001:db8:::1"));     // 多重缩写
/// ```
pub fn is_valid_ipv6(s: &str) -> bool {
    std::net::Ipv6Addr::from_str(s).is_ok()
}

/// 手机号前缀白名单校验。
///
/// - `allowed` 空 → 默认通过（提取正则 `\b1\d{10}\b` 已保证 1 开头 + 11 位）。
/// - `allowed` 非空 → 提取值前 3 位必须在列表内（大小写敏感，但前缀本就是数字）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::check_phone_prefix;
/// assert!(check_phone_prefix("13412345678", &[]));                    // 空 = 默认通过
/// assert!(check_phone_prefix("13412345678", &["134".into()]));        // 命中
/// assert!(!check_phone_prefix("15912345678", &["134".into()]));       // 不命中
/// ```
pub fn check_phone_prefix(s: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    allowed.iter().any(|p| s.starts_with(p.as_str()))
}

/// 身份证号校验码系数（GB 11643-1999），第一位到第十七位依次乘。
const IDCARD_WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];

/// 身份证号校验码查表（余数 0..=10 → 校验码）。
const IDCARD_CHECK_CODES: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];

/// 身份证号严格校验（GB 11643-1999 校验码算法）。
///
/// 规则：18 位，前 17 位纯 ASCII 数字，第 18 位为数字或 'X'/'x'。
/// 前 17 位分别乘 [`IDCARD_WEIGHTS`]，加权和 mod 11，查
/// [`IDCARD_CHECK_CODES`] 得第 18 位校验码，匹配则有效。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::is_valid_idcard;
/// assert!(is_valid_idcard("11010519491231002X"));  // 校验码 X
/// assert!(is_valid_idcard("110105194912310038"));  // 校验码 8
/// assert!(is_valid_idcard("11010519491231002x"));  // 小写 x
/// assert!(!is_valid_idcard("110105194912310021")); // 校验码错
/// assert!(!is_valid_idcard("12345"));              // 长度不足
/// assert!(!is_valid_idcard("11010519491231002A"));// 非法末位
/// ```
pub fn is_valid_idcard(s: &str) -> bool {
    if s.len() != 18 {
        return false;
    }
    let bytes = s.as_bytes();
    // 前 17 位必须是 ASCII 数字
    let digits: Vec<u32> = match bytes[..17]
        .iter()
        .map(|b| (b - b'0') as u32)
        .collect::<Vec<u32>>()
    {
        ds if bytes[..17].iter().all(|b| b.is_ascii_digit()) => ds,
        _ => return false,
    };
    // 第 18 位：数字或 'X'/'x'（大写化后比对）
    let last = bytes[17];
    if !last.is_ascii_digit() && last != b'X' && last != b'x' {
        return false;
    }
    let expected = if last.is_ascii_digit() {
        last as char
    } else {
        'X'
    };
    // 加权求和 mod 11 → 查表
    let sum: u32 = digits
        .iter()
        .zip(IDCARD_WEIGHTS.iter())
        .map(|(d, w)| d * w)
        .sum();
    let check = IDCARD_CHECK_CODES[(sum % 11) as usize];
    check == expected
}

/// 从身份证号第 17 位（顺序码末位，0-indexed 16）推断性别。
///
/// 奇数 → `'男'`，偶数 → `'女'`。输入非合法身份证前 17 位（长度不足或
/// 第 17 位非数字）返回 `None`。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::idcard_gender;
/// assert_eq!(idcard_gender("110105194912310038"), Some('男')); // 第 17 位=3 奇
/// assert_eq!(idcard_gender("11010519491231002X"), Some('女')); // 第 17 位=2 偶
/// assert_eq!(idcard_gender("123"), None);                       // 长度不足
/// ```
pub fn idcard_gender(s: &str) -> Option<char> {
    let bytes = s.as_bytes();
    if bytes.len() < 17 || !bytes[16].is_ascii_digit() {
        return None;
    }
    let d = (bytes[16] - b'0') % 2;
    if d == 1 {
        Some('男')
    } else {
        Some('女')
    }
}

// ---- T57：行级校验独立函数 ----
//
// 以下 5 个函数服务于 `validate_rows_to_two_sheets` 命令（行级多字段校验），
// 不依赖 `ExtractParams` / DB rule 记录，直接对字段原值做严格校验。
// 已有的 `is_valid_idcard` / `idcard_gender` / `check_phone_prefix` 在此复用。

/// 用户名校验：纯字母数字（非空）。
///
/// 规则：`^[a-zA-Z0-9]+$`。有效 `admin` / `lufe1jian` / `91xxev`；
/// 无效 `ab.cd` / `ad_1in` / `a-123`（含 `.` / `_` / `-`）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::is_valid_username;
/// assert!(is_valid_username("admin"));
/// assert!(is_valid_username("lufe1jian"));
/// assert!(is_valid_username("91xxev"));
/// assert!(!is_valid_username("ab.cd"));  // 含点
/// assert!(!is_valid_username("ad_1in")); // 含下划线
/// assert!(!is_valid_username("a-123"));  // 含连字符
/// assert!(!is_valid_username(""));       // 空串
/// ```
pub fn is_valid_username(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// 性别校验：仅「男」或「女」（trim 后精确匹配，区分大小写）。
///
/// 不接受 `male` / `m` / `1` 等别名（行级校验对原值严格匹配；性别归一化
/// 用于跨字段比对，见 [`normalize_gender`] in commands::processor）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::is_valid_sex;
/// assert!(is_valid_sex("男"));
/// assert!(is_valid_sex("女"));
/// assert!(!is_valid_sex("male"));
/// assert!(!is_valid_sex(""));
/// ```
pub fn is_valid_sex(s: &str) -> bool {
    matches!(s.trim(), "男" | "女")
}

/// 出生日期校验：8 位 ASCII 数字。
///
/// 仅校验格式 `^\d{8}$`，不校验日期合法性（如 19990230 也会通过）。
/// 与身份证号出生日期码的联合校验（比对 idcard 第 7-14 位）在
/// `validate_rows_to_two_sheets_inner` 中进行，不在此函数。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::is_valid_birth;
/// assert!(is_valid_birth("19491231"));
/// assert!(is_valid_birth("20000101"));
/// assert!(!is_valid_birth("1949-12-31")); // 含分隔符
/// assert!(!is_valid_birth("1949123"));    // 长度不足
/// assert!(!is_valid_birth("194912311"));  // 长度超
/// assert!(!is_valid_birth(""));           // 空串
/// ```
pub fn is_valid_birth(s: &str) -> bool {
    s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit())
}

/// 手机号校验：11 位、1 开头、纯 ASCII 数字；可选前缀白名单。
///
/// - `allowed` 空 → 仅检查 1 开头 + 11 位（默认通过，同 `phone-extract` 语义）。
/// - `allowed` 非空 → 前 3 位必须在列表内（复用 [`check_phone_prefix`]）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::is_valid_phone;
/// assert!(is_valid_phone("13412345678", &[]));
/// assert!(is_valid_phone("13412345678", &["134".into()]));
/// assert!(!is_valid_phone("13412345678", &["159".into()])); // 前缀不匹配
/// assert!(!is_valid_phone("23412345678", &[]));             // 非 1 开头
/// assert!(!is_valid_phone("1341234567", &[]));              // 长度不足
/// assert!(!is_valid_phone("1341234567a", &[]));             // 含非数字
/// ```
pub fn is_valid_phone(s: &str, allowed: &[String]) -> bool {
    s.len() == 11
        && s.starts_with('1')
        && s.bytes().all(|b| b.is_ascii_digit())
        && check_phone_prefix(s, allowed)
}

/// 地址校验：全中文前缀 + 号(1-1500) + 室(101-999)。
///
/// 正则：`^[\u4e00-\u9fa5]+(?:[1-9]\d{0,2}|1[0-4]\d{2}|1500)号(?:10[1-9]|1[1-9]\d|[2-9]\d{2})室$`
/// - 中文前缀：1 个及以上中文字符
/// - 号：1-1500（1-999 / 1000-1499 / 1500）
/// - 室：101-999（101-109 / 110-199 / 200-999）
///
/// 正例：`内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室`
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::func_validator::is_valid_address;
/// assert!(is_valid_address("内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室"));
/// assert!(is_valid_address("北京市朝阳区1号101室"));
/// assert!(!is_valid_address("北京市朝阳区1501号101室")); // 号超 1500
/// assert!(!is_valid_address("北京市朝阳区1号100室"));    // 室不足 101
/// assert!(!is_valid_address("北京市朝阳区1号1000室"));   // 室超 999
/// assert!(!is_valid_address("1234号101室"));            // 前缀非中文
/// assert!(!is_valid_address("北京市朝阳区1号101"));      // 缺「室」
/// ```
pub fn is_valid_address(s: &str) -> bool {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        // 号范围 1-1500：[1-9]\d{0,2} | 1[0-4]\d{2} | 1500
        // 室范围 101-999：10[1-9] | 1[1-9]\d | [2-9]\d{2}
        regex::Regex::new(
            r"^[\u4e00-\u9fa5]+(?:[1-9]\d{0,2}|1[0-4]\d{2}|1500)号(?:10[1-9]|1[1-9]\d|[2-9]\d{2})室$",
        )
        .expect("address regex 编译失败（静态字面量，不应发生）")
    });
    re.is_match(s)
}

/// 按 `rule.params` 分发到对应函数式校验器。
///
/// 返回 `(是否有效, 说明)`：
/// - `params=None` → `(true, "")`（仅正则提取，不额外校验，向后兼容 name-extract）。
/// - `PhonePrefix{[]}` → 默认通过；非空 → 前缀白名单。
/// - `Luhn` → Luhn 算法；未通过说明 "未通过 Luhn 校验"。
/// - `Ipv4` → 段范围 0-255 + 禁前导零；未通过说明 "非合法 IPv4 地址"。
/// - `Ipv6` → `std::net::Ipv6Addr::from_str`（RFC 4291）；未通过说明 "非合法 IPv6 地址"。
/// - `IdCard` → GB 11643-1999 校验码；有效说明列写性别（"男"/"女"），
///   无效说明 "非合法身份证号"。性别联合校验（比对指定性别列）在
///   `extract_validate_to_new_sheet_inner` 中进行，不在此函数。
pub fn validate_extracted(rule: &Rule, value: &str) -> (bool, String) {
    match &rule.params {
        None => (true, String::new()),
        Some(ExtractParams::PhonePrefix { allowed_prefixes }) => {
            if check_phone_prefix(value, allowed_prefixes) {
                (true, String::new())
            } else {
                (false, "前缀不在允许列表内".to_string())
            }
        }
        Some(ExtractParams::Luhn) => {
            if luhn_check(value) {
                (true, String::new())
            } else {
                (false, "未通过 Luhn 校验".to_string())
            }
        }
        Some(ExtractParams::Ipv4) => {
            if is_valid_ipv4(value) {
                (true, String::new())
            } else {
                (false, "非合法 IPv4 地址".to_string())
            }
        }
        Some(ExtractParams::Ipv6) => {
            if is_valid_ipv6(value) {
                (true, String::new())
            } else {
                (false, "非合法 IPv6 地址".to_string())
            }
        }
        Some(ExtractParams::IdCard) => {
            if is_valid_idcard(value) {
                // 有效 → 说明列写推断的性别（性别联合校验在外层处理）
                let gender = idcard_gender(value)
                    .map(|c| c.to_string())
                    .unwrap_or_default();
                (true, gender)
            } else {
                (false, "非合法身份证号".to_string())
            }
        }
        // v1.1.4 T67：4 条新变体分发（用于 username/sex/birth/address validate 规则）
        Some(ExtractParams::Username) => {
            if is_valid_username(value) {
                (true, String::new())
            } else {
                (false, "用户名须为纯字母数字".to_string())
            }
        }
        Some(ExtractParams::Sex) => {
            if is_valid_sex(value) {
                (true, String::new())
            } else {
                (false, "性别须为「男」或「女」".to_string())
            }
        }
        Some(ExtractParams::Birth) => {
            if is_valid_birth(value) {
                (true, String::new())
            } else {
                (false, "出生日期须为 8 位数字".to_string())
            }
        }
        Some(ExtractParams::Address) => {
            if is_valid_address(value) {
                (true, String::new())
            } else {
                (
                    false,
                    "地址格式不符（全中文+号1-1500+室101-999）".to_string(),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Luhn ----

    #[test]
    fn luhn_valid_samples() {
        // 用户给的有效样例（标准 Luhn 校验位：`6222021234567890128` 总和 80，%10==0）
        assert!(luhn_check("6222021234567890128"));
        // 常见测试卡号（Visa / Mastercard / Amex 经典 Luhn 通过号）
        assert!(luhn_check("4111111111111111")); // Visa 测试号
        assert!(luhn_check("5500000000000004")); // Mastercard 测试号
        assert!(luhn_check("378282246310005")); // Amex 测试号
    }

    #[test]
    fn luhn_invalid_samples() {
        // 用户给的无效样例（`6222021234567890123` 总和 75，%10≠0）
        assert!(!luhn_check("6222021234567890123"));
        // 末位错一位
        assert!(!luhn_check("4111111111111112"));
    }

    #[test]
    fn luhn_edge_cases() {
        assert!(!luhn_check("")); // 空串
        assert!(!luhn_check("7")); // 单字符
        assert!(!luhn_check("12a4")); // 含非数字
                                      // 最短合法 Luhn：00 → 0+0=0；18 → 8 + (1×2=2) = 10 → 都 %10==0
        assert!(luhn_check("00"));
        assert!(luhn_check("18"));
        // 12 → 2 + (1×2=2) = 4 ≠ 0 → 无效
        assert!(!luhn_check("12"));
    }

    // ---- IPv4 ----

    #[test]
    fn ipv4_valid_samples() {
        // 用户给的有效样例
        assert!(is_valid_ipv4("192.168.1.1"));
        assert!(is_valid_ipv4("10.0.0.1"));
        assert!(is_valid_ipv4("255.255.255.255"));
        // 边界
        assert!(is_valid_ipv4("0.0.0.0"));
        assert!(is_valid_ipv4("1.2.3.4"));
    }

    #[test]
    fn ipv4_invalid_samples() {
        // 用户给的无效样例
        assert!(!is_valid_ipv4("256.1.1.1")); // 超范围
        assert!(!is_valid_ipv4("192.168.1")); // 段数不足
        assert!(!is_valid_ipv4("192.168.01.1")); // 前导零
                                                 // 其他非法
        assert!(!is_valid_ipv4("192.168.1.1.1")); // 段数过多
        assert!(!is_valid_ipv4("192.168..1")); // 空段
        assert!(!is_valid_ipv4("192.168.a.1")); // 非数字
        assert!(!is_valid_ipv4("192.168.1.999")); // 超范围
        assert!(!is_valid_ipv4("")); // 空串
        assert!(!is_valid_ipv4("192.168.1.01")); // 末段前导零
    }

    // ---- IPv6 ----

    #[test]
    fn ipv6_valid_samples() {
        assert!(is_valid_ipv6("::1"));
        assert!(is_valid_ipv6("::"));
        assert!(is_valid_ipv6("2001:db8::1"));
        assert!(is_valid_ipv6("fe80::1"));
        assert!(is_valid_ipv6("2001:0db8:0000:0000:0000:0000:0000:0001")); // 全写
        assert!(is_valid_ipv6("::ffff:192.168.1.1")); // IPv4-mapped
    }

    #[test]
    fn ipv6_invalid_samples() {
        assert!(!is_valid_ipv6("::g")); // 非法十六进制
        assert!(!is_valid_ipv6("2001:db8:::1")); // 多重缩写
        assert!(!is_valid_ipv6("2001:db8")); // 段数不足且无缩写
        assert!(!is_valid_ipv6("")); // 空串
        assert!(!is_valid_ipv6("gggg::1")); // 非法字符
    }

    // ---- check_phone_prefix ----

    #[test]
    fn phone_prefix_empty_list_passes() {
        assert!(check_phone_prefix("13412345678", &[]));
        assert!(check_phone_prefix("15987654321", &[]));
        assert!(check_phone_prefix("19898765432", &[]));
    }

    #[test]
    fn phone_prefix_match() {
        let allowed = vec!["134".to_string(), "159".to_string()];
        assert!(check_phone_prefix("13412345678", &allowed));
        assert!(check_phone_prefix("15987654321", &allowed));
    }

    #[test]
    fn phone_prefix_no_match() {
        let allowed = vec!["134".to_string()];
        assert!(!check_phone_prefix("15987654321", &allowed));
        assert!(!check_phone_prefix("19898765432", &allowed));
    }

    // ---- IdCard ----

    #[test]
    fn idcard_valid_samples() {
        // 校验码 X（余数 2），性别女（第 17 位 2 偶）
        assert!(is_valid_idcard("11010519491231002X"));
        // 校验码 8（余数 4），性别男（第 17 位 3 奇）
        assert!(is_valid_idcard("110105194912310038"));
        // 小写 x
        assert!(is_valid_idcard("11010519491231002x"));
    }

    #[test]
    fn idcard_invalid_samples() {
        // 校验码错（应为 X，给 1）
        assert!(!is_valid_idcard("110105194912310021"));
        // 长度不足
        assert!(!is_valid_idcard("12345"));
        // 长度超
        assert!(!is_valid_idcard("11010519491231002X1"));
        // 前 17 位含非数字
        assert!(!is_valid_idcard("11010519491231002A"));
        // 空串
        assert!(!is_valid_idcard(""));
    }

    #[test]
    fn idcard_gender_inference() {
        // 第 17 位（0-indexed 16）= 3 奇 → 男
        assert_eq!(idcard_gender("110105194912310038"), Some('男'));
        // 第 17 位 = 2 偶 → 女
        assert_eq!(idcard_gender("11010519491231002X"), Some('女'));
        // 第 17 位 = 0 偶 → 女
        assert_eq!(idcard_gender("110105194912310000"), Some('女'));
        // 第 17 位 = 9 奇 → 男
        assert_eq!(idcard_gender("110105194912310090"), Some('男'));
        // 长度不足 → None
        assert_eq!(idcard_gender("123"), None);
        // 第 17 位非数字 → None
        assert_eq!(idcard_gender("1101051949123100X8"), None);
    }

    // ---- validate_extracted (分发) ----

    #[test]
    fn validate_extracted_none_passes() {
        // name-extract 风格：params=None → 直接通过
        let mut rule = RuleRegistry_like_name_extract();
        rule.params = None;
        assert_eq!(
            validate_extracted(&rule, "anything"),
            (true, "".to_string())
        );
    }

    #[test]
    fn validate_extracted_phone_prefix() {
        let mut rule = RuleRegistry_like_name_extract();
        rule.params = Some(ExtractParams::PhonePrefix {
            allowed_prefixes: vec!["134".into()],
        });
        assert!(validate_extracted(&rule, "13412345678").0);
        assert!(!validate_extracted(&rule, "15987654321").0);
        // 空列表 → 默认通过
        rule.params = Some(ExtractParams::PhonePrefix {
            allowed_prefixes: vec![],
        });
        assert!(validate_extracted(&rule, "13412345678").0);
    }

    #[test]
    fn validate_extracted_luhn() {
        let mut rule = RuleRegistry_like_name_extract();
        rule.params = Some(ExtractParams::Luhn);
        assert!(validate_extracted(&rule, "6222021234567890128").0);
        assert!(!validate_extracted(&rule, "6222021234567890123").0);
    }

    #[test]
    fn validate_extracted_ipv4_ipv6() {
        // T55b：Ipv4 / Ipv6 两个变体分别校验
        let mut rule_v4 = RuleRegistry_like_name_extract();
        rule_v4.params = Some(ExtractParams::Ipv4);
        assert!(validate_extracted(&rule_v4, "192.168.1.1").0);
        assert!(!validate_extracted(&rule_v4, "256.1.1.1").0); // 超范围
        assert!(!validate_extracted(&rule_v4, "192.168.01.1").0); // 前导零
        assert!(!validate_extracted(&rule_v4, "::1").0); // IPv6 不应通过 IPv4 校验

        let mut rule_v6 = RuleRegistry_like_name_extract();
        rule_v6.params = Some(ExtractParams::Ipv6);
        assert!(validate_extracted(&rule_v6, "::1").0);
        assert!(validate_extracted(&rule_v6, "2001:db8::1").0);
        assert!(!validate_extracted(&rule_v6, "192.168.1.1").0); // IPv4 不应通过 IPv6 校验
        assert!(!validate_extracted(&rule_v6, "1:2:3:4:5:6:7:8:9").0); // 段数超 8
    }

    #[test]
    fn validate_extracted_idcard() {
        // T55c：IdCard 变体校验码 + 性别推断
        let mut rule = RuleRegistry_like_name_extract();
        rule.params = Some(ExtractParams::IdCard);
        // 有效女（第 17 位 2 偶，校验码 X）
        let (ok, note) = validate_extracted(&rule, "11010519491231002X");
        assert!(ok);
        assert_eq!(note, "女");
        // 有效男（第 17 位 3 奇，校验码 8）
        let (ok, note) = validate_extracted(&rule, "110105194912310038");
        assert!(ok);
        assert_eq!(note, "男");
        // 无效（校验码错）
        let (ok, note) = validate_extracted(&rule, "110105194912310021");
        assert!(!ok);
        assert_eq!(note, "非合法身份证号");
        // 无效（长度不足）
        let (ok, _) = validate_extracted(&rule, "12345");
        assert!(!ok);
    }

    // ---- v1.1.4 T67：4 条新变体分发测试 ----

    #[test]
    fn validate_extracted_username() {
        // T67：Username 变体 → is_valid_username
        let mut rule = RuleRegistry_like_name_extract();
        rule.params = Some(ExtractParams::Username);
        // 有效（纯字母数字）
        assert!(validate_extracted(&rule, "admin").0);
        let (ok, note) = validate_extracted(&rule, "lufe1jian");
        assert!(ok);
        assert_eq!(note, "");
        // 无效（含点）
        let (ok, note) = validate_extracted(&rule, "ab.cd");
        assert!(!ok);
        assert_eq!(note, "用户名须为纯字母数字");
        // 无效（含下划线）
        let (ok, _) = validate_extracted(&rule, "ad_1in");
        assert!(!ok);
        // 无效（空串）
        let (ok, _) = validate_extracted(&rule, "");
        assert!(!ok);
    }

    #[test]
    fn validate_extracted_sex() {
        // T67：Sex 变体 → is_valid_sex
        let mut rule = RuleRegistry_like_name_extract();
        rule.params = Some(ExtractParams::Sex);
        // 有效
        let (ok, note) = validate_extracted(&rule, "男");
        assert!(ok);
        assert_eq!(note, "");
        assert!(validate_extracted(&rule, "女").0);
        // trim 后匹配
        assert!(validate_extracted(&rule, " 男 ").0);
        // 无效
        let (ok, note) = validate_extracted(&rule, "male");
        assert!(!ok);
        assert_eq!(note, "性别须为「男」或「女」");
        let (ok, _) = validate_extracted(&rule, "");
        assert!(!ok);
        let (ok, _) = validate_extracted(&rule, "未知");
        assert!(!ok);
    }

    #[test]
    fn validate_extracted_birth() {
        // T67：Birth 变体 → is_valid_birth
        let mut rule = RuleRegistry_like_name_extract();
        rule.params = Some(ExtractParams::Birth);
        // 有效（8 位数字）
        let (ok, note) = validate_extracted(&rule, "19491231");
        assert!(ok);
        assert_eq!(note, "");
        assert!(validate_extracted(&rule, "20000101").0);
        // 无效（含分隔符）
        let (ok, note) = validate_extracted(&rule, "1949-12-31");
        assert!(!ok);
        assert_eq!(note, "出生日期须为 8 位数字");
        // 无效（长度不足）
        let (ok, _) = validate_extracted(&rule, "1949123");
        assert!(!ok);
        // 无效（长度超）
        let (ok, _) = validate_extracted(&rule, "194912311");
        assert!(!ok);
        // 无效（空串）
        let (ok, _) = validate_extracted(&rule, "");
        assert!(!ok);
    }

    #[test]
    fn validate_extracted_address() {
        // T67：Address 变体 → is_valid_address
        let mut rule = RuleRegistry_like_name_extract();
        rule.params = Some(ExtractParams::Address);
        // 有效
        let (ok, note) =
            validate_extracted(&rule, "内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室");
        assert!(ok);
        assert_eq!(note, "");
        assert!(validate_extracted(&rule, "北京市朝阳区1号101室").0);
        // 无效（号超 1500）
        let (ok, note) = validate_extracted(&rule, "北京市朝阳区1501号101室");
        assert!(!ok);
        assert_eq!(note, "地址格式不符（全中文+号1-1500+室101-999）");
        // 无效（室不足 101）
        let (ok, _) = validate_extracted(&rule, "北京市朝阳区1号100室");
        assert!(!ok);
        // 无效（前缀非中文）
        let (ok, _) = validate_extracted(&rule, "1234号101室");
        assert!(!ok);
        // 无效（缺「室」）
        let (ok, _) = validate_extracted(&rule, "北京市朝阳区1号101");
        assert!(!ok);
        // 无效（空串）
        let (ok, _) = validate_extracted(&rule, "");
        assert!(!ok);
    }

    // ---- T57：行级校验独立函数 ----

    #[test]
    fn username_valid_samples() {
        // 用户给的有效样例
        assert!(is_valid_username("admin"));
        assert!(is_valid_username("lufe1jian"));
        assert!(is_valid_username("91xxev"));
        // 混合大小写 + 数字
        assert!(is_valid_username("AbCd123"));
        assert!(is_valid_username("Z9"));
    }

    #[test]
    fn username_invalid_samples() {
        // 用户给的无效样例
        assert!(!is_valid_username("ab.cd")); // 含点
        assert!(!is_valid_username("ad_1in")); // 含下划线
        assert!(!is_valid_username("a-123")); // 含连字符
                                              // 其他非法
        assert!(!is_valid_username("")); // 空串
        assert!(!is_valid_username("用户名")); // 非字母数字
        assert!(!is_valid_username("abc def")); // 含空格
        assert!(!is_valid_username("a@b")); // 含特殊字符
    }

    #[test]
    fn sex_valid_samples() {
        assert!(is_valid_sex("男"));
        assert!(is_valid_sex("女"));
        // trim 后匹配
        assert!(is_valid_sex(" 男 "));
        assert!(is_valid_sex("\t女"));
    }

    #[test]
    fn sex_invalid_samples() {
        assert!(!is_valid_sex("male"));
        assert!(!is_valid_sex("female"));
        assert!(!is_valid_sex("m"));
        assert!(!is_valid_sex("f"));
        assert!(!is_valid_sex("1"));
        assert!(!is_valid_sex("2"));
        assert!(!is_valid_sex(""));
        assert!(!is_valid_sex("未知"));
    }

    #[test]
    fn birth_valid_samples() {
        assert!(is_valid_birth("19491231"));
        assert!(is_valid_birth("20000101"));
        assert!(is_valid_birth("99991231"));
        assert!(is_valid_birth("00000101"));
    }

    #[test]
    fn birth_invalid_samples() {
        assert!(!is_valid_birth("1949-12-31")); // 含分隔符
        assert!(!is_valid_birth("1949123")); // 长度不足
        assert!(!is_valid_birth("194912311")); // 长度超
        assert!(!is_valid_birth("1949ab31")); // 含非数字
        assert!(!is_valid_birth("")); // 空串
    }

    #[test]
    fn phone_valid_samples() {
        // 空前缀列表 → 仅检查 1 开头 + 11 位
        assert!(is_valid_phone("13412345678", &[]));
        assert!(is_valid_phone("15987654321", &[]));
        assert!(is_valid_phone("19898765432", &[]));
        // 前缀白名单命中
        assert!(is_valid_phone("13412345678", &["134".into()]));
        assert!(is_valid_phone("15987654321", &["134".into(), "159".into()]));
    }

    #[test]
    fn phone_invalid_samples() {
        // 前缀不匹配
        assert!(!is_valid_phone("13412345678", &["159".into()]));
        // 非 1 开头
        assert!(!is_valid_phone("23412345678", &[]));
        // 长度不足
        assert!(!is_valid_phone("1341234567", &[]));
        // 长度超
        assert!(!is_valid_phone("134123456789", &[]));
        // 含非数字
        assert!(!is_valid_phone("1341234567a", &[]));
        // 空串
        assert!(!is_valid_phone("", &[]));
    }

    #[test]
    fn address_valid_samples() {
        // 用户给的正例
        assert!(is_valid_address(
            "内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室"
        ));
        // 边界：号=1 / 号=1500 / 室=101 / 室=999
        assert!(is_valid_address("北京市朝阳区1号101室"));
        assert!(is_valid_address("北京市朝阳区1500号999室"));
        assert!(is_valid_address("北京市朝阳区999号101室"));
        // 号=1000（1[0-4]\d{2} 分支）
        assert!(is_valid_address("北京市朝阳区1000号500室"));
    }

    #[test]
    fn address_invalid_samples() {
        // 号超 1500
        assert!(!is_valid_address("北京市朝阳区1501号101室"));
        assert!(!is_valid_address("北京市朝阳区9999号101室"));
        // 室不足 101
        assert!(!is_valid_address("北京市朝阳区1号100室"));
        assert!(!is_valid_address("北京市朝阳区1号99室"));
        // 室超 999
        assert!(!is_valid_address("北京市朝阳区1号1000室"));
        // 前缀非中文
        assert!(!is_valid_address("1234号101室"));
        assert!(!is_valid_address("abc1号101室"));
        // 缺「室」
        assert!(!is_valid_address("北京市朝阳区1号101"));
        // 缺「号」
        assert!(!is_valid_address("北京市朝阳区1室"));
        // 空串
        assert!(!is_valid_address(""));
        // 含字母
        assert!(!is_valid_address("北京市朝阳区A1号101室"));
    }

    /// 测试辅助：构造一个最小 Rule（params 可后续覆盖）。
    fn RuleRegistry_like_name_extract() -> Rule {
        use crate::processor::rules::{Rule, RuleKind};
        Rule {
            id: "test".into(),
            name: "test".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: String::new(),
            template: None,
            params: None,
        }
    }
}
