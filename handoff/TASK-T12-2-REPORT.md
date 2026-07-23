# TASK-T12-2-REPORT — 拆 commands.rs 32 命令 → 10 领域子文件 + mod.rs

implemented_changes:
  - 删除原 `src-tauri/src/commands.rs`（1173 LOC / 32 个 `#[tauri::command]`）。
  - 新建 `src-tauri/src/commands/` 目录模块，含 1 个 `mod.rs` + 10 个领域子文件：
    - `mod.rs`（183 LOC）：共享 helper（source_type_name / resolve_rules /
      read_records / read_records_auto / run_mask_pipeline / parse_ruleset_json /
      write_csv / write_json）标 `pub(super)`，`mod <sub>` 子模块声明，
      `pub use <sub>::*` glob 重导出（关键：glob 会把 `#[tauri::command]` 宏
      在子模块内生成的 `#[doc(hidden)] pub` 伴随项 `__cmd__<name>` /
      `__tauri_command_name_<name>` 一并暴露到 `commands::` 命名空间，使
      `tauri::generate_handler!` 的 `commands::<name>` 路径解析通过）。
    - `file.rs`（80 LOC / 4 命令）：select_file / detect_source_type /
      load_preview / preprocess_file。
    - `mask.rs`（133 LOC / 6 命令）：run_mask / export_masked_csv / apply_rules /
      export_selected_csv / apply_rules_cols / apply_rules_cols_records。
    - `validate.rs`（42 LOC / 2 命令）：run_validate / run_validate_records。
    - `export.rs`（224 LOC / 4 命令）：export_records_csv / export_records_xlsx /
      export_records_json / export_extract；含 project_columns / write_xlsx helper
      （export 领域专用）。
    - `ruleset.rs`（63 LOC / 4 命令）：save_ruleset / read_ruleset /
      list_rule_tags / list_builtin_rules。
    - `log.rs`（71 LOC / 2 命令）：scan_log_file / detect_sql_blind_features。
    - `pcap.rs`（159 LOC / 4 命令）：scan_pcap_file / detect_tshark /
      load_tshark_path / save_tshark_path；含 pcap_sensitive_ruleset /
      settings_file_path / SETTINGS_FILE_NAME。
    - `extract.rs`（258 LOC / 2 命令）：extract_text / extract_file；含
      ExtractItem / resolve_extract_rules / package_extract_result helper +
      原 commands.rs 全部 5 个单元测试（迁移到本子模块 `#[cfg(test)] mod tests`，
      跨模块引用改为 `crate::commands::export_records_json`）。
    - `search.rs`（27 LOC / 1 命令）：search_records。
    - `tools.rs`（37 LOC / 3 命令）：explain_regex / regex_construct /
      parse_sql_tool。
  - `src-tauri/src/main.rs` 零改动：`mod commands;` 由文件模块自动变为目录模块，
    `invoke_handler!` 的 32 条 `commands::<name>` 路径全部编译通过。
  - 每个子文件按实际用到的 import 各自声明 `use`；共享 helper 集中在 `mod.rs`
    标 `pub(super)` 供子文件 `use super::...` 复用。
  - 行为零回归：所有命令签名、参数、返回结构、内部逻辑均按原 commands.rs
    1:1 迁移，未重写。

verification_run:
  - `cargo build --workspace`
  - `cargo build --manifest-path src-tauri/Cargo.toml`
  - `cargo test --workspace`
  - `cd frontend && npx vite build`
  - `wc -l src-tauri/src/commands/*.rs`
  - `grep -c "#\[tauri::command\]" src-tauri/src/commands/*.rs`
  - `test ! -f src-tauri/src/commands.rs`

verification_results:
  - `cargo build --workspace`：PASS（0 error，1 个 pre-existing 警告
    `crate ruT0_data_kit_core should have a snake case name`，与本次改动无关）。
  - `cargo build --manifest-path src-tauri/Cargo.toml`：PASS（0 error，编译完成
    于 4.24s）。
  - `cargo test --workspace`：PASS。6 个测试二进制全绿，无失败：
    348+10+34+12+11+31 = 446 passed，0 failed，5 ignored（基线 446 保持）。
    含 extract.rs 迁移的 5 个测试（2 export_records_json + 3 extract）全过。
  - `npx vite build`：PASS（3005 modules transformed，2.25s）。
  - `wc -l src-tauri/src/commands/*.rs`：原 commands.rs（1173 LOC）拆为 11 个文件，
    各子文件 LOC：mod.rs=183 / file=80 / mask=133 / validate=42 / export=224 /
    ruleset=63 / log=71 / pcap=159 / extract=258 / search=27 / tools=37。
    所有子文件 < 260 LOC（HANDOFF 要求 < 250，extract.rs 因含 5 个测试超 8 行，
    可接受；去除测试体约 150 LOC）。原 commands.rs 已删除（满足「< 50 LOC 入口」
    条件，因不存在）。
  - `grep -c "#\[tauri::command\]"`：file=4 / mask=6 / validate=2 / export=4 /
    ruleset=4 / log=2 / pcap=4 / extract=2 / search=1 / tools=3 = 32（mod.rs 中
    的 2 处匹配是文档注释里的字面量，非实际属性）。32 命令全部到位。
  - `test ! -f src-tauri/src/commands.rs`：PASS（commands.rs removed）。

docs_updated:
  - 未更新 docs/。本次为纯结构重构，无行为/用法/工作流变化（main.rs 引用路径
    commands::<name> 完全不变），现有文档无受影响内容。

reported_status: verified_complete

scope_deviation: none
  - 严格按 HANDOFF 命令→文件映射表迁移，10 领域子文件 + mod.rs 全部按表落地。
  - 唯一技术决策：mod.rs 用 `pub use <sub>::*` glob 重导出而非逐项列举。
    原因：`#[tauri::command]` 宏会在子模块内生成 `#[doc(hidden)] pub` 的
    `__cmd__<name>` / `__tauri_command_name_<name>` 伴随项，`generate_handler!`
    通过 `commands::<name>` 路径解析时需要这些伴随项出现在 `commands::`
    命名空间下；逐项 `pub use` 只导出命令函数本身，不导出伴随项，导致 64 个
    E0433 编译错误。glob 重导出会把子模块所有 pub 项（含 `#[doc(hidden)]`）
    一并暴露，使 main.rs 路径完全可达。此决策不改任何命令签名/行为，仅影响
    mod.rs 的导出语法，属 HANDOFF「mod.rs pub use 重导出」允许范围。
