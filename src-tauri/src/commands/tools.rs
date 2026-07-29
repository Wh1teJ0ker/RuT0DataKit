//! SQL 解析工具命令（v0.7.0 起仅保留本命令，正则解析相关命令已移除）。

use ruT0_data_kit_core::tools::{parse_sqls as core_parse_sqls, SqlParseInput, SqlParseResult};

/// 对一批纯 SQL 文本做盲注探针提取 + 聚类 + 数据库还原（v0.5.0 T5-9）。
///
/// 调 core 的 `tools::parse_sqls`：接受 `Vec<SqlParseInput>`（每项含 sql +
/// 可选 response_body_size + 可选 source_ip），返回 `SqlParseResult`
/// `{ probes, aggregated, reconstructed, parsed_payloads }`。
///
/// 与 `scan_log_file` 的区别：本命令不读日志文件、不依赖 `LogEntry`，前端
/// 直接粘贴 SQL 文本列表即可还原数据库结构（T5-10 GUI 调用）。
///
/// v0.7.0：PreprocessView 表头「SQL 解析」按钮复用本命令，将该列全部
/// 非空行作为 `SqlParseInput[]` 提交，结果预填到 `sqlParseResult`，
/// 然后跳转到 Tools/Sql 子面板复用现有 UI 继续调整或重跑。
#[tauri::command]
pub fn parse_sql_tool(inputs: Vec<SqlParseInput>) -> Result<SqlParseResult, String> {
    Ok(core_parse_sqls(inputs))
}
