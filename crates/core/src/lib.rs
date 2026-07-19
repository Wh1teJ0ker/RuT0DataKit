//! ruT0-data-kit-core
//!
//! v0.1.0 骨架：暴露 error / pipeline / rules / validators / maskers 模块，
//! 以及 T0-5 新增的 readers / report 模块，以及 T0-6 新增的 scan 模块。
//! v0.2.0 新增 log 模块（CLF/Nginx Combined 访问日志解析）与 logsign 模块
//! （SQLi 签名引擎，6 类内置 YAML 签名）。

pub mod error;
pub mod log;
pub mod logsign;
pub mod maskers;
pub mod pipeline;
pub mod readers;
pub mod report;
pub mod rules;
pub mod scan;
pub mod validators;
