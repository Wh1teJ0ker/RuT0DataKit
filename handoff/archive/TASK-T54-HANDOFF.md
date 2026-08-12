# T54 — 拆分整段脱敏与分段脱敏为两条独立规则 HANDOFF

> 版本：v1.1.3
> 任务：T54
> 状态：dev_complete
> 依赖：T53（已 dev_complete，T54 在 seeding + UI 层独立推进，masker 算子零改动）

## 1. 需求

用户原话：
> 把整段脱敏和分段脱敏作为两种不同的规则存在，不要都放在通用脱敏里面

当前 `general-mask` 一条规则用 `TemplateParams` untagged enum（`Simple`/`Segment` 两种变体）+ 前端「模板类型」Select 切换。用户要拆成**两条独立规则**，各自只承载一种脱敏方式。

**决定**（已通过 AskUserQuestion + ExitPlanMode 审批）：
1. **彻底改名**：新建 `simple-mask`（整段脱敏）+ `segment-mask`（分段脱敏），旧 `general-mask` 加入 `cleanup_deprecated_rules()` 删除。
2. v1.1.3 未发布、dev 库无生产数据，可安全改名（无需保留 general-mask 向后兼容）。
3. `TemplateParams` untagged enum 结构不变（DB JSON 向后兼容，SCHEMA_VERSION 保持 4）。
4. masker dispatch 不变（按 `template` 变体分流，不看 rule.id）——拆分是 seeding + UI 层面改动。
5. 前端删除「模板类型」Select + `handleTemplateTypeChange`（每条规则固定一种模板类型，无需切换）。

## 2. 改动文件清单

### 后端核心（2 文件，改）

1. **`crates/core/src/processor/rules.rs`** — 删 `general_mask_rule()` → 新建两个构造函数：
   - 删除 `general_mask_rule()`
   - 新增 `simple_mask_rule()`：`id: "simple-mask"`, `name: "整段脱敏"`, `kind: Mask`, `template: Some(TemplateParams::Simple(SimpleTemplate::default()))`（空 Simple 模板 = 透传）
   - 新增 `segment_mask_rule()`：`id: "segment-mask"`, `name: "分段脱敏"`, `kind: Mask`, `template: Some(TemplateParams::Segment(SegmentTemplate::default()))`（空 Segment 模板 = 透传）
   - `with_defaults()`：删 `general_mask_rule()`，注册 `simple_mask_rule()` + `segment_mask_rule()`。共 5 条（3 name + simple-mask + segment-mask）
   - `TemplateParams` enum / `SimpleTemplate` / `SegmentTemplate` **结构不变**
   - 4 个预设函数（idcard/phone/birthdate/bankcard）不变
   - doc 注释：`general-mask` 改为 `simple-mask`/`segment-mask`
   - 测试更新：
     - `with_defaults_loads_four_rules` → `with_defaults_loads_five_rules`，断言 5 条 + `mask_count == 3`
     - `general_mask_rule_has_empty_template` → 拆为 `simple_mask_rule_has_empty_template` + `segment_mask_rule_has_empty_template`（用 `matches!` 宏断言变体类型）
     - `get_by_id_works`：断言 `get("simple-mask")` + `get("segment-mask")` 存在、`get("general-mask")` 为 None
     - `rule_template_field_serde_present_when_some`：用 `simple_mask_rule()`
     - `register_overwrites_same_id`：len==5

2. **`crates/core/src/processor/masker.rs`** — 测试辅助重命名 + 测试改用新构造函数（dispatch 逻辑不变）：
   - 删除 `general_with_template` → 新增 `simple_mask_with_template`（用 `simple_mask_rule()` 克隆 + 覆盖 template）+ `segment_mask_with_template`（用 `segment_mask_rule()` 克隆 + 覆盖 template）
   - 所有 `general_with_template(TemplateParams::Simple ...)` → `simple_mask_with_template(...)`
   - 所有 `general_with_template(TemplateParams::Segment ...)` → `segment_mask_with_template(...)`
   - 所有 `general_with_template(idcard_preset/phone_preset/birthdate_preset/bankcard_preset ...)` → `simple_mask_with_template(...)`
   - `template_mask_char_in_preset_used` 测试：`RuleRegistry::general_mask_rule()` → `RuleRegistry::simple_mask_rule()`
   - `segment_replacement_overrides_mask_char` 测试：`RuleRegistry::general_mask_rule()` → `RuleRegistry::segment_mask_rule()`
   - doc 注释：`general-mask` → `simple-mask`/`segment-mask`

### Tauri 后端（2 文件，改）

3. **`src-tauri/src/db/mod.rs`** — cleanup 加 general-mask + seed 5 条 + 测试：
   - `cleanup_deprecated_rules()`：在 id 列表加入 `"general-mask"`（现 5 个 id：idcard-mask/phone-mask/birthdate-mask/bankcard-mask/general-mask）
   - `seed_builtin_rules()` doc：4 → 5 条，general-mask → simple-mask/segment-mask
   - 测试 `seed_builtin_rules_inserts_four_when_empty` → `_inserts_five_when_empty`，count==5 + 重复 seed 幂等
   - 测试 `seed_builtin_rules_upserts_missing_on_existing_db`：断言 simple-mask + segment-mask 模板 is_empty，count==5，general-mask 不存在
   - 测试 `cleanup_deprecated_rules_removes_legacy_ids`：加 general-mask 到清理断言（5 个遗留 id 全删，5 条内置规则保留）
   - 测试 `update_rule_template_persists_and_reads_back`：`"general-mask"` → `"simple-mask"`
   - 测试 `rule_kind_round_trip_through_db`：list.len()==5
   - SCHEMA_VERSION 保持 4（无新列）

