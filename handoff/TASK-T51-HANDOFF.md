# T51 — 规则管理面板补齐通用脱敏模板参数 HANDOFF

> 版本：v1.1.3
> 任务：T51
> 状态：verified_complete
> 依赖：T50（已 verified_complete）

## 1. 需求

用户要求：
> 规则管理里通用脱敏的可填参数不齐全，不便于测试

规则管理面板（RulesPanel）选 `general-mask` 规则时，「可填参数」卡片只显示一个「掩码字符」输入框，没有暴露 `general-mask` 规则持有的 6 个模板参数（keepPrefix/keepSuffix/maskChar/maskMinLen/minLen/maxLen），也不支持子规则（预设）下拉，导致用户无法在规则管理面板直接编辑/测试 `general-mask` 的模板参数。脱敏面板（MaskPanel）在 T49 已经有这套 UI，但 RulesPanel 没有对齐。

## 2. 改动文件清单

### 前端（3 文件，新增 1 + 改 2）

1. **`frontend/src/components/panels/maskTemplate.js`**（新建）— 共享模块，抽取自 MaskPanel + 新增若干工具函数：
   - 常量：`DEFAULT_MASK_CHAR`、`MASK_PRESETS`（4 预设 + custom + empty）、`EMPTY_TEMPLATE`
   - 工具函数：`normalizeTemplate(tpl)`（补全缺失字段为 null）、`stripNulls(t)`、`detectPreset(t)`、`templateFromPreset(key)`（custom 返回 null 表示不填充）、`buildTemplateForRun(template)`（组装 IPC 参数，全空 → null）、`previewMask(template, input, fallbackMaskChar)`（后端 `apply_template` 等价移植，含空模板透传 + min_len/max_len guard + 保留段重叠处理，返回 `{ output, skipped, passthrough }`）、`resolveMaskChar(rule)`（template.maskChar > replacement[0] > 默认 *）

2. **`frontend/src/components/panels/MaskPanel.jsx`**（改）— 删除本地重复定义的 `DEFAULT_MASK_CHAR` / `MASK_PRESETS` / `EMPTY_TEMPLATE` / `stripNulls` / `detectPreset` / `initTemplateFromRule` 内部拼接 / `handlePresetChange` 内部填充 / `buildTemplateForRun` 内部实现 / 末尾 `resolveMaskChar` 函数，统一改 import 自 `./maskTemplate`。行为不变（纯重构去重）。

3. **`frontend/src/components/panels/RulesPanel.jsx`**（改）— 选 `general-mask` 时「可填参数」卡片渲染子规则下拉 + 6 个模板参数框：
   - import 新增 `InputNumber` / `Select`（antd）+ `updateRuleTemplate`（tauri.js）+ maskTemplate 共享模块
   - 新增 state：`draftTemplate`（6 字段草稿）、`presetKey`（子规则下拉值）
   - 选中规则变化时同步草稿：`general-mask` → `normalizeTemplate(selected.template)` + `detectPreset`；其他 → 重置为 `EMPTY_TEMPLATE`
   - 新增 `handlePresetChange` / `handleTemplateFieldChange`（编辑后重新 `detectPreset`）
   - `handleSaveParams` 分支：`general-mask` → `buildTemplateForRun(draftTemplate)` + `updateRuleTemplate`；其他 → `updateRuleParams`（旧逻辑）
   - 重置按钮分支：`general-mask` → 从 `selected.template` 回显；其他 → 旧逻辑
   - `runMaskTest` 分支：`general-mask` → `previewMask`（模板预览，含 passthrough/skipped 提示）；`name-mask` → 旧逻辑（≥3 保留首尾）
   - 内联测试结果区：`general-mask` 模板预览时额外显示「空模板（透传）」或「guard 命中」提示 Alert
   - `name-mask` 仍只显示「掩码字符」输入（旧逻辑不变）

### 文档（3 文件）

