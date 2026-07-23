//! v0.2.0 SQLi 签名引擎模块。
//!
//! 与 `rules` 模块保持解耦：本模块只做「对一条 [`crate::log::LogEntry`] 的
//! decoded query + path 跑签名正则匹配」，输出 [`SignatureHit`] 列表，
//! 不引用 `MaskOp` / `ValidateOp` / pipeline。
//!
//! 设计：
//! - 签名规则以 YAML 形式存于 [`sqli_signatures.yaml`]，编译期用 `include_str!`
//!   嵌入（见 [`loader`]）。
//! - [`SignatureEngine`] 在构造时一次性编译所有 `pattern` + `context_anchor`
//!   正则，避免在热路径重复编译。
//! - 匹配目标 = `path` + `parse_query(query)` 的所有 value 拼接（已双重 URL
//!   解码，故 pattern 一律按 decoded 形态编写，不写 `%23` / `%27`）。
//! - 408 超时行（method/path 为空）直接 early return，不产 finding，避免
//!   对 `-` 形态的 request line 误判（见 HANDOFF 提醒）。

pub mod blind;
pub mod loader;
pub mod payload_parser;

pub use blind::{
    extract_blind_probe, extract_blind_probe_with_line, looks_like_blind_probe, AggregatedResult,
    BlindAggregator, BlindProbe, PositionDetail, ProbeKind, ReconstructedDatabase,
    ReconstructedRow, ReconstructedTable,
};
pub use loader::{load_builtin_signatures, load_signatures_from_str, BUILTIN_YAML};
pub use payload_parser::{parse_payload, ParsedPayload};

use std::sync::Arc;

use regex::Regex;

use crate::error::CoreError;
use crate::log::{parse_query, LogEntry};

/// 一条签名规则（YAML 反序列化形态）。
///
/// 字段集合对应 HANDOFF acceptance_criteria；`context_anchor` 为可选的
/// 额外锚点正则列表，必须全部同时命中才算命中（抑制 `or` / `--` 等弱信号误报）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignatureRule {
    /// 唯一标识，如 `sqli_blind_binary`。
    pub rule_id: String,
    /// 命中类别，如 `blind_sql_injection`。
    pub category: String,
    /// 主正则（regex crate 兼容语法，禁用 look-around）。
    pub pattern: String,
    /// 是否启用。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 可选：额外锚点正则列表，必须全部同时命中才算命中。
    #[serde(default)]
    pub context_anchor: Vec<String>,
    /// 人类可读说明。
    #[serde(default)]
    pub description: String,
}

fn default_enabled() -> bool {
    true
}

/// 单条签名命中结果。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SignatureHit {
    /// 命中的 rule_id。
    pub rule_id: String,
    /// 命中的类别。
    pub category: String,
    /// 命中的日志行号。
    pub line_no: usize,
    /// 命中的文本片段（截取首段匹配，便于回引）。
    pub matched_value: String,
    /// 命中文本来源：`path` 或 `query`。
    pub context: String,
    /// 对命中文本段的语义解析（盲注二分/UNION 列数/报错目标等）；无匹配为 `None`。
    pub parsed_payload: Option<payload_parser::ParsedPayload>,
}

/// 编译后的规则内部形态：缓存 Regex 以免热路径重复编译。
struct CompiledRule {
    rule_id: String,
    category: String,
    pattern: Regex,
    anchors: Vec<Regex>,
    enabled: bool,
}

/// SQLi 签名引擎：对 [`LogEntry`] 跑签名匹配，返回 [`SignatureHit`] 列表。
pub struct SignatureEngine {
    rules: Vec<CompiledRule>,
}

impl SignatureEngine {
    /// 用默认内置 6 条 SQLi 签名构造引擎。
    pub fn new() -> Result<Self, CoreError> {
        Self::from_rules(load_builtin_signatures())
    }

    /// 用给定规则列表构造引擎，跳过 `enabled == false` 的规则。
    ///
    /// 正则编译失败返回 `Err`。`enabled == false` 的规则不参与扫描但保留在
    /// `rules` 中以便 GUI 编辑后重新装载（T2-5）。
    pub fn from_rules(rules: Vec<SignatureRule>) -> Result<Self, CoreError> {
        let mut compiled = Vec::with_capacity(rules.len());
        for r in rules {
            let pattern = Regex::new(&r.pattern).map_err(|e| {
                CoreError::Other(format!(
                    "compile signature pattern '{}' failed: {}",
                    r.rule_id, e
                ))
            })?;
            let mut anchors = Vec::with_capacity(r.context_anchor.len());
            for a in &r.context_anchor {
                anchors.push(Regex::new(a).map_err(|e| {
                    CoreError::Other(format!(
                        "compile context_anchor '{}' in rule '{}' failed: {}",
                        a, r.rule_id, e
                    ))
                })?);
            }
            compiled.push(CompiledRule {
                rule_id: r.rule_id,
                category: r.category,
                pattern,
                anchors,
                enabled: r.enabled,
            });
        }
        Ok(Self { rules: compiled })
    }

