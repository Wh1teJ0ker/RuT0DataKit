# T77 Reviewer 审查报告

## verdict: review_passed

无阻塞问题。

## 审查依据

- HANDOFF/REPORT 文件在 `handoff/` 下不存在（流程缺口，见下方 minor）。本次审查以主会话 prompt 中提供的 goal/scope/acceptance/审查要点为判断基准，结合 `git show 97450c3` 实际 diff 与仓库现状交叉验证。
- 改动范围与描述一致：10 个文件，单一逻辑目的（generic-validate 特殊符号白名单化），无夹带无关改动。

## 6 项审查范围

**1. Goal 核对** — 满足。`allow_special: bool`（全开/全关）→ `allow_special_chars: String`（自定义白名单）已落地：
- `func_validator.rs:432` 签名改为 `allow_special_chars: &str`
- `func_validator.rs:437` 全空判定 `!allow_digits && !allow_letters && allow_special_chars.is_empty() → false` 正确
- `func_validator.rs:461` 逐字符 `(is_digit && allow_digits) || (is_letter && allow_letters) || allow_special_chars.contains(c)` 正确
- `func_validator.rs:561/569` `validate_extracted_with_params` Generic 分支解构与传参正确
- `rules.rs:242-243` `#[serde(default)] allow_special_chars: String` + `rename_all = "camelCase"` → `allowSpecialChars`

**2. Scope 核对** — none。改动严格限定在 generic-validate 相关字段重命名 + 白名单逻辑 + 前端同步 + 文档同步，无越界。

**3. Verification 核对** — 满足。commit message 记录 `cargo fmt + clippy -D warnings + test --all (165 pass) + pnpm build ✓`，覆盖 fmt/lint/test/前端构建四项关键命令。测试覆盖到位：
- `func_validator.rs:1063-1099` 单测覆盖白名单正例（`a@b` with `@`、多字符 `a-b.c_d` with `-._`）与反例（`a#b` with `@` 不在白名单）
- `rules.rs:1146/1154` serde roundtrip 验证 `allowSpecialChars":"_-.@"`
- `processor.rs:2979` 集成测试 params_override 更新为 `String::new()`

**4. 文档核对** — synced。技术设计文档（`docs/02-技术设计文档.md:368`）、更新日志（`docs/versions/1.1.4/更新日志.md`）、RELEASE-NOTES、QA 审计报告 E85/E86 均同步到 `allow_special_chars: String` 白名单语义。

**5. commit 核对** — 满足。Conventional Commits `feat(validate): T77 ...`，单一逻辑目的，无夹带格式化/无关重构噪音。

**6. 风险核对** — 无关键回归风险。serde 向后兼容性正确：`ExtractParams` 枚举（`rules.rs:194`）未加 `deny_unknown_fields`，旧 DB JSON 残留的 `allowSpecial: true/false` 字段会被 serde 默认忽略，`allowSpecialChars` 走 `#[serde(default)]` → 空串。无 SQL 拼接、无凭据字面量。

## defects

```yaml
defects:
  - severity: minor
    file: handoff/TASK-T77-HANDOFF.md (缺失)
    issue: handoff/ 下不存在 TASK-T77-HANDOFF.md 与 TASK-T77-REPORT.md
    impact: 流程上 reviewer 无法按 spec 5.2 读取 handoff 文件建立判断依据；本次靠主会话 prompt 内嵌的 goal/scope/acceptance 完成审查
    fix: 后续任务请按 orchestrator-workflow 规范落盘 HANDOFF + REPORT 到 handoff/ 目录
scope_check: none
docs_check: synced
```

## 附加观察（非缺陷，记录供主会话参考）

- 语义边界：若用户在白名单 `allow_special_chars` 中填入数字/字母字符（如 `"5"`），由于 `contains(c)` 在 digit/letter 判定之后兜底，会绕过 `allow_digits=false` 的限制。这是用户主动行为，与"希望特殊符号应该是可以随意填写的"需求一致，不判为缺陷。
- 行为变化：旧 DB 中 `allowSpecial: true` 的规则（全开任何特殊字符）升级后默认变为空串（不允许任何特殊字符）。这是 v1.1.4 开发期内的预期 breaking change，文档已说明，不判为缺陷。

## E2E 验证状态

```
cargo fmt --all ✓
cargo clippy --all-targets --all-features -- -D warnings ✓
cargo test --all ✓ (165 passed, 0 failed, 3 ignored)
pnpm --prefix frontend build ✓
```

## 安全约束

- SQL 全部参数绑定，无拼接 ✓
- 无凭据字面量 ✓
- Mimosa 完整审计未拿到结论（library_source_unavailable / callgraph_fact_partial），不宣称项目安全，按兼容策略继续
- 不创建 tag，不自动 merge main（等待用户手工验证 v1.1.4）
