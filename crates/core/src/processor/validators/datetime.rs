//! 出生日期校验（清理分隔符 + 多格式支持）。

use std::sync::OnceLock;

use regex::Regex;

/// 缓存的日期格式正则集，避免每次调用 `is_valid_birth_format` 都重新编译。
struct BirthFormatRegexes {
    yyyymmdd: Regex,
    yyyy_mm_dd: Regex,
    yyyy_sl_dd: Regex,
    yyyy_dot_dd: Regex,
}

impl BirthFormatRegexes {
    fn get() -> &'static Self {
        static REGEXES: OnceLock<BirthFormatRegexes> = OnceLock::new();
        REGEXES.get_or_init(|| BirthFormatRegexes {
            yyyymmdd: Regex::new(r"^\d{8}$").unwrap(),
            yyyy_mm_dd: Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$").unwrap(),
            yyyy_sl_dd: Regex::new(r"^(\d{4})/(\d{2})/(\d{2})$").unwrap(),
            yyyy_dot_dd: Regex::new(r"^(\d{4})\.(\d{2})\.(\d{2})$").unwrap(),
        })
    }
}

/// 清理出生日期字符串：过滤掉所有非 ASCII 数字字符。
///
/// 用于支持多种分隔符格式："2003-12-23" → "20031223"，
/// "2003/12/23" → "20031223"，"2003.12.23" → "20031223"。
///
/// v1.1.4 续轮 T70：跨字段 birth 比对也用本函数归一化（如 "1949-12-31"
/// 与身份证号 [6..14]="19491231" 视为一致）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::clean_birth;
/// assert_eq!(clean_birth("20031223"), "20031223");
/// assert_eq!(clean_birth("2003-12-23"), "20031223");
/// assert_eq!(clean_birth("2003/12/23"), "20031223");
/// assert_eq!(clean_birth("2003.12.23"), "20031223");
/// assert_eq!(clean_birth(" 2003 12 23 "), "20031223");
/// ```
pub fn clean_birth(s: &str) -> String {
    s.bytes()
        .filter(|b| b.is_ascii_digit())
        .map(|b| b as char)
        .collect()
}

