//! HTTP 访问日志解析模块（CLF / Nginx Combined）。
//!
//! v0.2.0 新增。把一行 CLF/Nginx Combined 访问日志解析为 [`LogEntry`]，
//! 不依赖 chrono（`timestamp` 保留原始字符串，避免引入时序依赖）。
//!
//! 设计要点：
//! - `LogEntry` 字段一旦确定即冻结，下游 T2-2/T2-3/T2-5 都依赖此结构。
//! - `query` 保留为 `Option<String>` 原始串；辅助函数 [`parse_query`] 把它
//!   拆成 `Vec<(String, String)>`，key/value 均按 form-urlencoded 语义
//!   先把 `+` 解为空格，再双重 `%XX` 解码。
//! - [`url_decode_twice`] 手写实现（零新依赖）：先解一遍 `%XX`，再解一遍；
//!   非法 `%` 原样保留，不 panic。
//! - [`LogEntry::decoded_path`] / [`LogEntry::decoded_query`] /
//!   [`LogEntry::decoded_ua`] 在 [`parse_line_with_re`] 末尾一次性填充，
//!   为下游 payload_parser 提供「已解码完整文本」输入，避免重复解码。
//! - [`LogReader::read`] 读整个文件，逐行用 [`parse_line`] 解析；任一行解析
//!   失败即返回 `Err(CoreError::InvalidInput)`，调用方可据此判断文件是否
//!   全部为合法 CLF/Nginx Combined 格式。

use std::path::Path;

use regex::Regex;

use crate::error::CoreError;

/// 一行访问日志解析结果。
///
/// 字段集合对应 HANDOFF acceptance_criteria，下游 T2-2/T2-3/T2-5 依赖。
/// `line_no` 从 1 开始；`raw` 保留原始行字符串，便于 finding.context 回引。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LogEntry {
    /// 行号（从 1 开始）。
    pub line_no: usize,
    /// 客户端 IP（原始字符串，不做 IP 类型转换）。
    pub ip: String,
    /// 时间戳原始字符串（如 `17/Nov/2023:03:44:21 +0000`）。
    pub timestamp: String,
    /// HTTP 方法（GET/POST/...）。
    pub method: String,
    /// 请求路径，不含 query。
    pub path: String,
    /// 原始 query 串（`?` 之后的部分），无 query 时为 `None`。
    pub query: Option<String>,
    /// HTTP 状态码。
    pub status: u16,
    /// 响应大小（字节），CLF 用 `-` 表示无 → `None`。
    pub size: Option<u64>,
    /// User-Agent 原始字符串。
    pub user_agent: String,
    /// 原始行字符串，便于回引。
    pub raw: String,
    /// path 双重 `%XX` 解码结果（不解 `+`，path 段无 `+`=空格语义）。
    /// 由 `parse_line_with_re` 末尾填充，供下游 payload_parser 直接使用。
    pub decoded_path: String,
    /// query 经 `parse_query` 解码后拼回的 `k=v&k2=v2` 完整串（已 `+`→space
    /// 与双重 `%XX` 解码）；无 query 时为 `None`。供下游 payload_parser 直接使用。
    pub decoded_query: Option<String>,
    /// UA 双重 `%XX` 解码结果（不解 `+`）。由 `parse_line_with_re` 末尾填充。
    pub decoded_ua: String,
}

/// 日志读取器：把一个文件读成 `Vec<LogEntry>`。
///
/// 复用 `SourceReader` 思路但不实现该 trait：`LogEntry` 不是表格形态，
/// 与 `Records` 不同构，单独提供读取入口。
pub struct LogReader {
    line_re: Regex,
}

impl LogReader {
    /// 构造一个默认读取器（编译一次 CLF/Nginx Combined 正则）。
    pub fn new() -> Result<Self, CoreError> {
        // request line 形态：`METHOD URI PROTO`（标准）或单 `-`（某些服务器
        // 在连接异常/超时如 408 时记录 `"-"`）。后者 method/path/query/proto
        // 全部置空，status/size 仍解析。
        let line_re = Regex::new(
            r#"^(?P<ip>\S+) \S+ \S+ \[(?P<ts>[^\]]+)\] "(?:(?P<method>[A-Z]+) (?P<uri>\S+) (?P<proto>[^"]+)|(?P<dash>-))" (?P<status>\d{3}) (?P<size>\S+)(?: "(?P<ref>[^"]*)")?(?: "(?P<ua>[^"]*)")?"#,
        )
        .map_err(|e| CoreError::Other(format!("compile log regex failed: {e}")))?;
        Ok(Self { line_re })
    }

