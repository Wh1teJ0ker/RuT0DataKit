# T49 — 4 条脱敏规则收敛为通用脱敏的子规则（预设）HANDOFF

> 版本：v1.1.3
> 任务：T49
> 状态：verified_complete
> 依赖：T48（已 verified_complete）

## 1. 需求

用户要求："4 条脱敏规则 作为通用脱敏规则的子规则，不要单独列出来，请完善"。

T48 落地的 4 条独立脱敏规则（`idcard-mask`/`phone-mask`/`birthdate-mask`/`bankcard-mask`）收敛为 1 条 `general-mask` 规则的 4 个**预设**（`TemplateParams` 常量），不再单独 seed 到 DB。前端选预设 → 填充 6 个可编辑参数框 → 用户可继续修改 → 执行脱敏时透传给 `mask_column`，或经 `update_rule_template` 持久化到 DB。空模板（不选预设/全清）= 不脱敏（透传）。

经 AskUserQuestion 澄清两点语义：
- **选预设即填充可编辑参数**（选预设后参数框可继续修改）
- **空模板 = 不脱敏**（`TemplateParams` 所有字段 `None` → `SimpleMasker` 原样返回）

## 2. 改动文件清单

### Rust core（2 文件）
1. `crates/core/src/processor/rules.rs` — 删除 4 条独立规则构造函数 + 新增 `general_mask_rule()`（id `general-mask`，持空模板）+ 4 个预设函数（`idcard_preset()`/`phone_preset()`/`birthdate_preset()`/`bankcard_preset()`，均为 `TemplateParams` 常量构造器）+ `TemplateParams::is_empty()` 方法 + `with_defaults()` 从 7 条降为 4 条（3 name + general-mask）+ 测试更新（`with_defaults_loads_four_rules`、`general_mask_rule_has_empty_template`、4 个预设参数断言、`get_by_id_works` 断言旧 id 已不存在）
2. `crates/core/src/processor/masker.rs` — `mask()` 在 `rule.template.is_some()` 分支内新增空模板检测：`tpl.is_empty()` → 原样返回（不脱敏）；非空 → 走 `apply_template`。测试改用 `general_with_template(idcard_preset())` 等预设函数 + 新增 `template_empty_passthrough`、`template_mask_char_in_preset_used`、`name_mask_no_template_unchanged`

### Rust tauri（3 文件）
3. `src-tauri/src/db/mod.rs` — 新增 `update_rule_template(id, template: Option<&TemplateParams>)` 方法（`UPDATE rules SET template = ?1 WHERE id = ?2` + `params![template_json, id]`，参数绑定，不拼接 SQL）+ 测试 `update_rule_template_persists_and_reads_back` + seed/rule_kind 测试从 7→4
4. `src-tauri/src/commands/processor.rs` — `mask_column` 签名新增 `template: Option<TemplateParams>` 参数（临时覆盖，不写回 DB）；`template_override = template.as_ref().filter(|t| !t.is_empty()).cloned()`（空模板透传给规则自身 template）；新增 `update_rule_template` IPC 命令（调用 `db.update_rule_template`）；import 改为 `use serde::Serialize;`（移除未用的 Deserialize）
5. `src-tauri/src/lib.rs` — `invoke_handler!` 列表在 `update_rule_params,` 之后注册 `commands::processor::update_rule_template,`

### 前端（2 文件）
6. `frontend/src/tauri.js` — `maskColumn` 加第 5 参数 `template`；新增 `updateRuleTemplate(ruleId, template)` wrapper
7. `frontend/src/components/panels/MaskPanel.jsx` — 全量重写：`MASK_PRESETS` 常量（6 项：4 预设 + custom + empty）+ `EMPTY_TEMPLATE` 常量 + `presetKey`/`template`(6 字段)/`nameMaskChar`/`isGeneralMask` state + `stripNulls`/`detectPreset`/`initTemplateFromRule`/`buildTemplateForRun` helper + `handlePresetChange`（选预设填充 6 参数框，custom 保留当前，empty 清空）+ `handleTemplateFieldChange`（改一个参数后重新 detectPreset）+ `handleRun`/`handleSaveSetting` 分支（general-mask → maskColumn/updateRuleTemplate 带 template；name-mask → maskColumn/updateRuleParams 不带 template）+ JSX 条件渲染（general-mask 显示预设 Select + 6 个 Form.Item；name-mask 仅显示掩码字符 Input）