4. `docs/versions/1.1.3/更新日志.md` — 追加 T51 进度行 + E15~E19 验收 + 关键设计决策新增 T51 条目 + 已知边界新增 previewMask 说明
5. `handoff/TASK-BOARD.md` — 追加 T51 任务行 + DAG + E15~E19 验收
6. `handoff/TASK-T51-HANDOFF.md`（本文件）+ `handoff/TASK-T51-REPORT.md`

## 3. 关键设计决策

### 3.1 抽取共享模块 maskTemplate.js（去重）

MaskPanel（T49）已有 MASK_PRESETS / EMPTY_TEMPLATE / stripNulls / detectPreset / buildTemplateForRun / resolveMaskChar，RulesPanel（T51）需要同一套逻辑。为避免两处重复定义，抽到 `frontend/src/components/panels/maskTemplate.js`，两个面板都 import。MaskPanel 行为不变（纯重构）。

### 3.2 previewMask：后端 apply_template 的等价移植

RulesPanel 的内联测试是前端纯逻辑（不写 DB），原 `runMaskTest` 只支持 name-mask 的「保留首尾」旧逻辑。T51 为 general-mask 新增模板预览：把后端 `crates/core/src/processor/masker.rs::apply_template` 的逻辑（keep_prefix/keep_suffix/mask_char/mask_min_len + min_len/max_len guard + 保留段重叠处理）移植成 `previewMask(template, input, fallbackMaskChar)`，返回 `{ output, skipped, passthrough }`：
- 空模板（全 null）→ `passthrough: true`，原样返回
- 长度不在 `[minLen, maxLen]` 区间 → `skipped: true`，原样返回（guard 命中）
- 否则 → 应用模板脱敏

UI 在测试结果区额外显示 Alert 提示透传/guard，便于用户理解为什么输入没被脱敏。实际脱敏仍由后端 `mask_column` 执行（结果一致）。

### 3.3 保存分支：updateRuleTemplate vs updateRuleParams

`handleSaveParams` 按规则 id 分支：
- `general-mask` → `buildTemplateForRun(draftTemplate)` 组装 template（全空 → null）+ `updateRuleTemplate(selected.id, tpl)` 持久化到 DB `rules.template` 列
- 其他 mask 规则（name-mask）→ `updateRuleParams(selected.id, pattern, replacement)` 持久化 replacement（旧逻辑）
- validate/extract → `updateRuleParams` 持久化 pattern（旧逻辑）

与 MaskPanel 的 `handleSaveSetting` 保持一致（general-mask 走 template，name-mask 走 replacement）。

### 3.4 草稿回显：从 DB template 初始化

选中规则变化时，`general-mask` 的 6 个参数框从 `selected.template`（DB 读取的 TemplateParams）经 `normalizeTemplate` 补全 null 字段后初始化，`presetKey` 经 `detectPreset` 回显匹配的预设。这样用户保存 template 后切走再切回，草稿能从 DB 回显。

## 4. 验收结果

- [x] `cargo fmt --all -- --check`：通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings`：通过（无 warning）
- [x] `cargo test --workspace`：全绿（83 + 80 单测，无新增后端测试 — T51 纯前端改动）
- [x] `pnpm --prefix frontend build`：通过（3082 modules transformed，无 lint/构建错误）

## 5. 安全约束

- T51 纯前端改动，不涉及 DB 查询 / SQL / 凭据 — 后端 `update_rule_template` IPC 在 T49 已用参数绑定实现，本次只是前端调用方
- 不引入新的外部依赖（maskTemplate.js 是项目内 ES module）
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）

## 6. 已知边界

- `previewMask` 是后端 `apply_template` 的等价移植（纯前端逻辑，不写 DB），用于规则管理面板内联测试预览；实际脱敏仍由后端 `mask_column` 执行（结果一致）。若后端 `apply_template` 逻辑变更，需同步更新 `previewMask`
- RulesPanel 的 `general-mask` 模板参数保存后，MaskPanel 也会读到同一份 DB template（两面板共享 `rules.template` 列）
- `name-mask` 规则在 RulesPanel 仍只显示「掩码字符」输入（旧逻辑不变），不暴露模板参数（name-mask 无 template）
