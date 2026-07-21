//! 正则 token 解释器。
//!
//! 输入一条正则字符串，输出每个 token 的 `RegexTokenDesc`（token 原文 / kind /
//! description / 起始位置），供 GUI 展示「这段正则是什么意思」。
//!
//! 设计取舍：
//! - **不**自己实现正则引擎，**不**追求与 `regex` crate 内部 AST 对齐。
//! - 用 `regex::Regex::new` 做合法性校验：非法正则直接返回 `CoreError`，
//!   不 panic。
//! - 用手写逐字符扫描 + 状态机解析常见 token：字面量、字符类、量词、锚点、
//!   分组、反向引用、转义。
//! - 不支持 `regex` crate 本身不支持的 PCRE 专属语法（look-around
//!   `(?=...)` / `(?!...)` / `(?<=...)` / `(?<!...)`），遇到时标 `unsupported`
//!   并给出对应描述，不报错（仍由 `regex::Regex::new` 兜底判定非法）。

use crate::error::CoreError;

/// 单个正则 token 的解释描述。
///
/// `kind` 取值固定为以下之一（小写蛇形字符串，便于前端枚举匹配）：
/// `literal` / `char_class` / `quantifier` / `anchor` / `group` /
/// `backref` / `assertion` / `escape` / `unsupported`。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RegexTokenDesc {
    /// token 原文（如 `[a-z]`、`\d`、`{9}`、`(?P<name>...)`）。
    pub token: String,
    /// token 类别，见模块文档。
    pub kind: String,
    /// 人类可读说明（中文）。
    pub description: String,
    /// token 在原始 pattern 中的起始字节偏移。
    pub position: usize,
}

/// 解释一条正则字符串。
///
/// 流程：
/// 1. 先用 `regex::Regex::new(pattern)` 校验合法性，失败则返回 `CoreError`。
/// 2. 逐字符扫描，按状态机产出 token 描述。
/// 3. 量词的 lazy / possessive 后缀（`*?` / `++` 等）合并进同一条量词 token
///    的 description（保持「一个量词 = 一条描述」）。
pub fn explain_regex(pattern: &str) -> Result<Vec<RegexTokenDesc>, CoreError> {
    // 先校验合法性：regex crate 不支持的语法会在此报错。
    // 注意：反向引用（`\1`）是合法 PCRE 语法，但 `regex` crate 引擎不支持，
    // 会报 "backreferences are not supported"。这类情况仍按 HANDOFF 要求解释
    // 成 backref token，因此这里只在错误**不是**反向引用类时返回错误。
    if let Err(e) = regex::Regex::new(pattern) {
        let msg = e.to_string();
        if !msg.contains("backreferences are not supported") {
            return Err(CoreError::InvalidInput(format!("invalid regex: {msg}")));
        }
    }

    let bytes = pattern.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let c = bytes[i] as char;
        let token = match c {
            '\\' => parse_escape(pattern, &mut i)?,
            '[' => parse_char_class(pattern, &mut i),
            '(' => parse_group(pattern, &mut i),
            '^' | '$' => {
                let desc = if c == '^' {
                    "锚点：匹配字符串开头".to_string()
                } else {
                    "锚点：匹配字符串结尾".to_string()
                };
                i += 1;
                RegexTokenDesc {
                    token: c.to_string(),
                    kind: "anchor".into(),
                    description: desc,
                    position: start,
                }
            }
            '*' | '+' | '?' => parse_quantifier_simple(pattern, &mut i, start),
            '{' => parse_quantifier_brace(pattern, &mut i, start),
            '.' => {
                i += 1;
                RegexTokenDesc {
                    token: ".".into(),
                    kind: "char_class".into(),
                    description: "任意单字符（不含换行）".into(),
                    position: start,
                }
            }
            ')' | ']' | '}' => {
                // 合法正则里这些字符只可能作为转义字面量出现（已被 escape
                // 分支处理），这里按字面量收尾。
                i += 1;
                RegexTokenDesc {
                    token: c.to_string(),
                    kind: "literal".into(),
                    description: format!("字面量字符 {c:?}"),
                    position: start,
                }
            }
            _ => {
                i += 1;
                RegexTokenDesc {
                    token: c.to_string(),
                    kind: "literal".into(),
                    description: format!("字面量字符 {c:?}"),
                    position: start,
                }
            }
        };
        out.push(token);
    }
    Ok(out)
}