### 文档（4 文件）
8. `docs/versions/1.1.3/更新日志.md` — T48 + T49 两行进度 + E1~E14 验收清单 + 关键设计决策（子规则化、空模板=不脱敏、mask_column template 参数、update_rule_template IPC、DB schema v4、seed upsert-missing）+ 已知边界
9. `docs/versions/1.1.3/RELEASE-NOTES.md` — T49 子规则设计说明 + 预设表 + 子规则下拉 + 6 可编辑参数框 + mask_column template 参数 + update_rule_template IPC
10. `tests/脱敏/README.md` — 2 条 mask 规则 + 4 预设表 + T49 收敛说明 + 验证要点
11. `docs/02-技术设计文档.md` — §3.6 rules 表 id 示例改 `general-mask` + template 列描述加空模板 + seed_builtin_rules 段落改写（4 条规则、general-mask 空模板、4 预设、update_rule_template IPC、T49 删除旧 4 id）+ 顶部 v1.1.3 摘要行

### 交接（本文件 + 报告 + TASK-BOARD）
12. `handoff/TASK-BOARD.md`（追加 T49）
13. `handoff/TASK-T49-HANDOFF.md`（本文件）
14. `handoff/TASK-T49-REPORT.md`

## 3. 关键设计决策

### 3.1 子规则化：4 条独立规则 → 1 条 general-mask + 4 个预设

原 T48 的 4 条独立规则（`idcard-mask`/`phone-mask`/`birthdate-mask`/`bankcard-mask`）收敛为 `general-mask` 规则的 4 个**预设函数**（`TemplateParams` 常量构造器），不再单独 seed 到 DB：

| 预设 | 函数 | keep_prefix | keep_suffix | mask_min_len | min=max |
|---|---|---|---|---|---|
| 身份证号 | `idcard_preset()` | 6 | 4 | 8 | 18 |
| 手机号 | `phone_preset()` | 3 | 4 | 4 | 11 |
| 出生日期 | `birthdate_preset()` | 8 | 0 | 2 | 10 |
| 银行卡号 | `bankcard_preset()` | 4 | 4 | 1 | — |

`with_defaults()` 从 7 条降为 4 条（3 name + general-mask）。`general-mask` 持空模板（`TemplateParams::default()`，所有字段 `None`）。

### 3.2 空模板 = 不脱敏（透传）

`TemplateParams::is_empty()` 判断所有 6 个字段是否为 `None`。`SimpleMasker::mask()` 在 `rule.template.is_some()` 分支内：

```rust
if let Some(tpl) = &r.template {
    if tpl.is_empty() {
        return Ok(MaskResult { output: input.to_string(), rule_id: r.id.clone() });
    }
    return Ok(apply_template(tpl, &chars, n, mask_char, &r.id));
}
```

`general-mask` 默认持空模板 → 原样返回，承载"未选预设 → 不脱敏"的语义。前端选预设后填充字段 → `is_empty()` 返回 false → 走模板脱敏逻辑。

### 3.3 mask_column 新增 template 临时参数

`mask_column(sheet_id, column, rule_id, replacement, template, db)`。前端选预设 → 填充 6 参数 → 透传给 `template` 参数执行脱敏（临时覆盖规则自身 template，**不写回 DB**）。

`template_override = template.as_ref().filter(|t| !t.is_empty()).cloned()` — 空模板参数被过滤为 `None`（等价于"用规则自身 template"），避免前端传空模板意外覆盖规则已有的非空 template。

### 3.4 update_rule_template IPC（持久化）

新增 `update_rule_template(rule_id, template)` IPC → `DbManager::update_rule_template` 方法：`UPDATE rules SET template = ?1 WHERE id = ?2` + `params![template_json, id]`，参数绑定，不拼接 SQL。前端"保存设置"按钮在 general-mask 规则下调用此 IPC 持久化 template。

### 3.5 前端子规则下拉 + 6 个可编辑参数框

`MASK_PRESETS` 6 项（4 预设 + custom + empty）。选预设 → `handlePresetChange` 填充 6 个参数框（keepPrefix/keepSuffix/maskChar/maskMinLen/minLen/maxLen）；custom 保留当前值；empty 清空全部。手动改任一参数 → `detectPreset` 重新匹配最接近的预设 key（找不到匹配 → custom）。`buildTemplateForRun` 组装当前 6 参数为 template 对象（全空返回 null）。

## 4. 验收结果

- [x] `cargo fmt --check`：通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings`：通过
- [x] `cargo test --workspace`：全绿（80 个单测）
- [x] `pnpm --prefix frontend build`：通过

## 5. 安全约束

- DB 查询全部参数绑定（`params![]`），不拼接 SQL — `update_rule_template` 用 `?1`/`?2` 占位 + `params![template_json, id]`
- 测试数据为合成数据，不含真实凭据/隐私
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）
