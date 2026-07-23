# TASK-T12-2-HANDOFF — 拆 commands.rs 32 命令 → 8 领域子文件

```yaml
task_id: T12-2
goal: |
  将 src-tauri/src/commands.rs（1173 LOC / 32 个 #[tauri::command]）按业务领域
  拆为 src-tauri/src/commands/ 目录下 8 个子文件 + 1 个 mod.rs 入口，
  main.rs 引用路径不变（commands::<name>）。行为零回归。
in_scope:
  - src-tauri/src/commands.rs → 拆分（内容迁移到子文件）
  - src-tauri/src/commands/mod.rs（NEW，pub use 重导出 + 共享 helper）
  - src-tauri/src/commands/file.rs（NEW）
  - src-tauri/src/commands/mask.rs（NEW）
  - src-tauri/src/commands/validate.rs（NEW）
  - src-tauri/src/commands/export.rs（NEW）
  - src-tauri/src/commands/ruleset.rs（NEW）
  - src-tauri/src/commands/log.rs（NEW）
  - src-tauri/src/commands/pcap.rs（NEW）
  - src-tauri/src/commands/extract.rs（NEW）
  - src-tauri/src/commands/search.rs（NEW）
  - src-tauri/src/commands/tools.rs（NEW）
  - src-tauri/src/main.rs（改 mod commands; 声明，引用路径不变）
out_of_scope:
  - 不得改任何 command 的实现逻辑、参数签名、返回结构
  - 不得动 frontend/ 或 crates/core/
  - 不得改 tauri.conf.json
  - 不得改 main.rs 的 invoke_handler! 命令清单（只改 mod 声明方式）
acceptance_criteria:
  - 原 commands.rs 单文件删除或仅保留 < 50 LOC 入口
  - 8 个领域子文件各自 < 250 LOC
  - main.rs 的 commands::<name> 引用全部编译通过
  - cargo build --manifest-path src-tauri/Cargo.toml 0 error
  - 前端 npx vite build 成功（无调用链变化）
verification_commands:
  - cargo build --manifest-path src-tauri/Cargo.toml
  - cd frontend && npx vite build
  - test ! -f src-tauri/src/commands.rs || test $(wc -l < src-tauri/src/commands.rs) -lt 50
files_likely_to_change:
  - src-tauri/src/commands.rs
  - src-tauri/src/commands/mod.rs
  - src-tauri/src/commands/{file,mask,validate,export,ruleset,log,pcap,extract,search,tools}.rs
  - src-tauri/src/main.rs
risks:
  - 共享 helper（source_type_name / select_file 内部逻辑）需提到 mod.rs 或 file.rs pub(crate)
  - import 块要按子文件职责拆分，不能全堆 mod.rs（否则子文件依赖过宽）
  - main.rs 用 `mod commands;` 后，commands 从文件模块变目录模块，路径 commands::select_file 不变
depends_on: []
status: planned
```

## 命令 → 子文件映射表

| 子文件 | 命令 | 原行号 |
|--------|------|--------|
| `file.rs` | select_file / detect_source_type / load_preview / preprocess_file | 72,88,164,562 |
| `mask.rs` | run_mask / export_masked_csv / apply_rules / export_selected_csv / apply_rules_cols / apply_rules_cols_records | 133,150,194,219,302,717 |
| `validate.rs` | run_validate / run_validate_records | 321,740 |
| `export.rs` | export_records_csv / export_records_xlsx / export_records_json / export_extract | 338,365,662,1115 |
| `ruleset.rs` | save_ruleset / read_ruleset / list_rule_tags / list_builtin_rules | 394,410,428,445 |
| `log.rs` | scan_log_file / detect_sql_blind_features | 502,959 |
| `pcap.rs` | scan_pcap_file / detect_tshark / load_tshark_path / save_tshark_path | 534,1007,1024,1051 |
| `extract.rs` | extract_text / extract_file | 1082,1092 |
| `search.rs` | search_records | 933 |
| `tools.rs` | explain_regex / regex_construct / parse_sql_tool | 621,630,647 |
| `mod.rs` | source_type_name helper + pub use 重导出全部命令 | — |

注：原审计报告列 8 子文件，实际按 32 命令领域细分为 10 子文件 + mod.rs，更贴合领域边界（每个子文件内聚度更高）。
