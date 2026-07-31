---
id: T12
title: Rust 死代码清理（placeholder / 未用 model 结构 / db dead_code 标注）
depends_on: [T10, T11]
version: 1.0.0
type: refactor
status: done
---

# TASK-T12-REPORT — Rust 死代码清理

## goal

清理 `crates/core` 与 `src-tauri` 中本轮架构审计确认的 Rust 死代码 / 占位空骨架，
使 workspace 在 `cargo check --workspace` + `cargo test --workspace` 下零错误零警告且
行为不变。纯删/标注，不改任何业务逻辑、不改公共 API 对外行为。

## implemented_changes / scope_changes

### 1. 删除 `crates/core/src/processor/` 整个模块（in_scope #1）

- 删除 5 个文件：`mod.rs`、`extractor.rs`、`masker.rs`、`rules.rs`、`validator.rs`
  （目录已 `rm -rf`，不再存在）。
- `crates/core/src/lib.rs`：删除 `pub mod processor;` 声明；同步更新模块级 doc 注释，
  说明 v1.1+ 需处理器时应新建模块（不再保留空骨架）。

### 2. 删除 `crates/core/src/model.rs` 中 `Sheet` / `Operation` / `Column`（in_scope #2）

- 仅保留 `Record` struct（datasource 子模块在用）。
- 更新文件头 doc 注释为「v1.0.0 仅定义 `Record`；Sheet/Operation/Column 推迟到 v1.1+
  真实需要时再定义」。

### 3. `src-tauri/src/db/mod.rs` 4 处 `#[allow(dead_code)]` 改标注（in_scope #3）

将以下 4 处标注由 `#[allow(dead_code)]` 改为
`#[allow(dead_code, reason = "v1.1+ IPC 将接入（list_sessions/get_session 等）；单测已覆盖")]`：

- `SessionSummary`（lib.rs:33 区）
- `SheetSummary`（lib.rs:46 区）
- `SessionDetail`（lib.rs:59 区）
- `impl DbManager`（lib.rs:82 区）

保留各结构原有上方注释 `// v1.1+ IPC 将调用；单测已覆盖。`。未删任何字段 / 方法 / 测试，
未改 serde 派生。

### 验证「零外部消费者」前置检查

- `grep -rn "processor" crates/ src-tauri/src/ --include="*.rs"` 命中：
  - `crates/core/src/lib.rs:14 pub mod processor;`（本次删除）
  - `crates/core/src/error.rs:20 #[error("processor error: {0}")]` + `:21 Processor(String)`
    —— `CoreError::Processor` 变体定义在 `error.rs`，全仓 `grep CoreError::Processor / Processor(`
    零外部引用，属 `out_of_scope`（不改 datasource/commands/error），按规范保留未动。
  - `crates/core/src/datasource/mod.rs:13` 仅文档注释提及「不改动 processor」，非代码引用。
- `grep` `Sheet` / `Operation` / `Column` 在 `model.rs` 之外零代码命中
  （`commands/data.rs:152` 与 `db/mod.rs:100/366` 仅为文档/测试注释中的英文词「Sheet」）。

## verification_run

按 handoff `verification_commands` 顺序运行：

1. `cargo check --workspace`（删除前基线 + 每步后均跑，最终结果见下）
2. `cargo test --workspace`
3. `cargo clippy --workspace -- -D warnings 2>/dev/null || cargo clippy --workspace`

另跑 `git stash` + `cargo clippy --workspace` 对照基线，确认 clippy 警告是否预存在。

## verification_output

### 1. `cargo check --workspace`（最终，全部改动后）

```
    Checking ruT0-data-kit v1.0.0 (/Users/joker/Code/RuT0DataKit/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.46s
```

零错误零警告。

### 2. `cargo test --workspace`

```
    Compiling ruT0-data-kit-core v1.0.0 (/Users/joker/Code/RuT0DataKit/crates/core)
    Compiling ruT0-data-kit v1.0.0 (/Users/joker/Code/RuT0DataKit/src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.04s
     Running unittests src/lib.rs (target/debug/deps/ruT0_data_kit_lib-23fe3a6c176f0673)

running 9 tests
test db::migrate::tests::migrate_first_time_bootstraps ... ok
test db::tests::new_creates_tables_and_schema_version ... ok
test db::migrate::tests::migrate_repeat_is_idempotent ... ok
test db::tests::settings_roundtrip ... ok
test db::tests::list_and_get_session ... ok
test db::tests::log_operation_inserts_row ... ok
test db::tests::write_and_query_cells_paginated ... ok
test db::tests::write_and_query_cells_paginated_multi_column ... ok
test db::tests::write_cells_upsert_on_conflict ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running unittests src/main.rs (target/debug/deps/ruT0_data_kit-dde9632aede5876b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/ruT0_data_kit_core-55402b088b37d9ef)

running 13 tests
test pcap::detect::tests::detect_tshark_local ... ignored, 本机 tshark 探测，CI 无 tshark 时跳过
test pcap::detect::tests::probe_tshark_nonexistent_returns_none ... ok
test pcap::detect::tests::candidate_paths_covers_three_platforms ... ok
test pcap::detect::tests::set_tshark_path_ignores_empty_or_whitespace ... ok
test pcap::detect::tests::resolve_tshark_cmd_falls_back_to_literal ... ok
test pcap::detect::tests::set_and_get_tshark_path_roundtrip ... ok
test datasource::tests::detect_format_routes_by_extension ... ok
test pcap::reader::tests::hex_to_bytes_basic ... ok
test pcap::reader::tests::hex_to_bytes_case_insensitive ... ok
test pcap::reader::tests::read_fixture_pcap ... ignored, 本机 tshark 读取，CI 无 tshark/无样本时跳过
test pcap::reader::tests::hex_to_bytes_ignores_odd_trailing ... ok
test datasource::csv::tests::csv_reader_headers_and_rows ... ok
test datasource::csv::tests::csv_reader_flexible_columns ... ok

test result: ok. 11 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ruT0_data_kit_lib

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ruT0_data_kit_core

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

合计 20 通过 / 0 失败 / 2 ignored（tshark 本机探测，CI 跳过）。db 模块 9 个测试全过，
未回归。

### 3. `cargo clippy --workspace -- -D warnings`（首条，按 handoff 写法）

```
    Checking ruT0-data-kit v1.0.0 (/Users/joker/Code/RuT0DataKit/src-tauri)
    Checking ruT0-data-kit-core v1.0.0 (/Users/joker/Code/RuT0DataKit/crates/core)
