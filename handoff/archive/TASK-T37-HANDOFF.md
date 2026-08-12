```yaml
task_id: T37
goal: |
  在 columns.rs 新增 base64_column 命令：对指定列做 Base64 编码或解码（就地变更，
  单事务 + before/after 快照，可撤销）。mode="encode" 编码，mode="decode" 解码。
in_scope:
  - src-tauri/src/commands/columns.rs（新增 base64_column_inner + base64_column 命令 + Base64Mode 枚举 + Base64Result 结构体）
  - src-tauri/src/lib.rs（generate_handler! 注册 base64_column）
  - src-tauri/src/db/mod.rs（新增 base64_transform_column_cells 方法，镜像 replace_in_column_cells 模式）
out_of_scope:
  - 不改前端（T38 做）
  - 不改 Cargo.toml 依赖（用 base64 crate 需加依赖，但只在 src-tauri/Cargo.toml 加）
  - 不改其他 commands 模块
  - 不改 schema（operations.kind 是自由文本列）
acceptance_criteria:
  - base64_column_inner(mode="encode") 对 "hello" 列编码后值为 "aGVsbG8="（Base64 标准编码）
  - base64_column_inner(mode="decode") 对 "aGVsbG8=" 列解码后值为 "hello"
  - 解码时遇到非法 Base64 字符串的行被跳过（不 panic），skipped 计数正确
  - operations 表有 before_snapshot_json（供撤销）
  - undo_operation 可恢复原始值
  - 编码 + 解码往返一致（encode → decode 恢复原文）
  - 新增单元测试 ≥ 4 个（编码 / 解码 / 非法输入跳过 / 撤销可恢复）
verification_commands:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
files_likely_to_change:
  - src-tauri/Cargo.toml（加 base64 = "0.22" 依赖）
  - src-tauri/src/commands/columns.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/db/mod.rs
risks:
  - base64 crate 版本需与现有依赖兼容；用 base64 0.22（engine API）
  - 空值（None）行跳过不编码
  - 解码失败行跳过，不中断整体操作
depends_on: []
status: planned
```

## 实现指引

### 依赖

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 加：

```toml
base64 = "0.22"
```

### Base64Mode 枚举

```rust
/// Base64 操作模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Base64Mode {
    Encode,
    Decode,
}
```

### DB 方法 base64_transform_column_cells

在 `db/mod.rs` 新增方法，镜像 `replace_cells_inner` 模式（单事务 + before/after 快照）：

```rust
/// Base64 编/解码列数据。单事务：查 before 快照 → 计算 → 筛变化行 → upsert after。
/// 返回 (affected, before_cells, after_cells)。
pub fn base64_transform_column_cells(
    &self,
    sheet_id: i64,
    col_idx: u32,
    mode: Base64Mode, // 需要 use crate::commands::columns::Base64Mode 或在 db 里定义
) -> Result<(u32, Vec<Cell>, Vec<Cell>), DbError>
```

**安全约束**：SQL 全部用 `?N` 参数绑定，禁止字符串拼接。

### columns.rs 命令

```rust
pub fn base64_column_inner(
    db: &DbManager,
    sheet_id: i64,
    column: &str,
    mode: Base64Mode,
) -> Result<Base64Result, String> {
    // find_col_idx → base64_transform_column_cells → log_operation_with_snapshot → 返回
}
```

`Base64Result { affected: u32, skipped: u32 }`（camelCase）。

### base64 crate 用法

```rust
use base64::{engine::general_purpose::STANDARD, Engine};

// 编码
let encoded = STANDARD.encode(input_bytes);
// 解码
let decoded = STANDARD.decode(input_str)?; // 失败则 skip
```

解码用 `STANDARD.decode()` 的 `Result`，失败行 `skipped += 1`。
编码时 `value` 为 None 的行跳过。

### 注册命令

在 `lib.rs` 的 `generate_handler!` 加 `commands::columns::base64_column`。

### 测试

参考 `replace_in_column_replaces_only_target_column` / `replace_in_column_logs_before_snapshot` 的模式写：

1. `base64_encode_column_basic` — "hello" → "aGVsbG8="
2. `base64_decode_column_basic` — "aGVsbG8=" → "hello"
3. `base64_decode_column_skips_invalid` — 非法 Base64 行跳过
4. `base64_encode_then_decode_roundtrip` — 编码再解码恢复原文
5. `base64_column_logs_before_snapshot` — operations 行有 before_snapshot_json
