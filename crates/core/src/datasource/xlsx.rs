//! XLSX 读取器。取首个工作表，首行作表头，其余行映射为 `Record`。

use calamine::{open_workbook_auto, Reader as CalamineReader};

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::util::cell_to_string;
use super::{Dataset, Reader};

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
    /// 单次解析：打开工作簿一次，首行作表头，其余行映射为 `Record`。
    fn read(&self) -> CoreResult<Dataset> {
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

        let mut rows_iter = range.rows();
        let headers: Vec<String> = match rows_iter.next() {
            Some(header_row) => header_row.iter().map(cell_to_string).collect(),
            None => {
                return Ok(Dataset {
                    headers: Vec::new(),
                    rows: Vec::new(),
                })
            }
        };

        let mut rows: Vec<Record> = Vec::new();
        for row in rows_iter {
            let mut fields = std::collections::HashMap::new();
            for (i, cell) in row.iter().enumerate() {
                let key = headers.get(i).cloned().unwrap_or_else(|| format!("col{i}"));
                let value = cell_to_string(cell);
                fields.insert(key, value);
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
        Ok(self.read()?.headers)
    }
}
