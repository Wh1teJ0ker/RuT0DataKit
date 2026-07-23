# TASK-T12-4-REVIEW — 建 rules/patterns.rs 集中正则表 + 消除 phone/ip 正则散布

```yaml
task_id: T12-4
verdict: verified_complete
```

## 审查依据

- `handoff/TASK-T12-4-HANDOFF.md`（任务定义、验收标准）
- `handoff/TASK-T12-4-REPORT.md`（coder 自报告）
- 实际产物：`crates/core/src/rules/patterns.rs`（新）、`rules/mod.rs`、`scan/mod.rs`、
  `validators/{phone,ip,mac,email,name,username,bankcard}.rs`、`rules/validate_op.rs`、
  `rules/builtin.rs`、`rules/presets.rs`、`validators/idcard.rs`
- 独立运行 `cargo build --workspace` / `cargo test --workspace` / `grep` 验收命令

---

## 1. 逐 scope 正则一致性核对表

extract 字段对照源：`git show HEAD:crates/core/src/scan/mod.rs::extract_pattern`
validate 字段对照源：`git show HEAD:crates/core/src/validators/<scope>.rs` 的 `Regex::new(...)` 字面量
（ip 例外：原 ip.rs 引用 `presets::IP_REGEX`，对照源为 `presets.rs::IP_REGEX`）

| scope    | extract (patterns.rs)                                       | extract (原 scan)                                            | 一致 | validate (patterns.rs)                                                                 | validate (原 validator)                                                              | 一致 |
|----------|-------------------------------------------------------------|--------------------------------------------------------------|------|----------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------|------|
| idcard   | `r"\b\d{17}[\dXx]\b"`                                       | `r"\b\d{17}[\dXx]\b"`                                        | OK   | `r"^\d{17}[\dXx]$"`                                                                    | 无正则（idcard.rs 走 GB11643 算法），patterns 仅提供格式锚定串，文档已注明            | OK   |
| phone    | `r"\b\d{11}\b"`                                             | `r"\b\d{11}\b"`                                              | OK   | `r"^1\d{10}$"`                                                                         | `r"^1\d{10}$"`                                                                       | OK   |
| bankcard | `r"\b\d{13,19}\b"`                                          | `r"\b\d{13,19}\b"`                                           | OK   | `r"^\d{13,19}$"`                                                                       | `r"^\d{13,19}$"`                                                                     | OK   |
| email    | `r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"`         | 同                                                           | OK   | `r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$"`                                  | `r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$"`                               | OK   |
| ip       | `r"\b(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)(?:\.(?:...)){3}\b"` | 同                                                            | OK   | `r"^((25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\.){3}(25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)$"` | `presets::IP_REGEX` 同字面量（单元测试 `ip_validate_matches_presets` 已断言）        | OK   |
| mac      | `r"\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b"`              | 同                                                            | OK   | `r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$"`                                           | `r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$"`                                         | OK   |
| username | `r"[A-Za-z0-9]{3,}"`                                        | 同                                                            | OK   | `r"^[A-Za-z0-9]+$"`                                                                    | `r"^[A-Za-z0-9]+$"`                                                                  | OK   |
| name     | `r"[\x{4e00}-\x{9fa5}]{2,}"`                                | 同                                                            | OK   | `r"^[\x{4e00}-\x{9fa5}]+$"`                                                            | `r"^[\x{4e00}-\x{9fa5}]+$"`                                                          | OK   |

关键点核对：
- `PHONE.validate` = `r"^1\d{10}$"`（首位 1），与 `validators/phone.rs` 原字面量一致；与
  `presets::PHONE_REGEX`（`r"^\d{11}$"`，11 位不限首位）**刻意区分**，未误用 presets 版。
  patterns.rs:124-125 单元测试 `phone_validate_is_first_digit_one` 明确断言此点。
- `IP.extract`（`\b` 边界，scan 版）与 `IP.validate`（`^...$` 锚定，validator 版）正确区分，
  与 HANDOFF risks 第 2 条要求一致。
