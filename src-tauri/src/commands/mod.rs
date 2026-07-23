//! Tauri commands for RuT0DataKit GUI — 领域子模块入口。
//!
//! v0.5.0 T12-2：原 `commands.rs`（1173 LOC / 32 个 `#[tauri::command]`）按
//! 业务领域拆分为 `commands/` 目录下 10 个子文件 + 本 `mod.rs` 入口。
//! `main.rs` 的 `mod commands;` 由文件模块变为目录模块，`commands::<name>`
//! 路径不变（经本文件 `pub use` 重导出）。
//!
//! 状态策略：每个 command 独立重算（v0.1.0 最简），不在 command 间缓存
//! `MaskResult`。若需缓存可后续引入 `tauri::State<Mutex<...>>`。
//!
//! 子文件划分（按领域内聚）：
//! - [`file`]：文件选择 / 类型探测 / 预览 / 预处理归一化（4 命令）
//! - [`mask`]：脱敏 pipeline + 勾选行/列脱敏 + 导出（6 命令）
//! - [`validate`]：校验 pipeline（2 命令）
//! - [`export`]：records 脱敏后导出 CSV/XLSX/JSON + extract 导出（4 命令）
//! - [`ruleset`]：规则集 YAML 存取 + 标签/内置规则（4 命令）
//! - [`log`]：日志扫描（2 命令）
//! - [`pcap`]：pcap 扫描 + tshark 路径配置（4 命令）
//! - [`extract`]：文本/文件 PII 提取（2 命令）
//! - [`search`]：records 搜索（1 命令）
//! - [`tools`]：正则工具 + SQL 解析工具（3 命令）

mod extract;
mod export;
mod file;
mod log;
mod mask;
mod pcap;
mod ruleset;
mod search;
mod tools;
mod validate;

// ── 共享 helper（被多领域子文件复用，标 pub(super) 限定在 commands 模块内）──

use std::fs::File;
use std::path::Path;

use ruT0_data_kit_core::pipeline::{detect_type, SourceType};
use ruT0_data_kit_core::readers::{CsvReader, SourceReader, XlsxReader};
use ruT0_data_kit_core::rules::{RuleSet, load_ruleset};
use ruT0_data_kit_core::report::csv_report::build_csv_mask_report;

/// 把 [`SourceType`] 序列化为前端可读的小写字符串。
pub(super) fn source_type_name(t: SourceType) -> &'static str {
    match t {
        SourceType::Csv => "csv",
        SourceType::Xlsx => "xlsx",
        SourceType::Log => "log",
        SourceType::Pcap => "pcap",
        SourceType::Sql => "sql",
        SourceType::Json => "json",
        SourceType::Txt => "txt",
        SourceType::Unknown => "unknown",
    }
}

/// 加载规则：`rules_path` 为 None 时用空规则集（v0.4.4：内置规则集已删除，
/// 规则池初始为空，等价于 `RuleSet::default()`），Some 时按路径加载 YAML。
pub(super) fn resolve_rules(rules_path: &Option<String>) -> Result<RuleSet, String> {
    match rules_path {
        Some(p) => load_ruleset(p).map_err(|e| e.to_string()),
        None => Ok(RuleSet::default()),
    }
}

/// 按源类型选择 reader 读取文件，返回 [`ruT0_data_kit_core::readers::Records`]。
pub(super) fn read_records(
    path: &str,
    t: SourceType,
) -> Result<ruT0_data_kit_core::readers::Records, String> {
    let p = Path::new(path);
    match t {
        SourceType::Csv => CsvReader::new().read(p).map_err(|e| e.to_string()),
        SourceType::Xlsx => XlsxReader::new().read(p).map_err(|e| e.to_string()),
        _ => Err("v0.1.0 仅支持 csv/xlsx".into()),
    }
}

/// 执行脱敏 pipeline 并构造报告。返回 `(MaskResult, Report)` 的 JSON 表达。
///
/// 复用给 `run_mask` / `export_masked_csv`，避免状态共享。
pub(super) fn run_mask_pipeline(
    input_path: &str,
    rules_path: &Option<String>,
) -> Result<
    (
        ruT0_data_kit_core::pipeline::MaskResult,
        ruT0_data_kit_core::report::Report,
    ),
    String,
