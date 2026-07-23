# TASK-T12-3-REVIEW — operator.rs → mask_op.rs + validate_op.rs 拆分审查

```yaml
task_id: T12-3
verdict: verified_complete
```

## 审查输入
- `handoff/TASK-T12-3-HANDOFF.md`
- `handoff/TASK-T12-3-REPORT.md`
- `crates/core/src/rules/mask_op.rs` / `validate_op.rs` / `mod.rs`
- git status：rules/ 下 operator.rs `D`，mask_op.rs / validate_op.rs `??`，mod.rs `M`

## 逐项审查结论

### 1. goal / acceptance 对齐

| 验收项 | 要求 | 实测 | 结论 |
|--------|------|------|------|
| operator.rs 删除 / < 50 LOC | 二选一 | `test ! -f` 通过，文件已删除 | 通过 |
| mask_op.rs < 600 LOC | < 600 | 578 | 通过 |
| validate_op.rs < 500 LOC | < 500 | 416 | 通过 |
| rules/mod.rs pub use 符号集合不变 | 13 符号 | mask_op 7 + validate_op 7 = 14（见下） | 通过（见注） |
| cargo build --workspace | 0 error | 0 error，1 pre-existing crate 命名 warning | 通过 |
| cargo test --workspace | 全绿基线 446 | 348+10+34+12+11+31+0 = 446 passed，0 failed | 通过 |

注：HANDOFF 称「13 个符号」，实测 mod.rs `pub use` 重导出共 14 个符号
（mask_op: apply_mask_op / ConstReplaceOp / MaskOp / MatchMode / RegexReplaceOp /
SplitTemplateOp / TemplateOp = 7；validate_op: apply_validate_op / AlgorithmOp /
AlgoKind / GuardKind / RegexOp / RegexWithGuardOp / ValidateOp = 7）。REPORT 亦
自称「13 个符号」与实际 14 不符，属计数笔误，不影响符号集合的完整性与外部
零感知。外部消费者 `crates/core/tests/e2e.rs` 通过 `rules::{MaskOp, ValidateOp,
apply_mask_op, apply_validate_op, MatchMode, RegexOp, SplitTemplateOp, TemplateOp,
RegexReplaceOp}` 等路径访问，全部经 `pub use` 命中，编译 / 测试均通过。

### 2. 行为零回归（抽样核对）

- `MaskOp::from_rule`（mask_op.rs:144-165）：按 `name` + `params` match 4 个通用
  算子名构造，未知名 `return None`，与原 operator.rs 一致。
- `apply_mask_op`（mask_op.rs:175-182）：4 变体 dispatch 原样。
- `apply_template`（mask_op.rs:184-227）：guard（min/max_len 不匹配原样返回）、
  cjk 分支、tail_start <= head_end 边界（仅输出 mask 段）、mask_len =
  mid_len.max(mask_min_len) 全部逐字保留。
- `ValidateOp::from_rule`（validate_op.rs:141-219）：regex / algorithm /
  regex_with_guard 三分支，algo 缺省 IdCard、bankcard 过滤、phone 守卫 prefix_set
  自 v0.4.3 起被忽略的注释均保留。无逻辑改动。
- `apply_validate_op` / `apply_regex` / `apply_algorithm` /
  `apply_regex_with_guard`：dispatch 与各 apply_* 函数体原样迁移。

抽样未发现算法 / 参数解析 / 正则字符串改动。本任务为纯结构移动。

### 3. 共享 helper 可见性

- `parse_match_mode` / `parse_usize` / `parse_bool` 留在 mask_op.rs，标
  `pub(super)`（mask_op.rs:346 / 357 / 365）。
- validate_op.rs `grep "parse_usize|parse_bool|parse_match_mode|mask_op::"` 结果为
  NONE，确实不跨文件调用这三个 helper（v0.4.4 from_rule 直接 `params.get().as_str()`
  读取，不经过 helper）。无遗漏跨文件依赖。
- helper 在 mask_op.rs 内部被 `template_from_params` / `split_template_from_params`
  使用，`cargo build` 无 dead_code / unused 警告。可见性选择合理。

### 4. tests 拆分

