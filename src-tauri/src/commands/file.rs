//! 文件选择 / 类型探测 / 预览 / 预处理归一化命令（4 命令）。

use std::path::Path;

use ruT0_data_kit_core::pipeline::detect_type;
use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use super::{read_records, source_type_name};

/// 弹出文件选择对话框，返回选中文件的绝对路径字符串。
///
/// 用户取消返回 `Ok(None)`。v0.1.0 用单选模式（一次选一个文件）。
#[tauri::command]
pub async fn select_file(app: AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<std::path::PathBuf>>();
    app.dialog()
        .file()
        .add_filter("数据文件", &["csv", "xlsx", "sql", "json", "log", "pcap", "pcapng", "txt"])
        .pick_file(move |path| {
            let v = path.and_then(|p| p.into_path().ok());
            let _ = tx.send(v);
        });

    let chosen = rx.recv().map_err(|e| e.to_string())?;
    Ok(chosen.map(|p| p.to_string_lossy().into_owned()))
}

/// 调 core 的 `detect_type` 返回数据源类型名（csv/xlsx/log/pcap/unknown）。
#[tauri::command]
pub fn detect_source_type(path: String) -> Result<String, String> {
    let t = detect_type(Path::new(&path)).map_err(|e| e.to_string())?;
    Ok(source_type_name(t).to_string())
}

/// 读取 CSV/XLSX/sql/json/log/pcap/txt 返回预览所需的完整数据。前端自行截断显示行数。
///
/// v0.6.8.2：原 v0.1.0 仅支持 csv/xlsx，与 `preprocess_file` 行为不一致；现统一
/// 委托 core `read_records`，支持全格式。`rows` 为全量数据行（不含表头）。
#[tauri::command]
pub fn load_preview(path: String) -> Result<Value, String> {
    let p = Path::new(&path);
    let t = detect_type(p).map_err(|e| e.to_string())?;
    let records = read_records(&path, t)?;
    Ok(json!({
        "headers": records.headers,
        "rows": records.rows,
        "source_type": source_type_name(t),
        "row_count": records.rows.len(),
    }))
}

/// 把任意支持格式的文件（csv/xlsx/sql/json/pcap/log）统一读成 `Records`，
/// 前端 PreviewView / 后续 mask/validate/export 复用同一份表格。
///
/// - csv/xlsx/sql/json：按后缀走对应 reader。
/// - pcap：调 `PcapRecordsReader` 适配器（tshark 缺失时返回
///   `DependencyMissing("tshark")`，前端弹提示并禁用按钮）。
/// - log：调 `LogRecordsReader` 适配器。
/// - 不识别的后缀：返回 `Err("unsupported source type")`。
///
/// 返回 `{ headers, rows, source_type, row_count }`，与 `load_preview` 形态
/// 对齐，前端可复用同一渲染逻辑。
#[tauri::command]
pub fn preprocess_file(path: String) -> Result<Value, String> {
    let p = Path::new(&path);
    let t = detect_type(p).map_err(|e| e.to_string())?;
    let records = ruT0_data_kit_core::readers::read_records(p).map_err(|e| e.to_string())?;
    Ok(json!({
        "headers": records.headers,
        "rows": records.rows,
        "source_type": source_type_name(t),
        "row_count": records.rows.len(),
    }))
}
