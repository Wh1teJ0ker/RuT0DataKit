//! TXT 读取器：整段文本读成单 cell `content`。

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::Reader;

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
    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| CoreError::DataSource(format!("txt open: {e}")))?;
        let mut records: Vec<Record> = Vec::new();
        // 表头行。
        let mut h_fields = std::collections::HashMap::new();
        h_fields.insert("content".to_string(), "content".to_string());
        records.push(Record { fields: h_fields });
        // 数据行。
        let mut fields = std::collections::HashMap::new();
        fields.insert("content".to_string(), content);
        records.push(Record { fields });
        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        Ok(vec!["content".to_string()])
    }
}