    /// 加载的规则总数（含禁用）。
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// 扫描单条 [`LogEntry`]，返回所有命中。
    ///
    /// - 跳过 `method.is_empty() || path.is_empty()`（408 超时行）。
    /// - 匹配目标：`path` + `parse_query(query)` 的所有 value（已双重 URL 解码）。
    /// - `context_anchor` 非空时要求每个 anchor 也在该段文本中命中才算命中。
    /// - 同一规则在同一文本段命中只产 1 个 `SignatureHit`（取首个匹配子串）。
    pub fn scan_log_entry(&self, entry: &LogEntry) -> Vec<SignatureHit> {
        if entry.method.is_empty() || entry.path.is_empty() {
            return Vec::new();
        }

        // 组装待匹配文本段：(context_label, text) 列表。
        let mut segments: Vec<(&str, String)> = Vec::new();
        segments.push(("path", entry.path.clone()));
        if let Some(q) = entry.query.as_deref() {
            for (_k, v) in parse_query(q) {
                if !v.is_empty() {
                    segments.push(("query", v));
                }
            }
        }

        let mut hits: Vec<SignatureHit> = Vec::new();
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            for (ctx, text) in &segments {
                if let Some(m) = rule.pattern.find(text) {
                    // 锚点必须全部命中。
                    if rule.anchors.iter().all(|a| a.is_match(text)) {
                        let matched_value = m.as_str().to_string();
                        // 对命中文本段做语义解析（6 类首命中），填入 parsed_payload。
                        let parsed_payload = parse_payload(text);
                        hits.push(SignatureHit {
                            rule_id: rule.rule_id.clone(),
                            category: rule.category.clone(),
                            line_no: entry.line_no,
                            matched_value,
                            context: (*ctx).to_string(),
                            parsed_payload,
                        });
                        break; // 同一规则同一段命中后跳下一段
                    }
                }
            }
        }
        hits
    }
}

impl Default for SignatureEngine {
    fn default() -> Self {
        Self::new().expect("builtin signatures must compile")
    }
}

