//! PCAP 读取器：适配 `crate::pcap::PcapReader`。
//!
//! 调 tshark 子进程提取 HTTP 请求字段，每条 `HttpRequest` → 1 record。
//! headers = `[frame_no, src_ip, dst_ip, method, host, uri, body, user_agent]`。
//! tshark 缺失返回 `DependencyMissing`，前端据此弹提示。

use std::path::Path;

use crate::error::CoreResult;
use crate::model::Record;
use crate::pcap::reader::{HttpRequest, PcapReader as CorePcapReader};

use super::{Dataset, Reader};

/// PCAP 读取器：适配 `crate::pcap::PcapReader`。
pub struct PcapReader {
    path: String,
}

impl PcapReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    const HEADERS: &'static [&'static str] = &[
        "frame_no",
        "src_ip",
        "dst_ip",
        "method",
        "host",
        "uri",
        "body",
        "user_agent",
    ];

    fn read_requests(&self) -> CoreResult<Vec<HttpRequest>> {
        let reader = CorePcapReader::new();
        reader.read(Path::new(&self.path))
    }
}

impl Reader for PcapReader {
    /// 单次解析：调一次 tshark 子进程，产出 headers + rows。
    fn read(&self) -> CoreResult<Dataset> {
        let requests = self.read_requests()?;
        let headers: Vec<String> = Self::HEADERS.iter().map(|s| s.to_string()).collect();

        let mut rows: Vec<Record> = Vec::with_capacity(requests.len());
        for req in &requests {
            let mut fields = std::collections::HashMap::new();
            fields.insert("frame_no".to_string(), req.frame_no.clone());
            fields.insert("src_ip".to_string(), req.src_ip.clone());
            fields.insert("dst_ip".to_string(), req.dst_ip.clone());
            fields.insert("method".to_string(), req.method.clone());
            fields.insert("host".to_string(), req.host.clone());
            fields.insert("uri".to_string(), req.uri.clone());
            fields.insert("body".to_string(), req.body.clone());
            fields.insert("user_agent".to_string(), req.user_agent.clone());
            rows.push(Record { fields });
        }
        Ok(Dataset { headers, rows })
    }

    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let Dataset { headers, rows } = self.read()?;
        let mut records: Vec<Record> = Vec::with_capacity(rows.len() + 1);
        let mut h_fields = std::collections::HashMap::new();
        for h in &headers {
            h_fields.insert(h.clone(), h.clone());
        }
        records.push(Record { fields: h_fields });
        records.extend(rows);
        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        Ok(Self::HEADERS.iter().map(|s| s.to_string()).collect())
    }
}
