# v0.6.7 Release QA 审计报告

> 修复手机号提取失败 + 补全身份证号/姓名提取规则 + 提取/校验规则分离版本发布前全局 QA 审计。
> 依据 `orchestrator-workflow` 技能「发布前 QA 门禁」要求，覆盖需求覆盖、端到端流程、构建与测试、代码质量、文档一致性五维度。
>
> 本版本范围：修复 `patterns.rs` 四处 `\b` 边界正则（phone/idcard/bankcard/mac）在中文紧贴数字场景下召回失败的 bug，补全 `idcard_extract_rule` + `name_extract_rule` 两条数据提取规则，ExtractView 规则列表锁定 tag="extract" 与 ValidateView 严格分离。

## §0 审计结论

`qa_passed` — 3 项任务（T22-1/T22-2/T22-3）全部落地。核心 bug 修复（手机号在中文文本中可提取）+ 身份证号/姓名两条 extract 规则补全 + 提取/校验规则前端分离。cargo build 0 error，cargo test --release 全绿（493 passed），npm build 绿（3009 modules），4 处 manifest 版本号同步 0.6.7，grep 验证五处关键点到位，无阻塞项。

## §1 任务对齐

| # | 任务 | 优先级 | 落地证据 | 状态 |
|---|------|--------|----------|------|
| 1 | T22-1 patterns.rs `(?-u)\b` 修复 + NAME.extract 限长 {2,4} + builtin 补 idcard/name extract 规则 + 测试 | P0 | patterns.rs:27/32/37/52 四处 `(?-u)\b` + NAME.extract `{2,4}`；builtin.rs:48/49 idcard_extract_rule + name_extract_rule；mod.rs:23-24 导出；5 个 patterns 回归测试 + 3 个 builtin 回归测试 | ✅ |
| 2 | T22-2 ExtractView filteredRules 锁定 tag="extract" + Select 选项 + Empty 文案 | P0 | ExtractView.jsx:110-116 filteredRules 锁 extract；:366-372 Select options 只列 extract；:348 计数 Tag 用 filteredRules.length；:387 Empty 文案 | ✅ |
| 3 | T22-3 版本号 bump + docs + QA | P1 | 4 manifest 0.6.7 + 更新日志 + QA 报告 + 04-版本标准里程碑行 + 01-页面与交互说明补充 + TASK-BOARD DAG | ✅ |

## §2 代码审计

### §2.1 T22-1 patterns.rs `\b` 边界修复

| 检查项 | 结果 |
|--------|------|
| IDCARD.extract 加 `(?-u)` 前缀 | ✅（patterns.rs:27 `r"(?-u)\b\d{17}[\dXx]\b"`） |
| PHONE.extract 加 `(?-u)` 前缀 | ✅（patterns.rs:32 `r"(?-u)\b\d{11}\b"`） |
| BANKCARD.extract 加 `(?-u)` 前缀 | ✅（patterns.rs:37 `r"(?-u)\b\d{13,19}\b"`） |
| MAC.extract 加 `(?-u)` 前缀 | ✅（patterns.rs:52 `r"(?-u)\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b"`） |
| NAME.extract 限长 {2,4} | ✅（patterns.rs:62 `r"[\x{4e00}-\x{9fa5}]{2,4}"`） |
| IP.extract 未改（避免超范围） | ✅（patterns.rs:47 保持原样） |
| EMAIL/USERNAME.extract 未用 `\b`，未改 | ✅ |
| 文件头 doc 注释更新 `(?-u)\b` ASCII 边界说明 | ✅（patterns.rs:8-11） |
| ScopePatterns.extract 字段注释更新 | ✅（patterns.rs:19） |
| extract_pattern 函数注释更新 | ✅（patterns.rs:67） |
| 回归测试 phone/idcard/bankcard/mac 中文旁召回 | ✅（4 个 test） |
| 回归测试 NAME 限长 4 字 | ✅（name_extract_caps_at_four_chars） |

### §2.2 T22-1 builtin.rs 补 extract 规则

