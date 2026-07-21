//! v0.4.1 T6-5：自然语言描述 → 正则（规则化推断，非 LLM）。
//!
//! 从用户语句中识别 6 类线索（位数 / 字符集 / 锚定前缀 / 邮箱 / URL / 身份证），
//! 组装正则字符串，最后用 [`regex::Regex::new`] 校验可编译。
//!
//! ## 安全约束
//! 纯本地规则化推断，不调用网络，不外发 statement 或生成结果。

use crate::error::CoreError;
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ConstructedRegex {
    pub pattern: String,
    pub explanation: String,
    pub matched_clues: Vec<String>,
}

/// 从自然语言描述构造正则。
///
/// 识别顺序（语义类优先于通用类，避免被位数线索提前吃掉）：
/// 1. 邮箱：statement 含「邮箱」/「email」→ 邮箱正则
/// 2. URL：含「url」/「链接」/「网址」→ URL 正则
/// 3. 身份证：含「身份证」→ 18 位身份证正则
/// 4. 通用：位数 + 字符集 + 锚定前缀组合
pub fn construct_regex(statement: &str) -> Result<ConstructedRegex, CoreError> {
    let lower = statement.to_lowercase();
    let mut clues: Vec<String> = Vec::new();
    let pattern: String;

    // 语义类（优先匹配，命中即返回，不与通用组合混用）
    if lower.contains("邮箱") || lower.contains("email") {
        clues.push("邮箱语义".into());
        pattern = r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$".to_string();
    } else if lower.contains("url") || lower.contains("链接") || lower.contains("网址") {
        clues.push("URL 语义".into());
        pattern = r"^https?://[A-Za-z0-9.-]+(?:/[^\s]*)?$".to_string();
    } else if lower.contains("身份证") {
        clues.push("身份证语义（18 位，末位 X/x）".into());
        pattern = r"^\d{17}[\dXx]$".to_string();
    } else {
        // 通用：位数 + 字符集 + 锚定前缀
        // 锚定前缀：「以 X 开头」→ 提取 X 作为前缀
        let prefix = parse_prefix(&lower);
        if let Some(p) = &prefix {
            clues.push(format!("以 {} 开头", p));
        }

        // 字符集：「大写字母」/「小写字母」/「数字」/「字母」/「十六进制」
        // 返回 (字符类, label)。label=None 表示用户未明确说字符集，默认数字 \d。
        let (charclass, class_label) = parse_charset(&lower);

        // 位数：「N 位」/「N 个数字」/「长度 N」
        let count = parse_count(&lower);

        match (prefix.as_ref(), &count, class_label) {
            (Some(p), Some(c), Some(label)) => {
                clues.push(format!("字符集：{}", label));
                clues.push(format!("位数：{}", c));
                // 前缀已消耗 |p| 字符，剩余 c - |p| 位用 charclass
                let prefix_len = p.chars().count();
                let rest = c.saturating_sub(prefix_len);
                let rest_str = charclass.repeat(rest);
                pattern = format!("^{}{}$", p, rest_str);
            }
            (Some(p), Some(c), None) => {
                clues.push(format!("位数：{}", c));
                let prefix_len = p.chars().count();
                let rest = c.saturating_sub(prefix_len);
                pattern = format!("^{}\\d{{{}}}$", p, rest);
            }
            (None, Some(c), Some(label)) => {
                clues.push(format!("字符集：{}", label));
                clues.push(format!("位数：{}", c));
                pattern = format!("^{}{{{}}}$", charclass, c);
            }
            (None, Some(c), None) => {
                clues.push(format!("位数：{}", c));
                pattern = format!("^\\d{{{}}}$", c);
            }
            (Some(p), None, Some(label)) => {
                clues.push(format!("字符集：{}", label));
                pattern = format!("^{}{}+$", p, charclass);
            }
            (Some(p), None, None) => {
                pattern = format!("^{}.*$", p);
            }
            (None, None, Some(label)) => {
                clues.push(format!("字符集：{}", label));
                pattern = format!("^{}+$", charclass);
            }
            (None, None, None) => {
                return Err(CoreError::InvalidInput(
                    "无法从语句中识别线索，请补充：位数 / 字符集 / 锚定前缀 / 邮箱 / URL / 身份证".into(),
                ));
            }
        }
    }

    // 校验可编译
    Regex::new(&pattern).map_err(|e| CoreError::InvalidInput(format!("生成的正则无法编译: {}", e)))?;

    let explanation = format!("构造的正则：{}", pattern);
    Ok(ConstructedRegex {
        pattern,
        explanation,
        matched_clues: clues,
    })
}