/// 解析 `\X` 形式的转义 / 预定义字符类 / 反向引用。
fn parse_escape(pattern: &str, i: &mut usize) -> Result<RegexTokenDesc, CoreError> {
    let start = *i;
    let bytes = pattern.as_bytes();
    // 至少要有 `\` + 一个字符。
    if *i + 1 >= bytes.len() {
        return Err(CoreError::InvalidInput(
            "invalid regex: trailing backslash".into(),
        ));
    }
    let next = bytes[*i + 1] as char;

    // 反向引用 \1 ~ \9（仅单个数字）
    if next.is_ascii_digit() {
        *i += 2;
        return Ok(RegexTokenDesc {
            token: pattern[start..*i].to_string(),
            kind: "backref".into(),
            description: format!("反向引用第 {next} 个捕获组"),
            position: start,
        });
    }

    // 预定义字符类 / 锚点 / 转义
    let (kind, desc) = match next {
        'd' => ("char_class", "数字字符 [0-9]"),
        'D' => ("char_class", "非数字字符 [^0-9]"),
        'w' => ("char_class", "单词字符 [A-Za-z0-9_]"),
        'W' => ("char_class", "非单词字符"),
        's' => ("char_class", "空白字符"),
        'S' => ("char_class", "非空白字符"),
        'b' => ("assertion", "单词边界 \\b"),
        'B' => ("assertion", "非单词边界 \\B"),
        'A' => ("assertion", "字符串起始锚点 \\A"),
        'z' => ("assertion", "字符串末尾锚点 \\z"),
        'x' => return parse_hex_escape(pattern, i, start),
        'u' => return parse_unicode_escape(pattern, i, start),
        _ => ("escape", "转义字符（按字面量匹配）"),
    };
    *i += 2;
    Ok(RegexTokenDesc {
        token: pattern[start..*i].to_string(),
        kind: kind.into(),
        description: desc.to_string(),
        position: start,
    })
}

/// 解析 `\xHH` 或 `\x{HHHH}`。
fn parse_hex_escape(
    pattern: &str,
    i: &mut usize,
    start: usize,
) -> Result<RegexTokenDesc, CoreError> {
    let bytes = pattern.as_bytes();
    *i += 2; // 吃掉 `\x`
    if *i < bytes.len() && bytes[*i] as char == '{' {
        while *i < bytes.len() && bytes[*i] as char != '}' {
            *i += 1;
        }
        if *i < bytes.len() {
            *i += 1; // 吃掉 `}`
        }
    } else if *i + 2 <= bytes.len() {
        *i += 2;
    }
    Ok(RegexTokenDesc {
        token: pattern[start..*i].to_string(),
        kind: "escape".into(),
        description: "十六进制转义字符".into(),
        position: start,
    })
}

/// 解析 `\uHHHH` 或 `\u{HHHH}`。
fn parse_unicode_escape(
    pattern: &str,
    i: &mut usize,
    start: usize,
) -> Result<RegexTokenDesc, CoreError> {
    let bytes = pattern.as_bytes();
    *i += 2; // 吃掉 `\u`
    if *i < bytes.len() && bytes[*i] as char == '{' {
        while *i < bytes.len() && bytes[*i] as char != '}' {
            *i += 1;
        }
        if *i < bytes.len() {
            *i += 1;
        }
    } else if *i + 4 <= bytes.len() {
        *i += 4;
    }
    Ok(RegexTokenDesc {
        token: pattern[start..*i].to_string(),
        kind: "escape".into(),
        description: "Unicode 转义字符".into(),
        position: start,
    })
}

