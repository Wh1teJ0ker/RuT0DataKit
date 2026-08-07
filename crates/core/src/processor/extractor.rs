//! 提取器 trait + PII 原型提取器。
//!
//! v1.1.0：`Extractor` trait + `PiiExtractor`，内置 3 个正则（手机号/邮箱/身份证号）。
//! 不依赖规则，让提取面板无需规则即可演示。

use regex::Regex;

use crate::error::{CoreError, CoreResult};

/// 单个提取命中。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractItem {
    /// PII 类型（`"phone"` / `"email"` / `"idcard"`）。
    pub kind: String,
    /// 命中原文。
    pub value: String,
    /// 起始字节偏移。
    pub start: usize,
    /// 结束字节偏移。
    pub end: usize,
}

/// 提取器 trait。纯逻辑，不持有状态。
pub trait Extractor {
    /// 从单值中提取 PII，返回命中列表。
    fn extract(&self, input: &str) -> CoreResult<Vec<ExtractItem>>;
}

/// PII 原型提取器。内置 3 个正则。
pub struct PiiExtractor {
    phone_re: Regex,
    email_re: Regex,
    idcard_re: Regex,
}

impl PiiExtractor {
    pub fn new() -> CoreResult<Self> {
        Ok(Self {
            phone_re: Regex::new(r"1[3-9]\d{9}").map_err(invalid_re)?,
            email_re: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
                .map_err(invalid_re)?,
            idcard_re: Regex::new(r"[1-9]\d{16}[0-9Xx]").map_err(invalid_re)?,
        })
    }
}

fn invalid_re(e: regex::Error) -> CoreError {
    CoreError::Processor(format!("invalid pii regex: {e}"))
}

/// 默认实现。内置正则静态已知可编译；若仍失败则回退到运行期重建失败的
/// 空提取器（`extract` 始终返回空），绝不 panic。
impl Default for PiiExtractor {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::empty())
    }
}

impl PiiExtractor {
    /// 构造一个所有正则都匹配空串的退化实例，用于 `Default` 兜底。
    fn empty() -> Self {
        Self {
            phone_re: Regex::new(r"$^").unwrap_or_else(|_| Regex::new(r"").unwrap()),
            email_re: Regex::new(r"$^").unwrap_or_else(|_| Regex::new(r"").unwrap()),
            idcard_re: Regex::new(r"$^").unwrap_or_else(|_| Regex::new(r"").unwrap()),
        }
    }
}

impl Extractor for PiiExtractor {
    fn extract(&self, input: &str) -> CoreResult<Vec<ExtractItem>> {
        let mut out = Vec::new();
        for m in self.phone_re.find_iter(input) {
            out.push(ExtractItem {
                kind: "phone".into(),
                value: m.as_str().to_string(),
                start: m.start(),
                end: m.end(),
            });
        }
        for m in self.email_re.find_iter(input) {
            out.push(ExtractItem {
                kind: "email".into(),
                value: m.as_str().to_string(),
                start: m.start(),
                end: m.end(),
            });
        }
        for m in self.idcard_re.find_iter(input) {
            out.push(ExtractItem {
                kind: "idcard".into(),
                value: m.as_str().to_string(),
                start: m.start(),
                end: m.end(),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_phone_and_email() {
        let e = PiiExtractor::new().unwrap();
        let hits = e.extract("电话13812345678邮箱a@b.com").unwrap();
        let kinds: Vec<&str> = hits.iter().map(|h| h.kind.as_str()).collect();
        assert!(kinds.contains(&"phone"));
        assert!(kinds.contains(&"email"));
        let phone = hits.iter().find(|h| h.kind == "phone").unwrap();
        assert_eq!(phone.value, "13812345678");
        let email = hits.iter().find(|h| h.kind == "email").unwrap();
        assert_eq!(email.value, "a@b.com");
    }

    #[test]
    fn extracts_idcard() {
        let e = PiiExtractor::new().unwrap();
        let hits = e.extract("身份证110101199005151234").unwrap();
        let idcard = hits.iter().find(|h| h.kind == "idcard").unwrap();
        assert_eq!(idcard.value, "110101199005151234");
    }

    #[test]
    fn extracts_all_three_from_address_field() {
        let e = PiiExtractor::new().unwrap();
        let input = "联系人13812345678邮箱zhangsan@example.com身份证44030419920720123X";
        let hits = e.extract(input).unwrap();
        let kinds: Vec<&str> = hits.iter().map(|h| h.kind.as_str()).collect();
        assert!(kinds.contains(&"phone"));
        assert!(kinds.contains(&"email"));
        assert!(kinds.contains(&"idcard"));
    }

    #[test]
    fn no_hits_returns_empty() {
        let e = PiiExtractor::new().unwrap();
        let hits = e.extract("hello world").unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn empty_input_returns_empty() {
        let e = PiiExtractor::new().unwrap();
        assert!(e.extract("").unwrap().is_empty());
    }
}