/// 「以 X 开头」→ 提取 X（X 是紧邻「开头」前的连续非空白字符或引号内内容）。
/// 简化：取「以 1 开头」中的「1」、「以 abc 开头」中的「abc」。
fn parse_prefix(lower: &str) -> Option<String> {
    let re = Regex::new(r#"以\s*['"]?([^'"\s]+)['"]?\s*开头"#).unwrap();
    re.captures(lower)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

/// 字符集识别。返回 (正则字符类, 人类可读 label)。
/// label=None 表示用户未明确指定字符集，调用方按默认数字 \d 处理。
fn parse_charset(lower: &str) -> (&'static str, Option<&'static str>) {
    if lower.contains("大写字母") {
        ("[A-Z]", Some("大写字母 A-Z"))
    } else if lower.contains("小写字母") {
        ("[a-z]", Some("小写字母 a-z"))
    } else if lower.contains("字母") {
        ("[A-Za-z]", Some("字母 A-Za-z"))
    } else if lower.contains("十六进制") || lower.contains("hex") {
        ("[0-9a-fA-F]", Some("十六进制 0-9a-fA-F"))
    } else {
        // 默认数字，不算明确线索
        ("\\d", None)
    }
}

/// 位数识别：「N 位」/「N 位数字」/「N 个数字」/「长度 N」。
fn parse_count(lower: &str) -> Option<usize> {
    let re = Regex::new(r"(\d+)\s*(?:位|个)").unwrap();
    re.captures(lower)
        .and_then(|c| c.get(1).and_then(|m| m.as_str().parse::<usize>().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_11_digits() {
        let r = construct_regex("11 位手机号").unwrap();
        assert_eq!(r.pattern, r"^\d{11}$");
        assert!(r.matched_clues.iter().any(|c| c.contains("11")));
    }

    #[test]
    fn uppercase_8() {
        let r = construct_regex("大写字母 8 位").unwrap();
        assert_eq!(r.pattern, r"^[A-Z]{8}$");
        assert!(r.matched_clues.iter().any(|c| c.contains("大写")));
    }

    #[test]
    fn prefix_1_then_10() {
        let r = construct_regex("以 1 开头 11 位").unwrap();
        assert_eq!(r.pattern, r"^1\d{10}$");
    }

    #[test]
    fn email_semantic() {
        let r = construct_regex("邮箱").unwrap();
        assert!(r.pattern.contains("@"));
        assert!(r.matched_clues.iter().any(|c| c.contains("邮箱")));
    }

    #[test]
    fn url_semantic() {
        let r = construct_regex("http 链接").unwrap();
        assert!(r.pattern.starts_with("^https?"));
    }

    #[test]
    fn idcard_semantic() {
        let r = construct_regex("18 位身份证").unwrap();
        assert!(r.pattern.ends_with(r"[\dXx]$"));
    }

    #[test]
    fn lowercase_6() {
        let r = construct_regex("小写字母 6 位").unwrap();
        assert_eq!(r.pattern, r"^[a-z]{6}$");
    }

    #[test]
    fn hex_8() {
        let r = construct_regex("十六进制 8 位").unwrap();
        assert!(r.pattern.contains("0-9a-fA-F"));
    }

    #[test]
    fn negative_unrecognized() {
        assert!(construct_regex("今天天气不错").is_err());
    }

    #[test]
    fn pattern_compilable() {
        // 所有返回 Ok 的 pattern 必须可被 regex::Regex::new 编译
        let r = construct_regex("邮箱").unwrap();
        Regex::new(&r.pattern).unwrap();
    }
}
