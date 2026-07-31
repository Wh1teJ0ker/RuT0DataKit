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
use super::Reader;

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
    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| CoreError::DataSource(format!("json open: {e}")))?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
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

        // headers：所有对象键并集（首次出现顺序）。
        let mut headers: Vec<String> = Vec::new();
        for obj in &objects {
            for k in obj.keys() {
                if !headers.contains(k) {
                    headers.push(k.clone());
                }
            }
        }

        let mut records: Vec<Record> = Vec::new();
        // 表头行。
        let mut h_fields = std::collections::HashMap::new();
        for h in &headers {
            h_fields.insert(h.clone(), h.clone());
        }
        records.push(Record { fields: h_fields });

        // 数据行。
        for obj in &objects {
            let mut fields = std::collections::HashMap::new();
            for h in &headers {
                let v = obj
                    .get(h)
                    .map(value_to_string)
                    .unwrap_or_default();
                fields.insert(h.clone(), v);
            }
            records.push(Record { fields });
        }
        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        let recs = self.read_all()?;
        Ok(recs
            .into_iter()
            .next()
            .map(|r| {
                // 表头行 fields 的 key 即列名；按首次插入顺序（HashMap 无序），
                // 这里直接返回 keys。前端实际从首行 record 取值即可。
                r.fields.into_iter().map(|(k, _)| k).collect()
            })
            .unwrap_or_default())
    }
}
