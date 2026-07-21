//! log 适配器：把 [`crate::log::LogReader::read`] 产出的 `Vec<LogEntry>`
//! 映射为 [`Records`]，让 .log 与 csv/xlsx/sql/json/pcap 走同一 `read_records` 入口。
//!
//! 不修改 log 内部实现，仅在外层包一层 `SourceReader` 适配。
//!
//! 列对齐 [`crate::log::LogEntry`] 字段（`raw` 不进表，避免行表过长且重复原始文本）：
//! `line_no / ip / timestamp / method / path / query / status / size /
//! user_agent / decoded_path / decoded_query / decoded_ua`。
//! `query` / `size` / `decoded_query` 为 `None` 时 cell 为 `""`。

use std::path::Path;

use crate::error::CoreError;
use crate::log::LogReader;
use crate::readers::{Records, SourceReader};

/// log → Records 适配器。
#[derive(Debug, Default, Clone, Copy)]
pub struct LogRecordsReader;

impl LogRecordsReader {
    pub fn new() -> Self {
        Self
    }
}

/// log → Records 表头顺序（与 [`crate::log::LogEntry`] 字段对应，`raw` 不含）。
pub const HEADERS: [&str; 12] = [
    "line_no",
    "ip",
    "timestamp",
    "method",
    "path",
    "query",
    "status",
    "size",
    "user_agent",
    "decoded_path",
    "decoded_query",
    "decoded_ua",
];

impl SourceReader for LogRecordsReader {
    fn read(&self, path: &Path) -> Result<Records, CoreError> {
        let reader = LogReader::new()?;
        let entries = reader.read(path)?;
        let rows = entries
            .into_iter()
            .map(|e| {
                vec![
                    e.line_no.to_string(),
                    e.ip,
                    e.timestamp,
                    e.method,
                    e.path,
                    e.query.unwrap_or_default(),
                    e.status.to_string(),
                    e.size.map(|n| n.to_string()).unwrap_or_default(),
                    e.user_agent,
                    e.decoded_path,
                    e.decoded_query.unwrap_or_default(),
                    e.decoded_ua,
                ]
            })
            .collect();
        Ok(Records {
            headers: HEADERS.iter().map(|s| s.to_string()).collect(),
            rows,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, content: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(name);
        let mut f = std::fs::File::create(&dir).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn parses_single_log_line_to_row() {
        let line = r#"127.0.0.1 - - [17/Nov/2023:03:44:21 +0000] "GET /a?x=1 HTTP/1.1" 200 123 "ref" "Mozilla""#;
        let p = write_tmp("rut0_log_basic.log", line);
        let rec = LogRecordsReader::new().read(&p).expect("read log");
        assert_eq!(rec.headers.len(), 12);
        assert_eq!(rec.rows.len(), 1);
        assert_eq!(rec.rows[0][0], "1");
        assert_eq!(rec.rows[0][1], "127.0.0.1");
        assert_eq!(rec.rows[0][3], "GET");
        assert_eq!(rec.rows[0][4], "/a");
        assert_eq!(rec.rows[0][5], "x=1");
        assert_eq!(rec.rows[0][6], "200");
        assert_eq!(rec.rows[0][7], "123");
        assert_eq!(rec.rows[0][8], "Mozilla");
    }

    #[test]
    fn missing_file_errors() {
        let res = LogRecordsReader::new().read(Path::new("/nonexistent/RuT0DataKit/none.log"));
        assert!(res.is_err());
    }

    #[test]
    fn invalid_log_line_errors() {
        let p = write_tmp("rut0_log_bad.log", "this is not a clf line at all\n");
        let res = LogRecordsReader::new().read(&p);
        assert!(res.is_err());
    }
}
