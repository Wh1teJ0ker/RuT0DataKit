//! XLSX 数据源读取器：用 `calamine` 把 `.xlsx` 读成 [`Records`]。
//!
//! 规则：读首个 sheet 的 `Range`，第一行作 headers，其余作数据行。所有 cell
//! 转为 `String`（数字按 `Display` 转，布尔转 `true/false`，空 cell 为 `""`，
//! 日期按 ISO 字符串渲染）。行长度按 headers 对齐补 `""`。

use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};

use crate::error::CoreError;
use crate::readers::{Records, SourceReader};

/// XLSX 读取器。
#[derive(Debug, Default, Clone, Copy)]
pub struct XlsxReader;

impl XlsxReader {
    pub fn new() -> Self {
        Self
    }
}

/// 把 `calamine::Data` 转成字符串。空 cell → `""`。
fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Int(n) => n.to_string(),
        Data::Float(f) => {
            // 整数值不输出小数尾巴，避免 "123" → "123.0"。
            if f.fract() == 0.0 && f.is_finite() {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => format!("{dt:?}"),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(err) => format!("{err:?}"),
    }
}

impl SourceReader for XlsxReader {
    fn read(&self, path: &Path) -> Result<Records, CoreError> {
        let mut book = open_workbook_auto(path).map_err(|e| {
            CoreError::InvalidInput(format!("xlsx open error: {e}"))
        })?;

        // 取首个 sheet。
        let sheet_name = book
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| CoreError::InvalidInput("xlsx: no sheet".into()))?;

        let range = book
            .worksheet_range(&sheet_name)
            .map_err(|e| CoreError::InvalidInput(format!("xlsx sheet range error: {e}")))?;

        let mut rows_iter = range.rows();
        let header_row = rows_iter
            .next()
            .ok_or_else(|| CoreError::InvalidInput("xlsx: empty sheet".into()))?;
        let headers: Vec<String> = header_row.iter().map(cell_to_string).collect();
        let n_cols = headers.len();

        let mut rows: Vec<Vec<String>> = Vec::new();
        for r in rows_iter {
            let mut row: Vec<String> = r.iter().map(cell_to_string).collect();
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

    #[test]
    fn missing_xlsx_errors() {
        let res = XlsxReader::new().read(Path::new("/nonexistent/RuT0DataKit/none.xlsx"));
        assert!(res.is_err());
    }

    #[test]
    fn reads_sample_mask_xlsx_fixture() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/samples/csv/sample_mask.xlsx");
        if !p.exists() {
            // fixture 不存在时跳过（仅本机环境），不阻断其它平台跑测试。
            eprintln!("skip xlsx fixture test: {p:?} not found");
            return;
        }
        let rec = XlsxReader::new().read(&p).expect("read xlsx");
        assert_eq!(rec.headers, vec![
            "customer_id".to_string(),
            "name".to_string(),
            "id_card".to_string(),
            "phone".to_string(),
            "email".to_string(),
            "bank_card".to_string(),
        ]);
        assert_eq!(rec.rows.len(), 2);
        assert_eq!(rec.rows[0][0], "12345678");
        assert_eq!(rec.rows[0][1], "张三");
        assert_eq!(rec.rows[0][3], "13812345678");
        assert_eq!(rec.rows[1][5], "6225881111222233");
    }
}
