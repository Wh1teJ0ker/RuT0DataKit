# T73 REVIEW — hash_column 后端

```yaml
task_id: T73
reviewer_verdict: review_passed
reviewed_at: 2026-08-11
commits_reviewed: [89d1aff, 9e7bcd5]
defects: []
scope_check: none
docs_check: synced
```

## 验收对照

| 验收项 | 结果 | 证据 |
|---|---|---|
| MD5 对 "hello" → 5d41402abc4b2a76b9719d911017c592 | ✅ | columns.rs:360-365 闭包 MD5 分支；测试 `hash_column_md5_known_vector` (columns.rs:848) 断言 vals[0] = 已知向量，实测通过 |
| SHA1 对 "hello" → aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d | ✅ | columns.rs:366-369 闭包 SHA1 分支；测试 `hash_column_sha1_known_vector` (columns.rs:876) 通过 |
| SHA256 对 "hello" → 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824 | ✅ | columns.rs:370-373 闭包 SHA256 分支；测试 `hash_column_sha256_known_vector` (columns.rs:901) 通过 |
| 空值行不计 affected 也不计 skipped | ✅ | db/mod.rs:1431 `None => { /* 空值行不参与变换 */ }`；测试 `hash_column_empty_value_not_counted` (columns.rs:957) 显式写入 None 行，断言 affected=2/skipped=0，通过 |
| 写入 operations 表，kind="hash_column"，含 before/after 快照 | ✅ | columns.rs:396-413 `log_operation_with_snapshot(Some(sheet_id), "hash_column", &params, Some(&before_json), &after_json)`；测试 `hash_column_undo_restores_original` (columns.rs:926) 断言 before_snapshot_json 存在且含原文 |
| list_undoable_operations 白名单含 hash_column | ✅ | db/mod.rs:1470 SQL IN 子句追加 `'hash_column'`；测试 `hash_column_in_undo_list` (columns.rs:949) 断言 hash_column 出现在撤销栈 |
| undo 能恢复原文 | ✅ | 测试 `hash_column_undo_restores_original` (columns.rs:926) 解析 before_snapshot → write_cells 回写 → 断言恢复原文 ["hello","world"] |
| cargo fmt --all --check exit 0 | ✅ | 实测 EXIT=0，无输出 |
| cargo clippy --all-targets --all-features -- -D warnings exit 0 | ✅ | 实测 EXIT=0，无 warning |
| cargo test --lib（src-tauri crate）全绿 | ✅ | 实测 137 passed; 0 failed；新增 6 个 hash_column 测试全部通过 |
| HashAlgorithm enum：`#[serde(rename_all = "lowercase")]` 三变体 MD5/SHA1/SHA256 | ✅ | columns.rs:345-351 |
| HashResult struct：camelCase，affected + skipped 字段 | ✅ | columns.rs:358-360 `#[serde(rename_all = "camelCase")]` + 361-364 两字段 |
| hash_column_inner 复用 db.base64_transform_column_cells（不另写 transform） | ✅ | columns.rs:376-378 调用 `db.base64_transform_column_cells(sheet_id, col_idx, transform)`；db/mod.rs:1379 原方法签名 `F: Fn(&str) -> Option<String>` 闭包式通用 |
| #[tauri::command] hash_column 在 lib.rs generate_handler! 注册 | ✅ | lib.rs:68 `commands::columns::hash_column,` |
| Cargo.toml 加了 md-5/sha1/sha2/hex 依赖 | ✅ | Cargo.toml:25-28 `hex = "0.4"` / `md-5 = "0.10"` / `sha1 = "0.10"` / `sha2 = "0.10"` |

## 验证命令执行结果

- `cargo fmt --all --check`（src-tauri）：EXIT=0，无输出
- `cargo clippy --all-targets --all-features -- -D warnings`（src-tauri）：EXIT=0，`Finished dev profile`，无 warning
- `cargo test --lib -p ruT0-data-kit`（src-tauri crate）：EXIT=0，`test result: ok. 137 passed; 0 failed; 0 ignored`
- `cargo test --lib -p ruT0-data-kit hash_column`（T73 新增测试聚焦）：EXIT=0，6 passed
  - hash_column_md5_known_vector ok
  - hash_column_sha1_known_vector ok
  - hash_column_sha256_known_vector ok
  - hash_column_undo_restores_original ok
  - hash_column_in_undo_list ok
  - hash_column_empty_value_not_counted ok

