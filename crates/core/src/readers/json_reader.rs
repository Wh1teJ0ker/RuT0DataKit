//! JSON 数据源读取器：把 `.json` 文件读成 [`Records`]。
//!
//! 规则（v0.4.0 最小可用）：
//! - JSON 数组对象：headers = 各对象 keys 的并集（按首次出现序），rows = 每个对象
//!   按该并集顺序取值；缺失 key 对应 cell 补 `""`。
//! - JSON 单对象：包成 1 行（headers = 对象 keys）。
//! - JSON Lines（每行一对象）：与数组对象同处理。
//! - 非对象 JSON（纯标量数组 / 纯标量 / 字符串）：单列 `value`，每个标量一行。
//!
//! 所有 cell 渲染为字符串：null → `""`，布尔 → `true/false`，数字按
//! `serde_json::Number` 的 Display，字符串原样（不解 JSON 转义，serde 已解一层）。
//!
//! 限制：不支持嵌套对象自动展开（嵌套 Value 会 `to_string()` 成 JSON 串放到
//! 单 cell），留 v0.4.1+。

use std::path::Path;

use serde_json::Value;

use crate::error::CoreError;
use crate::readers::{Records, SourceReader};

/// JSON 读取器。
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonReader;

impl JsonReader {
    pub fn new() -> Self {
        Self
    }
}

impl SourceReader for JsonReader {
    fn read(&self, path: &Path) -> Result<Records, CoreError> {
        let content = std::fs::read_to_string(path)?;
        let value = parse_json_or_jsonl(&content)?;
        Ok(value_to_records(value))
    }
}

/// 把文件内容解析为 JSON `Value`：
///
/// - 先按整体 JSON 解析（支持数组 / 单对象 / 标量）。
/// - 整体解析失败但内容形如「每行一 JSON 对象」（JSON Lines）时，逐行解析
///   并打包为 `Value::Array`。
pub fn parse_json_or_jsonl(content: &str) -> Result<Value, CoreError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return Ok(v);
    }
    // 整体解析失败：尝试 JSON Lines。
    let mut items = Vec::new();
    let mut all_lines_objects = true;
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(v) if v.is_object() => items.push(v),
            _ => {
                all_lines_objects = false;
                break;
            }
        }
    }
    if all_lines_objects && !items.is_empty() {
        return Ok(Value::Array(items));
    }
    Err(CoreError::InvalidInput(
        "json parse failed: not valid JSON nor JSON Lines".into(),
    ))
}

/// 把 `Value` 映射为 `Records`：
///
/// - `Value::Array` 中全部为对象 → 对象 keys 并集为 headers。
/// - `Value::Array` 中全部为非对象（标量）→ 单列 `value`。
/// - `Value::Array` 元素类型混合（对象 + 标量）→ 标量元素按单列 `value`，
///   缺失其它列补 `""`（按统一对象并集处理：把标量也视作 keys=`[]` 的对象）。
/// - `Value::Object` → 包成 1 行。
/// - 其它（标量）→ 1 行 1 列 `value`。
pub fn value_to_records(value: Value) -> Records {
    match value {
        Value::Array(arr) => {
            if arr.is_empty() {
                return Records {
                    headers: vec!["value".to_string()],
                    rows: Vec::new(),
                };
            }
            // 全为标量 → 单列 value。
            if arr.iter().all(|v| !v.is_object()) {
                let rows = arr.into_iter().map(|v| vec![scalar_to_string(&v)]).collect();
                return Records {
                    headers: vec!["value".to_string()],
                    rows,
                };
            }
            // 至少一个对象 → 取对象 keys 并集（按首次出现序）。
            let mut headers: Vec<String> = Vec::new();
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for v in &arr {
                if let Some(obj) = v.as_object() {
                    for k in obj.keys() {
                        if seen.insert(k.clone()) {
                            headers.push(k.clone());
                        }
                    }
                }
            }
            let n_cols = headers.len();
            let idx: std::collections::HashMap<String, usize> = headers
                .iter()
                .enumerate()
                .map(|(i, k)| (k.clone(), i))
                .collect();
            let rows = arr
                .into_iter()
                .map(|v| {
                    let mut row = vec![String::new(); n_cols];
                    if let Some(obj) = v.as_object() {
                        for (k, val) in obj {
                            if let Some(&i) = idx.get(k) {
                                row[i] = scalar_to_string(val);
                            }
                        }
                    } else {
                        // 混合数组中的标量元素：填入 value 列（若存在）。
                        if let Some(&i) = idx.get("value") {
                            row[i] = scalar_to_string(&v);
                        }
                    }
                    row
                })
                .collect();
            Records { headers, rows }
        }
        Value::Object(obj) => {
            let headers: Vec<String> = obj.keys().cloned().collect();
            let idx: std::collections::HashMap<String, usize> = headers
                .iter()
                .enumerate()
                .map(|(i, k)| (k.clone(), i))
                .collect();
            let mut row = vec![String::new(); headers.len()];
            for (k, v) in obj {
                if let Some(&i) = idx.get(&k) {
                    row[i] = scalar_to_string(&v);
                }
            }
            Records {
                headers,
                rows: vec![row],
            }
        }
        // 标量（string/number/bool/null）
        v => Records {
            headers: vec!["value".to_string()],
            rows: vec![vec![scalar_to_string(&v)]],
        },
    }
}

