//! 正则工具 + SQL 解析工具命令（3 命令）。

use ruT0_data_kit_core::tools::{
    construct_regex as core_construct_regex, explain_regex as core_explain_regex,
    parse_sqls as core_parse_sqls, ConstructedRegex, RegexTokenDesc, SqlParseInput,
    SqlParseResult,
};

/// 解释正则字符串的每个 token，返回 `{ token, kind, description, position }` 列表。
///
/// 非法正则（`regex::Regex::new` 失败）返回 `Err`，不 panic。
#[tauri::command]
pub fn explain_regex(pattern: String) -> Result<Vec<RegexTokenDesc>, String> {
    core_explain_regex(&pattern).map_err(|e| e.to_string())
}

/// v0.4.1 T6-5：从自然语言描述构造正则。
///
/// 纯本地规则化推断（位数 / 字符集 / 锚定前缀 / 邮箱 / URL / 身份证），
/// 不调用网络。无法识别线索时返回 `Err`。
#[tauri::command]
pub fn regex_construct(statement: String) -> Result<ConstructedRegex, String> {
    core_construct_regex(&statement).map_err(|e| e.to_string())
}

/// 对一批纯 SQL 文本做盲注探针提取 + 聚类 + 数据库还原（v0.5.0 T5-9）。
///
/// 调 core 的 `tools::parse_sqls`：接受 `Vec<SqlParseInput>`（每项含 sql +
/// 可选 response_body_size + 可选 source_ip），返回 `SqlParseResult`
/// `{ probes, aggregated, reconstructed, parsed_payloads }`。
///
/// 与 `scan_log_file` 的区别：本命令不读日志文件、不依赖 `LogEntry`，前端
/// 直接粘贴 SQL 文本列表即可还原数据库结构（T5-10 GUI 调用）。
#[tauri::command]
pub fn parse_sql_tool(inputs: Vec<SqlParseInput>) -> Result<SqlParseResult, String> {
    Ok(core_parse_sqls(inputs))
}
