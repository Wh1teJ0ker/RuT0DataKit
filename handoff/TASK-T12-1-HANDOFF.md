# TASK-T12-1-HANDOFF — 拆 blind_aggregator.rs 1790 LOC → 3 子文件

```yaml
task_id: T12-1
goal: |
  将 crates/core/src/logsign/blind_aggregator.rs（1790 LOC / 58 函数）按三层职责
  拆为 logsign/blind/ 子目录下 3 个文件：probe.rs（探针抽取+正则）、aggregate.rs
  （位置聚类+true_size 众数算法）、reconstruct.rs（数据库还原+read_target 解析），
  并改 logsign/mod.rs 引用新结构。行为零回归。
in_scope:
  - crates/core/src/logsign/blind_aggregator.rs → 拆分（内容迁移到子文件）
  - crates/core/src/logsign/blind/mod.rs（NEW，模块入口+pub use 重导出）
  - crates/core/src/logsign/blind/probe.rs（NEW）
  - crates/core/src/logsign/blind/aggregate.rs（NEW）
  - crates/core/src/logsign/blind/reconstruct.rs（NEW）
  - crates/core/src/logsign/mod.rs（改 pub mod 声明）
out_of_scope:
  - 不得改任何函数实现逻辑、算法、正则字符串
  - 不得改 tests/ 下测试用例（测试调用路径不变）
  - 不得动 src-tauri/ 或 frontend/
  - 不得改其他 logsign 子模块（loader.rs / payload_parser.rs / mod.rs 中 SignatureEngine）
acceptance_criteria:
  - blind_aggregator.rs 原文件删除或仅保留 mod.rs 入口（< 50 LOC）
  - 3 个子文件各自 < 800 LOC
  - logsign/mod.rs 的 pub use 重导出签名不变（外部消费者无需改 import）
  - cargo test --workspace 全绿（基线 445 passed）
  - cargo build --workspace 0 error
verification_commands:
  - cargo build --workspace
  - cargo test --workspace
  - test ! -f crates/core/src/logsign/blind_aggregator.rs || test $(wc -l < crates/core/src/logsign/blind_aggregator.rs) -lt 50
files_likely_to_change:
  - crates/core/src/logsign/blind_aggregator.rs
  - crates/core/src/logsign/blind/mod.rs
  - crates/core/src/logsign/blind/probe.rs
  - crates/core/src/logsign/blind/aggregate.rs
  - crates/core/src/logsign/blind/reconstruct.rs
  - crates/core/src/logsign/mod.rs
risks:
  - BlindAggregator impl 块跨 probe/aggregate/reconstruct 三层，拆分时 impl 方法要按职责归位（reconstruct_database → reconstruct.rs，collect_from_entries → aggregate.rs）
  - 私有 helper（parse_table_name_predicate 等）跨 reconstruct 互调，需同文件或 pub(crate)
  - 正则缓存函数（ascii_binary_regex 等）属 probe.rs，但 aggregate.rs 也要用 → 需 pub(crate) 或 pub(super)
depends_on: []
status: planned
```

## 拆分映射表

| 原行号 | 内容 | 目标文件 |
|--------|------|----------|
| 1-140 | 模块文档注释 | blind/mod.rs |
| 141-265 | `looks_like_blind_probe` / `extract_blind_probe` / `extract_blind_probe_with_line` + `ProbeKind` enum + `BlindProbe` struct | probe.rs |
| 268-360 | `PositionDetail` / `AggregatedResult` structs | aggregate.rs（AggregatedResult 也被 reconstruct 用 → 放 mod.rs 或 pub use） |
| 361-502 | `BlindAggregator` struct + `collect_from_entries` + `aggregate_*_group` + `build_decoded_string` + `mode_per_position_true_size` + `aggregate_position` / `aggregate_equality_position` | aggregate.rs |
| 503-674 | `ReconstructedDatabase` / `ReconstructedTable` / `ReconstructedRow` + `reconstruct_database` | reconstruct.rs |
| 675-812 | `parse_table_name_predicate` / `parse_from_table` / `parse_row_data_columns` / `build_table_columns_and_rows` / `table_name_predicate_regex` / `from_table_regex` / `group_concat_args_regex` | reconstruct.rs |
| 813-966 | `aggregate_ascii_binary_group` / `aggregate_equality_group` / `aggregate_length_group` | aggregate.rs |
| 965-1058 | `build_decoded_string` / `mode_per_position_true_size` | aggregate.rs |
| 1059-1246 | `parse_separator_char` / `hex_literal_regex` / `aggregate_position` / `aggregate_equality_position` | aggregate.rs |
| 1247-1290 | `ascii_binary_regex` / `equality_regex` / `length_regex` 正则缓存 | probe.rs |
| tests mod | 原 #[cfg(test)] mod tests | 拆到各自子文件内 |
