# T51 — 规则管理面板补齐通用脱敏模板参数 REPORT

> 版本：v1.1.3
> 任务：T51
> 状态：verified_complete
> 依赖：T50（已 verified_complete）

## 1. 需求回顾

用户反馈：规则管理面板（RulesPanel）选 `general-mask` 规则时，可填参数只有「掩码字符」一个输入框，6 个模板参数（keepPrefix/keepSuffix/maskChar/maskMinLen/minLen/maxLen）没有暴露，也不支持子规则（预设）下拉，不便于在规则管理面板直接编辑/测试。脱敏面板（MaskPanel）在 T49 已有这套 UI，但 RulesPanel 没对齐。

## 2. 文件级改动

### 2.1 `frontend/src/components/panels/maskTemplate.js`（新建）

共享模块，抽取自 MaskPanel（T49）+ 新增若干工具函数：

- **常量**：`DEFAULT_MASK_CHAR = "*"`、`MASK_PRESETS`（4 预设：身份证/手机/出生日期/银行卡 + custom + empty）、`EMPTY_TEMPLATE`（6 字段全 null）
- **工具函数**：
  - `normalizeTemplate(tpl)`：从任意 template 对象补全缺失字段为 null（用于草稿初始化）
  - `stripNulls(t)`：去掉 null/空值字段（用于预设匹配比较）
  - `detectPreset(t)`：检测当前模板参数匹配哪个预设（回显子规则下拉）
  - `templateFromPreset(key)`：按预设 key 填充 6 参数框（custom 返回 null 表示不填充，调用方保留当前输入）
  - `buildTemplateForRun(template)`：组装 IPC 参数（null 字段不设，全空 → null 透传）
  - `previewMask(template, input, fallbackMaskChar)`：后端 `apply_template` 等价移植，返回 `{ output, skipped, passthrough }`
  - `resolveMaskChar(rule)`：template.maskChar > replacement[0] > 默认 *

`previewMask` 逻辑对齐 `crates/core/src/processor/masker.rs::apply_template`：
- 空模板（`buildTemplateForRun` 返回 null）→ `passthrough: true`，原样返回
- 长度 < minLen 或 > maxLen → `skipped: true`，原样返回（guard 命中）
- 保留段重叠（tailStart <= headEnd）→ 输出 maskMinLen 个 maskChar
- 否则 → head + mask(repeat max(midLen, mml)) + tail

### 2.2 `frontend/src/components/panels/MaskPanel.jsx`（改，纯重构去重）

- 删除本地重复定义的 `DEFAULT_MASK_CHAR` / `MASK_PRESETS` / `EMPTY_TEMPLATE` / `stripNulls` / `detectPreset` / `buildTemplateForRun` / 末尾 `resolveMaskChar` 函数
- 改 import 自 `./maskTemplate`：`DEFAULT_MASK_CHAR, EMPTY_TEMPLATE, MASK_PRESETS, buildTemplateForRun, detectPreset, normalizeTemplate, resolveMaskChar, templateFromPreset`
- `initTemplateFromRule` 改用 `normalizeTemplate(tpl)`
- `handlePresetChange` 改用 `templateFromPreset(key)`（custom → null 保留当前）
- `buildTemplateForRun` 本地实现 → 改为 `buildForRun = () => buildTemplateForRun(template)`
- 行为不变（纯重构，预设填充 / 透传判定 / 保存逻辑全部一致）

### 2.3 `frontend/src/components/panels/RulesPanel.jsx`（改，核心改动）

**import**：antd 新增 `InputNumber` / `Select`；tauri.js 新增 `updateRuleTemplate`；新增 maskTemplate 共享模块（`DEFAULT_MASK_CHAR, EMPTY_TEMPLATE, MASK_PRESETS, buildTemplateForRun, detectPreset, normalizeTemplate, previewMask, templateFromPreset`）

**state 新增**：
- `draftTemplate`（6 字段草稿，初始 `EMPTY_TEMPLATE`）
- `presetKey`（子规则下拉值，初始 `"empty"`）

