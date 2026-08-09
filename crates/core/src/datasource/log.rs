//! LOG 读取器：自动识别格式 + 结构化多列解析 + raw_line 保留。
//!
//! v1.1.2 增量：从「按行单列」升级为格式自动识别 + 结构化解析。支持四种格式：
//! - **Apache Combined Log Format** — 11 结构化列 + raw_line
//! - **Apache Common Log Format** — 9 结构化列 + raw_line
//! - **Syslog（RFC 3164）** — 5 结构化列 + raw_line
//! - **通用应用日志**（ISO 日期前缀 + level）— 4 结构化列 + raw_line
//!
//! 未识别格式回退到单列 `line`（v1.1.2 之前的行为）。解析失败的行各结构化字段留空，
//! `raw_line` 始终保留原始全文。

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::Reader;

/// 识别到的日志格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFormat {
    ApacheCombined,
    ApacheCommon,
    Syslog,
    AppLog,
    Fallback,
}

// -----------------------------------------------------------------------
// 正则缓存（OnceLock，首次访问编译一次）
// -----------------------------------------------------------------------

/// Apache Combined Log Format: `host ident user [time] "request" status bytes "referer" "ua"`
/// 示例: `10.0.0.1 - - [17/Nov/2023:03:44:24 +0000] "GET / HTTP/1.1" 200 874 "-" "Mozilla/5.0"`
static APACHE_COMBINED_RE: OnceLock<Regex> = OnceLock::new();

/// Apache Common Log Format: `host ident user [time] "request" status bytes`
static APACHE_COMMON_RE: OnceLock<Regex> = OnceLock::new();

/// Syslog RFC 3164: `Mon DD HH:MM:SS host process[pid]: message`
/// 示例: `Jan 12 03:14:15 myhost sshd[1234]: Accepted publickey for root`
static SYSLOG_RE: OnceLock<Regex> = OnceLock::new();

/// 通用应用日志: `YYYY-MM-DD HH:MM:SS(.ms)? LEVEL [thread]? message`
/// 示例: `2023-01-12 03:14:15.123 INFO [main] Starting application`
static APP_LOG_RE: OnceLock<Regex> = OnceLock::new();

fn combined_re() -> &'static Regex {
    APACHE_COMBINED_RE.get_or_init(|| {
        // 9 capture groups: host ident user time request status bytes referer ua
        Regex::new(
            r#"^(\S+) (\S+) (\S+) \[([^\]]+)\] "([^"]*)" (\d{3}) (\d+|-) "([^"]*)" "([^"]*)""#,
        )
        .expect("invalid combined regex")
    })
}

fn common_re() -> &'static Regex {
    APACHE_COMMON_RE.get_or_init(|| {
        // 7 capture groups: host ident user time request status bytes
        Regex::new(r#"^(\S+) (\S+) (\S+) \[([^\]]+)\] "([^"]*)" (\d{3}) (\d+|-)$"#)
            .expect("invalid common regex")
    })
}

fn syslog_re() -> &'static Regex {
    SYSLOG_RE.get_or_init(|| {
        // 5 capture groups: timestamp host process pid(opt) message
        Regex::new(
            r"^(\w{3}\s+\d+\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+([\w\-./]+)(?:\[(\d+)\])?:\s*(.*)$",
        )
        .expect("invalid syslog regex")
    })
}

fn app_log_re() -> &'static Regex {
    APP_LOG_RE.get_or_init(|| {
        // 4 capture groups: timestamp level thread(opt) message
        Regex::new(
            r"^(\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)\s+(INFO|WARN|ERROR|DEBUG|TRACE|FATAL|CRITICAL|NOTICE)\s*(?:\[([^\]]+)\])?\s*(.*)$",
        )
        .expect("invalid app_log regex")
    })
}

// -----------------------------------------------------------------------
// 列名定义
// -----------------------------------------------------------------------

const APACHE_COMBINED_HEADERS: &[&str] = &[
    "remote_host",
    "ident",
    "remote_user",
    "time_local",
    "request_method",
    "request_url",
    "request_protocol",
    "status",
    "body_bytes_sent",
    "http_referer",
    "http_user_agent",
    "raw_line",
];

const APACHE_COMMON_HEADERS: &[&str] = &[
    "remote_host",
    "ident",
    "remote_user",
    "time_local",
    "request_method",
    "request_url",
    "request_protocol",
    "status",
    "body_bytes_sent",
    "raw_line",
];

const SYSLOG_HEADERS: &[&str] = &["timestamp", "host", "process", "pid", "message", "raw_line"];

