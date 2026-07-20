//! Pipeline 分发入口。
//!
//! v0.1.0：`detect_type` 按文件后缀探测数据源类型；脱敏 pipeline 由
//! [`mask::mask_pipeline`] 承载。校验 pipeline 见 [`validate::validate_pipeline`]。
//! v0.2.0 新增 [`log_scan::scan_log`]：对 `Vec<LogEntry>` 跑签名引擎 + 弱口令
//! grep + 敏感字段扫描，产出 `kind="log_scan"` 报告。pcap 扫描在后续任务补齐。

pub mod columns;
pub mod log_scan;
pub mod mask;
pub mod pcap_scan;
pub mod validate;

pub use columns::mask_pipeline_columns;
pub use log_scan::scan_log;
pub use mask::{mask_pipeline, mask_pipeline_selected, MaskResult, MaskSummary};
pub use pcap_scan::scan_pcap;
pub use validate::{validate_pipeline, ValidateResult, ValidateSummary};

use std::path::Path;

use crate::error::CoreError;

/// 被处理的数据源类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Csv,
    Xlsx,
    Log,
    Pcap,
    Unknown,
}

/// 根据路径后缀探测数据源类型。
///
/// `.csv`→Csv、`.xlsx`→Xlsx、`.log`→Log、`.pcap`/`.pcapng`→Pcap，
/// 其余 `Unknown`。仅按后缀判定，不读取 magic。
pub fn detect_type(path: &Path) -> Result<SourceType, CoreError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let t = match ext.as_str() {
        "csv" => SourceType::Csv,
        "xlsx" => SourceType::Xlsx,
        "log" => SourceType::Log,
        "pcap" | "pcapng" => SourceType::Pcap,
        _ => SourceType::Unknown,
    };
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_csv_xlsx_log_pcap() {
        assert_eq!(
            detect_type(Path::new("a.csv")).unwrap(),
            SourceType::Csv
        );
        assert_eq!(
            detect_type(Path::new("a/b.XLSX")).unwrap(),
            SourceType::Xlsx
        );
        assert_eq!(
            detect_type(Path::new("a.log")).unwrap(),
            SourceType::Log
        );
        assert_eq!(
            detect_type(Path::new("a.pcap")).unwrap(),
            SourceType::Pcap
        );
        assert_eq!(
            detect_type(Path::new("a.pcapng")).unwrap(),
            SourceType::Pcap
        );
        assert_eq!(
            detect_type(Path::new("a.txt")).unwrap(),
            SourceType::Unknown
        );
        assert_eq!(
            detect_type(Path::new("noext")).unwrap(),
            SourceType::Unknown
        );
    }
}