**选中规则变化 useEffect**：`general-mask` → `normalizeTemplate(selected.template)` + `detectPreset` 回显；其他 → 重置为 `EMPTY_TEMPLATE` + `presetKey="empty"`

**新增 handler**：
- `handlePresetChange(key)`：`templateFromPreset(key)` 填充（custom → null 保留）
- `handleTemplateFieldChange(field, value)`：更新单字段 + 重新 `detectPreset`

**handleSaveParams 分支**：
- `general-mask` → `buildTemplateForRun(draftTemplate)` + `updateRuleTemplate(selected.id, tpl)`
- 其他 → `updateRuleParams(selected.id, pattern, replacement)`（旧逻辑）

**重置按钮分支**：`general-mask` → 从 `selected.template` 回显；其他 → 旧逻辑

**runMaskTest 分支**：
- `general-mask` → `previewMask(draftTemplate, input, fallback)`，testResult 带 `template: true` + `passthrough` / `skipped` 标志
- `name-mask` → 旧逻辑（≥3 保留首尾，2 保留首字符）

**可填参数卡片 JSX**：`selected.kind === "mask"` 时再按 `selected.id === "general-mask"` 分支：
- `general-mask` → 子规则下拉 + 6 个 Form.Item（keepPrefix/keepSuffix/maskMinLen/minLen/maxLen 用 InputNumber，maskChar 用 Input）
- 其他 mask（name-mask）→ 单个「掩码字符」Input（旧逻辑不变）
- validate/extract → 「正则模式」Input（旧逻辑不变）

**内联测试结果区**：mask 分支新增 `testResult.template && testResult.passthrough` → 显示「空模板（透传）」Alert；`testResult.template && testResult.skipped` → 显示「guard 命中」Alert

### 2.4 文档

- `docs/versions/1.1.3/更新日志.md`：追加 T51 进度行 + E15~E19 验收 + 关键设计决策 T51 条目 + 已知边界 previewMask 说明
- `handoff/TASK-BOARD.md`：追加 T51 任务行 + DAG + E15~E19
- `handoff/TASK-T51-HANDOFF.md` + `handoff/TASK-T51-REPORT.md`（本文件）

## 3. 验收清单

- [x] `cargo fmt --all -- --check`：通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings`：通过（无 warning）
- [x] `cargo test --workspace`：全绿（83 + 80 单测，T51 纯前端无新增后端测试）
- [x] `pnpm --prefix frontend build`：通过（3082 modules transformed）
- [x] E15：规则管理面板选 general-mask → 可填参数卡片显示子规则下拉 + 6 个模板参数框
- [x] E16：选身份证预设 → 内联测试 `110101199001011234` → `110101********1234`（previewMask 等价 apply_template）
- [x] E17：保存参数 → 持久化 template 到 DB（updateRuleTemplate）；切回规则后草稿从 DB 回显
- [x] E18：选 name-mask → 仍只显示掩码字符输入（旧逻辑不变）
- [x] E19：MaskPanel 与 RulesPanel 共用 maskTemplate.js（无重复定义）

## 4. 安全合规

- T51 纯前端改动，不涉及 DB 查询 / SQL / 凭据
- 后端 `update_rule_template` IPC 在 T49 已用参数绑定（`UPDATE rules SET template = ?1 WHERE id = ?2` + `params![template_json, id]`），本次只是前端调用方
- 不引入新的外部依赖（maskTemplate.js 是项目内 ES module）
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）— **不要宣称项目安全**

## 5. 已知边界

- `previewMask` 是后端 `apply_template` 的等价移植（纯前端逻辑，不写 DB），用于规则管理面板内联测试预览；实际脱敏仍由后端 `mask_column` 执行（结果一致）。若后端 `apply_template` 逻辑变更，需同步更新 `previewMask`
- RulesPanel 的 `general-mask` 模板参数保存后，MaskPanel 也会读到同一份 DB template（两面板共享 `rules.template` 列）
- `name-mask` 规则在 RulesPanel 仍只显示「掩码字符」输入（旧逻辑不变），不暴露模板参数（name-mask 无 template）
