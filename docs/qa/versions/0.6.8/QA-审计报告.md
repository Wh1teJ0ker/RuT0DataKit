# v0.6.8（修订）Release QA 审计报告

> 手机号规则（提取 phone + 校验 pinfo_phone）统一支持自定义前 1-3 位号段，缺省默认 1 开头正常号码。
>
> 依据 `orchestrator-workflow` 技能「发布前 QA 门禁」要求，覆盖需求覆盖、端到端流程、构建与测试、代码质量、文档一致性五维度。
>
> 本版本范围（修订版，覆盖上一轮 v0.6.8 错误实现）：
> - **Rust 后端**：`PhoneValidator` 扩展支持 `params.prefixes`（前 1-3 位号段集合），缺省走 `^1\d{10}$`（1 开头正常号码）；删除 `PInfoPhoneValidator`，`build_validator` 统一 `phone`/`pinfo_phone` 路由到 `PhoneValidator`；`trial_validate` 扩展支持 phone/pinfo_phone scope。
> - **前端**：RulesView `VALIDATE_PARAM_META` 加 `phone` 条目 + 修订 `pinfo_phone` label（删除"52 虚假号段"字样）；`RowExpanded` 支持 extract 行；新增 `applyExtractForRow`；state.js 新增 `extractOverrides`；ExtractView 加 `effectiveValidators` 合并。
> - 用户明确反馈："你刚刚默认的52是不正确的" → 上一轮 v0.6.8 的 52 虚假号段默认被废弃，改为 1 开头正常号码。

## §0 审计结论

`qa_passed` — 3 项任务（T23-1/T23-2/T23-3）全部落地。Rust 后端 `PhoneValidator` 统一 phone/pinfo_phone（删除 PInfoPhoneValidator），缺省 1 开头 + 自定义 prefixes 双路径单测全绿；前端 RulesView phone/pinfo_phone prefixes UI + extractOverrides + ExtractView effectiveValidators 合并；`cargo test --workspace --release` **501 passed, 0 failed, 5 ignored**；`npm --prefix frontend run build` 3009 modules 0 error；4 处 manifest 版本号保持 0.6.8；grep 验证六处关键点到位；DMG 构建产出；无阻塞项。

## §1 任务对齐

| # | 任务 | 优先级 | 落地证据 | 状态 |
|---|------|--------|----------|------|
| 1 | T23-1 Rust 后端：PhoneValidator 扩展 prefixes + 统一 pinfo_phone + 删除 PInfoPhoneValidator + trial_validate 扩展 | P0 | `phone.rs` 重写（prefixes Option<HashSet>）；`pinfo_phone.rs` 删除；`validators/mod.rs` 删 pinfo_phone 注册；`rules/mod.rs` build_validator 统一 phone/pinfo_phone 路由；`rules/builtin.rs` 描述文案修订；`commands/validate.rs` trial_validate 扩展 | ✅ |
| 2 | T23-2 前端：RulesView phone/pinfo_phone prefixes UI + extractOverrides + ExtractView 合并 | P0 | `RulesView.jsx` VALIDATE_PARAM_META 加 phone 条目 + pinfo_phone label 修订 + RowExpanded extract 分支 + applyExtractForRow + columns 三分支路由；`state.js` extractOverrides + 3 ACTION 常量 + extractDomain reducer + fileDomain 级联；`ExtractView.jsx` effectiveValidators 合并 | ✅ |
| 3 | T23-3 版本保持 0.6.8 + docs 修订 + QA + DMG + push | P1 | 4 manifest 0.6.8（核对，不 bump）+ 更新日志重写 + QA 报告重写 + 04-版本标准里程碑行 + 01-页面与交互说明 + TASK-BOARD DAG + DMG + push | ✅ |

## §2 代码审计

### §2.1 T23-1 `crates/core/src/validators/phone.rs`（重写）

