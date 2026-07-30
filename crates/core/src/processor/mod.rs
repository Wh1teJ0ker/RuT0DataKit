//! 处理器模块。
//!
//! v1.0.0: 仅 trait 骨架。具体 masker/validator/extractor/rules 实现推迟 v1.1+。

pub mod extractor;
pub mod masker;
pub mod rules;
pub mod validator;

// ProcessorRegistry 占位：v1.1+ 新增处理器只需实现对应 trait 并注册到此处。
