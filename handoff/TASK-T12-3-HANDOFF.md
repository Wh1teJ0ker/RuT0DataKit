# TASK-T12-3-HANDOFF — 拆 operator.rs 976 LOC → mask_op.rs + validate_op.rs 分文件

```yaml
task_id: T12-3
goal: |
  将 crates/core/src/rules/operator.rs（976 LOC / 41 函数）按算子族拆为
  rules/mask_op.rs（MaskOp + 4 变体 + apply + from_params helpers）+
  rules/validate_op.rs（ValidateOp + 3 变体 + apply + from_rule），
  rules/mod.rs 改 pub use 引用。行为零回归。
in_scope:
  - crates/core/src/rules/operator.rs → 拆分（删除或保留 < 50 LOC 入口）
  - crates/core/src/rules/mask_op.rs（NEW）
  - crates/core/src/rules/validate_op.rs（NEW）
  - crates/core/src/rules/mod.rs（改 pub use 声明）
out_of_scope:
  - 不得改任何算子实现逻辑、参数解析、正则字符串
  - 不得动 tests/ 下测试（测试随实现迁到各自子文件）
  - 不得动 src-tauri/ 或 frontend/
  - 不得改其他 rules 子模块（builtin.rs / loader.rs / types.rs / registry.rs / presets.rs）
  - 不得动 build_validator / build_masker 函数位置（留在 mod.rs）
acceptance_criteria:
  - operator.rs 删除或仅保留 < 50 LOC 入口
  - mask_op.rs < 600 LOC，validate_op.rs < 500 LOC
  - rules/mod.rs 的 pub use 重导出签名不变（外部消费者无需改 import）
  - cargo test --workspace 全绿（基线 445 passed）
  - cargo build --workspace 0 error
verification_commands:
  - cargo build --workspace
  - cargo test --workspace
  - test ! -f crates/core/src/rules/operator.rs || test $(wc -l < crates/core/src/rules/operator.rs) -lt 50
files_likely_to_change:
  - crates/core/src/rules/operator.rs
  - crates/core/src/rules/mask_op.rs
  - crates/core/src/rules/validate_op.rs
  - crates/core/src/rules/mod.rs
risks:
  - MatchMode enum 被 MaskOp 用，归 mask_op.rs
  - AlgoKind / GuardKind enum 被 ValidateOp 用，归 validate_op.rs
  - parse_usize / parse_bool / parse_match_mode helpers 被两边用 → 放 mask_op.rs pub(super) 或新建 shared helper（优先放 mask_op.rs，validate_op.rs 跨文件 use super::mask_op::）
  - #[cfg(test)] mod tests 要按算子族拆到各自子文件
depends_on: []
status: planned
```

## 拆分映射表

| 原行号 | 内容 | 目标文件 |
|--------|------|----------|
| 1-32 | 模块文档注释 | 分拆到 mask_op.rs / validate_op.rs 顶部 |
| 33-53 | `MatchMode` enum | mask_op.rs |
| 55-129 | `TemplateOp` / `SplitTemplateOp` / `RegexReplaceOp` / `ConstReplaceOp` structs + impls | mask_op.rs |
| 130-189 | `MaskOp` enum + impl + `apply_mask_op` | mask_op.rs |
| 191-294 | `apply_template` / `apply_split_template` / `apply_regex_replace` | mask_op.rs |
| 295-386 | `template_from_params` / `split_template_from_params` / `regex_replace_from_params` / `parse_match_mode` / `parse_usize` / `parse_bool` | mask_op.rs |
| 388-483 | `AlgoKind` / `GuardKind` / `RegexOp` / `AlgorithmOp` / `RegexWithGuardOp` structs + impls | validate_op.rs |
| 484-638 | `ValidateOp` enum + `from_rule` + `apply_validate_op` / `apply_regex` / `apply_algorithm` / `apply_regex_with_guard` | validate_op.rs |
| tests mod | 原 #[cfg(test)] mod tests | 拆：mask 相关 → mask_op.rs，validate 相关 → validate_op.rs |
