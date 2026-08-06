//! XLSX 读取器。取首个工作表，首行作表头，其余行映射为 `Record`。

use calamine::{open_workbook_auto, Reader as CalamineReader};

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::util::cell_to_string;
use super::Reader;

/// XLSX 读取器。取首个工作表，首行作表头，其余行映射为 `Record`。
pub struct XlsxReader {
    path: String,
}

impl XlsxReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Reader for XlsxReader {
    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let mut workbook = open_workbook_auto(&self.path)
            .map_err(|e| CoreError::DataSource(format!("xlsx open: {e}")))?;
        let sheet_name = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| CoreError::DataSource("xlsx no sheet".into()))?;

        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| CoreError::DataSource(format!("xlsx range: {e}")))?;

        let mut rows = range.rows();
        let mut records: Vec<Record> = Vec::new();

        let headers: Option<Vec<String>> = rows
            .next()
            .map(|header_row| header_row.iter().map(cell_to_string).collect());

        // 首行作为表头同时也是 row_idx=0 的 Record。
        if let Some(h) = headers.as_ref() {
            let mut fields = std::collections::HashMap::new();
            for v in h {
                fields.insert(v.clone(), v.clone());
            }
            records.push(Record { fields });
        }

        let header_ref = headers.as_deref();
        for row in rows {
            let mut fields = std::collections::HashMap::new();
            for (i, cell) in row.iter().enumerate() {
                let key = header_ref
                    .and_then(|h| h.get(i))
                    .cloned()
                    .unwrap_or_else(|| format!("col{}", i));
                let value = cell_to_string(cell);
                fields.insert(key, value);
            }
            records.push(Record { fields });
        }

        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        let mut workbook = open_workbook_auto(&self.path)
            .map_err(|e| CoreError::DataSource(format!("xlsx open: {e}")))?;
        let sheet_name = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| CoreError::DataSource("xlsx no sheet".into()))?;
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| CoreError::DataSource(format!("xlsx range: {e}")))?;
        let header_row = range
            .rows()
            .next()
            .ok_or_else(|| CoreError::DataSource("xlsx empty sheet".into()))?;
        Ok(header_row.iter().map(cell_to_string).collect())
    }
}