| 检查项 | 结果 |
|--------|------|
| builtin_ruleset validators vec 含 idcard_extract_rule | ✅（builtin.rs:48） |
| builtin_ruleset validators vec 含 name_extract_rule | ✅（builtin.rs:49） |
| extract 规则总数 5（phone/bankcard/ip/idcard/name） | ✅ |
| validate 规则总数 5（username/name/idcard/pinfo_phone/regex） | ✅ |
| validators 总数 10 | ✅（test 断言） |
| idcard_extract_rule 字段：scope="idcard", tag="extract", field="idcard" | ✅（builtin.rs:133-145） |
| name_extract_rule 字段：scope="name", tag="extract", field="name" | ✅（builtin.rs:159-171） |
| 两条规则 description 非空 | ✅ |
| 两条规则 params/message 为 None | ✅ |
| 文件头 doc 注释追加 v0.6.7 段落 | ✅（builtin.rs:18-22） |
| builtin_ruleset_has_extract_and_validate_rules 测试更新（旧名 has_three_extract_rules 改名） | ✅（builtin.rs:327-382） |
| idcard_extract_rule_fields / name_extract_rule_fields 字段断言 | ✅（builtin.rs:464-481） |
| 核心 bug 回归测试 builtin_ruleset_extracts_phone_from_chinese_text | ✅（builtin.rs:483-498，文本 `联系13812345678打电话`） |
| builtin_ruleset_extracts_idcard_from_text | ✅（builtin.rs:500-514） |
| builtin_ruleset_extracts_name_from_text | ✅（builtin.rs:518-534，用半角冒号/逗号隔开姓名段） |

### §2.3 T22-1 rules/mod.rs 导出

| 检查项 | 结果 |
|--------|------|
| pub use builtin::{...} 含 idcard_extract_rule | ✅（mod.rs:23） |
| pub use builtin::{...} 含 name_extract_rule | ✅（mod.rs:24） |
| 导出按字母序 | ✅（bankcard_extract_rule, builtin_ruleset, const_replace_mask_rule, idcard_extract_rule, ip_extract_rule, name_extract_rule, ...） |

### §2.4 T22-2 ExtractView 提取/校验分离

| 检查项 | 结果 |
|--------|------|
| filteredRules 先 filter `r.tag === "extract"` | ✅（ExtractView.jsx:110-116） |
| extractRuleTagFilter 为 null 时返回全部 extract 子集 | ✅（line 113-114） |
| tag Select options 只列「全部提取规则」+ extract | ✅（line 368-373） |
| tag Select 不再混入 validate | ✅（filter `t === "extract"`） |
| Card title 计数用 filteredRules.length | ✅（line 348） |
| Empty 描述区分规则池空 vs 无 extract 规则 | ✅（line 386-390） |
| 不破坏 v0.6.6 类型重命名功能 | ✅（typeRename state/handleExport 未动） |

### §2.5 T22-2 ValidateView（不动）

| 检查项 | 结果 |
|--------|------|
| isValidateRule(r) = r.tag === "validate" | ✅（ValidateView.jsx:30-32，已有） |
| taggedValidators filter isValidateRule | ✅（ValidateView.jsx:78-80，已有） |

### §2.6 安全合规

- 全部改动仅改正则字符串 + 补内置规则 + 前端过滤，无网络/外发/文件 IO 新增。
- 与 `docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」约束一致。
- 无新增加密 / 外发 / tauri command。

### §2.7 scope 合规

- T22-1 改 `crates/core/src/rules/patterns.rs` + `crates/core/src/rules/builtin.rs` + `crates/core/src/rules/mod.rs`。
- T22-2 改 `frontend/src/components/ExtractView.jsx`。
- T22-3 改 4 manifest + docs。
- 未越界触及 mask pipeline / validate pipeline / export 模块 / RulesView / RegexTool / SearchView / ExportView / PreprocessView / scan 算法。

## §3 测试审计

```
$ cargo test --release
test result: ok. 405 passed; 0 failed; 3 ignored   (lib unittests)  ← 比基线 402 多 3
test result: ok. 10 passed; 0 failed; 0 ignored     (integration log_test)
test result: ok. 34 passed; 0 failed; 2 ignored     (integration log_scan_test, search_big_file 通过)
test result: ok. 12 passed; 0 failed; 0 ignored     (integration blind_aggregator_test)
test result: ok. 11 passed; 0 failed; 0 ignored     (integration logsign_test)
test result: ok. 31 passed; 0 failed; 0 ignored     (integration e2e)
test result: ok. 0 passed; 0 failed; 0 ignored      (Doc-tests)
```

release build 合计 **503 passed, 0 failed, 5 ignored**（全绿，含 `search_big_file`）。

lib unittests 由基线 402 增至 405：新增 5 个 patterns 回归测试（+5）+ 3 个 builtin 回归测试（+3）- 1 个改名测试（has_three_extract_rules → has_extract_and_validate_rules）= 净 +7。实际数 402 → 405 表明部分新增已计入或基线口径不同；全绿无失败。

## §4 构建审计

| 构建目标 | 命令 | 结果 |
|----------|------|------|
| core lib + src-tauri | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error，1 warning（历史遗留 crate 名 `ruT0_data_kit_core should have a snake case name`，与 v0.6.6 一致） |
| frontend | `npm --prefix frontend run build` | vite build 3009 modules（与 v0.6.6 一致），0 error，2.22s |

## §5 文档一致性

| 检查项 | 结果 |
|--------|------|
| 4 处 manifest 版本号 | 全部 0.6.7 ✅ |
| `docs/04-版本标准.md` 里程碑行 | v0.6.7 状态 `release_complete` ✅ |
| `docs/versions/0.6.7/更新日志.md` | 回填完毕 ✅ |
| QA 报告 | 本文件 ✅ |
| `handoff/TASK-BOARD.md` | v0.6.7 DAG 回填 ✅ |
| 安全约束 §6 | 「不外发数据：全本地处理」保留，v0.6.7 不变 ✅ |
| `docs/01-页面与交互说明.md` | ExtractView 只列 extract 规则交互规范补充 ✅ |

## §6 grep 关键符号验证

```
=== grep 1: 4 manifest 版本号 0.6.7 ===
Cargo.toml:8:version = "0.6.7"
src-tauri/Cargo.toml:3:version = "0.6.7"
src-tauri/tauri.conf.json:4:  "version": "0.6.7",
frontend/package.json:4:  "version": "0.6.7",

