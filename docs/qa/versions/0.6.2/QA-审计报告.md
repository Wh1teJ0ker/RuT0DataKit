# v0.6.2 Release QA 审计报告

> 新功能版本发布前全局 QA 审计。依据 `orchestrator-workflow` 技能「发布前 QA 门禁」要求，覆盖需求覆盖、端到端流程、构建与测试、代码质量、文档一致性五维度。
>
> 本版本范围：数据校验通用规则 — 正则校验。新增内置 `regex_validate_rule` + `trial_validate` Tauri 命令 + RulesView 校验参数表单/试运行/应用，与现有脱敏模版 UX 完全对称。

## §0 审计结论

`qa_passed` — 4 项任务（T17-1/T17-2/T17-3/T17-4）全部落地，三层改动（core builtin / tauri command / frontend UI）齐备，cargo test 全绿（493 passed，比 v0.6.1 基线 491 +2 新测试），3 构建目标 0 error，4 处 manifest 版本号同步 0.6.2，docs 一致，grep 验证三处关键符号到位，无阻塞项。

## §1 任务对齐

| # | 任务 | 优先级 | 落地证据 | 状态 |
|---|------|--------|----------|------|
| 1 | T17-1 Core regex_validate_rule | P0 | `builtin.rs:199` 函数定义 + `builtin.rs:48` 加入 ruleset + `mod.rs:24` pub use + 2 新测试 | ✅ |
| 2 | T17-2 Tauri trial_validate | P0 | `validate.rs:64` 命令定义 + `main.rs:42` 注册 + Value::Null 修复 | ✅ |
| 3 | T17-3 Frontend RulesView 校验 | P0 | `tauri.js:324` trialValidate + `RulesView.jsx:75` VALIDATE_PARAM_META + getParamMeta + runValidateTrialForRow + applyValidateForRow | ✅ |
| 4 | T17-4 版本号 + docs | P1 | 4 manifest 0.6.2 + 更新日志 + QA 报告 + 04-版本标准里程碑行 | ✅ |

## §2 代码审计

### §2.1 Core regex_validate_rule（T17-1）

| 检查项 | 结果 |
|--------|------|
| `field: "自定义正则"` | ✅ |
| `scope: "regex"` | ✅（与 `ValidateOp::from_rule` 的 `"regex"` arm 对齐） |
| `tag: "validate"` | ✅（单值，与 v0.4.4 规则引擎重构一致） |
| `params: None`（出厂无参数） | ✅（用户填参数后由 SET_VALIDATE_OVERRIDE 写入） |
| `description: Some("正则校验（自定义 pattern）")` | ✅ |
| `builtin_ruleset().validators.len()` 7→8 | ✅ |
| 新增 2 测试（fields + with_pattern_builds_and_validates） | ✅ |

### §2.2 Tauri trial_validate（T17-2）

| 检查项 | 结果 |
|--------|------|
| 命令签名 `(scope, params_json, sample_value) -> Result<Value, String>` | ✅（与 trial_mask 对称） |
| params_json 解析（None/""/null→None，空 map→None，否则 HashMap） | ✅ |
| 合成 FieldRule `{ field: "", scope, tag: "validate", params, ... }` | ✅ |
| `ValidateOp::from_rule` → None 时返回 `{ ok: false, error: "未识别的校验算子 scope" }` | ✅ |
| `apply_validate_op` → `{ ok: true, valid, message, error: null }` | ✅ |
| `main.rs` `generate_handler!` 注册 `commands::trial_validate` | ✅ |
| `Value::Null` 替代 `null`（json! 宏 scope 修复） | ✅ |

### §2.3 Frontend RulesView 校验（T17-3）

| 检查项 | 结果 |
|--------|------|
| `tauri.js` `trialValidate` 导出，invoke `"trial_validate"` | ✅ |
| `VALIDATE_PARAM_META = { regex: [pattern/message/empty_message] }` | ✅ |
| `getParamMeta(scope)`：先 MASK 后 VALIDATE 后 [] | ✅ |
| `buildParams` 用 `getParamMeta(scope)` | ✅ |
| `RowExpanded` 有 meta 就渲染表单（不再只看 isMaskRow） | ✅ |
| 结果区 mask→masked / validate→valid（合法✓ / 非法✗+message） | ✅ |
| 应用按钮文案随规则类型（mask→脱敏 / validate→校验） | ✅ |
| `runValidateTrialForRow` 调 trialValidate | ✅ |
| `applyValidateForRow` dispatch SET_VALIDATE_OVERRIDE + 跳 validate | ✅ |
| `expandedRowRender` 按 `__kind` 传 onRun/onApply | ✅ |
| 操作列有 meta 就显示按钮 | ✅ |
| 底部提示文本追加校验说明 | ✅ |

### §2.4 安全合规

