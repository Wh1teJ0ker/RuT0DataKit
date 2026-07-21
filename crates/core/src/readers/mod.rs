//! 数据源读取抽象：`Records` + `SourceReader` trait + CSV/XLSX/SQL/JSON/pcap/log 实现。
//!
//! 所有 reader 把源文件读成统一的 [`Records`] 结构：第一行 headers，
//! 其余为字符串 rows。空 cell 一律转为 `""`。后续 pipeline / report 在
//! `Records` 上做脱敏 / 导出，与具体源格式解耦。
//!
//! v0.4.0 新增：
//! - [`SqlReader`] / [`JsonReader`]：直接把 .sql / .json 文件读成 Records。
//! - [`PcapRecordsReader`] / [`LogRecordsReader`]：适配器，把 pcap / log 模块
//!   原本产出的 `Vec<HttpRequest>` / `Vec<LogEntry>` 包成 Records，不动原模块。
//! - [`read_records`]：按文件后缀 dispatch 到对应 reader 的统一入口。

use std::path::Path;

use crate::error::CoreError;

pub mod csv_reader;
pub mod json_reader;
pub mod log_reader;
pub mod pcap_reader;
pub mod sql_reader;
pub mod txt_reader;
pub mod xlsx_reader;

pub use csv_reader::CsvReader;
pub use json_reader::JsonReader;
pub use log_reader::LogRecordsReader;
pub use pcap_reader::PcapRecordsReader;
pub use sql_reader::SqlReader;
pub use txt_reader::TxtReader;
pub use xlsx_reader::XlsxReader;

/// 统一表格式数据：headers + rows。
///
/// `rows[i][j]` 对应 `headers[j]` 列在第 i 行的值。所有 cell 均为 `String`，
/// 空 cell 一律 `""`，调用方无需处理 `Option`。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Records {
    /// 表头（第一行）。
    pub headers: Vec<String>,
    /// 数据行（不含表头）。
    pub rows: Vec<Vec<String>>,
}

/// 数据源读取器 trait。每种源（csv / xlsx / sql / json / pcap / log）实现该接口。
pub trait SourceReader {
    /// 把 `path` 指向的文件读成 [`Records`]。
    fn read(&self, path: &Path) -> Result<Records, CoreError>;
}

/// 按 [`crate::pipeline::detect_type`] 探测的源类型 dispatch 到对应 reader，
/// 把文件读成 [`Records`]。
///
/// - csv → [`CsvReader`]
/// - xlsx → [`XlsxReader`]
/// - sql → [`SqlReader`]
/// - json → [`JsonReader`]
/// - pcap → [`PcapRecordsReader`]（tshark 缺失时返回
///   [`CoreError::DependencyMissing`]）
/// - log → [`LogRecordsReader`]
/// - txt → [`TxtReader`]
/// - unknown → [`CoreError::InvalidInput`]
pub fn read_records(path: &Path) -> Result<Records, CoreError> {
    let t = crate::pipeline::detect_type(path)?;
    match t {
        crate::pipeline::SourceType::Csv => CsvReader::new().read(path),
        crate::pipeline::SourceType::Xlsx => XlsxReader::new().read(path),
        crate::pipeline::SourceType::Sql => SqlReader::new().read(path),
        crate::pipeline::SourceType::Json => JsonReader::new().read(path),
        crate::pipeline::SourceType::Pcap => PcapRecordsReader::new().read(path),
        crate::pipeline::SourceType::Log => LogRecordsReader::new().read(path),
        crate::pipeline::SourceType::Txt => TxtReader::new().read(path),
        crate::pipeline::SourceType::Unknown => Err(CoreError::InvalidInput(format!(
            "unsupported source type for path: {}",
            path.display()
        ))),
    }
}
