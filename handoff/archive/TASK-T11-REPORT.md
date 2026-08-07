# TASK-T11-REPORT — commands.rs 拆分

```yaml
task_id: T11
implemented_changes:
  - 删除 src-tauri/src/commands.rs（374 行 god module），替换为模块目录 src-tauri/src/commands/
  - 新增 src-tauri/src/commands/mod.rs：声明 data / settings / update 三个子模块，pub(crate) 重导出 read_settings 供 lib.rs setup 钩子使用
  - 新增 src-tauri/src/commands/data.rs：AI 契约占位（AiContext/AiSuggestion/ai_suggest/invoke_ai_op）+ 导入流（ImportResult/PageData/import_file/get_sheet_data + 私有 const IMPORT_BATCH_ROWS）
  - 新增 src-tauri/src/commands/settings.rs：TsharkSettings + read_settings/write_settings 私有 helper + detect_tshark/load_tshark_path/save_tshark_path 命令
  - 新增 src-tauri/src/commands/update.rs：UpdateStatus + check_update/install_update 命令
  - src-tauri/src/lib.rs：invoke_handler! 的 9 条命令路径由 commands::xxx 改为 commands::{data,settings,update}::xxx 全路径，命令名/参数/返回结构未变；setup 钩子仍通过 commands::read_settings 访问（read_settings 在 mod.rs 内 pub(crate) 重导出）
verification_run:
  - cargo check --workspace
  - cargo build --manifest-path src-tauri/Cargo.toml
  - cargo test --workspace
verification_results:
  - cargo check --workspace: PASS（Finished dev profile，无 error；仅 crates/core 中既存的 unused import 警告，与 T11 无关）
  - cargo build --manifest-path src-tauri/Cargo.toml: PASS（Finished dev profile in 8.10s，无 error）
  - cargo test --workspace: PASS（test result: ok 全部通过；ruT0-data-kit-core lib 11 passed / 2 ignored；ruT0-data-kit lib 0；其余 0；无失败）
docs_updated:
  - 无；本任务为纯结构调整，未改外部行为，docs/ 下无需同步
commit_summary: |
  commit f1744f2
  refactor(tauri): T11 split commands.rs god module into concern submodules
  5 files changed, 418 insertions(+), 6 deletions(-)
  仅含 src-tauri/src/commands.rs 删除 + commands/ 4 个新文件 + lib.rs invoke_handler 路径同步
reported_status: verified_complete
scope_deviation: none
```

## 验证证据

- `cargo check --workspace`：无 error，仅 crates/core 既存的 unused import 警告（pcap.rs `CoreError`、xlsx.rs `Data`），属上游既有现状，非本任务引入。
- `cargo build --manifest-path src-tauri/Cargo.toml`：`Finished dev profile [unoptimized + debuginfo] target(s) in 8.10s`。
- `cargo test --workspace`：5 段 test result 全 `ok`，合计 20 passed / 2 ignored / 0 failed。

## 范围与边界

- 仅触碰 handoff in_scope 内文件：`commands.rs`（删除）+ `commands/{mod,data,settings,update}.rs`（新建）+ `lib.rs`（invoke_handler 路径同步）。
- 工作树中来自前序 UI/export 会话的未提交改动未被纳入本 commit，未被回退。
- 所有 #[tauri::command] 签名、返回类型、错误映射（含 check_update 吞噬 error、CoreError/DbError JSON 映射）逐字节搬运，未做行为修复——符合 handoff「只搬运不修复」要求。

## 备注

- reported_status 为 verified_complete 仅作为子 agent 建议状态，最终完成判定由主会话给出。
- HANDOFF 文件保留未删。
