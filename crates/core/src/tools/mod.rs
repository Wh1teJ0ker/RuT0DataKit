//! 正则解释 / 正则模板生成 / SQL 解析工具模块。
//!
//! v0.5.0 新增：
//! - `regex_explain`：把正则字符串拆解成用户可读的 token 描述。
//! - `regex_template`：按预设场景（email / phone_cn / ipv4 等）生成可编译
//!   的正则骨架，供前端直接复用。
//! - `sql_parse`（T5-9）：接受纯 SQL 文本列表，自动识别 7 类盲注探针并
//!   还原结构化数据库，复用 v0.2.4 的 blind_aggregator 算法。
//!
//! 三者相互独立。

pub mod encrypt;
pub mod regex_construct;
pub mod regex_explain;
pub mod regex_template;
pub mod sql_parse;

pub use encrypt::{decrypt_text, encrypt_text, EncryptAlgo};
pub use regex_construct::{construct_regex, ConstructedRegex};
pub use regex_explain::{explain_regex, RegexTokenDesc};
pub use sql_parse::{parse_sqls, SqlParseInput, SqlParseResult};
// 注意：不再 re-export regex_template 的 pub API（generate_regex / list_regex_templates / TemplateMeta）。
// regex_template.rs 文件保留（避免破坏既有 e2e 文件级引用），但前端/Tauri 不再调用它。
