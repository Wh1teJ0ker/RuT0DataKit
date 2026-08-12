# T49 — 4 条脱敏规则收敛为通用脱敏的子规则（预设）REPORT

> 版本：v1.1.3
> 任务：T49
> 状态：verified_complete
> 依赖：T48（已 verified_complete）

## 1. 完成内容

### 1.1 Rust core（2 文件）

**`crates/core/src/processor/rules.rs`**：
- 删除 4 条独立规则构造函数（`idcard_mask_rule`/`phone_mask_rule`/`birthdate_mask_rule`/`bankcard_mask_rule`）
- 新增 `general_mask_rule()`：id `general-mask`，name "通用脱敏"，kind Mask，`template: Some(TemplateParams::default())`（空模板）
- 新增 4 个预设函数（`TemplateParams` 常量构造器，模块级函数，非 Rule）：
  - `idcard_preset()`：`TemplateParams::new(6, 4, 8).with_len_range(18, 18)`
  - `phone_preset()`：`TemplateParams::new(3, 4, 4).with_len_range(11, 11)`
  - `birthdate_preset()`：`TemplateParams::new(8, 0, 2).with_len_range(10, 10)`
  - `bankcard_preset()`：`TemplateParams::new(4, 4, 1)`（无 len guard）
- 新增 `TemplateParams::is_empty()` 方法：6 个字段全 `None` → true
- `with_defaults()` 从 7 条降为 4 条（3 name + general-mask）
- 测试更新：
  - `with_defaults_loads_four_rules`（7→4，mask_count 4→2）
  - 新增 `general_mask_rule_has_empty_template`
  - 新增 4 个预设参数断言（`idcard_preset_params`/`phone_preset_params`/`birthdate_preset_params`/`bankcard_preset_params`）
  - `get_by_id_works` 断言旧 4 id（idcard-mask/phone-mask/birthdate-mask/bankcard-mask）已不存在
  - `register_overwrites_same_id`（7→4）

**`crates/core/src/processor/masker.rs`**：
- `mask()` 在 `rule.template.is_some()` 分支内新增空模板检测：
  ```rust
  if tpl.is_empty() {
      return Ok(MaskResult { output: input.to_string(), rule_id: r.id.clone() });
  }
  ```
- 测试改用 `general_with_template(idcard_preset())` 等 helper（构造 general-mask + 指定 template）
- 新增测试：`template_empty_passthrough`（general-mask 空模板 → 原样返回）、`template_mask_char_in_preset_used`（预设内 mask_char 优先于默认 *）、`name_mask_no_template_unchanged`（name-mask 无 template → 旧逻辑不变）
- 原有模板测试（idcard/phone/birthdate/bankcard 各 1 + guard + replacement 覆盖）改用预设函数，断言不变

### 1.2 Rust tauri（3 文件）

**`src-tauri/src/db/mod.rs`**：
- 新增 `update_rule_template(id, template: Option<&TemplateParams>) -> Result<bool, DbError>`：
  - `UPDATE rules SET template = ?1 WHERE id = ?2` + `params![template_json, id]`（参数绑定）
  - `template = Some` → `serde_json::to_string` 写 JSON；`None` → 写 NULL
  - 返回是否命中（n > 0 → true；n == 0 → 查 COUNT 判断 id 是否存在）
- 新增测试 `update_rule_template_persists_and_reads_back`：写 Some(tpl) → list_rules 读回断言 → 写 None → 读回断言 None → 不存在 id 返回 false
- seed 测试 `seed_builtin_rules_inserts_seven_when_empty` → `seed_builtin_rules_inserts_four_when_empty`（7→4）
- `rule_kind_round_trip_through_db`（7→4）
- 删除 `seed_builtin_rules_upserts_missing_on_existing_db`（T48 的 4 条新规则升级场景，T49 后 general-mask 单条规则由原 `seed_builtin_rules_inserts_three_when_empty` 覆盖）

**`src-tauri/src/commands/processor.rs`**：
- `mask_column` 签名新增第 5 参数 `template: Option<TemplateParams>`：
  ```rust
  pub fn mask_column(
      sheet_id: i64, column: String, rule_id: Option<String>,
      replacement: Option<String>, template: Option<TemplateParams>,
      db: tauri::State<'_, crate::db::DbManager>,
  ) -> Result<MaskResult, String>
  ```
- `template_override = template.as_ref().filter(|t| !t.is_empty()).cloned()`（借用 + clone，避免 move；空模板过滤为 None）
- 临时规则构造分支：`rule_override.is_some() || template_override.is_some()` → 克隆规则或构造 temp-mask，覆盖 replacement/template
- `params_json` 审计日志加 `"template": template`
- 新增 `update_rule_template` IPC 命令：
  ```rust
  #[tauri::command]
  pub fn update_rule_template(
      rule_id: String, template: Option<TemplateParams>,
      db: tauri::State<'_, crate::db::DbManager>,
  ) -> Result<bool, String> {
      db.update_rule_template(&rule_id, template.as_ref()).map_err(|e| e.to_string())
  }
  ```
- import 改 `use serde::Serialize;`（移除未用的 Deserialize，clippy 修复）

**`src-tauri/src/lib.rs`**：
- `invoke_handler!` 列表在 `commands::processor::update_rule_params,` 之后加 `commands::processor::update_rule_template,`

### 1.3 前端（2 文件）

