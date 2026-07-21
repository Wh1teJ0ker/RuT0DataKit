//! TXT 数据源读取器：把 `.txt` 文件整段文本读成单 cell Records。
//!
//! 规则（v0.4.3 最小可用）：
//! - `std::fs::read_to_string(path)?` 读全文（若文件含非 UTF-8 字节会报错，
//!   符合现有 core 一贯口径）。
//! - headers = `["content"]`，rows = `vec![vec![content]]`（单行单列）。
//! - 给 PreprocessView 预览用；数据提取模块（extract）走自己的
//!   `extract_file` 不经此 reader。
//!
//! 限制：大文件（如 8.1MB data.txt）整段读入内存，但不影响功能
//! （PreprocessView 只显示预览前若干行）。

use std::path::Path;

use crate::error::CoreError;
use crate::readers::{Records, SourceReader};

/// TXT 读取器：把整段文本读成单 cell Records。
#[derive(Debug, Default, Clone, Copy)]
pub struct TxtReader;

impl TxtReader {
    pub fn new() -> Self {
        Self
    }
}

impl SourceReader for TxtReader {
    fn read(&self, path: &Path) -> Result<Records, CoreError> {
        let content = std::fs::read_to_string(path)?;
        Ok(Records {
            headers: vec!["content".to_string()],
            rows: vec![vec![content]],
        })
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
    fn reads_whole_text_into_single_cell() {
        let text = "line1\nline2\nline3\n";
        let p = write_tmp("rut0_txt_basic.txt", text);
        let rec = TxtReader::new().read(&p).expect("read txt");
        assert_eq!(rec.headers, vec!["content".to_string()]);
        assert_eq!(rec.rows.len(), 1);
        assert_eq!(rec.rows[0].len(), 1);
        assert_eq!(rec.rows[0][0], text);
    }

    #[test]
    fn empty_file_yields_single_empty_cell() {
        let p = write_tmp("rut0_txt_empty.txt", "");
        let rec = TxtReader::new().read(&p).expect("read empty txt");
        assert_eq!(rec.headers, vec!["content".to_string()]);
        assert_eq!(rec.rows.len(), 1);
        assert_eq!(rec.rows[0][0], "");
    }

    #[test]
    fn missing_file_errors() {
        let res = TxtReader::new().read(Path::new("/nonexistent/RuT0DataKit/none.txt"));
        assert!(res.is_err());
    }
}
