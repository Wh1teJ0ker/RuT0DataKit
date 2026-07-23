# TASK-T12-4-REPORT — 建 rules/patterns.rs 集中正则表 + 消除 phone/ip 正则散布

```yaml
task_id: T12-4
```

## implemented_changes

- **NEW `crates/core/src/rules/patterns.rs`**（135 行）：
  - 定义 `pub struct ScopePatterns { extract, validate }`。
  - 定义 8 个 scope 常量：`IDCARD / PHONE / BANKCARD / EMAIL / IP / MAC / USERNAME / NAME`，每个含 extract（宽松召回，`\b` 边界，取自 scan/mod.rs）+ validate（严格校验，`^...$`，取自 validators/*.rs）。正则字符串与原散布版本逐字符一致（行为零回归）。
  - `pub fn extract_pattern(scope: &str) -> Option<&'static str>` — match scope -> 返回 `SCOPE.extract`。
  - `pub fn validate_pattern(scope: &str) -> Option<&'static str>` — match scope -> 返回 `SCOPE.validate`。
  - 注：`IDCARD.validate = r"^\d{17}[\dXx]$"` 仅作格式锚定串，IdCardValidator 仍走 GB11643 算法校验（无正则），未改 idcard.rs。
  - 4 个单元测试：查表往返、unknown 返回 None、PHONE.validate 字面断言、IP.validate == presets::IP_REGEX 断言。

- **`crates/core/src/rules/mod.rs`**：加 `pub mod patterns;`。

- **`crates/core/src/scan/mod.rs`**：`DefaultSensitiveScan::extract_pattern` 函数体改为查 `crate::rules::patterns::extract_pattern(scope)`，删除原 8 个硬编码正则字面量。

- **`crates/core/src/validators/phone.rs`**：`Regex::new(r"^1\d{10}$")` → `Regex::new(PHONE.validate)`，`use crate::rules::patterns::PHONE`。注释字面量改为引用 patterns 常量名。

- **`crates/core/src/validators/ip.rs`**：`use crate::rules::presets::IP_REGEX` + `Regex::new(IP_REGEX)` → `use crate::rules::patterns::IP` + `Regex::new(IP.validate)`。

- **`crates/core/src/validators/mac.rs`**：`Regex::new(r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$")` → `Regex::new(MAC.validate)`，`use crate::rules::patterns::MAC`。

- **`crates/core/src/validators/email.rs`**：硬编码 → `EMAIL.validate`，`use crate::rules::patterns::EMAIL`。

- **`crates/core/src/validators/name.rs`**：硬编码 → `NAME.validate`，`use crate::rules::patterns::NAME`。

- **`crates/core/src/validators/username.rs`**：硬编码 → `USERNAME.validate`，`use crate::rules::patterns::USERNAME`。

- **`crates/core/src/validators/bankcard.rs`**：硬编码 → `BANKCARD.validate`，`use crate::rules::patterns::BANKCARD`。

- **`crates/core/src/rules/validate_op.rs`**：3 处文档/代码注释中的 `^1\d{10}$` / `^1\d{10}` 字面量改为 `patterns::PHONE.validate` 引用名（注释，非代码逻辑）。

- **`crates/core/src/rules/builtin.rs`**：1 处文档注释 `^1\d{10}$` → `patterns::PHONE.validate`（注释）。

- **`crates/core/src/rules/presets.rs`**：保留原样不动（按主会话推荐方案 B，patterns.rs 与 presets.rs 并存；presets.rs 的 6 个 scope 正则常量与 patterns.rs 的 validate 字段逐字符一致，作为历史兼容层保留）。

- **未改**：`validators/idcard.rs`（用 GB11643 算法校验，无正则）、`tools/regex_*.rs`（正则工具示例，out_of_scope）、`frontend/`、`tests/` 断言。

## verification_run

- `cargo build --workspace`
- `cargo test --workspace`
- `grep -rn '1\\d{10}' crates/core/src/validators/phone.rs crates/core/src/scan/mod.rs crates/core/src/rules/validate_op.rs crates/core/src/rules/mask_op.rs crates/core/src/rules/builtin.rs`
- `grep -rn '1\\d{10}' crates/core/src/ | grep -v patterns.rs | grep -v tools/`
- `wc -l crates/core/src/rules/patterns.rs`

## verification_results

- `cargo build --workspace`：Finished，0 error（仅一个 pre-existing non_snake_case 警告，与本次无关）。
- `cargo test --workspace`：全部通过。各 `test result:` 汇总 passed = 450（基线 446，本任务新增 4 个 patterns.rs 单元测试，450 > 446）。
- grep #1（phone/scan/validate_op/mask_op/builtin）：0 命中（grep exit=1）。
- grep #2（全 src 排除 patterns.rs / tools/）：0 命中（grep exit=1）。tools/regex_construct.rs 的 `^1\d{10}$` 保留（out_of_scope 正则工具示例）。
- `wc -l patterns.rs`：135 行。

## docs_updated

- 未更新 `docs/`。本次为内部重构，行为零回归，无对外行为/用法/工作流变化；scan/validate 路径与 scope 语义未变。

## scope_deviation

- 轻微超出 HANDOFF 强制范围：HANDOFF 仅强制 phone.rs / ip.rs，mac.rs「如有硬编码改引用」。主会话执行指令明确要求把 email/name/username/bankcard 也统一改引用 patterns.rs，已按指令执行（仍属 in_scope 文件清单内，且行为零回归）。
- validate_op.rs / builtin.rs 的注释字面量改为 patterns 常量名引用：主会话指令要求，目的是通过 grep 验收（注释字面量保留会导致 grep 命中）。属注释改动，非代码逻辑。

## reported_status

verified_complete