    /// 读取 `path` 指向的日志文件，返回所有行的 `LogEntry`。
    ///
    /// 任一行无法解析即返回 `Err`，调用方可据此判断整文件是否合法。
    pub fn read(&self, path: &Path) -> Result<Vec<LogEntry>, CoreError> {
        let content = std::fs::read_to_string(path)?;
        self.parse(&content)
    }

    /// 解析多行文本为 `Vec<LogEntry>`，逐行调用 [`parse_line`]。
    pub fn parse(&self, content: &str) -> Result<Vec<LogEntry>, CoreError> {
        let mut out = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let entry = self.parse_line(line, i + 1)?;
            out.push(entry);
        }
        Ok(out)
    }

    /// 解析单行（CLF/Nginx Combined），`line_no` 由调用方传入。
    pub fn parse_line(&self, line: &str, line_no: usize) -> Result<LogEntry, CoreError> {
        parse_line_with_re(&self.line_re, line, line_no)
    }
}

impl Default for LogReader {
    fn default() -> Self {
        Self::new().expect("LogReader regex must compile")
    }
}

/// 用给定正则解析单行为 [`LogEntry`]。
///
/// 格式不匹配返回 [`CoreError::InvalidInput`]。
fn parse_line_with_re(re: &Regex, line: &str, line_no: usize) -> Result<LogEntry, CoreError> {
    let caps = re
        .captures(line)
        .ok_or_else(|| CoreError::InvalidInput(format!("log line {line_no} unmatched: {line}")))?;

    let ip = caps.name("ip").map(|m| m.as_str()).unwrap_or("").to_string();
    let timestamp = caps
        .name("ts")
        .map(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    // method/uri 命中表示标准 request line；命中 `dash` 表示 request line
    // 为单 "-"（连接异常 408 等），此时 method/path/query 留空。
    let method = caps
        .name("method")
        .map(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    let uri = caps.name("uri").map(|m| m.as_str()).unwrap_or("");
    let status: u16 = caps
        .name("status")
        .map(|m| m.as_str())
        .unwrap_or("0")
        .parse()
        .map_err(|_| {
            CoreError::InvalidInput(format!("log line {line_no} bad status: {line}"))
        })?;
    let size_raw = caps.name("size").map(|m| m.as_str()).unwrap_or("-");
    let size = if size_raw == "-" {
        None
    } else {
        Some(size_raw.parse::<u64>().map_err(|_| {
            CoreError::InvalidInput(format!("log line {line_no} bad size: {line}"))
        })?)
    };
    let user_agent = caps
        .name("ua")
        .map(|m| m.as_str())
        .unwrap_or("")
        .to_string();

    // uri 拆 path + query，query 原样保留（不做 URL 解码，仅 parse_query 时解码）。
    // request line 为 "-" 时 uri 为空，path 留空串、query 为 None。
    let (path, query) = split_path_query(uri);

    // 预解码 3 字段，为下游 payload_parser 提供「已解码完整文本」输入。
    // path / UA 不解 `+`（path 段无 `+`=空格语义）；query 走 parse_query
    // （内部已 `+`→space + 双重 `%XX`），拼回 `k=v&k2=v2`。
    let decoded_path = url_decode_twice(&path);
    let decoded_ua = url_decode_twice(&user_agent);
    let decoded_query = query.as_deref().map(|q| join_decoded_query(&parse_query(q)));

    Ok(LogEntry {
        line_no,
        ip,
        timestamp,
        method,
        path,
        query,
        status,
        size,
        user_agent,
        raw: line.to_string(),
        decoded_path,
        decoded_query,
        decoded_ua,
    })
}

/// 把 `parse_query` 输出的 `(key, value)` 列表拼回 `k=v&k2=v2` 形态。
///
/// 按 HANDOFF 约定：value 为空时只输出 `key`（不带 `=`，对应原 query 中
/// 无 `=` 的裸 key，如 `foo`）；value 非空输出 `key=value`。
///
/// 注意：调用方已确保传入的 pairs 经 `parse_query` 解码（`+` 与双重 `%XX`），
/// 本函数不再做任何解码，仅做拼接。
fn join_decoded_query(pairs: &[(String, String)]) -> String {
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
}

/// 把 `uri` 按 `?` 拆成 `(path, Option<query>)`。
///
/// `?` 之后全部归 query（不再对 `#` fragment 做特殊处理，访问日志里
/// 一般无 fragment；若出现也并入 query 由下游决定）。
fn split_path_query(uri: &str) -> (String, Option<String>) {
    match uri.find('?') {
        Some(idx) => (
            uri[..idx].to_string(),
            Some(uri[idx + 1..].to_string()),
        ),
        None => (uri.to_string(), None),
    }
}

/// 单次 URL 解码：把 `%XX`（XX 合法 hex）替换为对应字节，非法 `%` 原样保留。
///
/// 字节级处理，输出按 UTF-8 重新组装；非法 UTF-8 字节用 `String::from_utf8_lossy`
/// 的 replacement char 表示（访问日志场景不会触发）。
fn url_decode_once(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = hex_val(bytes[i + 1]);
            let l = hex_val(bytes[i + 2]);
            if let (Some(hv), Some(lv)) = (h, l) {
                out.push((hv << 4) | lv);
                i += 3;
                continue;
            }
            // 非法 % → 原样保留
            out.push(b'%');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[inline]
fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// 双重 URL 解码：连续解两遍。
///
/// - `%27` → `'`，`%20` → 空格。
/// - 已解码字符串再解一次：`%2527` → `%27` → `'`。
/// - 第二次解码遇到 `%` 后非合法 hex（如已经解出的 `%` 后面跟普通字符）时
///   原样保留，不 panic。
pub fn url_decode_twice(input: &str) -> String {
    let once = url_decode_once(input);
    url_decode_once(&once)
}

/// 检测字符串是否含 URL 编码序列 `%XX`（XX 为合法 hex）（v0.7.1）。
///
/// `parse_sqls` / `detect_sql_blind_features` 用它判断输入是否需要先做 URL
/// 解码再喂盲注探针正则。已解码输入（无 `%XX` hex 序列）返回 `false`，走
/// 原路径，零行为变化。
///
/// **已知边界**：`LIKE '%ab%'` 这种字面百分号后跟合法 hex（如 `%ab`）会被
/// 识别为编码序列并解码为字节 0xAB（replacement char）。SQLi payload 场景
/// 此形态极罕见，且即使误解码也不影响盲注探针正则匹配——这是合理保守的
/// 简化。若未来需更严格区分，可改为要求 `%XX` 后跟非 hex 字符。
pub fn looks_like_url_encoded(input: &str) -> bool {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)%[0-9a-f]{2}").expect("url-encoded detector regex must compile")
    })
    .is_match(input)
}

