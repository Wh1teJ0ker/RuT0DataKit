//! records 搜索命令（1 命令）。

/// 对预处理后的 `Records`（前端传入 `headers` + `rows`）跑搜索，返回命中列表。
///
/// `query_json` 是 core `SearchQuery` 的 JSON 序列化，形如：
/// - `{"kind":"keyword","terms":["foo"],"mode":"or"}`
/// - `{"kind":"regex","pattern":"@example\\.com$"}`
/// - `{"kind":"exact_field","field":"name","value":"Alice"}`
///
/// 与 `preprocess_file` 配合：前端先调 `preprocess_file` 拿到 `{headers, rows}`，
/// 再把同样的数据 + 查询条件传到这里。不在 Tauri State 缓存索引——每次现建，
/// v0.4.0 大文件（10w×10）性能基线 < 5s 满足交互式需求。
///
/// 返回 `SearchResult { hits: Vec<SearchHit> }`，每条 hit 含 row/col/field/
/// value/snippet（snippet ±20 字符上下文，UTF-8 友好）。非法正则返回
/// `Err`，不 panic（core 端 `CoreError::InvalidInput` → 字符串）。
#[tauri::command]
pub fn search_records(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    query_json: String,
) -> Result<ruT0_data_kit_core::search::SearchResult, String> {
    let query: ruT0_data_kit_core::search::SearchQuery =
        serde_json::from_str(&query_json).map_err(|e| format!("query_json 解析失败: {e}"))?;
    let records = ruT0_data_kit_core::readers::Records { headers, rows };
    ruT0_data_kit_core::search::search_records(&records, &query).map_err(|e| e.to_string())
}
