//! 数据源读取模块。
//!
//! v1.0.0（T5）：实现 CSV / XLSX 两种格式的 `Reader`。
//! - CSV：基于 `csv` crate，首行作为表头。
//! - XLSX：基于 `calamine` crate，取首个工作表，首行作为表头。
//!
//! v1.1+ 新增数据源（如 PCAP）只需实现 `Reader` trait 并在 `detect_format`
//! 工厂按扩展名分发，不改动 processor / db / table 模块。

use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader as CalamineReader};

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

/// 数据源读取器 trait。
pub trait Reader: Send + Sync {
    /// 读取全部记录（含表头行；表头行 `row_idx=0`）。
    fn read_all(&self) -> CoreResult<Vec<Record>>;

    /// 返回列名（首行）。
    fn headers(&self) -> CoreResult<Vec<String>>;
}

/// CSV 读取器。首行作为表头；其余行按表头映射为 `Record.fields`。
pub struct CsvReader {
    path: String,
}

impl CsvReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Reader for CsvReader {
    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(&self.path)
            .map_err(|e| CoreError::DataSource(format!("csv open: {e}")))?;

        let mut iter = rdr.records();
        let mut records: Vec<Record> = Vec::new();
        let mut headers: Option<Vec<String>> = None;

        // 第一行作表头，同时作为 row_idx=0 的 Record 写回（保留首行）。
        if let Some(header_result) = iter.next() {
            let header = header_result.map_err(|e| {
                CoreError::DataSource(format!("csv header read: {e}"))
            })?;
            let header_vec: Vec<String> =
                header.iter().map(|s| s.to_string()).collect();
            let mut fields = std::collections::HashMap::new();
            for h in &header_vec {
                fields.insert(h.clone(), h.clone());
            }
            records.push(Record { fields });
            headers = Some(header_vec);
        }

        let header_ref = headers.as_deref();
        for record_result in iter {
            let row = record_result
                .map_err(|e| CoreError::DataSource(format!("csv row read: {e}")))?;
            let mut fields = std::collections::HashMap::new();
            for (i, v) in row.iter().enumerate() {
                let key = header_ref
                    .and_then(|h| h.get(i))
                    .cloned()
                    .unwrap_or_else(|| format!("col{}", i));
                fields.insert(key, v.to_string());
            }
            records.push(Record { fields });
        }

        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(&self.path)
            .map_err(|e| CoreError::DataSource(format!("csv open: {e}")))?;
        let mut iter = rdr.records();
        let header = iter
            .next()
            .ok_or_else(|| CoreError::DataSource("csv empty file".into()))?
            .map_err(|e| CoreError::DataSource(format!("csv header read: {e}")))?;
        Ok(header.iter().map(|s| s.to_string()).collect())
    }
}

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
        let mut workbook =
            open_workbook_auto(&self.path).map_err(|e| {
                CoreError::DataSource(format!("xlsx open: {e}"))
            })?;
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

        let headers: Option<Vec<String>> = rows.next().map(|header_row| {
            header_row
                .iter()
                .map(cell_to_string)
                .collect()
        });

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
        let mut workbook =
            open_workbook_auto(&self.path).map_err(|e| {
                CoreError::DataSource(format!("xlsx open: {e}"))
            })?;
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

/// `calamine::Data` 转字符串。
fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::DateTimeIso(dt) => dt.to_string(),
        Data::DurationIso(d) => d.to_string(),
        Data::DateTime(_dt) => format!("{:?}", cell),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            // 整数浮点避免尾部 `.0`。
            if f.fract() == 0.0 && f.is_finite() {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Data::Bool(b) => b.to_string(),
        Data::Error(err) => format!("{:?}", err),
    }
}

/// 格式探测工厂：按扩展名分发到具体 Reader。
///
/// 支持扩展名：`.csv` → `CsvReader`，`.xlsx` → `XlsxReader`。
/// 其它扩展名返回 `NotImplemented`。
pub fn detect_format(path: &str) -> CoreResult<Box<dyn Reader>> {
    let p = Path::new(path);
    match p.extension().and_then(|e| e.to_str()) {
        Some("csv") => Ok(Box::new(CsvReader::new(path))),
        Some("xlsx") => Ok(Box::new(XlsxReader::new(path))),
        _ => Err(CoreError::NotImplemented("datasource::detect_format")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_csv(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new()
            .suffix(".csv")
            .tempfile()
            .unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn detect_format_routes_by_extension() {
        let csv = tmp_csv("a,b\n1,2\n");
        assert!(detect_format(csv.path().to_str().unwrap()).is_ok());
        let err = detect_format("/tmp/foo.txt").err().unwrap();
        assert!(matches!(err, CoreError::NotImplemented(_)));
    }

    #[test]
    fn csv_reader_headers_and_rows() {
        let f = tmp_csv("a,b\n1,2\n3,4\n");
        let reader = CsvReader::new(f.path().to_str().unwrap());
        assert_eq!(reader.headers().unwrap(), vec!["a".to_string(), "b".into()]);
        let recs = reader.read_all().unwrap();
        // 表头作为首行 record 也包含在内（与 import_file 行为对齐）。
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[1].fields.get("a").map(|s| s.as_str()), Some("1"));
    }

    #[test]
    fn csv_reader_flexible_columns() {
        let f = tmp_csv("a,b\n1\n3,4,5\n");
        let reader = CsvReader::new(f.path().to_str().unwrap());
        let recs = reader.read_all().unwrap();
        assert_eq!(recs[1].fields.get("a").map(|s| s.as_str()), Some("1"));
        assert_eq!(recs[2].fields.get("b").map(|s| s.as_str()), Some("4"));
        // 第三列无表头时回退 col2
        assert_eq!(recs[2].fields.get("col2").map(|s| s.as_str()), Some("5"));
    }
}