- 8 scope 齐全：idcard/phone/bankcard/email/ip/mac/username/name，无遗漏。

逐字符一致性结论：全部 16 条正则（8 extract + 8 validate）与原散布版本逐字符一致，行为零回归。

## 2. 独立验证命令输出

### 2.1 grep 验收 #1（HANDOFF verification_commands）
```
$ grep -rn '1\d{10}' crates/core/src/validators/phone.rs \
    crates/core/src/scan/mod.rs crates/core/src/rules/validate_op.rs \
    crates/core/src/rules/mask_op.rs crates/core/src/rules/builtin.rs
exit=1 (0 命中)
```
符合预期（0 命中）。

### 2.2 grep 验收 #2（全 src 排除 patterns.rs / tools/）
```
$ grep -rn '1\d{10}' crates/core/src/ | grep -v patterns.rs | grep -v tools/
exit=1 (0 命中)
```
符合预期。`tools/regex_construct.rs` 中的 `^1\d{10}$` 仍保留（out_of_scope 正则工具示例），
`tests/` 断言保留（out_of_scope），均未误改。

### 2.3 patterns.rs 自身命中（sanity）
```
patterns.rs:31:    validate: r"^1\d{10}$",
patterns.rs:124:        assert_eq!(PHONE.validate, r"^1\d{10}$");
```
符合预期：字面量集中收敛到 patterns.rs 一处，外加同文件单元测试断言一处。

### 2.4 cargo build --workspace
```
Finished `dev` profile [unoptimized + debuginfo] target(s)
```
0 error。仅 1 个 pre-existing `non_snake_case` 警告（crate 名 `ruT0-data-kit-core`），
与本次无关。

### 2.5 cargo test --workspace
```
test result: ok. 352 passed; 0 failed; 3 ignored
test result: ok. 10 passed; 0 failed
test result: ok. 34 passed; 0 failed; 2 ignored
test result: ok. 12 passed; 0 failed
test result: ok. 11 passed; 0 failed
test result: ok. 31 passed; 0 failed
test result: ok. 0 passed; 0 failed
---
passed total: 450
```
全绿，450 passed（基线 446 + 新增 4 个 patterns.rs 单元测试）。新增 4 测试：
- `extract_and_validate_lookup_roundtrip`（8 scope 查表往返）
- `unknown_scope_returns_none`
- `phone_validate_is_first_digit_one`（首位 1 断言）
- `ip_validate_matches_presets`（与 presets::IP_REGEX 逐字符一致断言）

设计合理，既覆盖正向查表又覆盖关键字面量回归断言。

## 3. patterns.rs 质量核对

- `pub struct ScopePatterns { extract, validate }` 设计清晰，字段语义文档化
  （extract 宽松召回带 `\b` 边界 / validate 严格校验 `^...$` 锚定）。
- 8 个 `pub const` 常量 + 2 个查表函数 `extract_pattern` / `validate_pattern`，
  match + `_ => return None`，未注册 scope 返回 None，调用方决定跳过/报错。
- 模块级文档明确说明 `idcard` 无正则校验器（走 GB11643 算法），
  `IDCARD.validate` 仅作格式锚定串，不替代 IdCardValidator — 与 idcard.rs 实际实现一致
  （idcard.rs:24-54 纯算法，无 Regex）。idcard.rs 未改，符合报告。
- 4 个单元测试覆盖查表、unknown、phone 字面、ip 与 presets 一致性。

## 4. scan/mod.rs 改造核对

- `DefaultSensitiveScan::extract_pattern` 函数体改为 `crate::rules::patterns::extract_pattern(scope)`
  一行委托，删除原 8 个硬编码正则字面量（git diff 确认）。
- 模块文档保留提取正则表注释（作为行为说明，非硬编码源），与 HANDOFF 「函数体仅 match scope → return 常量」
  的要求一致 — 注释不算硬编码源。
- `scan` 主体逻辑未变：scope -> extract_pattern -> find_iter -> build_validator 校验。