> {
    let t = detect_type(Path::new(input_path)).map_err(|e| e.to_string())?;
    if t != SourceType::Csv && t != SourceType::Xlsx {
        return Err("v0.1.0 仅支持 csv/xlsx".into());
    }
    let rules = resolve_rules(rules_path)?;
    let records = read_records(input_path, t)?;
    let result = ruT0_data_kit_core::pipeline::mask_pipeline(&records, &rules)
        .map_err(|e| e.to_string())?;
    let report = build_csv_mask_report(input_path, &result);
    Ok((result, report))
}

/// 把前端编辑的 `rules_json`（RuleSet 的 JSON 序列化）反序列化到 core 的
/// [`RuleSet`]。
///
/// core 的 `MaskRule.params` 为 `Option<HashMap<String, serde_yml::Value>>`。
/// 实测 serde_json 能直接把 JSON 标量反序列化成 `serde_yml::Value`
/// （Number/String/Bool/Sequence/Mapping 互通），无需中间转换结构。
pub(super) fn parse_ruleset_json(rules_json: &str) -> Result<RuleSet, String> {
    serde_json::from_str::<RuleSet>(rules_json).map_err(|e| format!("rules_json 解析失败: {e}"))
}

/// `read_records` 的无 `SourceType` 入参版本：自动 detect。仅支持 csv/xlsx，
/// 其他类型报错。
pub(super) fn read_records_auto(
    input_path: &str,
) -> Result<ruT0_data_kit_core::readers::Records, String> {
    let t = detect_type(Path::new(input_path)).map_err(|e| e.to_string())?;
    if t != SourceType::Csv && t != SourceType::Xlsx {
        return Err("v0.1.0 仅支持 csv/xlsx".into());
    }
    read_records(input_path, t)
}

/// 用 csv crate 写 UTF-8 CSV（表头 + 数据行）。
pub(super) fn write_csv(
    headers: &[String],
    rows: &[Vec<String>],
    out_path: &str,
) -> Result<(), String> {
    let file = File::create(out_path).map_err(|e| format!("创建文件失败: {e}"))?;
    let mut wtr = csv::Writer::from_writer(file);
    wtr.write_record(headers).map_err(|e| e.to_string())?;
    for row in rows {
        wtr.write_record(row).map_err(|e| e.to_string())?;
    }
    wtr.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// 把 `headers` + `rows` 序列化为 JSON 数组写盘。每行一个对象，key=headers[i]，
/// value=rows[r][i]（字符串）。空表头/空行也合法（输出 `[]` 或 `[{}]`）。
pub(super) fn write_json(
    headers: &[String],
    rows: &[Vec<String>],
    out_path: &str,
) -> Result<(), String> {
    use std::collections::HashMap;
    let arr: Vec<HashMap<&str, &str>> = rows
        .iter()
        .map(|row| {
            headers
                .iter()
                .enumerate()
                .map(|(i, h)| (h.as_str(), row.get(i).map(|s| s.as_str()).unwrap_or("")))
                .collect()
        })
        .collect();
    let json_str = serde_json::to_string(&arr).map_err(|e| format!("JSON 序列化失败: {e}"))?;
    std::fs::write(out_path, json_str).map_err(|e| format!("写入失败: {e}"))
}

// ── 命令重导出：保持 main.rs 的 `commands::<name>` 路径不变 ──
//
// 用 glob 重导出（`pub use file::*`）而非逐项列举：`#[tauri::command]` 宏会
// 在定义命令的子模块内生成隐藏的 `__cmd__<name>` / `__tauri_command_name_<name>`
// 伴随项（`pub` + `#[doc(hidden)]`），`tauri::generate_handler!` 通过
// `commands::<name>` 路径查找时需要这些伴随项也出现在 `commands::` 命名空间下。
// glob 重导出会把子模块所有 pub 项（含 `#[doc(hidden)]`）一并暴露到
// `commands::`，使 main.rs 的 `commands::select_file` 路径及其伴随项都可达。

pub use extract::*;
pub use export::*;
pub use file::*;
pub use log::*;
pub use mask::*;
pub use pcap::*;
pub use ruleset::*;
pub use search::*;
pub use tools::*;
pub use validate::*;
