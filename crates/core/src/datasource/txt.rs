//! TXT 读取器：按行/定长分块读为多行 `content`。
//!
//! 历史版本（≤ v1.1.4）把整个 TXT 文件读成单 cell `content`，对 8.5 MB 无换行
//! 文件会产出一条巨型记录，导致 DB 分页/搜索/提取失效。本版本改为：
//! - 含换行符的 TXT → 按 `\n` / `\r\n` 分割，每行一条 Record（保留空行）。
//! - 无换行符或仅 1 行的 TXT → 按 4096 字节定长分块，每块一条 Record
//!   （最后一块可能不足 4096；分块边界回退到 UTF-8 char boundary，不截断多字节字符）。
//! - 空文件 → 0 条数据行（headers 仍为 `["content"]`）。
//!
//! 这样 8.5 MB 无换行 TXT 将产出约 2080 行，DB 分页/搜索/提取恢复正常。

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::{Dataset, Reader};

/// 单个定长分块的最大字节数（仅对无换行或单行 TXT 生效）。
const CHUNK_SIZE: usize = 4096;

/// TXT 读取器：按行/定长分块读为多行 `content`。
pub struct TxtReader {
    path: String,
}

impl TxtReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Reader for TxtReader {
    /// 单次解析：优先按行分割；无换行或单行超长则按 4096 字节定长分块。
    fn read(&self) -> CoreResult<Dataset> {
        let bytes = std::fs::read(&self.path)
            .map_err(|e| CoreError::DataSource(format!("txt open: {e}")))?;
        let content = String::from_utf8_lossy(&bytes);

        // 按 \n / \r\n 分割（lines() 兼容两者）。
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        let rows: Vec<Record> = if lines.len() <= 1 {
            // 无换行或仅 1 行 → 定长分块。
            let single = lines.into_iter().next().unwrap_or_default();
            if single.is_empty() {
                Vec::new() // 空文件
            } else {
                chunk_string(&single, CHUNK_SIZE)
                    .into_iter()
                    .map(|chunk| {
                        let mut fields = std::collections::HashMap::new();
                        fields.insert("content".to_string(), chunk);
                        Record { fields }
                    })
                    .collect()
            }
        } else {
            // 多行 → 每行一条 Record（含空行）。
            lines
                .into_iter()
                .map(|line| {
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("content".to_string(), line);
                    Record { fields }
                })
                .collect()
        };

        Ok(Dataset {
            headers: vec!["content".to_string()],
            rows,
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

/// 按定长 `chunk_size` 字节分块，分块边界回退到 UTF-8 char boundary，
/// 确保不截断多字节字符。
///
/// 极端情况：`chunk_size` 比单个 char 还小（例如 1 字节对 3 字节中文）
/// 时，强制按一个完整 char 推进，避免死循环与空块。
fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0;
    while start < s.len() {
        let mut end = (start + chunk_size).min(s.len());
        // 回退到 char boundary，避免在多字节字符中间截断。
        while end < s.len() && !s.is_char_boundary(end) {
            end -= 1;
        }
        if end <= start {
            // 极端情况：chunk_size 比单个 char 还小 → 强制按一个完整 char 推进。
            let char_len = s[start..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            end = (start + char_len).min(s.len());
        }
        result.push(s[start..end].to_string());
        start = end;
    }
    result
}

// -----------------------------------------------------------------------
// 测试
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tempfile(content: &[u8]) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".txt").tempfile().unwrap();
        f.write_all(content).unwrap();
        f
    }

    fn content_field(record: &Record) -> &str {
        record.fields.get("content").map(|s| s.as_str()).unwrap_or("")
    }

    // ---- 空文件 → 0 行 ----

    #[test]
    fn empty_file_returns_no_rows() {
        let f = write_tempfile(b"");
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { headers, rows } = reader.read().unwrap();
        assert_eq!(headers, vec!["content".to_string()]);
        assert!(rows.is_empty(), "空文件应返回 0 行，实际 {}", rows.len());
    }

    // ---- 普通多行 TXT → 按行分割（含空行）----

    #[test]
    fn multiline_txt_splits_by_lines() {
        let f = write_tempfile(b"line1\nline2\nline3\n");
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { headers, rows } = reader.read().unwrap();
        assert_eq!(headers, vec!["content".to_string()]);
        assert_eq!(rows.len(), 3);
        assert_eq!(content_field(&rows[0]), "line1");
        assert_eq!(content_field(&rows[1]), "line2");
        assert_eq!(content_field(&rows[2]), "line3");
    }

    #[test]
    fn multiline_with_crlf_splits() {
        let f = write_tempfile(b"alpha\r\nbeta\r\n");
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { rows, .. } = reader.read().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(content_field(&rows[0]), "alpha");
        assert_eq!(content_field(&rows[1]), "beta");
    }

    #[test]
    fn multiline_preserves_empty_lines() {
        let f = write_tempfile(b"a\n\nb\n");
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { rows, .. } = reader.read().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(content_field(&rows[0]), "a");
        assert_eq!(content_field(&rows[1]), "");
        assert_eq!(content_field(&rows[2]), "b");
    }

    // ---- 单行短 TXT → 1 行 ----

    #[test]
    fn single_short_line_returns_one_row() {
        let f = write_tempfile(b"hello world");
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { headers, rows } = reader.read().unwrap();
        assert_eq!(headers, vec!["content".to_string()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(content_field(&rows[0]), "hello world");
    }

    // ---- 单行长 TXT（> 4096 字节无换行）→ 多行分块 ----

    #[test]
    fn single_long_line_chunks_to_multiple_rows() {
        // 4096 是分块边界：8192 字节纯 ASCII → 2 块。
        let big = "a".repeat(8192);
        let f = write_tempfile(big.as_bytes());
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { rows, .. } = reader.read().unwrap();
        assert!(
            rows.len() >= 2,
            "8192 字节无换行应产出 ≥ 2 行，实际 {}",
            rows.len()
        );
        // 每块不超过 4096 字节（最后一块可能更短）。
        for r in &rows {
            assert!(content_field(r).len() <= 4096);
        }
        // 拼接后等于原文。
        let joined: String = rows.iter().map(content_field).collect();
        assert_eq!(joined.len(), 8192);
        assert!(joined.chars().all(|c| c == 'a'));
    }

    #[test]
    fn exactly_4096_bytes_one_chunk() {
        // 恰好 4096 字节 → 1 块（边界不触发分块）。
        let big = "b".repeat(4096);
        let f = write_tempfile(big.as_bytes());
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { rows, .. } = reader.read().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(content_field(&rows[0]).len(), 4096);
    }

    #[test]
    fn chunk_boundary_just_over_4096() {
        // 4097 字节 → 2 块（4096 + 1）。
        let big = "c".repeat(4097);
        let f = write_tempfile(big.as_bytes());
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { rows, .. } = reader.read().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(content_field(&rows[0]).len(), 4096);
        assert_eq!(content_field(&rows[1]).len(), 1);
    }

    // ---- UTF-8 多字节字符在分块边界不被截断 ----

    #[test]
    fn utf8_multibyte_not_truncated_at_chunk_boundary() {
        // "中" 是 3 字节 UTF-8（0xE4 0xB8 0xAD）。构造一个字符串，使 4096
        // 字节边界正好落在 "中" 中间：前面 4094 字节 ASCII + 一个 "中"
        // （占 4094..4097），再补一个 "文" 让总长 > 4096 触发分块。
        let mut s = String::new();
        s.push_str(&"x".repeat(4094)); // 0..4094
        s.push('中'); // 4094..4097（横跨 4096 边界）
        s.push('文'); // 4097..4100
        let f = write_tempfile(s.as_bytes());
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { rows, .. } = reader.read().unwrap();
        assert_eq!(rows.len(), 2);
        // 第一块回退到 char boundary → 4094 字节（不应是 4096，因为 4096 不是边界）。
        let first = content_field(&rows[0]);
        assert_eq!(first.len(), 4094);
        // 每块必须是合法 UTF-8（String 本身保证，但显式校验语义）。
        assert!(first.chars().all(|c| c == 'x'));
        // 第二块应包含完整的 "中文"。
        let second = content_field(&rows[1]);
        assert_eq!(second, "中文");
    }

    #[test]
    fn utf8_multibyte_joined_equals_original() {
        // 大量中文，确保分块后拼接仍等于原文。
        let s = "中文测试".repeat(2000); // 2000 * 12 = 24000 字节
        let f = write_tempfile(s.as_bytes());
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let Dataset { rows, .. } = reader.read().unwrap();
        assert!(rows.len() >= 2);
        let joined: String = rows.iter().map(content_field).collect();
        assert_eq!(joined, s);
    }

    #[test]
    fn chunk_string_with_tiny_size_advances_by_char() {
        // 极端情况：chunk_size=1 但字符是 3 字节中文 → 强制按 char 推进。
        let chunks = chunk_string("中文", 1);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], "中");
        assert_eq!(chunks[1], "文");
    }

    #[test]
    fn chunk_string_empty_input() {
        assert!(chunk_string("", 4096).is_empty());
    }

    #[test]
    fn chunk_string_single_ascii_chunk() {
        let chunks = chunk_string("abc", 4096);
        assert_eq!(chunks, vec!["abc".to_string()]);
    }

    // ---- headers 始终为 ["content"] ----

    #[test]
    fn headers_always_content() {
        let f = write_tempfile(b"");
        let reader = TxtReader::new(f.path().to_str().unwrap());
        assert_eq!(reader.headers().unwrap(), vec!["content".to_string()]);

        let f2 = write_tempfile(b"a\nb\nc\n");
        let reader2 = TxtReader::new(f2.path().to_str().unwrap());
        assert_eq!(reader2.headers().unwrap(), vec!["content".to_string()]);

        let f3 = write_tempfile(&b"z".repeat(10000));
        let reader3 = TxtReader::new(f3.path().to_str().unwrap());
        assert_eq!(reader3.headers().unwrap(), vec!["content".to_string()]);
    }

    // ---- read_all 兼容：表头作为 row_idx=0 ----

    #[test]
    fn read_all_includes_header_as_first_row() {
        let f = write_tempfile(b"foo\nbar\n");
        let reader = TxtReader::new(f.path().to_str().unwrap());
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 3); // 1 header + 2 data
        assert_eq!(content_field(&records[0]), "content");
        assert_eq!(content_field(&records[1]), "foo");
        assert_eq!(content_field(&records[2]), "bar");
    }
}