## 5. validators 改造核对

- phone.rs / ip.rs / mac.rs / email.rs / name.rs / username.rs / bankcard.rs：7 个 validator
  的 `Regex::new(...)` 改引用 `patterns::<SCOPE>.validate`，`use crate::rules::patterns::<SCOPE>`，
  原硬编码字面量删除（git diff 确认）。
- ip.rs：`use crate::rules::presets::IP_REGEX` -> `use crate::rules::patterns::IP`，
  `Regex::new(IP_REGEX)` -> `Regex::new(IP.validate)`。presets::IP_REGEX 现仅被 patterns.rs
  单元测试引用（patterns.rs:132），其余业务路径不再依赖 — 作为兼容层保留，可接受（HANDOFF risks 方案 B）。

## 6. 注释改造核对（主会话授权范围）

- validate_op.rs:30 / :34 / :277：3 处文档/代码注释中的 `^1\d{10}$` 字面量改为
  `patterns::PHONE.validate` 引用名。属注释改动，非代码逻辑（实际 phone 校验仍委托
  `PhoneValidator::new`，validate_op.rs:279）。
- builtin.rs:34：1 处文档注释 `^1\d{10}$` -> `patterns::PHONE.validate`。
- 改动目的明确：通过 grep 验收（注释字面量保留会导致 grep 误命中），属主会话授权范围，非 scope creep。

## 7. 越界核对（scope_check）

- HANDOFF 强制范围：phone.rs / ip.rs（+ mac.rs「如有硬编码」）。
- 主会话 dispatch 指令明确授权扩展到 email/name/username/bankcard 改引用，以及
  validate_op.rs / builtin.rs 注释字面量改常量名引用。属授权范围，非越界。
- 未改：tools/regex_*.rs（out_of_scope 正则工具示例）、frontend/（out_of_scope）、
  tests/ 断言（out_of_scope）、idcard.rs（无正则）、presets.rs（保留作兼容层）。
- 无夹带无关重构、无关修复、风格性噪音改动。

结论：无越界改动。

## 8. 文档核对（docs_check）

本次为内部重构，行为零回归，无对外行为/用法/工作流变化。scan/validate 路径与
scope 语义未变。coder 报告「未更新 docs/」合理，无文档缺口。

## 9. 风险核对

- 回归风险：正则逐字符一致 + 450 测试全绿（含原 e2e / scan / validators 全套测试），
  行为零回归。
- 边界条件：unknown scope 返回 None，由 scan 跳过（scan/mod.rs:72 `let Some(...) else { continue }`），
  与原行为一致。
- 错误处理：`Regex::new` 失败仍 `let Ok(re) else { continue }`（scan/mod.rs:73），未变。
- 测试缺口：patterns.rs 已加 4 个针对性单测（查表/unknown/phone 字面/ip 一致性），
  覆盖充分。

## 10. 缺陷清单（defects）

无阻塞问题。

- 无 critical 缺陷
- 无 major 缺陷
- 无 minor 缺陷

## 最终建议状态

**verified_complete**

- goal 满足：patterns.rs 集中定义 8 scope 的 extract + validate 正则对，
  scan/mod.rs 改查表，validators/ 7 个 validator 改引用，硬编码散布消除。
- acceptance_criteria 全部满足：
  - patterns.rs 集中定义 8 scope 成对正则 OK
  - scan/mod.rs::extract_pattern 改查表（一行委托）OK
  - validators/phone.rs 不再硬编码 `^1\d{10}$`（改引用 PHONE.validate）OK
  - validators/ip.rs 不再硬编码（改引用 IP.validate）OK
  - grep `1\d{10}` 仅命中 patterns.rs（+ tools/ 示例 + tests 断言保留）OK
  - cargo test --workspace 全绿 450 passed OK
  - cargo build --workspace 0 error OK
- 无关键缺陷、无越界改动、验证充分、文档同步。

主会话可据此判定 T12-4 任务完成。