/// 解析字符类 `[...]`（含取反、范围、内置类 `\d\w\s` 等）。
fn parse_char_class(pattern: &str, i: &mut usize) -> RegexTokenDesc {
    let start = *i;
    let bytes = pattern.as_bytes();
    *i += 1; // 吃掉 `[`
    let mut negated = false;
    if *i < bytes.len() && bytes[*i] as char == '^' {
        negated = true;
        *i += 1;
    }
    // 第一个 `]` 视为字面量
    if *i < bytes.len() && bytes[*i] as char == ']' {
        *i += 1;
    }
    // 扫描到匹配的 `]`，处理嵌套 `[` 与转义。
    let mut depth = 1;
    while *i < bytes.len() && depth > 0 {
        let c = bytes[*i] as char;
        if c == '\\' {
            *i += 2;
            continue;
        }
        if c == '[' {
            depth += 1;
        } else if c == ']' {
            depth -= 1;
            if depth == 0 {
                *i += 1;
                break;
            }
        }
        *i += 1;
    }
    let token = pattern[start..*i].to_string();
    let desc = if negated {
        "取反字符类：匹配括号内未列出的任意字符".to_string()
    } else {
        "字符类：匹配括号内列出的任意一个字符".to_string()
    };
    RegexTokenDesc {
        token,
        kind: "char_class".into(),
        description: desc,
        position: start,
    }
}

/// 解析分组 `(...)`，包括 `(?:...)` / `(?P<name>...)` / `(?i)` 标志 / look-around。
fn parse_group(pattern: &str, i: &mut usize) -> RegexTokenDesc {
    let start = *i;
    let bytes = pattern.as_bytes();
    *i += 1; // 吃掉 `(`
    let mut kind = "group";
    let mut desc = "捕获分组".to_string();

    if *i < bytes.len() && bytes[*i] as char == '?' {
        *i += 1;
        if *i >= bytes.len() {
            return RegexTokenDesc {
                token: pattern[start..*i].to_string(),
                kind: "unsupported".into(),
                description: "未结束的分组前缀".into(),
                position: start,
            };
        }
        let c = bytes[*i] as char;
        match c {
            ':' => {
                *i += 1;
                kind = "group";
                desc = "非捕获分组 (?:...)".into();
            }
            'P' => {
                if *i + 1 < bytes.len() && bytes[*i + 1] as char == '<' {
                    *i += 2;
                    while *i < bytes.len() && bytes[*i] as char != '>' {
                        *i += 1;
                    }
                    if *i < bytes.len() {
                        *i += 1; // 吃 `>`
                    }
                    kind = "group";
                    desc = "命名捕获组 (?P<name>...)".into();
                } else {
                    kind = "unsupported";
                    desc = "未识别的 (?P 形式分组".into();
                    *i += 1;
                }
            }
            '<' => {
                // (?<=...) / (?<!...) look-behind — regex crate 不支持
                *i += 1;
                if *i < bytes.len() && (bytes[*i] as char == '=' || bytes[*i] as char == '!') {
                    *i += 1;
                    kind = "unsupported";
                    desc = "look-behind 断言（regex crate 不支持）".into();
                } else {
                    // (?<name>...) 命名组
                    while *i < bytes.len() && bytes[*i] as char != '>' {
                        *i += 1;
                    }
                    if *i < bytes.len() {
                        *i += 1;
                    }
                    kind = "group";
                    desc = "命名捕获组 (?<name>...)".into();
                }
            }
            '=' | '!' => {
                // (?=...) / (?!...) look-ahead — regex crate 不支持
                *i += 1;
                kind = "unsupported";
                desc = "look-ahead 断言（regex crate 不支持）".into();
            }
            _ => {
                // (?i) / (?m) / (?s) 等内联标志，可能带 `:` 体
                while *i < bytes.len() && bytes[*i] as char != ')' && bytes[*i] as char != ':' {
                    *i += 1;
                }
                if *i < bytes.len() && bytes[*i] as char == ':' {
                    *i += 1;
                    kind = "group";
                    desc = "带内联标志的非捕获分组".into();
                } else {
                    if *i < bytes.len() {
                        *i += 1; // 吃 `)`
                    }
                    return RegexTokenDesc {
                        token: pattern[start..*i].to_string(),
                        kind: "assertion".into(),
                        description: "内联标志开关".into(),
                        position: start,
                    };
                }
            }
        }
    }

    // 扫描到匹配的 `)`，处理嵌套分组与字符类。
    let mut depth = 1;
    while *i < bytes.len() && depth > 0 {
        let c = bytes[*i] as char;
        if c == '\\' {
            *i += 2;
            continue;
        }
        if c == '[' {
            *i += 1;
            while *i < bytes.len() {
                let cc = bytes[*i] as char;
                if cc == '\\' {
                    *i += 2;
                    continue;
                }
                if cc == ']' {
                    *i += 1;
                    break;
                }
                *i += 1;
            }
            continue;
        }
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                *i += 1;
                break;
            }
        }
        *i += 1;
    }

    RegexTokenDesc {
        token: pattern[start..*i].to_string(),
        kind: kind.into(),
        description: desc,
        position: start,
    }
}