4. **`src-tauri/src/commands/processor.rs`** — doc 注释更新（签名不变）：
   - `update_rule_template` doc：`general-mask` → `simple-mask（整段脱敏）或 segment-mask（分段脱敏）`，`6 个` → `7 个可编辑参数框（含 T53 反向脱敏开关）`

### 前端（4 文件，改）

5. **`frontend/src/components/panels/maskTemplate.js`** — doc 注释更新（逻辑不变）：
   - 模块头：`通用脱敏模板参数` → `脱敏模板参数`，追加 T54 说明
   - MASK_PRESETS doc：`general-mask 的子规则` → `simple-mask 的预设`，`6 个` → `7 个`
   - `TEMPLATE_TYPE_SIMPLE`/`TEMPLATE_TYPE_SEGMENT` 常量保留（不删除，避免破坏外部 import）

6. **`frontend/src/components/panels/MaskPanel.jsx`** — 删类型切换 + id 判断改 simple-mask/segment-mask：
   - 移除 `TEMPLATE_TYPE_SEGMENT`/`TEMPLATE_TYPE_SIMPLE` imports
   - `isGeneralMask = maskRuleId === "general-mask"` → `isSimpleMask` + `isSegmentMask` + `isTemplateMask`
   - 删除 `handleTemplateTypeChange` 函数
   - useEffect + `handleRuleChange`：`first.id === "general-mask"` → `isSimpleMask || isSegmentMask`
   - `handleRun` + `handleSaveSetting`：`isGeneralMask` → `isTemplateMask`
   - `handleReset`：按 rule.id 重置到对应空模板
   - JSX：删除「模板类型」Select，按 `isSegmentMask`/`isSimpleMask` 渲染对应参数区
   - 「脱敏规则」extra：`姓名脱敏（保留首尾）/ 通用脱敏（模板参数）` → `姓名脱敏（保留首尾）/ 整段脱敏（模板参数）/ 分段脱敏（按分隔符拆分）`

7. **`frontend/src/components/panels/RulesPanel.jsx`** — 镜像 MaskPanel：
   - 移除 `TEMPLATE_TYPE_SEGMENT`/`TEMPLATE_TYPE_SIMPLE` imports
   - 删除 `handleTemplateTypeChange` 函数
   - useEffect：`selected.id === "general-mask"` → 拆为 `simple-mask`（normalizeTemplate + detectPreset）+ `segment-mask`（normalizeTemplate + presetKey="custom"）
   - `handleSaveParams`：`selected.id === "general-mask"` → `simple-mask || segment-mask`
   - `runMaskTest`：同上
   - 「可填参数」卡片 JSX：删除「模板类型」Select，按 `segment-mask`/`simple-mask`/else 渲染对应参数区
   - 重置按钮：按 `selected.id` 回退到对应空模板/DB template
   - doc 注释更新

8. **`frontend/src/tauri.js`** — `updateRuleTemplate` JSDoc 更新

### 文档（3 文件）

9. **`docs/versions/1.1.3/更新日志.md`** — 追加 T54 进度行 + E38~E44 验收 + 设计决策 + 已知边界
10. **`handoff/TASK-BOARD.md`** — 追加 T54 DAG + 任务行 + E38~E44 验收
11. **`handoff/TASK-T54-HANDOFF.md`** + **`handoff/TASK-T54-REPORT.md`** — 本文件 + 报告

## 3. 验收状态

| 项 | 结果 | 备注 |
|----|------|------|
| cargo fmt --check | ✅ | 通过 |
| cargo clippy --workspace --all-targets -- -D warnings | ✅ | 通过（修复了 rules.rs doc_lazy_continuation）|
| cargo test --workspace | ✅ | 184 个单测全绿（83 core lib + 101 core + tauri）|
| pnpm --prefix frontend build | ✅ | 通过（3082 modules，无 lint/构建错误）|
| with_defaults 5 条 | ✅ | `with_defaults_loads_five_rules` 通过 |
| cleanup 含 general-mask | ✅ | `cleanup_deprecated_rules_removes_legacy_ids` 通过 |
| seed 5 条 idempotent | ✅ | `seed_builtin_rules_inserts_five_when_empty` + `upserts_missing_on_existing_db` 通过 |
| 前端删类型切换 | ✅ | MaskPanel + RulesPanel 均删除 handleTemplateTypeChange + 模板类型 Select |

## 4. 安全约束

- 不涉及 DB schema 迁移（无新列，SCHEMA_VERSION 保持 4）
- `cleanup_deprecated_rules` 用参数绑定 `DELETE FROM rules WHERE id = ?1`
- `TemplateParams` 序列化用 serde，不手动拼 JSON
- 不含可用凭据字面量
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续，不宣称项目安全）
- 不创建标签（v1.1.3 未完成）

## 5. 交接给 reviewer

- 验收点：E38~E44（见更新日志 + TASK-BOARD）
- 重点关注：
  1. `rules.rs` 的 `simple_mask_rule()`/`segment_mask_rule()` 是否持正确变体的空模板（Simple/Segment）
  2. `cleanup_deprecated_rules()` 是否含 5 个 id（含 general-mask）
  3. `MaskPanel.jsx`/`RulesPanel.jsx` 是否彻底删除了 `handleTemplateTypeChange` + 模板类型 Select + `TEMPLATE_TYPE_*` imports
  4. masker dispatch 是否真的零改动（只改测试辅助 + 测试构造函数）
  5. `TemplateParams` untagged enum 结构是否未变（DB JSON 向后兼容）
