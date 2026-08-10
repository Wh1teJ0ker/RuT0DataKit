//! v1.0.0 数据类 IPC 命令（导入 / 分页查询 / AI 契约占位）。
//!
//! 含 T5 导入流（CSV / XLSX → DB cells → Table）与 AI IPC 契约占位。

use serde::{Deserialize, Serialize};

use crate::db::Cell;
use ruT0_data_kit_core::datasource;

// ---------------------------------------------------------------------------
// AI IPC 契约占位（业务能力开发中）
// ---------------------------------------------------------------------------

/// AI 上下文输入（camelCase 序列化给前端）。
/// v1.0.0 仅定义契约占位，真实推理逻辑开发中。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContext {
    pub sheet_id: Option<i64>,
    pub selection: Option<Vec<i64>>,
    pub prompt: Option<String>,
}

/// AI 建议输出（camelCase 序列化给前端）。
/// v1.0.0 仅定义契约占位，字段为占位形态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestion {
    pub suggestion: String,
    pub confidence: f64,
}

/// AI 建议占位命令。v1.0.0 永远返回 `Err`，AI 能力开发中。
/// 真实推理逻辑接入前，前端据此展示「AI 能力 开发中」文案。
#[tauri::command]
pub async fn ai_suggest(_context: AiContext) -> Result<AiSuggestion, String> {
    Err("ai_suggest not implemented yet (capability under development)".to_string())
}

/// 通用 AI 操作占位命令。v1.0.0 永远返回 `Err`，接受任意 `op` + `params`。
/// 真实分发逻辑开发中。
#[tauri::command]
pub async fn invoke_ai_op(
    _op: String,
    _params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Err("invoke_ai_op not implemented yet (capability under development)".to_string())
}

// ---------------------------------------------------------------------------
// v1.0.0 导入流（T5）：CSV / XLSX → DB cells → Table
// ---------------------------------------------------------------------------

/// 每批 commit 的最大行数（含表头行）。
const IMPORT_BATCH_ROWS: usize = 5000;

/// 导入结果（camelCase 序列化给前端）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub session_id: i64,
    pub sheet_id: i64,
    pub row_count: u32,
    pub headers: Vec<String>,
}

/// 分页查询结果（camelCase 序列化给前端）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

/// 导入文件命令。
///
/// 解析 CSV/XLSX → `create_session` + `create_sheet` + `write_cells`（每 5000
/// 行 commit 一次）+ `log_operation(kind="import")` → 返回 `ImportResult`。
#[tauri::command]
pub async fn import_file(
    path: String,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<ImportResult, String> {
    // 1. 探测格式 + 单次解析（headers + rows 一次性产出，不再重复 IO）。
    let reader = datasource::detect_format(&path).map_err(|e| e.to_string())?;
    let dataset = reader.read().map_err(|e| e.to_string())?;
    let headers = dataset.headers;
    // row_count 包含表头行（与 DB cells 行数一致：1 header + N data）。
    let row_count = (dataset.rows.len() + 1) as u32;

    // 2. 文件名 stem 作为 session/sheet 名。
    let file_name = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("import")
        .to_string();
    let source_type = std::path::Path::new(&path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("csv")
        .to_string();

    // 3. 写库：session + sheet + 批量 cells。
    let session_id = db
        .create_session(&file_name, Some(&path), &source_type, row_count)
        .map_err(|e| e.to_string())?;
    let sheet_id = db
        .create_sheet(session_id, &file_name, 0)
        .map_err(|e| e.to_string())?;

    // 4. 分批 write_cells：每 5000 行一批 commit。
    //    首行（row_idx=0）写表头值，其后为数据行。表头行由 headers 直接构造，
    //    不再依赖 reader 二次产出，与单次解析语义对齐。
    let mut row_idx: u32 = 0;
    // 表头行单独成批（row_idx=0）。
    let header_cells: Vec<Cell> = headers
        .iter()
        .enumerate()
        .map(|(col_idx, h)| Cell {
            sheet_id,
            row_idx: 0,
            col_idx: col_idx as u32,
            value: Some(h.clone()).filter(|v| !v.is_empty()),
        })
        .collect();
    db.write_cells(sheet_id, &header_cells)
        .map_err(|e| e.to_string())?;
    row_idx = row_idx.saturating_add(1);
    for chunk in dataset.rows.chunks(IMPORT_BATCH_ROWS) {
        let mut cells: Vec<Cell> = Vec::with_capacity(chunk.len() * headers.len());
        for record in chunk {
            for (col_idx, key) in headers.iter().enumerate() {
                let value = record.fields.get(key).cloned().filter(|v| !v.is_empty());
                cells.push(Cell {
                    sheet_id,
                    row_idx,
                    col_idx: col_idx as u32,
                    value,
                });
            }
            row_idx = row_idx.saturating_add(1);
        }
        db.write_cells(sheet_id, &cells)
            .map_err(|e| e.to_string())?;
    }

    // 5. 操作日志。
    let params_json = serde_json::json!({
        "path": path,
        "row_count": row_count,
    })
    .to_string();
    db.log_operation(Some(sheet_id), "import", &params_json, "{}")
        .map_err(|e| e.to_string())?;

    Ok(ImportResult {
        session_id,
        sheet_id,
        row_count,
        headers,
    })
}

/// 分页查询 Sheet 数据。
///
/// `db.query_cells` → 按 `row_idx` 分组为 `Vec<Vec<Option<String>>>`；
/// headers 从首行 cell（row_idx=0）推导。
#[tauri::command]
pub async fn get_sheet_data(
    sheet_id: i64,
    page: u32,
    page_size: u32,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<PageData, String> {
    let cells = db
        .query_cells(sheet_id, page, page_size)
        .map_err(|e| e.to_string())?;
    let total = db.count_rows(sheet_id).map_err(|e| e.to_string())?;

    // headers：从 row_idx=0 的 cell value 取（import_file 写入时首行即表头值）。
    // 先尝试从本页取 row_idx=0；若本页不含首行（page>1），单独查首页首行。
    let mut headers: Vec<String> = cells
        .iter()
        .filter(|c| c.row_idx == 0)
        .map(|c| c.value.clone().unwrap_or_default())
        .collect();
    if headers.is_empty() {
        let first_page = db
            .query_cells(sheet_id, 1, page_size.max(1))
            .map_err(|e| e.to_string())?;
        headers = first_page
            .iter()
            .filter(|c| c.row_idx == 0)
            .map(|c| c.value.clone().unwrap_or_default())
            .collect();
    }
    // 按 col_idx 排序对齐（query_cells 已 ASC，这里稳定）。
    let col_count = headers.len().max(1);

    // 按 row_idx 分组，组内按 col_idx 排序（query_cells 已 ASC）。
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<u32, Vec<(u32, Option<String>)>> = BTreeMap::new();
    for c in &cells {
        grouped
            .entry(c.row_idx)
            .or_default()
            .push((c.col_idx, c.value.clone()));
    }

    // 行内按 col_idx 填充；缺列补 None；跳过表头行（row_idx=0）。
    let mut rows: Vec<Vec<Option<String>>> = Vec::with_capacity(grouped.len());
    for (row_idx, mut cells_row) in grouped {
        if row_idx == 0 {
            continue;
        }
        cells_row.sort_by_key(|(col, _)| *col);
        let mut row = vec![None; col_count];
        for (col, value) in cells_row {
            if (col as usize) < row.len() {
                row[col as usize] = value;
            }
        }
        rows.push(row);
    }

    Ok(PageData {
        headers,
        rows,
        total,
        page,
        page_size,
    })
}
