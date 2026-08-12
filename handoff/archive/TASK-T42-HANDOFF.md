```yaml
task_id: T42
goal: |
  新增 .log 文件导入支持：新建 LogReader（按行读取，每行一条记录，单列 "line"），
  在 detect_format 注册 .log 扩展名，在前端导入对话框 filter 加 "log"。
in_scope:
  - crates/core/src/datasource/log.rs（新建 LogReader）
  - crates/core/src/datasource/mod.rs（mod log + pub use + detect_format 注册 + 测试）
  - frontend/src/components/layout/TopToolbar.jsx（extensions 加 "log"）
out_of_scope:
  - 不改 import_file 命令（格式无关）
  - 不改前端 IPC / reducer / factory（格式无关）
  - 不做 .log 结构化字段解析（仅按行导入为单列文本）
  - 不改其他 Reader
acceptance_criteria:
  - detect_format("test.log") 返回 Ok(Box<LogReader>)
  - LogReader 对多行 .log 文件：headers() 返回 ["line"]，read_all() 返回表头行 + 每行一条记录
  - 空行保留为空字符串记录（不跳过）
  - 空文件：headers() 返回 ["line"]，read_all() 返回表头行 + 0 数据行
  - detect_format 测试覆盖 .log 扩展名
  - 前端导入对话框 filter 含 "log"
  - cargo test --workspace 全绿
  - pnpm build 通过
verification_commands:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
  - pnpm --prefix frontend build
files_likely_to_change:
  - crates/core/src/datasource/log.rs
  - crates/core/src/datasource/mod.rs
  - frontend/src/components/layout/TopToolbar.jsx
risks:
  - 大 .log 文件可能内存压力大（与 TxtReader 同样全量读取；import_file 有 5000 行批量写入，但 read_all 是全量读入）
  - 编码问题：用 read_to_string，非 UTF-8 文件会 panic；改用 read + from_utf8_lossy 更安全
depends_on: []
status: planned
```

## 实现指引

### 新建 crates/core/src/datasource/log.rs

参考 txt.rs 模式，但按行拆分而非整段：

```rust
//! LOG 读取器：按行读取 .log 文件，每行一条记录，单列 "line"。

use crate::error::{CoreError, CoreResult};
use crate::model::Record;

use super::Reader;

/// LOG 读取器：按行读取，每行一条记录。
pub struct LogReader {
    path: String,
}

impl LogReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Reader for LogReader {
    fn read_all(&self) -> CoreResult<Vec<Record>> {
        let bytes = std::fs::read(&self.path)
            .map_err(|e| CoreError::DataSource(format!("log open: {e}")))?;
        let content = String::from_utf8_lossy(&bytes);
        let mut records: Vec<Record> = Vec::new();
        // 表头行。
        let mut h_fields = std::collections::HashMap::new();
        h_fields.insert("line".to_string(), "line".to_string());
        records.push(Record { fields: h_fields });
        // 数据行：按行拆分，空行保留为空串。
        for line in content.lines() {
            let mut fields = std::collections::HashMap::new();
            fields.insert("line".to_string(), line.to_string());
            records.push(Record { fields });
        }
        Ok(records)
    }

    fn headers(&self) -> CoreResult<Vec<String>> {
        Ok(vec!["line".to_string()])
    }
}
```

**注意**：用 `std::fs::read` + `String::from_utf8_lossy` 而非 `read_to_string`，避免非 UTF-8 文件 panic。

### mod.rs 注册

在 `mod.rs` 加：

```rust
mod log;
pub use log::LogReader;
```

在 `detect_format` 加：

```rust
Some("log") => Ok(Box::new(LogReader::new(path))),
```

在 `detect_format_routes_by_extension` 测试加：

```rust
assert!(detect_format("/tmp/foo.log").is_ok());
```

### TopToolbar.jsx

在 extensions 数组加 `"log"`：

```javascript
extensions: [
  "csv", "xlsx", "json", "jsonl", "sql", "txt", "pcap", "pcapng", "log",
],
```

### 测试（在 log.rs 内）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_reader_reads_lines() {
        use std::io::Write;
        let mut f = tempfile::Builder::new().suffix(".log").tempfile().unwrap();
        f.write_all(b"INFO start\nERROR crash\nWARN retry\n").unwrap();
        let reader = LogReader::new(f.path().to_str().unwrap());
        assert_eq!(reader.headers().unwrap(), vec!["line"]);
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 4); // 1 header + 3 data
        assert_eq!(records[1].fields.get("line").unwrap(), "INFO start");
        assert_eq!(records[2].fields.get("line").unwrap(), "ERROR crash");
        assert_eq!(records[3].fields.get("line").unwrap(), "WARN retry");
    }

    #[test]
    fn log_reader_preserves_empty_lines() {
        use std::io::Write;
        let mut f = tempfile::Builder::new().suffix(".log").tempfile().unwrap();
        f.write_all(b"line1\n\nline3\n").unwrap();
        let reader = LogReader::new(f.path().to_str().unwrap());
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 4); // 1 header + 3 data (含空行)
        assert_eq!(records[1].fields.get("line").unwrap(), "line1");
        assert_eq!(records[2].fields.get("line").unwrap(), "");
        assert_eq!(records[3].fields.get("line").unwrap(), "line3");
    }

    #[test]
    fn log_reader_empty_file() {
        use std::io::Write;
        let mut f = tempfile::Builder::new().suffix(".log").tempfile().unwrap();
        f.write_all(b"").unwrap();
        let reader = LogReader::new(f.path().to_str().unwrap());
        assert_eq!(reader.headers().unwrap(), vec!["line"]);
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 1); // 仅表头行
    }
}
```

需要在 crates/core 的 dev-dependencies 有 tempfile（检查是否已有）。