/// 出生日期校验：先清理分隔符（- / . 空格等），再校验 8 位数字 + 基本日期
/// 有效性。
///
/// v1.1.4 续轮 T70：放宽原「8 位纯数字」格式要求，支持 "20031223" /
/// "2003-12-23" / "2003/12.23" / "2003.12.23" 等格式（先用
/// [`clean_birth`] 过滤非数字字符，再校验 8 位 + 日期范围）。
/// 清理后必须为 8 位数字，且年 1900-2100 / 月 1-12 / 日 1-31。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_birth;
/// assert!(is_valid_birth("20031223"));
/// assert!(is_valid_birth("2003-12-23"));
/// assert!(is_valid_birth("2003/12/23"));
/// assert!(is_valid_birth("2003.12.23"));
/// assert!(!is_valid_birth("20031323")); // 月13
/// assert!(!is_valid_birth("20031200")); // 日0
/// assert!(!is_valid_birth("2003-12"));  // 清理后6位
/// assert!(!is_valid_birth(""));         // 空串
/// ```
pub fn is_valid_birth(s: &str) -> bool {
    let cleaned = clean_birth(s);
    if cleaned.len() != 8 || !cleaned.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // 基本日期有效性
    let year: u32 = cleaned[0..4].parse().unwrap_or(0);
    let month: u32 = cleaned[4..6].parse().unwrap_or(0);
    let day: u32 = cleaned[6..8].parse().unwrap_or(0);
    (1900..=2100).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// 按指定格式校验出生日期（v1.1.5 T87 新增）。
///
/// 支持的格式标识：
/// - `"yyyymmdd"` — 8 位纯数字 + 有效日期
/// - `"yyyy-mm-dd"` — `^\d{4}-\d{2}-\d{2}$` + 有效日期
/// - `"yyyy/mm/dd"` — `^\d{4}/\d{2}/\d{2}$` + 有效日期
/// - `"yyyy.mm.dd"` — `^\d{4}\.\d{2}\.\d{2}$` + 有效日期
///
/// 未知格式 → false。有效日期 = 年 1900-2100、月 1-12、日 1-31（已有
/// [`is_valid_birth`] 的日期范围校验复用）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_birth_format;
/// assert!(is_valid_birth_format("20031223", "yyyymmdd"));
/// assert!(!is_valid_birth_format("2003-12-23", "yyyymmdd"));
/// assert!(is_valid_birth_format("2003-12-23", "yyyy-mm-dd"));
/// assert!(is_valid_birth_format("2003/12/23", "yyyy/mm/dd"));
/// assert!(is_valid_birth_format("2003.12.23", "yyyy.mm.dd"));
/// assert!(!is_valid_birth_format("20031223", "unknown")); // 未知格式
/// assert!(!is_valid_birth_format("20031323", "yyyymmdd")); // 月 13
/// ```
pub fn is_valid_birth_format(s: &str, format: &str) -> bool {
    // 先按格式正则匹配提取 8 位 yyyymmdd，再复用 is_valid_birth 的日期校验。
    // 正则缓存于 OnceLock，避免每次调用重新编译。
    let re = BirthFormatRegexes::get();
    let digits: String = match format {
        "yyyymmdd" => {
            if !re.yyyymmdd.is_match(s) {
                return false;
            }
            s.to_string()
        }
        "yyyy-mm-dd" => match re.yyyy_mm_dd.captures(s) {
            Some(c) => format!("{}{}{}", &c[1], &c[2], &c[3]),
            None => return false,
        },
        "yyyy/mm/dd" => match re.yyyy_sl_dd.captures(s) {
            Some(c) => format!("{}{}{}", &c[1], &c[2], &c[3]),
            None => return false,
        },
        "yyyy.mm.dd" => match re.yyyy_dot_dd.captures(s) {
            Some(c) => format!("{}{}{}", &c[1], &c[2], &c[3]),
            None => return false,
        },
        _ => return false,
    };
    // 复用 is_valid_birth 的日期范围校验（digits 已是纯数字）
    is_valid_birth(&digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn birth_valid_samples() {
        // T70：支持分隔符格式（清理后 8 位有效日期）
        assert!(is_valid_birth("19491231"));
        assert!(is_valid_birth("20000101"));
        assert!(is_valid_birth("20991231"));
        assert!(is_valid_birth("19000101"));
        // 含分隔符（清理后 8 位）
        assert!(is_valid_birth("2003-12-23"));
        assert!(is_valid_birth("2003/12/23"));
        assert!(is_valid_birth("2003.12.23"));
        assert!(is_valid_birth(" 2003 12 23 "));
    }

    #[test]
    fn birth_invalid_samples() {
        assert!(!is_valid_birth("20031323")); // 月 13
        assert!(!is_valid_birth("20031200")); // 日 0
        assert!(!is_valid_birth("1949123")); // 清理后 6 位（长度不足）
        assert!(!is_valid_birth("194912311")); // 长度超
        assert!(!is_valid_birth("1949ab31")); // 含非数字，清理后 6 位
        assert!(!is_valid_birth("")); // 空串
    }

    #[test]
    fn is_valid_birth_format_tests() {
        // v1.1.5 T87：按指定格式校验出生日期
        // yyyymmdd
        assert!(is_valid_birth_format("20031223", "yyyymmdd"));
        assert!(!is_valid_birth_format("2003-12-23", "yyyymmdd"));
        // yyyy-mm-dd
        assert!(is_valid_birth_format("2003-12-23", "yyyy-mm-dd"));
        assert!(!is_valid_birth_format("20031223", "yyyy-mm-dd"));
        // yyyy/mm/dd
        assert!(is_valid_birth_format("2003/12/23", "yyyy/mm/dd"));
        // yyyy.mm.dd
        assert!(is_valid_birth_format("2003.12.23", "yyyy.mm.dd"));
        // 未知格式
        assert!(!is_valid_birth_format("20031223", "unknown"));
        // 无效日期（月 13）
        assert!(!is_valid_birth_format("20031323", "yyyymmdd"));
    }
}
