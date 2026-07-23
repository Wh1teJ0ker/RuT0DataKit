//! records 脱敏后导出 CSV/XLSX/JSON + extract 导出命令（4 命令）。

use std::collections::HashSet;

use ruT0_data_kit_core::pipeline::mask_pipeline_columns;

use super::{parse_ruleset_json, read_records_auto, write_csv, write_json};

/// 按 `column_order` 顺序选出在 `selected_columns` 中的列，返回投影后的
/// `(headers, rows)`。
///
/// - `column_order` 中不在原 `headers` 的列名忽略。
/// - `selected_columns` 控制哪些列被保留；`column_order` 控制顺序。
/// - `column_order` 为空时退化为「按 selected_columns 顺序，再按 headers 顺序」。
fn project_columns(
    records: &ruT0_data_kit_core::readers::Records,
    selected_columns: &[String],
    column_order: &[String],
) -> (Vec<String>, Vec<Vec<String>>) {
    let selected: HashSet<&str> = selected_columns.iter().map(|s| s.as_str()).collect();
    let header_idx: std::collections::HashMap<&str, usize> = records
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.as_str(), i))
        .collect();

    // 决定输出列序：优先 column_order，column_order 为空时按 selected_columns 顺序。
    let order: Vec<&str> = if !column_order.is_empty() {
        column_order
            .iter()
            .map(|s| s.as_str())
            .filter(|name| selected.contains(name) && header_idx.contains_key(*name))
            .collect()
    } else {
        selected_columns
            .iter()
            .map(|s| s.as_str())
            .filter(|name| header_idx.contains_key(*name))
            .collect()
    };

    let out_headers: Vec<String> = order
        .iter()
        .map(|name| records.headers[header_idx[*name]].clone())
        .collect();
    let out_indices: Vec<usize> = order.iter().map(|name| header_idx[*name]).collect();
    let out_rows: Vec<Vec<String>> = records
        .rows
        .iter()
        .map(|row| {
            out_indices
                .iter()
                .map(|&i| row.get(i).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
    (out_headers, out_rows)
}

/// 用 rust_xlsxwriter 写 XLSX（表头第 0 行，数据从第 1 行起）。
fn write_xlsx(headers: &[String], rows: &[Vec<String>], out_path: &str) -> Result<(), String> {
    use rust_xlsxwriter::{Format, Workbook};
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let header_fmt = Format::new().set_bold();
    for (c, h) in headers.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, h, &header_fmt)
            .map_err(|e| e.to_string())?;
    }
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            ws.write_string((r + 1) as u32, c as u16, cell)
                .map_err(|e| e.to_string())?;
        }
    }
    wb.save(out_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// 应用列脱敏 → 按 `column_order` 投影列 → 可选按行索引过滤 → 写 CSV。
///
/// `selected_row_indices = None` 表示导出全部行；`Some(vec)` 只导出指定行号
/// （0-based，越界忽略）。投影后行号按原 records 顺序。
#[tauri::command]
pub fn export_records_csv(
    input_path: String,
    rules_json: String,
    selected_columns: Vec<String>,
    column_order: Vec<String>,
    selected_row_indices: Option<Vec<usize>>,
    out_path: String,
) -> Result<(), String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = read_records_auto(&input_path)?;
    let selected: HashSet<String> = selected_columns.iter().cloned().collect();
    let result = mask_pipeline_columns(&records, &rules, &selected).map_err(|e| e.to_string())?;
    let (headers, mut rows) = project_columns(&result.masked, &selected_columns, &column_order);
    if let Some(indices) = selected_row_indices {
        let idx_set: HashSet<usize> = indices.into_iter().collect();
        rows = rows
            .into_iter()
            .enumerate()
            .filter(|(i, _)| idx_set.contains(i))
            .map(|(_, r)| r)
            .collect();
    }
    write_csv(&headers, &rows, &out_path)
}

