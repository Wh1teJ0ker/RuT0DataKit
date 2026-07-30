//! 数据源读取模块。
//!
//! v1.0.0: 仅 `Reader` trait 占位。CsvReader/XlsxReader 实现推迟到 T5。

use crate::error::CoreResult;
use crate::model::Record;

/// 数据源读取器 trait。
///
/// v1.1+ 新增数据源（如 PCAP）只需实现本 trait 并在 `detect_format`
/// 工厂按扩展名分发，不改动 processor / db / table 模块。
pub trait Reader: Send + Sync {
    /// 读取全部记录。v1.0.0 不实现具体逻辑。
    fn read_all(&self) -> CoreResult<Vec<Record>>;

    /// 返回列名。v1.0.0 不实现具体逻辑。
    fn headers(&self) -> CoreResult<Vec<String>>;
}

/// 格式探测工厂占位。T5 实现按扩展名分发到具体 Reader。
///
/// v1.0.0: 返回未实现错误，避免误用。
pub fn detect_format(_path: &str) -> CoreResult<Box<dyn Reader>> {
    Err(crate::error::CoreError::NotImplemented(
        "datasource::detect_format",
    ))
}