/// 把 query 串拆成 `Vec<(key, value)>`，key/value 按 form-urlencoded 语义解码。
///
/// 解码顺序（关键，符合 application/x-www-form-urlencoded 标准）：
/// 1. 先把 `+` 解为空格（form-urlencoded 中 `+` 即空格）。
/// 2. 再做双重 `%XX` 解码（[`url_decode_twice`]）。
///
/// 顺序不可颠倒：若先 `%XX` 再 `+`，会把 `%2B`（字面 `+`）解出后又错解为空格。
/// 例：`x=%2B` → 第 1 步无 `+`，第 2 步 `%2B`→`+`，结果 `[("x","+")]`（保留字面 `+`）。
///
/// - 分隔符 `&`。
/// - `key=value` 形式；`key` 无 `=` 时 value 视为空串。
/// - 空 query（如 `""`）返回空 Vec。
pub fn parse_query(query: &str) -> Vec<(String, String)> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.find('=') {
            Some(idx) => (&pair[..idx], &pair[idx + 1..]),
            None => (pair, ""),
        };
        // form-urlencoded 标准：先 `+`→space，再双重 `%XX` 解码。
        out.push((
            url_decode_twice(&k.replace('+', " ")),
            url_decode_twice(&v.replace('+', " ")),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_url_encoded_pct20() {
        assert!(looks_like_url_encoded("1'%20or%201=1#"));
    }

    #[test]
    fn looks_like_url_encoded_pct3e_pct23() {
        assert!(looks_like_url_encoded(
            "username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1"
        ));
    }

    #[test]
    fn looks_like_url_encoded_plain_sql() {
        // 纯文本 SQL，无 `%XX` hex 序列。
        assert!(!looks_like_url_encoded("SELECT * FROM users"));
        assert!(!looks_like_url_encoded("1' or 1=1#"));
    }

    #[test]
    fn looks_like_url_encoded_bare_pct_no_hex() {
        // `%` 后跟非 hex 字符（如 `%q`）→ 非合法编码序列 → false。
        assert!(!looks_like_url_encoded("100% sure"));
    }
}
