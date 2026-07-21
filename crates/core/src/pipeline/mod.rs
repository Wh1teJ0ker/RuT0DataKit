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

/// v0.4.0 T5-5：搜索统一入口（薄包装，re-export 自 `search` 模块）。
///
/// 调用方既可以从 `pipeline::search_records` 也可以从 `search::search_records`
/// 进入，两者等价；保留 pipeline 入口语义为「表格数据通用处理入口」。
pub use crate::search::{search_records, SearchHit, SearchMode, SearchQuery, SearchResult};

use std::path::Path;

use crate::error::CoreError;
use crate::readers::Records;

/// 被处理的数据源类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Csv,
    Xlsx,
    Log,
    Pcap,
    /// v0.4.0 新增：`.sql` 文件。
    Sql,
    /// v0.4.0 新增：`.json` 文件。
    Json,
    Unknown,
}

/// 根据路径后缀探测数据源类型。
///
/// `.csv`→Csv、`.xlsx`→Xlsx、`.log`→Log、`.pcap`/`.pcapng`→Pcap、`.sql`→Sql、
/// `.json`→Json，其余 `Unknown`。仅按后缀判定，不读取 magic。
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
        "sql" => SourceType::Sql,
        "json" => SourceType::Json,
        _ => SourceType::Unknown,
    };
    Ok(t)
}

/// 统一读取入口：按 [`detect_type`] dispatch 到对应 reader，把文件读成
/// [`Records`]。
///
/// 实际委托给 [`crate::readers::read_records`]，这里仅做 re-export 语义的
/// 短路径，方便调用方从 `pipeline` 入口取数据。
pub fn read_records(path: &Path) -> Result<Records, CoreError> {
    crate::readers::read_records(path)
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
            detect_type(Path::new("a.sql")).unwrap(),
            SourceType::Sql
        );
        assert_eq!(
            detect_type(Path::new("a.json")).unwrap(),
            SourceType::Json
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