const APP_LOG_HEADERS: &[&str] = &["timestamp", "level", "thread", "message", "raw_line"];

const FALLBACK_HEADERS: &[&str] = &["line"];

// -----------------------------------------------------------------------
// LogFormat 探测
// -----------------------------------------------------------------------

/// 取前 N 条非空行作为 probe 样本。
fn probe_lines<'a>(lines: &'a [&'a str], n: usize) -> Vec<&'a str> {
    lines
        .iter()
        .filter(|l| !l.is_empty())
        .take(n)
        .copied()
        .collect()
}

/// 对一组行探测最匹配的日志格式。全部 0 命中 → Fallback。
fn detect_format(lines: &[&str]) -> LogFormat {
    let probe = probe_lines(lines, 10);
    if probe.is_empty() {
        return LogFormat::Fallback;
    }

    let mut combined = 0u32;
    let mut common = 0u32;
    let mut syslog = 0u32;
    let mut app_log = 0u32;

    for line in &probe {
        if combined_re().is_match(line) {
            combined += 1;
        }
        if common_re().is_match(line) {
            common += 1;
        }
        if syslog_re().is_match(line) {
            syslog += 1;
        }
        if app_log_re().is_match(line) {
            app_log += 1;
        }
    }

    // Combined 正则要求有 referer+ua，common 行不会被 combined_re 命中；
    // 选命中数最高的格式。
    let best = [
        (LogFormat::ApacheCombined, combined),
        (LogFormat::ApacheCommon, common),
        (LogFormat::Syslog, syslog),
        (LogFormat::AppLog, app_log),
    ]
    .into_iter()
    .max_by_key(|(_, c)| *c)
    .filter(|(_, c)| *c > 0);

    match best {
        Some((fmt, _)) => fmt,
        None => LogFormat::Fallback,
    }
}

/// 把 request 字段 `"GET /path HTTP/1.1"` 拆分为 (method, url, protocol)。
/// request 为 `"-"` 或无法拆分时返回三个空串。
fn split_request(request: &str) -> (&str, &str, &str) {
    if request == "-" || request.is_empty() {
        return ("", "", "");
    }
    let parts: Vec<&str> = request.split_whitespace().collect();
    match parts.as_slice() {
        [method, url, protocol] => (*method, *url, *protocol),
        _ => ("", "", ""),
    }
}

// -----------------------------------------------------------------------
// LogReader
// -----------------------------------------------------------------------

/// LOG 读取器：自动识别格式 + 结构化多列解析。
pub struct LogReader {
    path: String,
}

impl LogReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// 读全部行 + 探测格式。返回 (lines, format)。
    fn load(&self) -> CoreResult<(Vec<String>, LogFormat)> {
        let bytes = std::fs::read(&self.path)
            .map_err(|e| CoreError::DataSource(format!("log open: {e}")))?;
        let content = String::from_utf8_lossy(&bytes);
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let fmt = detect_format(&line_refs);
        Ok((lines, fmt))
    }
}

impl Reader for LogReader {
    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let (lines, fmt) = self.load()?;
        let headers = match fmt {
            LogFormat::ApacheCombined => APACHE_COMBINED_HEADERS,
            LogFormat::ApacheCommon => APACHE_COMMON_HEADERS,
            LogFormat::Syslog => SYSLOG_HEADERS,
            LogFormat::AppLog => APP_LOG_HEADERS,
            LogFormat::Fallback => FALLBACK_HEADERS,
        };

        let mut records: Vec<Record> = Vec::with_capacity(lines.len() + 1);

        // 表头行：fields[h] = h
        let mut h_fields = HashMap::new();
        for h in headers {
            h_fields.insert(h.to_string(), h.to_string());
        }
        records.push(Record { fields: h_fields });

