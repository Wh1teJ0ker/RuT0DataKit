//! CSV 读取器。首行作为表头；其余行按表头映射为 `Record.fields`。

use csv::ReaderBuilder;

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::{Dataset, Reader};

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
    /// 单次解析：读取首行表头 + 其余数据行，不重新打开文件。
    fn read(&self) -> CoreResult<Dataset> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(&self.path)
            .map_err(|e| CoreError::DataSource(format!("csv open: {e}")))?;

        let mut iter = rdr.records();

        // 第一行作表头。
        let headers: Vec<String> = match iter.next() {
            Some(header_result) => header_result
                .map_err(|e| CoreError::DataSource(format!("csv header read: {e}")))?
                .iter()
                .map(|s| s.to_string())
                .collect(),
            None => {
                return Ok(Dataset {
                    headers: Vec::new(),
                    rows: Vec::new(),
                })
            }
        };

        let mut rows: Vec<Record> = Vec::new();
        for record_result in iter {
            let row =
                record_result.map_err(|e| CoreError::DataSource(format!("csv row read: {e}")))?;
            let mut fields = std::collections::HashMap::new();
            for (i, v) in row.iter().enumerate() {
                let key = headers.get(i).cloned().unwrap_or_else(|| format!("col{i}"));
                fields.insert(key, v.to_string());
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
        // 复用 read() 的单次解析，避免再开一次文件。
        Ok(self.read()?.headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_csv(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
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

    // ---- T60：单次解析路径覆盖 ----

    #[test]
    fn csv_read_single_parse_consistency() {
        // read() 一次性返回 headers 与 rows，且二者保持一致映射。
        let f = tmp_csv("a,b\n1,2\n3,4\n");
        let reader = CsvReader::new(f.path().to_str().unwrap());
        let Dataset { headers, rows } = reader.read().unwrap();
        assert_eq!(headers, vec!["a".to_string(), "b".into()]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fields.get("a").map(|s| s.as_str()), Some("1"));
        assert_eq!(rows[0].fields.get("b").map(|s| s.as_str()), Some("2"));
        assert_eq!(rows[1].fields.get("a").map(|s| s.as_str()), Some("3"));
        assert_eq!(rows[1].fields.get("b").map(|s| s.as_str()), Some("4"));
    }

    #[test]
    fn csv_read_empty_file() {
        let f = tmp_csv("");
        let reader = CsvReader::new(f.path().to_str().unwrap());
        let Dataset { headers, rows } = reader.read().unwrap();
        assert!(headers.is_empty());
        assert!(rows.is_empty());
    }

    #[test]
    fn csv_read_header_only() {
        let f = tmp_csv("a,b\n");
        let reader = CsvReader::new(f.path().to_str().unwrap());
        let Dataset { headers, rows } = reader.read().unwrap();
        assert_eq!(headers, vec!["a".to_string(), "b".into()]);
        assert!(rows.is_empty());
    }

    #[test]
    fn csv_read_irregular_columns() {
        // 行比表头少 / 多列：缺列不补 key，多列回退 colN。
        let f = tmp_csv("a,b\n1\n3,4,5\n");
        let reader = CsvReader::new(f.path().to_str().unwrap());
        let Dataset { headers, rows } = reader.read().unwrap();
        assert_eq!(headers, vec!["a".to_string(), "b".into()]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fields.get("a").map(|s| s.as_str()), Some("1"));
        assert!(rows[0].fields.get("b").is_none());
        assert_eq!(rows[1].fields.get("b").map(|s| s.as_str()), Some("4"));
        assert_eq!(rows[1].fields.get("col2").map(|s| s.as_str()), Some("5"));
    }
}