- mask_op.rs:377 起 `#[cfg(test)] mod tests`，含 template_idcard_equivalence /
  regex_replace_all_phone / regex_extract_first_match / split_template_email /
  const_replace_delete_and_replace / mask_op_from_rule_* / bug_repro_tests 等。
- validate_op.rs:296 起 `#[cfg(test)] mod tests`，含 validate_regex_preset_email /
  validate_algorithm_idcard / validate_op_from_rule_regex_with_params /
  validate_op_from_rule_algorithm_with_algo_param /
  validate_op_from_rule_regex_with_guard_params。
- mod.rs:66 起 `#[cfg(test)] mod tests` 保留 build_validator / build_masker 相关
  集成测试。
- 全量 cargo test 446 passed 与基线持平，无测试丢失。

### 5. scope 合规

git status rules/ 仅显示：operator.rs `D`、mask_op.rs / validate_op.rs `??`、
mod.rs `M`。其余 `loader.rs` / `presets.rs` / `types.rs` 的 `M` 与 `builtin.rs` 的
`??` 属 v0.4.4 工作树基线遗留（主会话已裁定不属于 T12-x scope creep），非本任务
触碰。未动 builtin/loader/types/registry/presets 内容（仅基线既有改动），
build_validator / build_masker 仍留在 mod.rs:41 / :61。无越界改动。

### 6. verification 复核（独立重跑）

```
$ wc -l crates/core/src/rules/mask_op.rs crates/core/src/rules/validate_op.rs
  578 mask_op.rs
  416 validate_op.rs

$ test ! -f crates/core/src/rules/operator.rs && echo DELETED OK
DELETED OK

$ grep -n "pub use\|pub mod" crates/core/src/rules/mod.rs
13:pub mod builtin;
14:pub mod loader;
15:pub mod mask_op;
16:pub mod presets;
17:pub mod registry;
18:pub mod types;
19:pub mod validate_op;
21:pub use builtin::{...};
22:pub use loader::{...};
23:pub use mask_op::{...};
26:pub use registry::{...};
27:pub use types::{...};
28:pub use validate_op::{...};

$ cargo build --workspace
Finished `dev` profile ... 0 error，1 pre-existing non_snake_case warning

$ cargo test --workspace
test result: ok. 348 passed; 0 failed; 3 ignored
test result: ok. 10 passed; 0 failed; 0 ignored
test result: ok. 34 passed; 0 failed; 2 ignored
test result: ok. 12 passed; 0 failed; 0 ignored
test result: ok. 11 passed; 0 failed; 0 ignored
test result: ok. 31 passed; 0 failed; 0 ignored
test result: ok. 0 passed; 0 failed; 0 ignored
合计 446 passed / 0 failed
```

全部验收命令独立复跑通过。

### 7. 文档核对

本任务为纯结构拆分，外部 import 路径经 `rules/mod.rs` 重导出零感知，无行为 /
用法 / 工作流变化。REPORT 称「未更新 docs/」合理，docs_check 无缺口。

## defects
无阻塞问题。

仅有 1 项非阻塞观察（不计入缺陷）：
- minor / 计数笔误：HANDOFF 与 REPORT 均称「13 个符号」，实际 `pub use` 重导出
  14 个符号（mask_op 7 + validate_op 7）。不影响符号集合完整性，仅文档计数与
  实际不符。无需 coder 修复，建议后续文档同步时更正。

## scope_check
无越界。rules/ 下本任务实际触碰仅 operator.rs(删除) / mask_op.rs(新增) /
validate_op.rs(新增) / mod.rs(pub use 改写)。其余 rules 子模块改动属 v0.4.4
工作树基线遗留，主会话已裁定不属 T12-3 scope。

## docs_check
已同步。纯结构拆分无需改永久文档，REPORT 判断正确。

## 最终建议状态
verified_complete

T12-3 实现为纯结构重构：operator.rs 删除、按算子族拆为 mask_op.rs(578) +
validate_op.rs(416)、mod.rs pub use 重导出集合完整、共享 helper 可见性合理、
测试按族拆分无丢失、cargo build/test 全绿 446 基线持平、无越界改动。可进入
主会话最终验收。