/// 把标量 `Value` 渲染为字符串：null → `""`，bool → `true/false`，
/// 数字 → Display，字符串 → 原值。嵌套对象/数组 → JSON 串（最小实现）。
fn scalar_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        // 嵌套结构最小处理：序列化为 JSON 串放入单 cell。
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, content: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(name);
        let mut f = std::fs::File::create(&dir).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn array_of_objects_union_keys_in_first_seen_order() {
        let json = r#"[
            {"id": 1, "name": "a"},
            {"name": "b", "email": "x@y"}
        ]"#;
        let p = write_tmp("rut0_json_arr.json", json);
        let rec = JsonReader::new().read(&p).expect("read json");
        assert_eq!(rec.headers, vec!["id", "name", "email"]);
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0], vec!["1".to_string(), "a".to_string(), "".to_string()]);
        assert_eq!(rec.rows[1], vec!["".to_string(), "b".to_string(), "x@y".to_string()]);
    }

    #[test]
    fn single_object_wraps_one_row() {
        let json = r#"{"a": 1, "b": true}"#;
        let p = write_tmp("rut0_json_obj.json", json);
        let rec = JsonReader::new().read(&p).expect("read json");
        assert_eq!(rec.headers, vec!["a", "b"]);
        assert_eq!(rec.rows.len(), 1);
        assert_eq!(rec.rows[0], vec!["1".to_string(), "true".to_string()]);
    }

    #[test]
    fn json_lines_array_of_objects() {
        let json = "{\"x\": 1}\n{\"x\": 2, \"y\": \"q\"}\n";
        let p = write_tmp("rut0_jsonl.json", json);
        let rec = JsonReader::new().read(&p).expect("read json");
        assert_eq!(rec.headers, vec!["x", "y"]);
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0], vec!["1".to_string(), "".to_string()]);
        assert_eq!(rec.rows[1], vec!["2".to_string(), "q".to_string()]);
    }

    #[test]
    fn scalar_array_single_value_column() {
        let json = r#"[1, "two", true, null]"#;
        let p = write_tmp("rut0_json_scalar.json", json);
        let rec = JsonReader::new().read(&p).expect("read json");
        assert_eq!(rec.headers, vec!["value"]);
        assert_eq!(rec.rows.len(), 4);
        assert_eq!(rec.rows[0], vec!["1".to_string()]);
        assert_eq!(rec.rows[1], vec!["two".to_string()]);
        assert_eq!(rec.rows[2], vec!["true".to_string()]);
        assert_eq!(rec.rows[3], vec!["".to_string()]);
    }

    #[test]
    fn empty_array_yields_value_header_no_rows() {
        let p = write_tmp("rut0_json_empty.json", "[]");
        let rec = JsonReader::new().read(&p).expect("read json");
        assert_eq!(rec.headers, vec!["value"]);
        assert!(rec.rows.is_empty());
    }

    #[test]
    fn invalid_json_errors() {
        let p = write_tmp("rut0_json_bad.json", "not json");
        let res = JsonReader::new().read(&p);
        assert!(res.is_err());
    }

    #[test]
    fn missing_file_errors() {
        let res = JsonReader::new().read(Path::new("/nonexistent/RuT0DataKit/none.json"));
        assert!(res.is_err());
    }
}
