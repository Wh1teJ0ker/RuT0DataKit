//! v0.3.0 pcap 模块：tshark 子进程 + HTTP 字段提取 + 解码 + 敏感扫描。
//!
//! 模块布局：
//! - [`reader`]：调系统 `tshark` 把 pcap 转成 TSV，解析为 [`HttpRequest`] 列表。
//! - [`decoder`]：双重 URL 解码 + base64 字段解码 + 规则化重组接口。
//! - [`scanner`]：[`PcapScanner::scan`] 把 reader + decoder + `SensitiveScan` 串成
//!   `kind="pcap_scan"` 的 [`Report`](crate::report::Report)。
//!
//! 全部本地处理，tshark 仅在用户本机执行，不上传任何 pcap/规则/样本。
//!
//! tshark 缺失时返回 [`CoreError::DependencyMissing("tshark")`](crate::error::CoreError::DependencyMissing)，
//! GUI 层据此弹提示并禁用按钮。

pub mod decoder;
pub mod reader;
pub mod scanner;

pub use decoder::{decode_url_twice, extract_decoded_fields, reassemble_base64, try_decode_base64_field};
pub use reader::{HttpRequest, PcapReader};
pub use scanner::PcapScanner;
