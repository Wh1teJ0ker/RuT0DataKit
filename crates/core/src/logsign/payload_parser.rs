//! SQLi payload 语义解析模块（v0.2.0 T2-8）。
//!
//! 与 [`crate::logsign`][`super`] 签名匹配互补：签名只判「命中类别」（rule_id），
//! 本模块对命中后的 decoded 文本做「结构化语义解析」，把盲注二分的「读取
//! database() 第 N 字符 ascii>X」、UNION 的「取 N 列」、报错的「读取 user()」、
//! 时间盲注的「延时 N 秒」、恒真的「N=M」、注释符的「截断」结构化出来。
//!
//! ## 输入约定
//!
//! - 输入应为 **已解码文本**：`+` 已转为空格、`%XX` 已双重 URL 解码（由
//!   [`crate::log::parse_query`] 完成）。本模块不做解码，直接吃 decoded 形态。
//! - 内部对文本先 `.to_lowercase()` 再匹配，兼容 `UNION SELECT` / `ASCII(SUBSTR(...))`
//!   等大写形态；正则 pattern 一律按小写编写。
//!
//! ## 解析顺序
//!
//! 按 blind → union → error → time → tautology → comment 顺序首命中返回；
//! 一条 payload 只归一类。例如 `1 or 1=1#` 同时含恒真和注释符，按顺序归 tautology
//! （更具体的语义优先）。无匹配返回 `None`。
//!
//! ## regex 限制
//!
//! regex crate 无 look-around，blind 的 `substr((database()),1,1)` 嵌套括号
//! 只捕获第一层 `([a-z_]+\(\))`（如 `database()`），不递归匹配 `substr(database(),...)`
//! 内部。这是已知简化，满足 v0.2.0 acceptance。

use regex::Regex;

/// 6 类语义解析后的结构化 payload 信息。
///
/// 一条 payload 只产一个 `ParsedPayload`（首命中返回），`attack_type` 对应
/// 6 类之一，其余字段按类别填充（无关字段为 `None`）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ParsedPayload {
    /// 类别：blind_boolean / union / error / time / tautology / comment。
    pub attack_type: String,
    /// 技术：binary_search / union_columns / extractvalue_error / sleep_delay /
    /// tautology_bypass / comment_truncate。
    pub technique: String,
    /// 读取目标 SQL 函数（带括号），如 `database()` / `user()` / `version()` /
    /// `current_user()`。blind/error 类填充，其余为 `None`。
    pub read_target: Option<String>,
    /// 盲注读取第几个字符（1-based）。仅 blind_boolean 填充。
    pub char_position: Option<u32>,
    /// 与哪个 ascii 值比较。仅 blind_boolean 填充。
    pub compared_ascii: Option<u32>,
    /// 比较运算符：`>` / `<` / `=` / `>=` / `<=`。仅 blind_boolean 填充。
    pub comparator: Option<String>,
    /// UNION 注入取几列。仅 union 填充。
    pub union_columns: Option<u32>,
    /// 时间盲注延时秒数。仅 time 填充。
    pub sleep_seconds: Option<u32>,
    /// 人类可读一句话总结，前端可直接展示。
    pub summary: String,
}

/// 编译后的 6 类正则 + 上下文，构造一次复用。
struct PayloadPatterns {
    blind_boolean: Regex,
    union: Regex,
    error_based: Regex,
    time_based: Regex,
    tautology: Regex,
    comment: Regex,
}

impl PayloadPatterns {
    fn new() -> Self {
        Self {
            // 捕获组：1=read_target（如 database()）、2=char_position、3=comparator、4=compared_ascii
            blind_boolean: Regex::new(
                r"ascii\s*\(\s*substr\s*\(\s*\(?\s*([a-z_]+\(\))\s*\)?\s*,\s*(\d+)\s*,\s*\d+\s*\)\s*\)\s*(>=?|<=?|=)\s*(\d+)",
            ).expect("blind_boolean regex"),
            // 捕获组 1 = 逗号分隔的数字串
            union: Regex::new(r"union\s+(?:all\s+)?select\s+(\d+(?:\s*,\s*\d+)*)")
                .expect("union regex"),
            // 捕获组 1 = read_target（如 user()）
            error_based: Regex::new(
                r"(?:extractvalue|updatexml)\s*\(\s*\d+\s*,\s*concat\s*\([^,]+,\s*([a-z_]+\(\))",
            ).expect("error_based regex"),
            // 捕获组 1 = sleep_seconds
            time_based: Regex::new(r"(?:sleep|benchmark|pg_sleep)\s*\(\s*(\d+)")
                .expect("time_based regex"),
            // 捕获组 1/2 = 两边数字
            tautology: Regex::new(r"\b(?:or|and)\s+(\d+)\s*=\s*(\d+)")
                .expect("tautology regex"),
            // 捕获组 1 = 注释符
            comment: Regex::new(r"(--|#|/\*)").expect("comment regex"),
        }
    }
}