/// 解析单字符量词 `*` / `+` / `?`，合并 lazy（`?`）/ possessive（`+`）后缀。
fn parse_quantifier_simple(pattern: &str, i: &mut usize, start: usize) -> RegexTokenDesc {
    let bytes = pattern.as_bytes();
    let c = bytes[start] as char;
    *i += 1;
    let mut suffix = "";
    if *i < bytes.len() {
        let n = bytes[*i] as char;
        if n == '?' {
            suffix = "? (lazy 非贪婪)";
            *i += 1;
        } else if n == '+' {
            suffix = "+ (possessive)";
            *i += 1;
        }
    }
    let base = match c {
        '*' => "量词 *：匹配前一项 0 次或多次",
        '+' => "量词 +：匹配前一项 1 次或多次",
        '?' => "量词 ?：匹配前一项 0 次或 1 次",
        _ => "量词",
    };
    let desc = if suffix.is_empty() {
        base.to_string()
    } else {
        format!("{base}，后缀 {suffix}")
    };
    RegexTokenDesc {
        token: pattern[start..*i].to_string(),
        kind: "quantifier".into(),
        description: desc,
        position: start,
    }
}

/// 解析 `{n}` / `{n,}` / `{n,m}` 量词，合并 lazy / possessive 后缀。
fn parse_quantifier_brace(pattern: &str, i: &mut usize, start: usize) -> RegexTokenDesc {
    let bytes = pattern.as_bytes();
    // 扫描到匹配的 `}`
    while *i < bytes.len() && bytes[*i] as char != '}' {
        *i += 1;
    }
    if *i < bytes.len() {
        *i += 1; // 吃 `}`
    }
    // 合并 lazy / possessive 后缀
    if *i < bytes.len() {
        let n = bytes[*i] as char;
        if n == '?' || n == '+' {
            *i += 1;
        }
    }
    RegexTokenDesc {
        token: pattern[start..*i].to_string(),
        kind: "quantifier".into(),
        description: "区间量词 {n,m}：限定前一项的重复次数".into(),
        position: start,
    }
}