error: iterating on a map's keys
   --> crates/core/src/datasource/json.rs:109:17
    |
109 |                 r.fields.into_iter().map(|(k, _)| k).collect()
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: try: `r.fields.into_keys()`
    |
    = help: for further information about https://rust-lang.github.io/rust-clippy/master/index.html#iter_kv_map
    = note: `-D clippy::iter-kv-map` implied by `-D warnings`
    = help: to override `-D warnings` add `#[allow(clippy::iter_kv_map)]`

error: iterating on a map's keys
   --> crates/core/src/datasource/sql.rs:243:22
    |
243 |             .map(|r| r.fields.into_iter().map(|(k, _)| k).collect())
    |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: try: `r.fields.into_keys()`
    = help: for further information about https://rust-lang.github.io/rust-clippy/master/index.html#iter_kv_map

error: could not compile `ruT0-data-kit-core` (lib) due to 2 previous errors
warning: build failed, waiting for other jobs to finish
```

### 3b. 回退 fallback `cargo clippy --workspace`（按 `||` 分支）

```
    Checking ruT0-data-kit v1.0.0 (/Users/joker/Code/RuT0DataKit/src-tauri)
    Checking ruT0-data-kit-core v1.0.0 (/Users/joker/Code/RuT0DataKit/crates/core)
warning: iterating on a map's keys
   --> crates/core/src/datasource/json.rs:109:17
    |
109 |                 r.fields.into_iter().map(|(k, _)| k).collect()
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: try: `r.fields.into_keys()`
    |
    = help: for further information about https://rust-lang.github.io/rust-clippy/master/index.html#iter_kv_map
    = note: `#[warn(clippy::iter_kv_map)]` on by default

warning: iterating on a map's keys
   --> crates/core/src/datasource/sql.rs:243:22
    |
243 |             .map(|r| r.fields.into_iter().map(|(k, _)| k).collect())
    |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: try: `r.fields.into_keys()`
    |
    = help: for further information about https://rust-lang.github.io/rust-clippy/master/index.html#iter_kv_map

warning: `ruT0-data-kit-core` (lib) generated 2 warnings (run `cargo clippy --fix --lib -p ruT0-data-kit-core` to apply 2 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.77s
```

### 3c. 基线对照（`git stash` 后跑 `cargo clippy --workspace`）

```
warning: iterating on a map's keys
   --> crates/core/src/datasource/json.rs:109:17
    ...（同 3b）
warning: iterating on a map's keys
   --> crates/core/src/datasource/sql.rs:243:22
    ...（同 3b）
warning: `ruT0-data-kit-core` (lib) generated 2 warnings
```

**结论**：2 条 `clippy::iter_kv_map` 警告在 baseline（HEAD dcd34ac，T12 改动前）
**预存在**，位于 `crates/core/src/datasource/{json,sql}.rs`，属 `out_of_scope`
（不改 datasource 代码）。T12 改动未引入新 clippy 警告，也未消除既有（按规范不越界修）。

## docs_updated

未更新 `docs/`。handoff `out_of_scope` 明确「不改 `docs/`」（T14 同步 docs/02，T12 不碰）。

## scope_deviation

**none**。所有改动严格落在 `in_scope` 三项内。

附注（非越界，仅记录给主会话裁定，未自行处理）：
- `crates/core/src/error.rs:20-21` 的 `CoreError::Processor(String)` 变体在删除
  `processor` 模块后变为零外部引用的死代码（rustc 当前未报 dead_code 因属 enum 变体）。
  该项不在 handoff `in_scope`（属「不改 error 代码」），T12 未动。是否追加清理由
  主会话裁定（可能需配合 v1.1+ processor 实现一并处理）。
- `clippy::iter_kv_map` × 2（`datasource/json.rs:109`、`datasource/sql.rs:243`）
  为 baseline 预存在，属 `out_of_scope`（不改 datasource），未修。

## reported_status

**verified_complete**

- `cargo check --workspace`：通过（零错误零警告）。
- `cargo test --workspace`：通过（20 passed / 0 failed / 2 ignored，db 模块未回归）。
- `cargo clippy --workspace -- -D warnings`：失败 2 条，但均为 baseline 预存在且
  属 `out_of_scope`（datasource 代码）；fallback `cargo clippy --workspace` 通过
  （无新增警告）。T12 改动零新增 clippy 警告。

按 handoff `acceptance_criteria` #3「若环境支持 clippy；若 clippy 不可用则跳过，但 check
必过」与 #1/#2/#4/#5/#6/#7/#8 全部满足，判定为 verified_complete（建议状态）。