关于 `cargo test --all`（workspace 全量）的 1 个失败（`datasource::db::tests::db_reader_multi_table_union`）：该测试来自 T75 的未提交 untracked 文件 `crates/core/src/datasource/db.rs`，与 T73 无关。T73 自身 src-tauri crate 测试 137/0 全绿。REPORT 已如实披露并用 `git stash` 验证 baseline。T75 的测试失败属于 T75 范围。

## 安全合规

- **SQL 全部参数绑定**：`list_undoable_operations` 的 kind IN 子句是硬编码常量字符串（`'mask', 'replace_in_column', 'replace_all', 'base64_column', 'hash_column'`），非外部输入拼接；`sheet_id`/`limit` 仍用 `?N` + `params![]` 绑定。`base64_transform_column_cells` 的 SELECT/INSERT 均用 `params![]` 绑定。无注入风险。
- **无凭据字面量**：grep 未发现任何 token/密钥/密码字面量。
- **不创建 tag、不 merge main**：commits 仅在 feat/v1.1.4-r3-hash-dbparse 分支新增两个 commit，无 tag 操作。
- **commit 规范**：两个 commit 均使用 Conventional Commits（`feat(codec):` / `docs(codec):`），单一逻辑目的（实现 / 文档分离），无夹带无关格式化噪音。

## 缺陷清单

无阻塞问题。无 blocker/major/minor 缺陷。

## Scope 核对

scope_check: none。严格遵守 in_scope / out_of_scope：

- 仅改 5 个 in_scope 文件（Cargo.lock、src-tauri/Cargo.toml、src-tauri/src/commands/columns.rs、src-tauri/src/db/mod.rs、src-tauri/src/lib.rs）+ 1 个文档文件（docs/02-技术设计文档.md）
- 未改前端（T74 负责）
- 未改 crates/core（T75 负责；T75 的 untracked 文件不在 T73 的两个 commit 内）
- 未改 CryptoPanel.jsx
- 未改 DB schema（SCHEMA_VERSION 不变，复用 v1.1.1 已加的 before_snapshot_json/result_snapshot_json 列）
- 未改 capabilities/default.json
- 未创建 tag / 未合并 main

与 HANDOFF 实现指引的唯一细微差异：HANDOFF 示例写 `use md5::Md5` 后调用 `Md5::digest`，实际需 `use md5::Digest`（trait 入口）+ `md5::Md5::digest` 才能解析 trait 方法。这是 Rust 0.10 digest API 标准用法，非 scope 偏离，REPORT 已如实说明。

## 文档核对

docs_check: synced。docs/02-技术设计文档.md 在 commit 9e7bcd5 同步：

- §3 模块表 commands::columns 行加 `hash_column`
- §4.9 IPC 表加 hash_column 行 + HashAlgorithm/HashResult struct 示例（lowercase / camelCase，与代码一致）
- §4.9 加 v1.1.4 R3 实现要点段（闭包复用 / 算法分发 / 空值语义 / 白名单扩展 / 新增依赖）
- §4.7 list_undoable_operations 行 + undo/redo kind 集合 + IN 子句常量列表均追加 hash_column
- §5 前端 undoStack 注释追加 hash_column

文档与代码语义一致，未发现文档比代码更乐观的描述。

## 最终意见

T73 实现忠实于 HANDOFF 的 goal/scope/acceptance_criteria：HashAlgorithm 三变体 lowercase 序列化、HashResult camelCase、hash_column_inner 复用既有 base64_transform_column_cells 闭包式列变换（不另写 transform）、三个已知向量测试全绿、空值行语义与 base64 一致、撤销栈白名单已加 hash_column、generate_handler! 已注册、Cargo.toml 已加 md-5/sha1/sha2/hex 依赖、文档同步。验证命令（cargo fmt/clippy/test --lib）全绿，无 blocker/major/minor 缺陷，无越界改动。

verdict: review_passed