// ─────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(p: &str) -> Vec<String> {
        explain_regex(p).unwrap().into_iter().map(|t| t.kind).collect()
    }

    fn tokens(p: &str) -> Vec<String> {
        explain_regex(p).unwrap().into_iter().map(|t| t.token).collect()
    }

    /// 验收口径：`^1[3-9]\d{9}$`
    #[test]
    fn test_phone_cn_explain() {
        let t = tokens("^1[3-9]\\d{9}$");
        assert_eq!(
            t,
            vec!["^", "1", "[3-9]", "\\d", "{9}", "$"]
                .into_iter().map(String::from).collect::<Vec<_>>()
        );
        let k = kinds("^1[3-9]\\d{9}$");
        assert_eq!(
            k,
            vec![
                "anchor", "literal", "char_class", "char_class", "quantifier", "anchor"
            ]
        );
    }

    #[test]
    fn test_literal_chars() {
        let t = explain_regex("abc").unwrap();
        assert_eq!(t.len(), 3);
        assert!(t.iter().all(|x| x.kind == "literal"));
        assert_eq!(t[0].token, "a");
        assert_eq!(t[0].position, 0);
        assert_eq!(t[2].position, 2);
    }

    #[test]
    fn test_char_classes() {
        let k = kinds("[a-z]");
        assert_eq!(k, vec!["char_class"]);
        let k = kinds("[^0-9]");
        assert_eq!(k, vec!["char_class"]);
        // 含内置类
        let t = explain_regex("[\\d\\w]").unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].kind, "char_class");
    }

    #[test]
    fn test_predefined_classes() {
        let k = kinds("\\d\\w\\s");
        assert_eq!(k, vec!["char_class", "char_class", "char_class"]);
        let t = explain_regex("\\D").unwrap();
        assert_eq!(t[0].kind, "char_class");
    }

    #[test]
    fn test_quantifiers_simple() {
        let t = explain_regex("a*b+c?").unwrap();
        let kinds_v: Vec<_> = t.iter().map(|x| x.kind.clone()).collect();
        assert_eq!(
            kinds_v,
            vec!["literal", "quantifier", "literal", "quantifier", "literal", "quantifier"]
        );
        let tokens_v: Vec<_> = t.iter().map(|x| x.token.clone()).collect();
        assert_eq!(tokens_v, vec!["a", "*", "b", "+", "c", "?"]);
    }

    #[test]
    fn test_quantifier_lazy_suffix() {
        let t = explain_regex("a*?").unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[1].token, "*?");
        assert!(t[1].description.contains("lazy"));
    }

    #[test]
    fn test_brace_quantifier() {
        let t = explain_regex("a{3,5}").unwrap();
        assert_eq!(t[1].token, "{3,5}");
        assert_eq!(t[1].kind, "quantifier");
        let t = explain_regex("a{9}").unwrap();
        assert_eq!(t[1].token, "{9}");
    }

    #[test]
    fn test_anchors() {
        let k = kinds("^abc$");
        assert_eq!(k[0], "anchor");
        assert_eq!(k[4], "anchor");
        let t = explain_regex("\\b").unwrap();
        assert_eq!(t[0].kind, "assertion");
        assert_eq!(t[0].token, "\\b");
    }

    #[test]
    fn test_groups() {
        let t = explain_regex("(abc)").unwrap();
        assert_eq!(t[0].kind, "group");
        assert!(t[0].token.starts_with('('));
        let t = explain_regex("(?:abc)").unwrap();
        assert_eq!(t[0].kind, "group");
        assert!(t[0].description.contains("非捕获"));
        let t = explain_regex("(?P<name>abc)").unwrap();
        assert_eq!(t[0].kind, "group");
        assert!(t[0].description.contains("命名"));
    }

    #[test]
    fn test_backref() {
        let t = explain_regex("(a)\\1").unwrap();
        // group / literal / backref
        assert_eq!(t[0].kind, "group");
        let back = t.iter().find(|x| x.kind == "backref").unwrap();
        assert_eq!(back.token, "\\1");
    }

    #[test]
    fn test_escape() {
        let t = explain_regex("\\.\\\\").unwrap();
        assert_eq!(t[0].kind, "escape");
        assert_eq!(t[0].token, "\\.");
        assert_eq!(t[1].token, "\\\\");
    }

    #[test]
    fn test_dot() {
        let t = explain_regex("a.c").unwrap();
        assert_eq!(t[1].kind, "char_class");
        assert_eq!(t[1].token, ".");
    }

    #[test]
    fn test_invalid_regex_returns_error() {
        // 未闭合字符类
        let r = explain_regex("[abc");
        assert!(r.is_err());
        // 未闭合分组
        let r = explain_regex("(abc");
        assert!(r.is_err());
        // 末尾反斜杠
        let r = explain_regex("abc\\");
        assert!(r.is_err());
    }

    #[test]
    fn test_unsupported_lookahead() {
        // regex crate 不支持 look-around，整条应被判为非法。
        let r = explain_regex("(?=abc)");
        assert!(r.is_err());
    }

    #[test]
    fn test_position_tracking() {
        let t = explain_regex("abc").unwrap();
        assert_eq!(t[0].position, 0);
        assert_eq!(t[1].position, 1);
        assert_eq!(t[2].position, 2);
    }

    #[test]
    fn test_complex_email() {
        let p = r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$";
        let t = explain_regex(p).unwrap();
        // 应包含 anchor ^ / anchor $ / 量词 {2,}
        assert_eq!(t[0].kind, "anchor");
        assert_eq!(t.last().unwrap().kind, "anchor");
        assert!(t.iter().any(|x| x.token == "{2,}"));
        assert!(t.iter().any(|x| x.kind == "char_class" && x.token == "[A-Za-z0-9._%+-]"));
    }
}
