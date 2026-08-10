//! TXT 读取器：整段文本读成单 cell `content`。

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::{Dataset, Reader};

/// TXT 读取器：整段文本读成单 cell `content`。
pub struct TxtReader {
    path: String,
}

impl TxtReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Reader for TxtReader {
    /// 单次解析：整段文本作为单 cell `content`。
    fn read(&self) -> CoreResult<Dataset> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| CoreError::DataSource(format!("txt open: {e}")))?;
        let mut fields = std::collections::HashMap::new();
        fields.insert("content".to_string(), content);
        Ok(Dataset {
            headers: vec!["content".to_string()],
            rows: vec![Record { fields }],
        })
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
        Ok(vec!["content".to_string()])
    }
}