- `trial_validate` 不读文件、不落盘，只对单条样例值在内存中校验。与 `trial_mask` 一致。
- 无网络调用，符合 §6「不外发数据：全本地处理；规则与样本不上传」约束。
- 无新增文件 IO / 加密 / 外发逻辑。

### §2.5 scope 合规

- T17-1 仅改 `crates/core/src/rules/builtin.rs` + `mod.rs`。
- T17-2 仅改 `src-tauri/src/commands/validate.rs` + `main.rs`。
- T17-3 仅改 `frontend/src/tauri.js` + `frontend/src/components/RulesView.jsx`。
- T17-4 仅改 4 manifest + docs。
- 未越界触及 extract / mask pipeline / 加密模块 / state.js。

## §3 测试审计

```
$ cargo test --workspace
test result: ok. 395 passed; 0 failed; 3 ignored   (lib unittests)
test result: ok. 10 passed; 0 failed; 0 ignored     (integration log_test)
test result: ok. 34 passed; 0 failed; 2 ignored     (integration log_scan_test)
test result: ok. 12 passed; 0 failed; 0 ignored     (integration blind_aggregator_test)
test result: ok. 11 passed; 0 failed; 0 ignored     (integration logsign_test)
test result: ok. 31 passed; 0 failed; 0 ignored     (integration e2e)
test result: ok. 0 passed; 0 failed; 0 ignored      (doc-tests)
```

合计 **493 passed, 0 failed, 5 ignored**，全绿。比 v0.6.1 基线 491 +2（T17-1 新增 `regex_validate_rule_fields` + `regex_validate_rule_with_pattern_builds_and_validates`）。

## §4 构建审计

| 构建目标 | 命令 | 结果 |
|----------|------|------|
| core lib | `cargo build -p ruT0-data-kit-core` | 0 error，1 warning（历史遗留 crate 名 `ruT0_data_kit_core should have a snake case name`） |
| src-tauri binary | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error |
| frontend | `npm --prefix frontend run build` | vite build 3008 modules，0 error，2.21s |

## §5 文档一致性

| 检查项 | 结果 |
|--------|------|
| 4 处 manifest 版本号（+ core workspace 继承） | 全部 0.6.2 ✅ |
| `docs/04-版本标准.md` 里程碑行 | v0.6.2 状态 `release_complete` ✅ |
| `docs/versions/0.6.2/更新日志.md` | 回填完毕，状态 `release_complete` ✅ |
| QA 报告 | 本文件 ✅ |
| 安全约束 §6 | 「不外发数据：全本地处理」保留，v0.6.2 不变 ✅ |

## §6 grep 关键符号验证

```
=== grep regex_validate_rule ===
crates/core/src/rules/builtin.rs:48:            regex_validate_rule(),
crates/core/src/rules/builtin.rs:199:pub fn regex_validate_rule() -> FieldRule {
crates/core/src/rules/builtin.rs:554:    fn regex_validate_rule_fields() {
crates/core/src/rules/builtin.rs:567:    fn regex_validate_rule_with_pattern_builds_and_validates() {
crates/core/src/rules/mod.rs:24:    phone_extract_rule, regex_replace_mask_rule, regex_validate_rule, ...

=== grep trial_validate ===
src-tauri/src/commands/validate.rs:1://! ...（v0.6.2 新增 trial_validate）。
src-tauri/src/commands/validate.rs:64:pub fn trial_validate(
src-tauri/src/main.rs:42:            commands::trial_validate,

=== grep VALIDATE_PARAM_META ===
frontend/src/components/RulesView.jsx:75:const VALIDATE_PARAM_META = {
frontend/src/components/RulesView.jsx:85:  return MASK_PARAM_META[scope] ?? VALIDATE_PARAM_META[scope] ?? [];
frontend/src/components/RulesView.jsx:433:      const r = await trialValidate(record.scope, paramsJson, rs.sample);
frontend/src/tauri.js:324:export async function trialValidate(scope, paramsJson, sampleValue) {
```

三处关键符号（core / tauri / frontend）全部到位。

## §7 端到端流程验证

用户预期流程（与脱敏模版对称）：
1. 启动 App → `list_builtin_rules` 返回 8 validators（含 `自定义正则` regex validate）→ RulesView 规则列表可见 ✅
2. 在 RulesView 展开正则校验行 → 填 pattern（如 `^\d{4}-\d{2}-\d{2}$`）+ 可选 message/empty_message → 选 CSV 表头字段（自动预填首行样例值）→ 点「运行」→ `trialValidate` → 显示「合法 ✓ / 非法 ✗ + message」 ✅
3. 点「应用并跳转数据校验」→ dispatch `SET_VALIDATE_OVERRIDE` → 跳转 validate 视图 → 表头映射区可见该规则 ✅
4. validate 视图点「应用」→ `runValidateRecords` 合并 override + 全局兜底 → 预览非法红底 ✅

## §8 阻塞项

无。全部任务落地，构建/测试/grep/docs 全绿。

## §9 结论

`qa_passed` — 可进入 `release_complete`。
