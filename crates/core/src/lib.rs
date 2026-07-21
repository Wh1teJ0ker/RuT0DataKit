//! ruT0-data-kit-core
//!
//! v0.1.0 骨架：暴露 error / pipeline / rules / validators / maskers 模块，
//! 以及 T0-5 新增的 readers / report 模块，以及 T0-6 新增的 scan 模块。
//! v0.2.0 新增 log 模块（CLF/Nginx Combined 访问日志解析）与 logsign 模块
//! （SQLi 签名引擎，6 类内置 YAML 签名）。
//! v0.3.0 新增 pcap 模块（tshark 子进程 + HTTP 字段提取 + base64 重组 +
//! 敏感扫描，产出 `kind="pcap_scan"` 报告）。
//! v0.4.0 readers 新增 `SqlReader` / `JsonReader` / `PcapRecordsReader` /
//! `LogRecordsReader`，统一 `read_records(path) -> Records` 入口，pcap/log
//! 不再只产 Report，也能转成表格参与预处理。

pub mod error;
pub mod log;
pub mod logsign;
pub mod maskers;
pub mod pcap;
pub mod pipeline;
pub mod readers;
pub mod report;
pub mod rules;
pub mod scan;
pub mod search;
pub mod tools;
pub mod validators;