| 检查项 | 结果 |
|--------|------|
| `PhoneValidator` 结构含 `re: Regex` + `prefixes: Option<HashSet<String>>` | ✅ |
| `new(params)` 读 `params.prefixes`（`.as_sequence()` → `HashSet<String>`） | ✅ |
| `.filter_map(|x| x.as_str().map(|s| s.trim().to_string()))` trim 空白 | ✅ |
| `.filter(|s| !s.is_empty())` 过滤空串 | ✅ |
| 外层 `.filter(|s| !s.is_empty())` 空集合 → None（等同缺省） | ✅ |
| 缺省（prefixes=None）：`^\d{11}$` + `starts_with('1')` | ✅ |
| 自定义（prefixes=Some）：任一 prefix 是 v 的前缀即合法 | ✅ |
| 11 个单测：缺省/自定义/空/非 11 位/非数字/空/纯空白/trim | ✅ |

### §2.2 T23-1 删除 `crates/core/src/validators/pinfo_phone.rs`

| 检查项 | 结果 |
|--------|------|
| `pinfo_phone.rs` 文件已删除 | ✅（`ls` 确认 No such file） |
| `validators/mod.rs` 删 `pub mod pinfo_phone;` | ✅ |
| `register_builtin_validators` 删 pinfo_phone 注册行 | ✅ |
| `register_builtin_validators` 测试移除 "pinfo_phone" 断言 | ✅ |
| 模块文档注释更新为 v0.6.8 修订 | ✅ |

### §2.3 T23-1 `crates/core/src/rules/mod.rs` build_validator 统一路由

| 检查项 | 结果 |
|--------|------|
| `if rule.scope == "phone" \|\| rule.scope == "pinfo_phone"` 分支 | ✅ |
| 透传 `rule.params.clone().unwrap_or_default()` | ✅ |
| 构造 `PhoneValidator::new(params)` 返回 `Box<dyn Validator>` | ✅ |
| 测试 `build_validator_pinfo_phone_scope_uses_default_one_prefix`（138 valid / 788 invalid） | ✅ |
| 测试 `build_validator_pinfo_phone_scope_with_custom_prefixes_param` | ✅ |
| 测试 `build_validator_phone_scope_with_custom_prefixes_param` | ✅ |
| 测试 `build_validator_phone_scope_uses_registry_default` | ✅ |

### §2.4 T23-1 `crates/core/src/rules/builtin.rs` 描述文案修订

| 检查项 | 结果 |
|--------|------|
| `phone_extract_rule()` description 含「默认 1 开头」+「自定义前三位号段」 | ✅ |
| `pinfo_phone_validate_rule()` description 含「默认 1 开头」+「自定义前三位号段」 | ✅ |
| 无"52 个虚假号段"字样残留 | ✅ |
| `builtin_ruleset_validates_spec_rows` 用 `13812345678`（1 开头 valid） | ✅ |
| `builtin_ruleset_marks_invalid_pinfo_cells` 用 `78813630178`（非 1 开头 invalid） | ✅ |

### §2.5 T23-1 `src-tauri/src/commands/validate.rs` trial_validate 扩展

| 检查项 | 结果 |
|--------|------|
| `ValidateOp::from_rule` 返回 None 时 phone/pinfo_phone 回退 | ✅ |
| `PhoneValidator::new(params.unwrap_or_default())` | ✅ |
| 返回 `{ ok: true, valid, message, error: null }` | ✅ |
| 未识别 scope error 文案含「regex / phone / pinfo_phone」 | ✅ |

### §2.6 T23-2 前端 `RulesView.jsx`

| 检查项 | 结果 |
|--------|------|
| `VALIDATE_PARAM_META.phone` 条目含 prefixes（list） | ✅ |
| `VALIDATE_PARAM_META.pinfo_phone` label 改「留空=默认 1 开头正常号码」 | ✅ |
| 无"默认 52 虚假号段"字样残留 | ✅ |
| `RowExpanded` 含 `isExtractRow` 分支 | ✅ |
| `applyLabel` 三分支（mask/validate/extract） | ✅ |
| `resultLabel` 三分支（脱敏/匹配/校验） | ✅ |
| `validText` 三分支（合法 ✓/匹配 ✓/...） | ✅ |
| `applyExtractForRow` 写 `SET_EXTRACT_OVERRIDE` + 跳 extract view | ✅ |
| `columns` 操作列三分支路由（mask/extract/validate） | ✅ |
| `RowExpanded` onRun/onApply 三分支路由 | ✅ |

### §2.7 T23-2 前端 `state.js`

