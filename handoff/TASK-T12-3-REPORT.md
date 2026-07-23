# TASK-T12-3-REPORT — 拆 operator.rs → mask_op.rs + validate_op.rs

implemented_changes:
  - 删除 `crates/core/src/rules/operator.rs`（原 976 LOC / 41 函数）。
  - 新增 `crates/core/src/rules/mask_op.rs`（578 LOC）：MatchMode + TemplateOp /
    SplitTemplateOp / RegexReplaceOp / ConstReplaceOp structs + impls + MaskOp enum
    + from_rule + apply_mask_op + apply_template/apply_split_template/apply_regex_replace
    + template_from_params/split_template_from_params/regex_replace_from_params
    + parse_match_mode/parse_usize/parse_bool（标 `pub(super)`）。
    - mask 相关 tests mod + bug_repro_tests mod 原样迁入。
  - 新增 `crates/core/src/rules/validate_op.rs`（416 LOC）：AlgoKind/GuardKind +
    RegexOp/AlgorithmOp/RegexWithGuardOp structs + impls + ValidateOp enum + from_rule
    + apply_validate_op/apply_regex/apply_algorithm/apply_regex_with_guard。
    - validate 相关 tests mod 原样迁入。
  - 改 `crates/core/src/rules/mod.rs`：
    - `pub mod operator;` → `pub mod mask_op;` + `pub mod validate_op;`
    - 原 `pub use operator::{...}` 拆为 `pub use mask_op::{...}` + `pub use validate_op::{...}`，
      重导出的符号集合完全不变（外部 `rules::MaskOp` / `rules::ValidateOp` /
      `rules::apply_mask_op` / `rules::apply_validate_op` 等路径零感知）。
  - 共享 helper 处理：`parse_usize` / `parse_bool` / `parse_match_mode` 留在
    mask_op.rs 标 `pub(super)`；validate_op.rs 未直接使用它们（v0.4.4 from_rule
    是直接从 params 读取，不调用这三个 helper），故无需 `use super::mask_op::`。
    实际验证：build 期无 unused import warning 来自 mask_op（除 pre-existing
    crate 命名 warning）。

verification_run:
  - cargo build --workspace
  - cargo test --workspace
  - test ! -f crates/core/src/rules/operator.rs || test $(wc -l < crates/core/src/rules/operator.rs) -lt 50
  - wc -l crates/core/src/rules/mask_op.rs crates/core/src/rules/validate_op.rs
  - grep -n "pub use" crates/core/src/rules/mod.rs

verification_results:
  - cargo build --workspace：0 error，1 warning（pre-existing crate 命名
    `ruT0_data_kit_core` non_snake_case，与 T12-3 无关）。
  - cargo test --workspace：全绿，按 test result 行汇总 passed = 446，与基线 446 持平。
    分项：348 / 10 / 34 / 12 / 11 / 31 / 0，0 failed。
  - operator.rs：已删除（test ! -f 通过）。
  - LOC：mask_op.rs = 578（< 600 ✓），validate_op.rs = 416（< 500 ✓）。
  - mod.rs pub use：mask_op 导出 apply_mask_op/ConstReplaceOp/MaskOp/MatchMode/
    RegexReplaceOp/SplitTemplateOp/TemplateOp；validate_op 导出 apply_validate_op/
    AlgorithmOp/AlgoKind/GuardKind/RegexOp/RegexWithGuardOp/ValidateOp。
    与原 operator.rs 重导出集合完全一致（13 个符号）。

docs_updated:
  - 未更新 docs/。本次为纯结构拆分，无行为 / 用法 / 工作流变化，外部 import 路径
    全部经 rules/mod.rs 重导出保持零感知，无需改永久文档。

reported_status: verified_complete

scope_deviation:
  - 删除 operator.rs 而非保留 < 50 LOC 入口。HANDOFF 验收标准为「删除或仅保留
    < 50 LOC 入口」，两条择一，选择删除（更彻底、无遗留 facade）。rules/mod.rs
    不再 `pub mod operator`，所有外部消费者经 `pub use mask_op::` /
    `pub use validate_op::` 重导出，符号集合不变。无其他越界改动。
