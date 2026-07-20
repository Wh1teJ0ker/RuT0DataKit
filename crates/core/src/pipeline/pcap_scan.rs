//! v0.3.0 pcap 扫描 pipeline 薄包装。
//!
//! 对外暴露 [`scan_pcap`]，内部委托给 [`crate::pcap::PcapScanner::scan`]。
//! 与 [`crate::pipeline::log_scan::scan_log`] 对称：调用方注入 `scan` + `rules`，
//! pipeline 层不重复构造，便于 GUI 多次扫描复用同一 scanner 实例。

use std::path::Path;

use crate::error::CoreError;
use crate::pcap::PcapScanner;
use crate::report::Report;
use crate::rules::RuleSet;
use crate::scan::DefaultSensitiveScan;

/// 对 pcap 文件跑敏感扫描，产出 `kind="pcap_scan"` 的 [`Report`]。
///
/// - tshark 缺失 → [`CoreError::DependencyMissing("tshark")`] 透传。
/// - 仅做敏感扫描，不跑 SQLi 签名（v0.3.0 scope）。
pub fn scan_pcap(
    path: &Path,
    scan: &DefaultSensitiveScan,
    rules: &RuleSet,
) -> Result<Report, CoreError> {
    PcapScanner::scan(path, scan, rules)
}