/// 与 [`export_records_csv`] 同签名，但写 XLSX（rust_xlsxwriter）。
#[tauri::command]
pub fn export_records_xlsx(
    input_path: String,
    rules_json: String,
    selected_columns: Vec<String>,
    column_order: Vec<String>,
    selected_row_indices: Option<Vec<usize>>,
    out_path: String,
) -> Result<(), String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = read_records_auto(&input_path)?;
    let selected: HashSet<String> = selected_columns.iter().cloned().collect();
    let result = mask_pipeline_columns(&records, &rules, &selected).map_err(|e| e.to_string())?;
    let (headers, mut rows) = project_columns(&result.masked, &selected_columns, &column_order);
    if let Some(indices) = selected_row_indices {
        let idx_set: HashSet<usize> = indices.into_iter().collect();
        rows = rows
            .into_iter()
            .enumerate()
            .filter(|(i, _)| idx_set.contains(i))
            .map(|(_, r)| r)
            .collect();
    }
    write_xlsx(&headers, &rows, &out_path)
}

/// 与 [`export_records_csv`] 同签名同流程，但写 JSON（v0.6.0 T5-8）。
///
/// 流程：read_records_auto → mask_pipeline_columns → project_columns →
/// 可选行过滤 → 把每行按 `column_order` 映射为 `HashMap<String, String>`
/// （headers 做 key），用 `serde_json` 序列化为 JSON 数组写盘。每行扁平对象，
/// value 全为字符串，不做嵌套（v0.6.0 scope）。
#[tauri::command]
pub fn export_records_json(
    input_path: String,
    rules_json: String,
    selected_columns: Vec<String>,
    column_order: Vec<String>,
    selected_row_indices: Option<Vec<usize>>,
    out_path: String,
) -> Result<(), String> {
    let rules = parse_ruleset_json(&rules_json)?;
    let records = read_records_auto(&input_path)?;
    let selected: HashSet<String> = selected_columns.iter().cloned().collect();
    let result = mask_pipeline_columns(&records, &rules, &selected).map_err(|e| e.to_string())?;
    let (headers, mut rows) = project_columns(&result.masked, &selected_columns, &column_order);
    if let Some(indices) = selected_row_indices {
        let idx_set: HashSet<usize> = indices.into_iter().collect();
        rows = rows
            .into_iter()
            .enumerate()
            .filter(|(i, _)| idx_set.contains(i))
            .map(|(_, r)| r)
            .collect();
    }
    write_json(&headers, &rows, &out_path)
}

/// 提取结果条目（前端友好结构）。
#[derive(serde::Serialize, serde::Deserialize)]
struct ExtractItem {
    #[serde(rename = "type")]
    r#type: String,
    value: String,
}

/// 把 findings 按指定格式（txt/csv/json）写到 out_path。
///
/// txt 格式：每行 `type_value`（小写 type + 下划线，匹配 PDF spec）。
/// csv 格式：type,value 两列（复用 write_csv）。
/// json 格式：结构化数组（复用 write_json）。
#[tauri::command]
pub fn export_extract(
    findings_json: String,
    format: String,
    out_path: String,
) -> Result<(), String> {
    let items: Vec<ExtractItem> = serde_json::from_str(&findings_json)
        .map_err(|e| format!("findings_json 解析失败: {e}"))?;
    match format.as_str() {
        "txt" => {
            let mut buf = String::new();
            for it in &items {
                buf.push_str(&format!("{}_{}\n", it.r#type, it.value));
            }
            // 去掉末尾多余换行
            if buf.ends_with('\n') {
                buf.pop();
            }
            std::fs::write(&out_path, buf).map_err(|e| format!("写入失败: {e}"))
        }
        "csv" => {
            let headers = vec!["type".to_string(), "value".to_string()];
            let rows: Vec<Vec<String>> = items
                .iter()
                .map(|it| vec![it.r#type.clone(), it.value.clone()])
                .collect();
            write_csv(&headers, &rows, &out_path)
        }
        "json" => {
            let headers = vec!["type".to_string(), "value".to_string()];
            let rows: Vec<Vec<String>> = items
                .iter()
                .map(|it| vec![it.r#type.clone(), it.value.clone()])
                .collect();
            write_json(&headers, &rows, &out_path)
        }
        _ => Err(format!("unsupported format: {format}")),
    }
}

// extract 模块的测试与 extract 命令耦合（package_extract_result / resolve_extract_rules
// 在 extract.rs），此处 export 子文件不再单独承载 tests。
