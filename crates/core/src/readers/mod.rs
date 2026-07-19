//! 数据源读取抽象：`Records` + `SourceReader` trait + CSV/XLSX 实现。
//!
//! 所有 reader 把源文件读成统一的 [`Records`] 结构：第一行 headers，
//! 其余为字符串 rows。空 cell 一律转为 `""`。后续 pipeline / report 在
//! `Records` 上做脱敏 / 导出，与具体源格式解耦。

use std::path::Path;

use crate::error::CoreError;

pub mod csv_reader;
pub mod xlsx_reader;

pub use csv_reader::CsvReader;
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

/// 数据源读取器 trait。每种源（csv / xlsx / log / pcap）实现该接口。
pub trait SourceReader {
    /// 把 `path` 指向的文件读成 [`Records`]。
    fn read(&self, path: &Path) -> Result<Records, CoreError>;
}
