//! pcap 适配器：把 [`crate::pcap::PcapReader::read`] 产出的 `Vec<HttpRequest>`
//! 映射为 [`Records`]，让 pcap 与 csv/xlsx/sql/json 走同一 `read_records` 入口。
//!
//! 不修改 pcap 内部实现，仅在外层包一层 `SourceReader` 适配。
//!
//! 列：`frame_no / src_ip / dst_ip / method / host / uri / body / user_agent`，
//! 与 [`crate::pcap::HttpRequest`] 字段一一对应；`body` 为 `None` 时 cell 为 `""`。
//!
//! tshark 缺失时 `PcapReader::read` 返回
//! [`CoreError::DependencyMissing("tshark")`](crate::error::CoreError)，本适配器
//! 原样透传，不 panic。

use std::path::Path;

use crate::error::CoreError;
use crate::pcap::PcapReader;
use crate::readers::{Records, SourceReader};

/// pcap → Records 适配器。
#[derive(Debug, Default, Clone, Copy)]
pub struct PcapRecordsReader;

impl PcapRecordsReader {
    pub fn new() -> Self {
        Self
    }
}

/// pcap → Records 表头顺序（与 [`crate::pcap::HttpRequest`] 字段对应）。
pub const HEADERS: [&str; 8] = [
    "frame_no",
    "src_ip",
    "dst_ip",
    "method",
    "host",
    "uri",
    "body",
    "user_agent",
];

impl SourceReader for PcapRecordsReader {
    fn read(&self, path: &Path) -> Result<Records, CoreError> {
        let requests = PcapReader::read(path)?;
        let rows = requests
            .into_iter()
            .map(|r| {
                vec![
                    r.frame_no.to_string(),
                    r.src_ip,
                    r.dst_ip,
                    r.method,
                    r.host,
                    r.uri,
                    r.body.unwrap_or_default(),
                    r.user_agent,
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

    #[test]
    fn missing_file_errors() {
        let res = PcapRecordsReader::new().read(Path::new("/nonexistent/RuT0DataKit/none.pcap"));
        // tshark 在场时 PcapReader::read 会因 tshark 解析失败报错；不在场时
        // 返回 DependencyMissing。两者都应 Err。
        assert!(res.is_err());
    }

    #[test]
    fn headers_match_http_request_fields() {
        assert_eq!(HEADERS.len(), 8);
        assert_eq!(HEADERS[0], "frame_no");
        assert_eq!(HEADERS[6], "body");
    }
}