**`frontend/src/tauri.js`**：
- `maskColumn(sheetId, column, ruleId, replacement, template)` 加第 5 参数
- 新增 `updateRuleTemplate(ruleId, template)` → `invoke("update_rule_template", { ruleId, template })`

**`frontend/src/components/panels/MaskPanel.jsx`**（全量重写）：
- `MASK_PRESETS` 常量（6 项）：4 预设（idcard/phone/birthdate/bankcard，各带 6 参数）+ custom（params: null）+ empty（params: {}）
- `EMPTY_TEMPLATE` 常量：6 字段全 null
- state：`presetKey`（默认 "empty"）、`template`（6 字段对象）、`nameMaskChar`、`isGeneralMask = maskRuleId === "general-mask"`
- helper：
  - `stripNulls(t)`：删除值为 null 的字段（用于 detectPreset 比较）
  - `detectPreset(t)`：把当前 template 与 4 预设比对，返回匹配的 key 或 "custom"
  - `initTemplateFromRule(tpl)`：从规则 template 初始化 6 参数框（null 补全）
  - `buildTemplateForRun()`：组装当前 6 参数为 template 对象，全空返回 null
- `handlePresetChange(key)`：idcard/phone/birthdate/bankcard → 填充 6 参数框；custom → 保留当前；empty → 清空全部
- `handleTemplateFieldChange(field, value)`：更新一个字段 + detectPreset 重新匹配
- `handleRun`：general-mask → `maskColumn(sheet.id, column, maskRuleId, ch, tpl)`；name-mask → `maskColumn(sheet.id, column, maskRuleId, ch, null)`
- `handleSaveSetting`：general-mask → `updateRuleTemplate(maskRuleId, tpl)`；name-mask → `updateRuleParams(maskRuleId, null, ch)`
- JSX 条件渲染：general-mask 显示预设 Select + 6 个 Form.Item（InputNumber 数值 / Input maskChar）；name-mask 仅显示掩码字符 Input

### 1.4 文档（4 文件）

- `docs/versions/1.1.3/更新日志.md`：T48+T49 进度表 + E1~E14 验收 + 关键设计决策 + 已知边界
- `docs/versions/1.1.3/RELEASE-NOTES.md`：T49 子规则设计 + 预设表 + 前端 UX + IPC 契约
- `tests/脱敏/README.md`：2 mask 规则 + 4 预设表 + T49 收敛说明
- `docs/02-技术设计文档.md`：§3.6 rules 表 + seed_builtin_rules 段落 + 顶部 v1.1.3 摘要

## 2. 验收清单

| 验收项 | 结果 | 备注 |
|---|---|---|
| `cargo fmt --check` | ✅ 通过 | |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ 通过 | 修复 borrow of moved value + unused import |
| `cargo test --workspace` | ✅ 全绿 | 80 个单测 |
| `pnpm --prefix frontend build` | ✅ 通过 | |
| 规则管理面板可见 4 条规则 | ✅ | 3 name + 1 general-mask |
| 脱敏面板下拉可选 2 条 mask 规则 | ✅ | name-mask + general-mask |
| general-mask 选身份证预设 → `110101********1234` | ✅ | 15 位原样（guard） |
| general-mask 选手机预设 → `138****5678` | ✅ | 10 位原样 |
| general-mask 选出生日期预设 → `1990-01-**` | ✅ | |
| general-mask 选银行卡预设 → `6222***********0123` | ✅ | |
| 姓名脱敏向后兼容 | ✅ | `张三丰` → `张*丰`（name-mask 无 template → 旧逻辑） |
| general-mask 不选预设（空模板）→ 原样返回 | ✅ | `template_empty_passthrough` 单测 |
| 掩码字符 `#` 临时覆盖 | ✅ | 身份证预设 → `110101########1234` |
| 6 个参数框可手动编辑 | ✅ | 选预设填充后可继续修改 + detectPreset 自动重匹配 |
| 保存设置 general-mask → updateRuleTemplate | ✅ | `update_rule_template_persists_and_reads_back` 单测 |
| 保存设置 name-mask → updateRuleParams | ✅ | 旧逻辑不变 |
| 版本号 3 处一致 1.1.3 | ✅ | Cargo.toml + tauri.conf.json + package.json |

## 3. 已知边界

- T49 删除了 T48 的 4 条独立规则 id（`idcard-mask`/`phone-mask`/`birthdate-mask`/`bankcard-mask`）。v1.1.3 T48 用户若已保存这 4 条规则的 template 参数，升级到 T49 后这 4 条规则会从 DB 消失（seed 不再注册），需改用 `general-mask` + 对应预设。因 v1.1.3 尚未发布（T48+T49 同版本迭代），无线上用户受影响。
- 4 个预设为固定模板参数（keep_prefix/keep_suffix/mask_min_len 硬编码），但前端 6 个参数框可手动编辑覆盖预设值
- 身份证/手机/出生日期预设有 min_len=max_len guard，长度不匹配原样返回；银行卡预设无 len guard
- 出生日期脱敏按字符串长度处理（keep 8/0 + mask 2），不校验日期合法性
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）

## 4. 安全约束遵守

- DB 查询全部参数绑定（`params![]`），不拼接 SQL — ✅ `update_rule_template` 用 `?1`/`?2` 占位 + `params![template_json, id]`
- 测试数据为合成数据，不含真实凭据/隐私 — ✅ 全部为虚构数据
- Mimosa 深度扫描需在 commit 前重跑完整审计 — ⏳ 待 commit 前执行
