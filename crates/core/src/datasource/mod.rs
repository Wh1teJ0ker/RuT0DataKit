//! 数据源读取模块。
//!
//! v1.0.0（T5 + 全格式扩展）：实现 CSV / XLSX / JSON / JSONL / SQL / TXT / PCAP
//! 七种格式的 `Reader`。
//! - CSV：基于 `csv` crate，首行作为表头。
//! - XLSX：基于 `calamine` crate，取首个工作表，首行作为表头。
//! - JSON / JSONL：基于 `serde_json`。JSON 数组按对象展开；JSONL 每行一个对象。
//! - TXT：整段文本读成单 cell `content`。
//! - SQL：用 `rusqlite` in-memory 执行全部语句并收集所有 SELECT 结果。
//! - PCAP：调 `crate::pcap::PcapReader`（tshark 子进程）提取 HTTP 请求字段。
//!
//! v1.1+ 新增数据源只需实现 `Reader` trait 并在 `detect_format`
//! 工厂按扩展名分发，不改动 processor / db / table 模块。
//!
//! 各格式实现位于独立子模块：[`csv`]、[`xlsx`]、[`json`]、[`txt`]、
//! [`sql`]、[`pcap`]；共享 helper 在 [`util`]。

use std::path::Path;

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

mod csv;
mod json;
mod log;
mod pcap;
mod sql;
mod txt;
mod util;
mod xlsx;

pub use csv::CsvReader;
pub use json::JsonReader;
pub use log::LogReader;
pub use pcap::PcapReader;
pub use sql::SqlReader;
pub use txt::TxtReader;
pub use xlsx::XlsxReader;

/// 单次解析产出的数据集：稳定 `headers` + 数据行 `rows`。
///
/// `rows` 不含表头行；表头仅由 `headers` 字段表达。这与历史 `read_all`
/// 返回「表头作为 row_idx=0」的语义不同，但 import_file 调用方据此分别
/// 写入表头 cells 与数据 cells，避免重复解析。
#[derive(Debug, Clone)]
pub struct Dataset {
    /// 列名（首次出现顺序，稳定）。
    pub headers: Vec<String>,
    /// 数据行（不含表头行）。
    pub rows: Vec<Record>,
}

/// 数据源读取器 trait。
///
/// v1.1.4（T60）：新增 [`Reader::read`] 作为单次解析入口，同一会话内
/// 同时产出 headers 与 rows，消除旧的 `headers() + read_all()` 双解析
/// 路径。`headers()` 与 `read_all()` 保留为基于 `read()` 的默认实现，
/// 兼容既有调用方与测试；新代码应直接使用 `read()`。
pub trait Reader: Send + Sync {
    /// 单次解析：读取表头与全部数据行（不含表头行）。
    ///
    /// 所有具体 Reader 必须直接重写此方法为真正的单次解析；
    /// 默认实现仅供 trait 对象在未重写时回退（不会与已重写的 `read_all`
    /// 形成无限递归，因为具体实现二者其一会被覆盖）。
    fn read(&self) -> CoreResult<Dataset>;

    /// 读取全部记录（含表头行；表头行 `row_idx=0`）。
    ///
    /// 默认实现基于 `read()` 拼回「表头作为首行」的旧形态，兼容既有
    /// 调用方与测试。
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

    /// 返回列名（首行）。默认实现复用 `read()`，避免重复解析。
    fn headers(&self) -> CoreResult<Vec<String>> {
        Ok(self.read()?.headers)
    }
}

/// 格式探测工厂：按扩展名分发到具体 Reader。
///
/// 支持扩展名：`.csv` → `CsvReader`、`.xlsx` → `XlsxReader`、
/// `.json`/`.jsonl` → `JsonReader`、`.txt` → `TxtReader`、
/// `.log` → `LogReader`、`.sql` → `SqlReader`、
/// `.pcap`/`.pcapng` → `PcapReader`。
/// 其它扩展名返回 `NotImplemented`。
pub fn detect_format(path: &str) -> CoreResult<Box<dyn Reader>> {
    let p = Path::new(path);
    match p.extension().and_then(|e| e.to_str()) {
        Some("csv") => Ok(Box::new(CsvReader::new(path))),
        Some("xlsx") => Ok(Box::new(XlsxReader::new(path))),
        Some("json") => Ok(Box::new(JsonReader::new(path))),
        Some("jsonl") => Ok(Box::new(JsonReader::new(path))),
        Some("txt") => Ok(Box::new(TxtReader::new(path))),
        Some("log") => Ok(Box::new(LogReader::new(path))),
        Some("sql") => Ok(Box::new(SqlReader::new(path))),
        Some("pcap") => Ok(Box::new(PcapReader::new(path))),
        Some("pcapng") => Ok(Box::new(PcapReader::new(path))),
        _ => Err(CoreError::NotImplemented("datasource::detect_format")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_routes_by_extension() {
        // v1.0.0：csv/xlsx/json/jsonl/txt/sql/pcap/pcapng 均可路由。
        let csv = tmp_csv("a,b\n1,2\n");
        assert!(detect_format(csv.path().to_str().unwrap()).is_ok());
        assert!(detect_format("/tmp/foo.txt").is_ok());
        assert!(detect_format("/tmp/foo.json").is_ok());
        assert!(detect_format("/tmp/foo.jsonl").is_ok());
        assert!(detect_format("/tmp/foo.sql").is_ok());
        assert!(detect_format("/tmp/foo.pcap").is_ok());
        assert!(detect_format("/tmp/foo.pcapng").is_ok());
        assert!(detect_format("/tmp/foo.log").is_ok());
        // 不支持的扩展名仍返回 NotImplemented。
        let err = detect_format("/tmp/foo.unknown").err().unwrap();
        assert!(matches!(err, CoreError::NotImplemented(_)));
    }

    fn tmp_csv(content: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut f = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }
}
