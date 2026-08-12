# TASK-T73-HANDOFF

```yaml
task_id: T73
goal: |
  在 src-tauri 新增 `hash_column` Tauri 命令，对指定列做 MD5 / SHA1 / SHA256 哈希变换
  （不可逆，输出 hex 小写）。复用既有 `base64_transform_column_cells` 闭包式列变换
  DB 方法（允许重命名为更通用的 `transform_column_cells`），扩展 `list_undoable_operations`
  的 kind IN 白名单包含 `hash_column`。新增 3 个 hash crate 依赖。

in_scope:
  - src-tauri/Cargo.toml（新增 md-5 / sha1 / sha2 / hex 直接依赖）
  - src-tauri/src/commands/columns.rs（新增 HashAlgorithm enum + HashResult struct + hash_column_inner + #[tauri::command] hash_column + 单元测试）
  - src-tauri/src/db/mod.rs（base64_transform_column_cells → 重命名为 transform_column_cells，或保留旧名新增 alias；扩展 list_undoable_operations kind IN 白名单加 hash_column）
  - src-tauri/src/lib.rs（generate_handler! 注册 hash_column）

out_of_scope:
  - 不改 crates/core（哈希逻辑放在 src-tauri，沿用 base64 先例；crates/core 不引入 hash 依赖）
  - 不改前端（T74 负责）
  - 不改 detect_format / datasource（T75 负责）
  - 不改 DB schema（不加表/列；SCHEMA_VERSION 不变）
  - 不改 capabilities/default.json（自定义命令无需注册权限）
  - 不改 CryptoPanel.jsx
  - 不创建 tag / 不合并 main

acceptance_criteria:
  - `hash_column(sheet_id, column, algorithm=MD5, db)` 对 "hello" 输出 "5d41402abc4b2a76b9719d911017c592"
  - `hash_column(sheet_id, column, algorithm=SHA1, db)` 对 "hello" 输出 "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
  - `hash_column(sheet_id, column, algorithm=SHA256, db)` 对 "hello" 输出 "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
  - 空值行（None）不计入 affected 也不计入 skipped（与 base64_column 语义一致）
  - hash_column 操作写入 operations 表，kind="hash_column"，含 before/after snapshot（可撤销）
  - `list_undoable_operations` 返回的 kind IN 白名单包含 "hash_column"
  - `undo_operation` 能恢复 hash_column 操作（before_snapshot 回写）
  - cargo fmt --all --check exit 0
  - cargo clippy --all-targets --all-features -- -D warnings exit 0
  - cargo test --all 全绿（新增测试不破坏既有 302 passed）

verification_commands:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all

files_likely_to_change:
  - src-tauri/Cargo.toml
  - src-tauri/src/commands/columns.rs
  - src-tauri/src/db/mod.rs
  - src-tauri/src/lib.rs

risks:
  - 重命名 base64_transform_column_cells 可能破坏既有调用方（base64_column_inner）；要么保留旧名 + 新增别名，要么同步改调用点
  - list_undoable_operations 的 kind IN 白名单是硬编码 SQL，必须加 hash_column 否则 undo 栈不显示
  - sha2 / hex 已在 Cargo.lock 作为传递依赖，但未在 Cargo.toml 声明；声明为直接依赖不会额外网络拉取
  - md-5 / sha-1 crate 不在 Cargo.lock，需要网络拉取（Rust 生态标准 crate，无安全顾虑）

depends_on: []
status: planned
```

## 实现指引

### HashAlgorithm enum

```rust
/// 哈希算法选择（serde lowercase，与前端 mode 字符串对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    MD5,
    SHA1,
    SHA256,
}
```

### hash_column_inner 实现

镜像 `base64_column_inner`（columns.rs:271-318），区别：
- 闭包永远返回 `Some(hex_string)`（哈希永远成功，任意字节可哈希）
- `log_operation_with_snapshot` kind = `"hash_column"`
- 闭包按 algorithm 分发：
  - MD5 → `md5::compute(val.as_bytes())` → `format!("{:x}", digest)`（md-5 crate 0.10）
  - SHA1 → `Sha1::digest(val.as_bytes())` → `hex::encode(digest)`（sha1 crate 0.10 + hex 0.4）
  - SHA256 → `Sha256::digest(val.as_bytes())` → `hex::encode(digest)`（sha2 crate 0.10）

### transform_column_cells 重命名策略（推荐）

保留 `base64_transform_column_cells` 旧名（避免破坏既有 base64_column_inner 调用 + db/mod.rs 测试），新增 `pub alias transform_column_cells = base64_transform_column_cells` 不可行（Rust 无 type alias for method）。**推荐：保留旧名不改**，hash_column_inner 直接调用 `db.base64_transform_column_cells(sheet_id, col_idx, hash_closure)` — 闭包是泛型 `F: Fn(&str) -> Option<String>`，与函数名无关。函数名虽然带 base64，但语义是通用列变换，沿用既有名不破坏调用链。

### list_undoable_operations 白名单扩展

`src-tauri/src/db/mod.rs` `list_undoable_operations` 方法（~L1464）的 SQL：
```sql
SELECT ... WHERE kind IN ('mask', 'replace_in_column', 'replace_all', 'base64_column')
```
追加 `'hash_column'`：
```sql
WHERE kind IN ('mask', 'replace_in_column', 'replace_all', 'base64_column', 'hash_column')
```
参数绑定不变（硬编码 IN 不需要参数化，无注入风险）。

### 测试用例（columns.rs tests mod）

- `hash_column_md5_known_vector`：sheet 1 列 3 行 ["hello","world",""]，MD5 后第 1 行 = "5d41402abc4b2a76b9719d911017c592"
- `hash_column_sha1_known_vector`：同上 SHA1 → "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
- `hash_column_sha256_known_vector`：同上 SHA256 → "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
- `hash_column_undo_restores_original`：hash 后 undo → 原文恢复
- `hash_column_in_undo_list`：hash 后 list_undoable_operations 包含 kind="hash_column"
- `hash_column_empty_value_not_counted`：空值行不影响 affected/skipped 计数

## 参考

- 既有 `base64_column_inner`：`src-tauri/src/commands/columns.rs:271-318`
- 既有 `base64_transform_column_cells`：`src-tauri/src/db/mod.rs:1379`
- `list_undoable_operations` 白名单：`src-tauri/src/db/mod.rs:~1464`
- `log_operation_with_snapshot`：既有，无需改
- generate_handler! 注册：`src-tauri/src/lib.rs:47-82`