// 避免未使用 Arc 警告（预留多线程共享用）。
#[allow(dead_code)]
type _SharedRule = Arc<SignatureRule>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogEntry;

    fn mk_entry(line_no: usize, method: &str, path: &str, query: Option<&str>) -> LogEntry {
        LogEntry {
            line_no,
            ip: "127.0.0.1".to_string(),
            timestamp: "10/Oct/2023:13:55:36 +0000".to_string(),
            method: method.to_string(),
            path: path.to_string(),
            query: query.map(|s| s.to_string()),
            status: 200,
            size: Some(100),
            user_agent: "curl/7.88.0".to_string(),
            raw: format!("{method} {path} {query:?}"),
            decoded_path: crate::log::url_decode_twice(path),
            decoded_query: query.map(|q| {
                let pairs = crate::log::parse_query(q);
                pairs
                    .iter()
                    .map(|(k, v)| {
                        if v.is_empty() {
                            k.clone()
                        } else {
                            format!("{k}={v}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("&")
            }),
            decoded_ua: crate::log::url_decode_twice("curl/7.88.0"),
        }
    }

    #[test]
    fn engine_loads_six_builtin_rules() {
        let eng = SignatureEngine::default();
        assert_eq!(eng.rule_count(), 6);
    }

    #[test]
    fn scan_skips_408_timeout_line() {
        // 408 超时行：method/path 为空。
        let e = mk_entry(18, "", "", None);
        let eng = SignatureEngine::default();
        assert!(eng.scan_log_entry(&e).is_empty());
    }

    #[test]
    fn scan_normal_request_no_hit() {
        let e = mk_entry(1, "GET", "/", None);
        let eng = SignatureEngine::default();
        assert!(eng.scan_log_entry(&e).is_empty());
    }

    #[test]
    fn scan_blind_binary_payload_hits() {
        // decoded 形态：`1' or ascii(substr((database()),1,1))>79#`
        // URL 编码后双重解码回到上面的形态。
        let q = "id=1%2527%20or%20ascii(substr((database())%2C1%2C1))%3E79%23";
        let e = mk_entry(2, "GET", "/search.php", Some(q));
        let eng = SignatureEngine::default();
        let hits = eng.scan_log_entry(&e);
        assert!(hits.iter().any(|h| h.rule_id == "sqli_blind_binary"),
            "blind_binary must hit, hits={:?}", hits);
    }

    #[test]
    fn scan_union_payload_hits() {
        let q = "id=1+union+select+1,2,3";
        let e = mk_entry(3, "GET", "/news.php", Some(q));
        let eng = SignatureEngine::default();
        let hits = eng.scan_log_entry(&e);
        assert!(hits.iter().any(|h| h.rule_id == "sqli_union"),
            "union must hit, hits={:?}", hits);
    }

    #[test]
    fn scan_error_based_payload_hits() {
        let q = "id=1+and+extractvalue(1,concat(0x7e,version()))";
        let e = mk_entry(4, "GET", "/news.php", Some(q));
        let eng = SignatureEngine::default();
        let hits = eng.scan_log_entry(&e);
        assert!(hits.iter().any(|h| h.rule_id == "sqli_error_based"),
            "error_based must hit, hits={:?}", hits);
    }

    #[test]
    fn scan_time_based_payload_hits() {
        let q = "id=1+and+sleep(5)";
        let e = mk_entry(5, "GET", "/news.php", Some(q));
        let eng = SignatureEngine::default();
        let hits = eng.scan_log_entry(&e);
        assert!(hits.iter().any(|h| h.rule_id == "sqli_time_based"),
            "time_based must hit, hits={:?}", hits);
    }

    #[test]
    fn scan_tautology_payload_hits() {
        // decoded 后：`1 or 1=1`（T2-1 只解码 %XX，不解 `+`，故用 %20）。
        let q = "id=1%20or%201=1";
        let e = mk_entry(6, "GET", "/news.php", Some(q));
        let eng = SignatureEngine::default();
        let hits = eng.scan_log_entry(&e);
        assert!(hits.iter().any(|h| h.rule_id == "sqli_tautology"),
            "tautology must hit, hits={:?}", hits);
    }

    #[test]
    fn scan_comment_payload_hits() {
        let q = "id=1'--";
        let e = mk_entry(7, "GET", "/news.php", Some(q));
        let eng = SignatureEngine::default();
        let hits = eng.scan_log_entry(&e);
        assert!(hits.iter().any(|h| h.rule_id == "sqli_comment"),
            "comment must hit, hits={:?}", hits);
    }

    // ---- 负例（正常请求 / 含弱词但不构成注入） ----

    #[test]
    fn normal_home_request_no_fp() {
        let e = mk_entry(1, "GET", "/", None);
        let eng = SignatureEngine::default();
        assert!(eng.scan_log_entry(&e).is_empty());
    }

    #[test]
    fn normal_search_query_no_fp() {
        let e = mk_entry(2, "GET", "/search", Some("q=hello+world&page=1"));
        let eng = SignatureEngine::default();
        assert!(eng.scan_log_entry(&e).is_empty());
    }

    #[test]
    fn normal_order_by_no_fp() {
        // `order` 含 `or`，但不构成 or 数字=数字 模式。
        let e = mk_entry(3, "GET", "/list", Some("sort=order&dir=asc"));
        let eng = SignatureEngine::default();
        assert!(eng.scan_log_entry(&e).is_empty());
    }

    #[test]
    fn normal_path_with_hash_no_fp() {
        // 不含引号上下文，注释类不命中。
        let e = mk_entry(4, "GET", "/page.html", None);
        let eng = SignatureEngine::default();
        assert!(eng.scan_log_entry(&e).is_empty());
    }

    #[test]
    fn normal_select_keyword_no_fp() {
        // 含 select 但无 union，不命中 union。
        let e = mk_entry(5, "GET", "/docs", Some("kind=select-all"));
        let eng = SignatureEngine::default();
        assert!(eng.scan_log_entry(&e).is_empty());
    }

    #[test]
    fn normal_tautology_text_no_fp() {
        // `or 1` 单独不成 `or 数字=数字`（T2-1 不解 +，故用 %20）。
        let e = mk_entry(6, "GET", "/p", Some("x=or%201"));
        let eng = SignatureEngine::default();
        assert!(eng.scan_log_entry(&e).is_empty());
    }
}