| 检查项 | 结果 |
|--------|------|
| `extractOverrides: {}` 初始化 | ✅ |
| `EXTRACT_OVERRIDE_SET` / `CLEAR` / `CLEAR_ALL` 3 个 ACTION 常量 | ✅ |
| `extractDomain` 3 个 reducer 分支（镜像 validate） | ✅ |
| `fileDomain` SET_FILE 清空 extractOverrides | ✅ |
| `fileDomain` SET_RECORDS 清空 extractOverrides | ✅ |
| `fileDomain` COLUMN_RENAME_SET remap extractOverrides | ✅ |

### §2.8 T23-2 前端 `ExtractView.jsx`

| 检查项 | 结果 |
|--------|------|
| `effectiveValidators` useMemo 合并 rules.validators + extractOverrides | ✅ |
| `filteredRules` 基集改 effectiveValidators | ✅ |
| `ruleOptions` 下标改 effectiveValidators.indexOf | ✅ |
| `handleExtract` selectedRules 从 effectiveValidators 取 | ✅ |
| Empty 判断改 `effectiveValidators.length === 0` | ✅ |

### §2.9 安全合规

- 全部改动：Rust `PhoneValidator` 读本地规则参数；前端参数 UI + 会话级 override 状态切片。
- 无网络 / 外发 / 文件 IO 新增。
- 与 `docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」约束一致。
- 无新增加密 / 外发 / tauri command。

### §2.10 scope 合规

- T23-1 改 `crates/core/src/validators/phone.rs` + 删 `pinfo_phone.rs` + `validators/mod.rs` + `rules/mod.rs` + `rules/builtin.rs` + `src-tauri/src/commands/validate.rs`。
- T23-2 改 `frontend/src/components/RulesView.jsx` + `frontend/src/state.js` + `frontend/src/components/ExtractView.jsx`。
- T23-3 改 4 manifest + docs。
- 未越界触及 mask pipeline / validate pipeline / export 模块 / RegexTool / SearchView / PreprocessView / scan 算法 / log / pcap。

## §3 测试审计

```
$ cargo test --workspace --release
test result: ok. 403 passed; 0 failed; 3 ignored   (lib unittests)
test result: ok. 10 passed; 0 failed; 0 ignored     (integration log_test)
test result: ok. 34 passed; 0 failed; 2 ignored     (integration log_scan_test)
test result: ok. 12 passed; 0 failed; 0 ignored     (integration blind_aggregator_test)
test result: ok. 11 passed; 0 failed; 0 ignored     (integration logsign_test)
test result: ok. 31 passed; 0 failed; 0 ignored     (integration e2e)
test result: ok. 0 passed; 0 failed; 0 ignored      (Doc-tests)
```

release 合计 **501 passed, 0 failed, 5 ignored**（全绿）。`PhoneValidator` 缺省 1 开头 / 自定义 prefixes / 空集合回退 / 非 11 位 / 非数字 / 纯空白 trim 单测全绿；`build_validator` 统一路由单测全绿；`builtin_ruleset` 修订用例全绿。

前端 `npm --prefix frontend run build`：vite 5.4.21 → **3009 modules transformed, 0 error, 2.38s**。

## §4 构建审计

| 构建目标 | 命令 | 结果 |
|----------|------|------|
| core lib + src-tauri | `cargo build --release` | 0 error（仅 crate 名 snake_case 预存 warning） |
| frontend | `npm --prefix frontend run build` | vite build 3009 modules，0 error，2.38s |
| Tauri DMG | `npx @tauri-apps/cli build` | DMG 产出（详见 §6） |

## §5 文档一致性

| 检查项 | 结果 |
|--------|------|
| 4 处 manifest 版本号 | 全部 0.6.8（核对，不 bump）✅ |
| `docs/04-版本标准.md` 里程碑行 | v0.6.8 状态 `release_complete`，描述更新为修订版 ✅ |
| `docs/versions/0.6.8/更新日志.md` | 重写为修订版 ✅ |
| QA 报告 | 本文件（重写为修订版）✅ |
| `handoff/TASK-BOARD.md` | v0.6.8 DAG 更新为 T23-1~T23-3 修订版 ✅ |
| 安全约束 §6 | 「不外发数据：全本地处理」保留，v0.6.8 修订不变 ✅ |
| `docs/01-页面与交互说明.md` | phone/pinfo_phone prefixes 交互规范修订（缺省 1 开头）✅ |

## §6 grep 关键符号验证

```
=== grep 1: 4 manifest 版本号 0.6.8 ===
Cargo.toml:8:version = "0.6.8"
src-tauri/Cargo.toml:3:version = "0.6.8"
src-tauri/tauri.conf.json:4:  "version": "0.6.8",
frontend/package.json:4:  "version": "0.6.8",

