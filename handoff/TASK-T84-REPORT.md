# TASK-T84-REPORT

```yaml
implemented_changes:
  - src-tauri/src/commands/processor.rs — `mask_column` 命令签名追加 4 个可选参数（validate_rule_id / invalid_text / params_override / phone_prefixes，放在 template 之后、db 之前）；加 `#[allow(clippy::too_many_arguments)]`（10 参数超阈值，与现有 IPC 命令风格一致）
  - src-tauri/src/commands/processor.rs — mask 循环内新增"先校验再脱敏"分支：当 validate_rule_id 为 Some 时，从 DB 查校验规则 + 预编译正则 + phone_prefixes_ref + invalid_out（默认 "INVALID"）；每行按 phone-validate / params(override 优先) / pattern / pass 分发校验，通过→正常脱敏，不通过→写 invalid_text
  - src-tauri/src/commands/processor.rs — log_operation_with_snapshot 的 params JSON：validate_rule_id 为 Some 时追加 validateRuleId / invalidText / paramsOverride / phonePrefixes 字段（便于撤销/重做/审计还原）；None 时保持原 5 字段不变
  - docs/02-技术设计文档.md — `mask_column` 签名表行更新为完整 9 参数签名 + T84 行为说明
  - docs/versions/1.1.5/更新日志.md — T84 行状态 pending → verified_complete + 验证证据
verification_run:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test -p ruT0-data-kit --lib
  - cargo test -p ruT0-data-kit --lib commands::processor
verification_results:
  - cargo fmt — ok（格式化通过）
  - cargo clippy --all-targets --all-features -- -D warnings — Finished，0 warning（加 too_many_arguments allow 后通过）
  - cargo test -p ruT0-data-kit --lib — 135 passed; 4 failed
  - cargo test -p ruT0-data-kit --lib commands::processor — 32 passed; 0 failed（含 mask_then_undo / mask_then_redo）
docs_updated:
  - docs/02-技术设计文档.md（mask_column 签名表行更新）
  - docs/versions/1.1.5/更新日志.md（T84 进度表行 + 变更摘要已预存）
commit_summary:
  - none（安全约束：不创建 git 分支，不 commit）
reported_status:
  - verified_complete
scope_deviation:
  - none
```

## 4 个 db 测试失败的根因分析（与本任务无关）

`cargo test -p ruT0-data-kit --lib` 全量运行时出现 4 个失败：

```
db::tests::cleanup_deprecated_rules_removes_legacy_ids
db::tests::rule_kind_round_trip_through_db
db::tests::seed_builtin_rules_inserts_ten_when_empty
db::tests::seed_builtin_rules_upserts_missing_on_existing_db
```

失败断言均为 `left: 18  right: 17`（规则数 +1）。

**这 4 个失败是 T81（邮箱校验规则，已 verified_complete）的预期副产物，不是 T84 引入的**：

- T81 在 `crates/core/src/processor/rules.rs` 的 `RuleRegistry::with_defaults()` 注册了 `email-validate_rule()`，内置规则总数从 17 增至 18（见 `rules.rs` 第 513-545 行注释："v1.1.5 T81：新增 `email-validate`... `with_defaults()` 共 18 条"）。
- 4 个失败测试硬编码了 17 这个数字（`assert_eq!(count, 17)` 形式），T81 落地后这些断言数字未同步更新。
- 这些测试位于 `src-tauri/src/db/mod.rs`（T81 范围外的文件），T84 handoff 的 in_scope 仅限 `src-tauri/src/commands/processor.rs`，按"不扩 scope、不顺手改无关测试"守则不在本任务修复。
- **验证证据**：`git stash`（撤销 T84 改动，保留 T81 改动）后重跑 `cargo test -p ruT0-data-kit --lib` 同样 4 failed；T84 改动仅触碰 processor.rs 的 `mask_column`，未触碰 db 测试或 rules.rs，不可能影响规则计数断言。

**processor 命令测试全绿（32/32）**，证明 T84 实现本身无回归。db 测试断言数字更新应由 T81 或后续清理任务负责。