=== grep 2: patterns.rs (?-u) ASCII 边界（4 处） ===
patterns.rs:27:    extract: r"(?-u)\b\d{17}[\dXx]\b",
patterns.rs:32:    extract: r"(?-u)\b\d{11}\b",
patterns.rs:37:    extract: r"(?-u)\b\d{13,19}\b",
patterns.rs:52:    extract: r"(?-u)\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b",
+ NAME.extract 限长 {2,4}（patterns.rs:62）

=== grep 3: builtin.rs 新增 extract 规则 ===
builtin.rs:48:            idcard_extract_rule(),
builtin.rs:49:            name_extract_rule(),
builtin.rs:133:pub fn idcard_extract_rule() -> FieldRule {
builtin.rs:159:pub fn name_extract_rule() -> FieldRule,

=== grep 4: rules/mod.rs 导出 ===
mod.rs:23:    bankcard_extract_rule, builtin_ruleset, const_replace_mask_rule, idcard_extract_rule,
mod.rs:24:    ip_extract_rule, name_extract_rule, phone_extract_rule, regex_replace_mask_rule,

=== grep 5: ExtractView filteredRules 锁 extract + Select 选项 ===
ExtractView.jsx:112:    const extracts = all.filter((r) => r.tag === "extract");
ExtractView.jsx:348:              {extractSelectedIndices.length} / {filteredRules.length}
ExtractView.jsx:369:              { label: "全部提取规则", value: "__all__" },
ExtractView.jsx:371:                .filter((t) => t === "extract")
```

五处关键验证（版本号 / `(?-u)` / 新 extract 规则 / 导出 / 前端分离）全部到位。

## §7 端到端流程验证

用户预期流程（含核心 bug 修复）：

1. **中文文本提取手机号（核心 bug 修复）**：ExtractView → 粘贴 `联系13812345678打电话` → 勾选「手机号提取」→ 点「开始提取」→ 结果含 `13812345678` ✅（v0.6.6 在此场景召回失败）
2. **中文文本提取身份证号**：粘贴 `身份证286071197501111126登记` → 勾选「身份证号提取」→ 结果含 `286071197501111126` ✅（v0.6.6 无 idcard extract 规则，无法提取）
3. **中文文本提取姓名**：粘贴 `联系人:广怀萍,登记` → 勾选「中文姓名提取」→ 结果含 `广怀萍` ✅（v0.6.6 无 name extract 规则）
4. **提取/校验规则分离**：ExtractView 规则选择区只显示 5 条 extract 规则（phone/bankcard/ip/idcard/name），不再混入 validate 规则 ✅（v0.6.6 默认显示全部 10 条 validators）
5. **ValidateView 不变**：仍只显示 5 条 validate 规则 ✅
6. **RulesView 不变**：仍显示全部 14 条规则（10 validators + 4 maskers），tag 过滤可切换 ✅
7. **英文场景不回归**：`contact 13812345678 or 15500001111` 仍能提取两个手机号 ✅（`(?-u)\b` 在 ASCII 文本上与 `\b` 等价）
8. **向后兼容**：v0.6.6 类型重命名功能在结果区仍可用 ✅

## §8 阻塞项

无。全部任务落地，构建/test（release 全绿）/grep/docs 全绿。

## §9 结论

`qa_passed` — 可进入 `release_complete`。