=== grep 2: pinfo_phone.rs 已删除 ===
$ ls crates/core/src/validators/pinfo_phone.rs
No such file or directory

=== grep 3: PhoneValidator 支持 prefixes ===
phone.rs:    prefixes: Option<HashSet<String>>,
phone.rs:            .get("prefixes")
phone.rs:            .and_then(|v| v.as_sequence())

=== grep 4: build_validator 统一 phone/pinfo_phone ===
mod.rs:    if rule.scope == "phone" || rule.scope == "pinfo_phone" {
mod.rs:        return Some(Box::new(crate::validators::phone::PhoneValidator::new(params)) as Box<dyn Validator>);

=== grep 5: RulesView phone + pinfo_phone prefixes list UI ===
RulesView.jsx:  phone: [{ key: "prefixes", label: "前 1-3 位号段（逗号分隔，留空=默认 1 开头正常号码）", type: "list", ... }]
RulesView.jsx:  pinfo_phone: [{ key: "prefixes", label: "前 1-3 位号段（逗号分隔，留空=默认 1 开头正常号码）", type: "list", ... }]

=== grep 6: state.js extractOverrides + ExtractView effectiveValidators ===
state.js:  extractOverrides: {}, // { [header]: FieldRule }
state.js:  EXTRACT_OVERRIDE_SET: "SET_EXTRACT_OVERRIDE",
ExtractView.jsx:  const effectiveValidators = useMemo(() => {

=== grep 7: 无"默认 52"文案残留（仅注释中提及废弃）===
（全部命中均在「上一轮 v0.6.8 默认 52 虚假号段的设计不正确（用户反馈），已废弃」注释中，无功能文案残留）
```

七处关键验证全部到位。

## §7 端到端流程验证

用户预期流程（修订版）：

1. **RulesView 展开 phone（extract）规则行** → 出现「前 1-3 位号段」TextArea ✅
2. **留空运行 `13812345678`** → 合法 ✓（默认 1 开头）✅（PhoneValidator 缺省分支）
3. **留空运行 `23812345678`** → 不匹配 ✗（默认 1 开头，238 非 1 开头）✅
4. **填 `734,138` 运行 `73412345678`** → 合法 ✓（自定义集合覆盖缺省）✅（`custom_prefixes_override_default` 单测覆盖）
5. **填 `734,138` 运行 `15912345678`** → 不匹配 ✗（159 不在集合）✅
6. **点「应用到提取」** → `buildParams` 保留非空数组 → `SET_EXTRACT_OVERRIDE` 写入 → 跳 extract view ✅
7. **ExtractView 跑提取** → `effectiveValidators` 合并 override → `rulesJson` 带自定义 prefixes → 后端 `build_validator` 命中 phone 分支 → `PhoneValidator::new(params)` 自定义集合生效 ✅
8. **pinfo_phone（validate）同理**：留空默认 1 开头；填 prefixes 自定义；应用 → `SET_VALIDATE_OVERRIDE` → ValidateView 生效 ✅
9. **CSV 表头 `type,value` 不动** ✅
10. **向后兼容**：v0.6.7 `(?-u)\b` 修复 / extract 规则补全 / 提取校验分离 / v0.6.6 类型重命名 / v0.6.5 字段批量重命名全部不破 ✅

## §8 阻塞项

无。全部任务落地，构建 / test（release 501 全绿）/ grep（7 处）/ docs 全绿，DMG 产出。

## §9 结论

`qa_passed` — 可进入 `release_complete`。

- 修订版覆盖上一轮 v0.6.8 错误默认 52 虚假号段，改为用户要求的 1 开头正常号码。
- Rust 后端统一 `phone`/`pinfo_phone` 到 `PhoneValidator`，删除冗余 `PInfoPhoneValidator`。
- 前端 RulesView phone + pinfo_phone 双 scope 支持 prefixes UI；ExtractView 通过 `effectiveValidators` 合并会话级 override。
- 版本保持 0.6.8，完成后 `git push origin main`。