        for line in &lines {
            let mut fields = HashMap::new();

            // raw_line 始终保留原始全文（所有格式通用）。
            let raw = line.as_str();

            match fmt {
                LogFormat::ApacheCombined => {
                    if let Some(caps) = combined_re().captures(raw) {
                        let request = caps.get(5).map(|m| m.as_str()).unwrap_or("");
                        let (method, url, proto) = split_request(request);
                        fields.insert("remote_host".into(), caps[1].to_string());
                        fields.insert("ident".into(), caps[2].to_string());
                        fields.insert("remote_user".into(), caps[3].to_string());
                        fields.insert("time_local".into(), caps[4].to_string());
                        fields.insert("request_method".into(), method.to_string());
                        fields.insert("request_url".into(), url.to_string());
                        fields.insert("request_protocol".into(), proto.to_string());
                        fields.insert("status".into(), caps[6].to_string());
                        fields.insert("body_bytes_sent".into(), caps[7].replace('-', "0"));
                        fields.insert("http_referer".into(), caps[8].to_string());
                        fields.insert("http_user_agent".into(), caps[9].to_string());
                    }
                    // 未命中的字段留空（HashMap::get → None → import_file 写 NULL）
                    fields.insert("raw_line".into(), raw.to_string());
                }
                LogFormat::ApacheCommon => {
                    if let Some(caps) = common_re().captures(raw) {
                        let request = caps.get(5).map(|m| m.as_str()).unwrap_or("");
                        let (method, url, proto) = split_request(request);
                        fields.insert("remote_host".into(), caps[1].to_string());
                        fields.insert("ident".into(), caps[2].to_string());
                        fields.insert("remote_user".into(), caps[3].to_string());
                        fields.insert("time_local".into(), caps[4].to_string());
                        fields.insert("request_method".into(), method.to_string());
                        fields.insert("request_url".into(), url.to_string());
                        fields.insert("request_protocol".into(), proto.to_string());
                        fields.insert("status".into(), caps[6].to_string());
                        fields.insert("body_bytes_sent".into(), caps[7].replace('-', "0"));
                    }
                    fields.insert("raw_line".into(), raw.to_string());
                }
                LogFormat::Syslog => {
                    if let Some(caps) = syslog_re().captures(raw) {
                        fields.insert("timestamp".into(), caps[1].to_string());
                        fields.insert("host".into(), caps[2].to_string());
                        fields.insert("process".into(), caps[3].to_string());
                        fields.insert(
                            "pid".into(),
                            caps.get(4).map(|m| m.as_str()).unwrap_or("").to_string(),
                        );
                        fields.insert(
                            "message".into(),
                            caps.get(5).map(|m| m.as_str()).unwrap_or("").to_string(),
                        );
                    }
                    fields.insert("raw_line".into(), raw.to_string());
                }
                LogFormat::AppLog => {
                    if let Some(caps) = app_log_re().captures(raw) {
                        fields.insert("timestamp".into(), caps[1].to_string());
                        fields.insert("level".into(), caps[2].to_string());
                        fields.insert(
                            "thread".into(),
                            caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string(),
                        );
                        fields.insert(
                            "message".into(),
                            caps.get(4).map(|m| m.as_str()).unwrap_or("").to_string(),
                        );
                    }
                    fields.insert("raw_line".into(), raw.to_string());
                }
                LogFormat::Fallback => {
                    // 单列 line（v1.1.2 之前行为）。
                    fields.insert("line".into(), raw.to_string());
                }
            }

            records.push(Record { fields });
        }

        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        let (_, fmt) = self.load()?;
        let headers = match fmt {
            LogFormat::ApacheCombined => APACHE_COMBINED_HEADERS,
            LogFormat::ApacheCommon => APACHE_COMMON_HEADERS,
            LogFormat::Syslog => SYSLOG_HEADERS,
            LogFormat::AppLog => APP_LOG_HEADERS,
            LogFormat::Fallback => FALLBACK_HEADERS,
        };
        Ok(headers.iter().map(|s| s.to_string()).collect())
    }
}

