//! 加密 / SQL 解析工具模块。
//!
//! - `encrypt`：本地对称加密 / 解密（v0.6.0 T15-2）。
//! - `sql_parse`（T5-9）：接受纯 SQL 文本列表，自动识别盲注探针并
//!   还原结构化数据库，复用 v0.2.4 的 blind_aggregator 算法。
//!
//! v0.7.0：移除 `regex_explain` / `regex_construct` / `regex_template`
//! 三个模块（用户删除「正则解析」特性，Tools 下不再提供该入口）。
//! 搜索 / masker / logsign 内部仍可使用 regex，但不再暴露为 Tools 工具。

pub mod encrypt;
pub mod sql_parse;

pub use encrypt::{decrypt_text, encrypt_text, EncryptAlgo};
pub use sql_parse::{parse_sqls, SqlParseInput, SqlParseResult};
