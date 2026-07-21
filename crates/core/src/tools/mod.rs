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

pub mod regex_explain;
pub mod regex_template;
pub mod sql_parse;

pub use regex_explain::{explain_regex, RegexTokenDesc};
pub use regex_template::{generate_regex, list_regex_templates, TemplateMeta};
pub use sql_parse::{parse_sqls, SqlParseInput, SqlParseResult};
