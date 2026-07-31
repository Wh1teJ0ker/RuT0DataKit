//! pcap 模块：tshark 子进程 + HTTP 字段提取。
//!
//! v1.0.0：从 v0.8.0 移植 detect + reader（不含 decoder/scanner）。
//! 全部本地处理，tshark 仅在用户本机执行，不上传任何 pcap/规则/样本。
//!
//! tshark 缺失时返回 [`DependencyMissing("tshark")`](crate::error::CoreError::DependencyMissing)，
//! GUI 层据此弹提示并禁用按钮。

pub mod detect;
pub mod reader;

pub use detect::{
    candidate_paths, detect_tshark, get_tshark_path, resolve_tshark_cmd, set_tshark_path, TsharkInfo,
};
pub use reader::{HttpRequest, PcapReader};