/// 对 decoded 文本跑语义解析；无匹配返回 `None`。
///
/// 解析顺序：blind → union → error → time → tautology → comment，首命中返回。
/// 内部对 `decoded_text` 先 `to_lowercase()` 再匹配。
pub fn parse_payload(decoded_text: &str) -> Option<ParsedPayload> {
    use std::sync::OnceLock;
    static PATTERNS: OnceLock<PayloadPatterns> = OnceLock::new();
    let p = PATTERNS.get_or_init(PayloadPatterns::new);

    let lower = decoded_text.to_lowercase();

    // 1. blind_boolean
    if let Some(caps) = p.blind_boolean.captures(&lower) {
        let read_target = caps.get(1).map(|m| m.as_str().to_string());
        let char_position = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
        let comparator = caps.get(3).map(|m| m.as_str().to_string());
        let compared_ascii = caps.get(4).and_then(|m| m.as_str().parse::<u32>().ok());
        let summary = format!(
            "盲注二分：读取 {} 第 {} 字符 ascii {} {}",
            read_target.as_deref().unwrap_or("?"),
            char_position.unwrap_or(0),
            comparator.as_deref().unwrap_or("?"),
            compared_ascii.unwrap_or(0),
        );
        return Some(ParsedPayload {
            attack_type: "blind_boolean".to_string(),
            technique: "binary_search".to_string(),
            read_target,
            char_position,
            compared_ascii,
            comparator,
            union_columns: None,
            sleep_seconds: None,
            summary,
        });
    }

    // 2. union
    if let Some(caps) = p.union.captures(&lower) {
        let nums_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        // 数逗号+1 得列数；无逗号（单数字）= 1 列。
        let cols = nums_str.split(',').count() as u32;
        let summary = format!("UNION 注入：取 {} 列", cols);
        return Some(ParsedPayload {
            attack_type: "union".to_string(),
            technique: "union_columns".to_string(),
            read_target: None,
            char_position: None,
            compared_ascii: None,
            comparator: None,
            union_columns: Some(cols),
            sleep_seconds: None,
            summary,
        });
    }

    // 3. error_based
    if let Some(caps) = p.error_based.captures(&lower) {
        let read_target = caps.get(1).map(|m| m.as_str().to_string());
        let summary = format!(
            "报错注入：extractvalue 报错读取 {}",
            read_target.as_deref().unwrap_or("?"),
        );
        return Some(ParsedPayload {
            attack_type: "error".to_string(),
            technique: "extractvalue_error".to_string(),
            read_target,
            char_position: None,
            compared_ascii: None,
            comparator: None,
            union_columns: None,
            sleep_seconds: None,
            summary,
        });
    }

    // 4. time_based
    if let Some(caps) = p.time_based.captures(&lower) {
        let sleep_seconds = caps
            .get(1)
            .and_then(|m| m.as_str().parse::<u32>().ok());
        let summary = format!("时间盲注：延时 {} 秒", sleep_seconds.unwrap_or(0));
        return Some(ParsedPayload {
            attack_type: "time".to_string(),
            technique: "sleep_delay".to_string(),
            read_target: None,
            char_position: None,
            compared_ascii: None,
            comparator: None,
            union_columns: None,
            sleep_seconds,
            summary,
        });
    }

    // 5. tautology
    if let Some(caps) = p.tautology.captures(&lower) {
        let n = caps.get(1).map(|m| m.as_str()).unwrap_or("?");
        let m = caps.get(2).map(|m| m.as_str()).unwrap_or("?");
        let summary = format!("恒真绕过：{}={}", n, m);
        return Some(ParsedPayload {
            attack_type: "tautology".to_string(),
            technique: "tautology_bypass".to_string(),
            read_target: None,
            char_position: None,
            compared_ascii: None,
            comparator: None,
            union_columns: None,
            sleep_seconds: None,
            summary,
        });
    }

    // 6. comment
    if let Some(caps) = p.comment.captures(&lower) {
        let matched = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        let summary = format!("注释符截断查询：{}", matched);
        return Some(ParsedPayload {
            attack_type: "comment".to_string(),
            technique: "comment_truncate".to_string(),
            read_target: None,
            char_position: None,
            compared_ascii: None,
            comparator: None,
            union_columns: None,
            sleep_seconds: None,
            summary,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blind_boolean_positive() {
        let r = parse_payload("1' or ascii(substr((database()),1,1))>79#").unwrap();
        assert_eq!(r.attack_type, "blind_boolean");
        assert_eq!(r.technique, "binary_search");
        assert_eq!(r.read_target.as_deref(), Some("database()"));
        assert_eq!(r.char_position, Some(1));
        assert_eq!(r.comparator.as_deref(), Some(">"));
        assert_eq!(r.compared_ascii, Some(79));
        assert!(r.summary.contains("盲注"));
    }

    #[test]
    fn blind_boolean_negative() {
        assert!(parse_payload("normal query").is_none());
    }

    #[test]
    fn union_positive() {
        let r = parse_payload("1 union select 1,2,3").unwrap();
        assert_eq!(r.attack_type, "union");
        assert_eq!(r.union_columns, Some(3));
        assert!(r.summary.contains("3"));
    }

    #[test]
    fn union_all_positive() {
        let r = parse_payload("1 union all select 1,2,3,4").unwrap();
        assert_eq!(r.attack_type, "union");
        assert_eq!(r.union_columns, Some(4));
    }

    #[test]
    fn union_negative() {
        assert!(parse_payload("select 1").is_none());
    }

    #[test]
    fn error_based_positive() {
        let r = parse_payload("1 and extractvalue(1,concat(0x7e,user()))").unwrap();
        assert_eq!(r.attack_type, "error");
        assert_eq!(r.technique, "extractvalue_error");
        assert_eq!(r.read_target.as_deref(), Some("user()"));
        assert!(r.summary.contains("报错"));
    }

    #[test]
    fn error_based_negative() {
        assert!(parse_payload("concat(1,2)").is_none());
    }

    #[test]
    fn time_based_positive() {
        let r = parse_payload("1 and sleep(5)").unwrap();
        assert_eq!(r.attack_type, "time");
        assert_eq!(r.sleep_seconds, Some(5));
        assert!(r.summary.contains("延时"));
    }

    #[test]
    fn time_based_negative() {
        assert!(parse_payload("sleep mode").is_none());
    }

    #[test]
    fn tautology_positive() {
        let r = parse_payload("1 or 1=1").unwrap();
        assert_eq!(r.attack_type, "tautology");
        assert_eq!(r.technique, "tautology_bypass");
        assert!(r.summary.contains("恒真"));
    }

    #[test]
    fn tautology_negative() {
        // 无 or/and 前缀，不命中恒真。
        assert!(parse_payload("1=1").is_none());
    }

    #[test]
    fn comment_positive() {
        let r = parse_payload("1'--").unwrap();
        assert_eq!(r.attack_type, "comment");
        assert_eq!(r.technique, "comment_truncate");
        assert!(r.summary.contains("注释"));
    }

    #[test]
    fn comment_negative() {
        assert!(parse_payload("normal").is_none());
    }

    #[test]
    fn precedence_tautology_before_comment() {
        // `1 or 1=1#` 同时含恒真和注释符，按首命中顺序归 tautology。
        let r = parse_payload("1 or 1=1#").unwrap();
        assert_eq!(r.attack_type, "tautology");
    }

    #[test]
    fn uppercase_union_select_matched() {
        let r = parse_payload("1 UNION SELECT 1,2,3").unwrap();
        assert_eq!(r.attack_type, "union");
        assert_eq!(r.union_columns, Some(3));
    }
}
