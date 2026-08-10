//! JSON / JSONL 读取器。
//!
//! - `.json`：文件可为一个对象（单行）或对象数组，数组按对象展开。
//! - `.jsonl`：每行一个 JSON 对象。
//!
//! headers 取所有对象键的并集（首次出现顺序）；首行 record 为表头行
//! （`key=value=headers[i]`），其后每个对象按 headers 顺序 zip 成 `Record`。

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::util::value_to_string;
use super::{Dataset, Reader};

/// JSON / JSONL 读取器。
pub struct JsonReader {
    path: String,
}

impl JsonReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Reader for JsonReader {
    /// 单次解析：一次性收集所有对象 → 计算 headers 并集（首次出现顺序）
    /// → 按 headers 顺序展开每个对象为数据行。不再二次解析。
    fn read(&self) -> CoreResult<Dataset> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| CoreError::DataSource(format!("json open: {e}")))?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Ok(Dataset {
                headers: Vec::new(),
                rows: Vec::new(),
            });
        }

        // 收集所有 JSON 对象（兼容 JSON 数组 + JSONL）。
        let mut objects: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
        let first = trimmed.chars().next().unwrap();
        if first == '[' {
            // 整体 JSON 数组。
            let arr: Vec<serde_json::Value> = serde_json::from_str(trimmed)?;
            for v in arr {
                if let serde_json::Value::Object(obj) = v {
                    objects.push(obj);
                }
            }
        } else if first == '{' {
            // 整体单个对象 或 JSONL。
            // 尝试整体解析；失败再按行解析。
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(serde_json::Value::Object(obj)) => objects.push(obj),
                Ok(_) => {}
                Err(_) => {
                    for line in trimmed.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        let v: serde_json::Value = serde_json::from_str(line)
                            .map_err(|e| CoreError::InvalidInput(format!("jsonl parse: {e}")))?;
                        if let serde_json::Value::Object(obj) = v {
                            objects.push(obj);
                        }
                    }
                }
            }
        }

        // headers：所有对象键并集（首次出现顺序，稳定）。
        let mut headers: Vec<String> = Vec::new();
        for obj in &objects {
            for k in obj.keys() {
                if !headers.contains(k) {
                    headers.push(k.clone());
                }
            }
        }

        // 数据行：按 headers 顺序 zip。
        let mut rows: Vec<Record> = Vec::with_capacity(objects.len());
        for obj in &objects {
            let mut fields = std::collections::HashMap::new();
            for h in &headers {
                let v = obj.get(h).map(value_to_string).unwrap_or_default();
                fields.insert(h.clone(), v);
            }
            rows.push(Record { fields });
        }
        Ok(Dataset { headers, rows })
    }

    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let Dataset { headers, rows } = self.read()?;
        let mut records: Vec<Record> = Vec::with_capacity(rows.len() + 1);
        let mut h_fields = std::collections::HashMap::new();
        for h in &headers {
            h_fields.insert(h.clone(), h.clone());
        }
        records.push(Record { fields: h_fields });
        records.extend(rows);
        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        // 直接复用 read() 的单次解析结果，保留稳定 header 顺序。
        Ok(self.read()?.headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_json(content: &str, suffix: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn json_read_single_object() {
        let f = tmp_json(r#"{"a":1,"b":"x"}"#, ".json");
        let Dataset { headers, rows } = JsonReader::new(f.path().to_str().unwrap()).read().unwrap();
        assert_eq!(headers, vec!["a".to_string(), "b".into()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fields.get("a").map(|s| s.as_str()), Some("1"));
        assert_eq!(rows[0].fields.get("b").map(|s| s.as_str()), Some("x"));
    }

    #[test]
    fn json_read_array() {
        let f = tmp_json(r#"[{"a":1},{"a":2,"b":"y"}]"#, ".json");
        let Dataset { headers, rows } = JsonReader::new(f.path().to_str().unwrap()).read().unwrap();
        // 并集首次出现顺序：a 先，b 后。
        assert_eq!(headers, vec!["a".to_string(), "b".into()]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fields.get("a").map(|s| s.as_str()), Some("1"));
        assert!(rows[0]
            .fields
            .get("b")
            .map(|s| s.is_empty())
            .unwrap_or(true));
        assert_eq!(rows[1].fields.get("a").map(|s| s.as_str()), Some("2"));
        assert_eq!(rows[1].fields.get("b").map(|s| s.as_str()), Some("y"));
    }

    #[test]
    fn json_read_jsonl() {
        let f = tmp_json("{\"a\":1}\n{\"a\":2,\"b\":\"z\"}\n", ".jsonl");
        let Dataset { headers, rows } = JsonReader::new(f.path().to_str().unwrap()).read().unwrap();
        assert_eq!(headers, vec!["a".to_string(), "b".into()]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].fields.get("b").map(|s| s.as_str()), Some("z"));
    }

    #[test]
    fn json_read_empty_file() {
        let f = tmp_json("   \n", ".json");
        let Dataset { headers, rows } = JsonReader::new(f.path().to_str().unwrap()).read().unwrap();
        assert!(headers.is_empty());
        assert!(rows.is_empty());
    }

    #[test]
    fn json_header_order_stable_across_calls() {
        // 多次 read() 返回的 header 顺序必须稳定一致。
        let f = tmp_json(r#"[{"c":1,"a":2,"b":3}]"#, ".json");
        let reader = JsonReader::new(f.path().to_str().unwrap());
        let h1 = reader.headers().unwrap();
        let h2 = reader.headers().unwrap();
        assert_eq!(h1, vec!["c".to_string(), "a".into(), "b".into()]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn json_read_all_includes_header_row() {
        // read_all() 兼容旧形态：首行为表头 record。
        let f = tmp_json(r#"{"a":1}"#, ".json");
        let recs = JsonReader::new(f.path().to_str().unwrap())
            .read_all()
            .unwrap();
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].fields.get("a").map(|s| s.as_str()), Some("a"));
        assert_eq!(recs[1].fields.get("a").map(|s| s.as_str()), Some("1"));
    }
}
