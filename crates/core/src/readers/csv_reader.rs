//! CSV 数据源读取器：用 `csv` crate 把 `.csv` 读成 [`Records`]。
//!
//! 规则：首行作 headers，其余作数据行。所有 cell 转为 `String`，空 cell 为 `""`。
//! 不假设列数恒定：每行 cell 数若不足 headers 长度，缺位补 `""`；若超出，
//! 多余 cell 保留在 row 里（调用方按 header 索引访问时自行截断）。

use std::path::Path;

use crate::error::CoreError;
use crate::readers::{Records, SourceReader};

/// CSV 读取器。
#[derive(Debug, Default, Clone, Copy)]
pub struct CsvReader;

impl CsvReader {
    pub fn new() -> Self {
        Self
    }
}

impl SourceReader for CsvReader {
    fn read(&self, path: &Path) -> Result<Records, CoreError> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(path)?;
        let headers = rdr
            .headers()?
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let n_cols = headers.len();

        let mut rows: Vec<Vec<String>> = Vec::new();
        for rec in rdr.records() {
            let rec = rec?;
            let mut row: Vec<String> = rec.iter().map(|s| s.to_string()).collect();
            if row.len() < n_cols {
                row.resize(n_cols, String::new());
            }
            rows.push(row);
        }
        Ok(Records { headers, rows })
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
    fn reads_sample_mask_csv() {
        let csv = "customer_id,name,id_card,phone,email,bank_card\n\
                   12345678,张三,110101199001011234,13812345678,zhangsan@example.com,6225887654321098\n\
                   87654321,李四海,110101198505051234,13987654321,lisi@x.cn,6225881111222233\n";
        let p = write_tmp("rut0_mask_sample.csv", csv);
        let rec = CsvReader::new().read(&p).expect("read csv");
        assert_eq!(rec.headers, vec![
            "customer_id", "name", "id_card", "phone", "email", "bank_card"
        ]);
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0][0], "12345678");
        assert_eq!(rec.rows[1][5], "6225881111222233");
    }

    #[test]
    fn short_row_is_padded_with_empty() {
        let csv = "a,b,c\n1,2\n";
        let p = write_tmp("rut0_short.csv", csv);
        let rec = CsvReader::new().read(&p).expect("read csv");
        assert_eq!(rec.rows[0], vec!["1".to_string(), "2".to_string(), "".to_string()]);
    }

    #[test]
    fn missing_file_errors() {
        let res = CsvReader::new().read(Path::new("/nonexistent/RuT0DataKit/none.csv"));
        assert!(res.is_err());
    }

    #[test]
    fn empty_csv_only_headers() {
        let csv = "h1,h2\n";
        let p = write_tmp("rut0_empty.csv", csv);
        let rec = CsvReader::new().read(&p).expect("read csv");
        assert_eq!(rec.headers, vec!["h1".to_string(), "h2".to_string()]);
        assert!(rec.rows.is_empty());
    }
}