// -----------------------------------------------------------------------
// 测试
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tempfile(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".log").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    // ---- 既有 3 测试（fallback 路径，不匹配任何格式）----

    #[test]
    fn log_reader_reads_lines() {
        let f = write_tempfile("INFO start\nERROR crash\nWARN retry\n");
        let reader = LogReader::new(f.path().to_str().unwrap());
        assert_eq!(reader.headers().unwrap(), vec!["line"]);
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 4); // 1 header + 3 data
        assert_eq!(records[1].fields.get("line").unwrap(), "INFO start");
        assert_eq!(records[2].fields.get("line").unwrap(), "ERROR crash");
        assert_eq!(records[3].fields.get("line").unwrap(), "WARN retry");
    }

    #[test]
    fn log_reader_preserves_empty_lines() {
        let f = write_tempfile("line1\n\nline3\n");
        let reader = LogReader::new(f.path().to_str().unwrap());
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 4); // 1 header + 3 data (含空行)
        assert_eq!(records[1].fields.get("line").unwrap(), "line1");
        assert_eq!(records[2].fields.get("line").unwrap(), "");
        assert_eq!(records[3].fields.get("line").unwrap(), "line3");
    }

    #[test]
    fn log_reader_empty_file() {
        let f = write_tempfile("");
        let reader = LogReader::new(f.path().to_str().unwrap());
        assert_eq!(reader.headers().unwrap(), vec!["line"]);
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 1); // 仅表头行
    }

    // ---- Apache Combined ----

    #[test]
    fn apache_combined_log_parses_to_columns() {
        let line = r#"10.0.0.1 - - [17/Nov/2023:03:44:24 +0000] "GET /?username=guest&password=123456 HTTP/1.1" 200 874 "-" "Mozilla/5.0 (Macintosh)""#;
        let f = write_tempfile(&format!("{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let headers = reader.headers().unwrap();
        assert_eq!(headers.len(), 12);
        assert_eq!(headers[0], "remote_host");
        assert_eq!(headers[11], "raw_line");
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 2); // 1 header + 1 data
        let row = &records[1].fields;
        assert_eq!(row.get("remote_host").unwrap(), "10.0.0.1");
        assert_eq!(row.get("ident").unwrap(), "-");
        assert_eq!(row.get("remote_user").unwrap(), "-");
        assert_eq!(row.get("time_local").unwrap(), "17/Nov/2023:03:44:24 +0000");
        assert_eq!(row.get("request_method").unwrap(), "GET");
        assert_eq!(
            row.get("request_url").unwrap(),
            "/?username=guest&password=123456"
        );
        assert_eq!(row.get("request_protocol").unwrap(), "HTTP/1.1");
        assert_eq!(row.get("status").unwrap(), "200");
        assert_eq!(row.get("body_bytes_sent").unwrap(), "874");
        assert_eq!(row.get("http_referer").unwrap(), "-");
        assert_eq!(
            row.get("http_user_agent").unwrap(),
            "Mozilla/5.0 (Macintosh)"
        );
        assert_eq!(row.get("raw_line").unwrap(), line);
    }

    #[test]
    fn apache_log_unparseable_request_keeps_raw() {
        // request 为 "-"（无请求行）
        let line = r#"10.0.0.1 - - [17/Nov/2023:03:44:24 +0000] "-" 408 0 "-" "-""#;
        let f = write_tempfile(&format!("{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let records = reader.read_all().unwrap();
        let row = &records[1].fields;
        assert_eq!(row.get("request_method").unwrap(), "");
        assert_eq!(row.get("request_url").unwrap(), "");
        assert_eq!(row.get("request_protocol").unwrap(), "");
        assert_eq!(row.get("status").unwrap(), "408");
        assert_eq!(row.get("body_bytes_sent").unwrap(), "0");
        assert_eq!(row.get("raw_line").unwrap(), line);
    }

    // ---- Apache Common ----

    #[test]
    fn apache_common_log_parses_to_columns() {
        let line = r#"10.0.0.1 - - [17/Nov/2023:03:44:24 +0000] "GET / HTTP/1.1" 200 874"#;
        let f = write_tempfile(&format!("{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let headers = reader.headers().unwrap();
        assert_eq!(headers.len(), 10);
        assert!(!headers.contains(&"http_referer".to_string()));
        assert!(!headers.contains(&"http_user_agent".to_string()));
        let records = reader.read_all().unwrap();
        let row = &records[1].fields;
        assert_eq!(row.get("remote_host").unwrap(), "10.0.0.1");
        assert_eq!(row.get("request_method").unwrap(), "GET");
        assert_eq!(row.get("request_url").unwrap(), "/");
        assert_eq!(row.get("request_protocol").unwrap(), "HTTP/1.1");
        assert_eq!(row.get("status").unwrap(), "200");
        assert_eq!(row.get("body_bytes_sent").unwrap(), "874");
        assert_eq!(row.get("raw_line").unwrap(), line);
    }

    // ---- Syslog ----

    #[test]
    fn syslog_parses_to_columns() {
        let line = "Jan 12 03:14:15 myhost sshd[1234]: Accepted publickey for root from 1.2.3.4";
        let f = write_tempfile(&format!("{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let headers = reader.headers().unwrap();
        assert_eq!(headers.len(), 6);
        assert_eq!(headers[0], "timestamp");
        assert_eq!(headers[5], "raw_line");
        let records = reader.read_all().unwrap();
        let row = &records[1].fields;
        assert_eq!(row.get("timestamp").unwrap(), "Jan 12 03:14:15");
        assert_eq!(row.get("host").unwrap(), "myhost");
        assert_eq!(row.get("process").unwrap(), "sshd");
        assert_eq!(row.get("pid").unwrap(), "1234");
        assert_eq!(
            row.get("message").unwrap(),
            "Accepted publickey for root from 1.2.3.4"
        );
        assert_eq!(row.get("raw_line").unwrap(), line);
    }

    #[test]
    fn syslog_without_pid() {
        let line = "Jan 12 03:14:15 myhost cron: (root) CMD (run-parts)";
        let f = write_tempfile(&format!("{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let records = reader.read_all().unwrap();
        let row = &records[1].fields;
        assert_eq!(row.get("process").unwrap(), "cron");
        assert_eq!(row.get("pid").unwrap(), "");
        assert_eq!(row.get("message").unwrap(), "(root) CMD (run-parts)");
    }

    // ---- 通用应用日志 ----

    #[test]
    fn app_log_parses_to_columns() {
        let line = "2023-01-12 03:14:15.123 INFO [main] Starting application";
        let f = write_tempfile(&format!("{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let headers = reader.headers().unwrap();
        assert_eq!(headers.len(), 5);
        assert_eq!(headers[0], "timestamp");
        assert_eq!(headers[4], "raw_line");
        let records = reader.read_all().unwrap();
        let row = &records[1].fields;
        assert_eq!(row.get("timestamp").unwrap(), "2023-01-12 03:14:15.123");
        assert_eq!(row.get("level").unwrap(), "INFO");
        assert_eq!(row.get("thread").unwrap(), "main");
        assert_eq!(row.get("message").unwrap(), "Starting application");
        assert_eq!(row.get("raw_line").unwrap(), line);
    }

    #[test]
    fn app_log_without_thread() {
        let line = "2023-01-12 03:14:15 INFO Something happened";
        let f = write_tempfile(&format!("{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let records = reader.read_all().unwrap();
        let row = &records[1].fields;
        assert_eq!(row.get("timestamp").unwrap(), "2023-01-12 03:14:15");
        assert_eq!(row.get("level").unwrap(), "INFO");
        assert_eq!(row.get("thread").unwrap(), "");
        assert_eq!(row.get("message").unwrap(), "Something happened");
    }

    // ---- 混合格式 fallback ----

    #[test]
    fn mixed_lines_fallback_to_single_column() {
        // 三行都不匹配任何结构化格式 → fallback 单列 line
        let f = write_tempfile("some random text\nanother line\nno pattern here\n");
        let reader = LogReader::new(f.path().to_str().unwrap());
        assert_eq!(reader.headers().unwrap(), vec!["line"]);
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 4);
        assert_eq!(records[1].fields.get("line").unwrap(), "some random text");
    }

    // ---- raw_line 保留原文 ----

    #[test]
    fn raw_line_preserves_original_text() {
        let line = r#"10.0.0.1 - - [17/Nov/2023:03:44:24 +0000] "GET / HTTP/1.1" 200 874 "-" "Mozilla/5.0""#;
        let f = write_tempfile(&format!("{line}\n{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 3); // 1 header + 2 data
        assert_eq!(records[1].fields.get("raw_line").unwrap(), line);
        assert_eq!(records[2].fields.get("raw_line").unwrap(), line);
    }

    // ---- body_bytes_sent 为 "-" 时归零 ----

    #[test]
    fn apache_combined_dash_bytes_normalized_to_zero() {
        let line =
            r#"10.0.0.1 - - [17/Nov/2023:03:44:24 +0000] "GET / HTTP/1.1" 200 - "-" "Mozilla/5.0""#;
        let f = write_tempfile(&format!("{line}\n"));
        let reader = LogReader::new(f.path().to_str().unwrap());
        let records = reader.read_all().unwrap();
        assert_eq!(records[1].fields.get("body_bytes_sent").unwrap(), "0");
    }

    // ---- 真实 fixture 集成测试（CI 无 fixture 时跳过）----

    #[test]
    #[ignore]
    fn apache_log_with_real_fixture() {
        let fixture = "tests/dasctf_ds/6可疑日志的附件/access.log";
        if !std::path::Path::new(fixture).exists() {
            eprintln!("fixture not found, skipping");
            return;
        }
        let reader = LogReader::new(fixture);
        let headers = reader.headers().unwrap();
        assert_eq!(headers.len(), 12);
        assert_eq!(headers[0], "remote_host");
        let records = reader.read_all().unwrap();
        assert!(records.len() > 100); // fixture has 1856 lines
        let first = &records[1].fields;
        assert!(first.get("remote_host").unwrap().starts_with("10."));
        assert!(!first.get("raw_line").unwrap().is_empty());
    }
}
