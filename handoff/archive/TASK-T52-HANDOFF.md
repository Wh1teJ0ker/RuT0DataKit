# T52 — 分段脱敏模板（按分隔符拆分 + 每段保留首尾）HANDOFF

> 版本：v1.1.3
> 任务：T52
> 状态：dev_complete
> 依赖：T51（已 verified_complete）

## 1. 需求

用户要求：
> 数据脱敏还有再加一个模版，按照某一个分隔符，可以对分隔符每一段的任意一部分进行脱敏，请完成这个。例如 192.168.11.1，可以按照 . 分隔，然后脱敏 11 为 *；或者是邮箱这种 zhangsan@example.com → z*****n@example.com，按照 @ 分隔。再次，请完善当前的这个新模版规则。

**用户决策**（已通过 AskUserQuestion 确认）：
1. 建模方式：**扩展 TemplateParams 为 untagged enum**（Simple / Segment 两种变体），复用现有 `general-mask` 规则、DB `template` 列、IPC，无需 DB 迁移、无需新规则。
2. 段配置粒度：**按段索引 + 每段 keepPrefix/keepSuffix/maskMinLen**。不在列表中的段原样保留。
3. 不内置 email/ip 预设——只提供自定义分段配置 UI（用户后续可自行配置）。

## 2. 改动文件清单

### 后端核心（2 文件，改）

1. **`crates/core/src/processor/rules.rs`** — `TemplateParams` 从 flat struct 改为 untagged enum：
   - `#[serde(rename_all = "camelCase", untagged)]` enum：`Simple(SimpleTemplate)` / `Segment(SegmentTemplate)`
   - `SimpleTemplate`：原 6 字段 flat struct（keep_prefix/keep_suffix/mask_char/mask_min_len/min_len/max_len），加 `#[serde(deny_unknown_fields)]` 确保 untagged 分流正确（serde 先尝试 Simple，因 delimiter/segments 不在 Simple 字段表内会失败再尝试 Segment）
   - `SegmentTemplate`：`mask_char` + `delimiter: String` + `segments: Vec<SegmentMask>`，builder `new(delimiter)` + `with_mask_char(c)` + `with_segment(index, kp, ks, mml)`
   - `SegmentMask`：`index: usize` + `keep_prefix/keep_suffix/mask_min_len: Option<usize>`
   - impl TemplateParams：`default()` → `Simple(SimpleTemplate::default())`；`new()` 返回 `Simple`；`with_mask_char/with_len_range` 只影响 Simple；`is_empty()` 按变体判定（Simple → 6 字段全 None；Segment → delimiter 空或 segments 空）；新增 `mask_char()` 方法返回 `Option<char>`（两变体都有 mask_char 字段）
   - 4 个 preset 函数（idcard/phone/birthdate/bankcard）仍调 `TemplateParams::new()` 返回 Simple，不变
   - 新增测试：`segment_template_serde_roundtrip`、`old_flat_json_deserializes_to_simple`、`segment_is_empty_when_no_segments`、`segment_is_empty_when_empty_delimiter`

2. **`crates/core/src/processor/masker.rs`** — mask 分支 match enum + 新增分段脱敏逻辑：
   - import 新增 `SegmentMask, SegmentTemplate, SimpleTemplate`
   - `mask()` 内 `t.mask_char` → `t.mask_char()`（方法访问）；`apply_template` 调用改为 match：`Simple(s) => apply_template(s, ...)` / `Segment(s) => apply_segment_template(s, input, mask_char, &r.id)`
   - `apply_template` 签名改为 `tpl: &SimpleTemplate`
   - 新增 `apply_segment_template(tpl, input, mask_char, rule_id)`：split by delimiter → 遍历段，命中 segments 中 index 的段调 `apply_segment_part`，未命中段原样保留 → join 回 delimiter。空 delimiter / 空 segments → 透传
   - 新增 `apply_segment_part(part, cfg, mask_char)`：单段保留首尾 + 中间替换 mask_char，无 min/max guard（guard 是整段模板的语义），保留段重叠时仅输出 `mask_min_len` 个 mask_char
   - 新增 9 个测试：`segment_email_mask`、`segment_email_short_local_part`、`segment_ip_mask`、`segment_multiple_segments_masked`、`segment_empty_delimiter_passthrough`、`segment_no_segments_passthrough`、`segment_unknown_index_untouched`、`segment_mask_char_from_template`、`segment_replacement_overrides_mask_char`

### 后端 tauri（1 文件，改测试）

3. **`src-tauri/src/db/mod.rs`** — `update_rule_template_persists_and_reads_back` 测试适配 enum：从 `got.keep_prefix` 等直接字段访问改为先 `match got { TemplateParams::Simple(s) => s, _ => panic }` 取出 Simple 变体再断言。import 不变（TemplateParams 仍是同一类型）

### 前端（3 文件，改）

4. **`frontend/src/components/panels/maskTemplate.js`** — 共享模块补充分段分支：
   - 新增常量：`TEMPLATE_TYPE_SIMPLE`、`TEMPLATE_TYPE_SEGMENT`、`EMPTY_SEGMENT_TEMPLATE = { maskChar: null, delimiter: "", segments: [] }`
   - 新增工具函数：`isSegmentTemplate(tpl)`（typeof tpl.delimiter === "string"）、`normalizeSegment(seg)`
   - `normalizeTemplate(tpl)`：按是否含 `delimiter` 字段分流 → Segment（maskChar/delimiter/segments）或 Simple（旧 6 字段）
   - `detectPreset(t)`：Segment 模板 → 永远 "custom"（不内置分段预设）
   - `buildTemplateForRun(template)`：按 isSegmentTemplate 分流 → `buildSegmentForRun`（delimiter 空/segments 空 → null）或 `buildSimpleForRun`（旧逻辑）
   - `previewMask(template, input, fallbackMaskChar)`：分流 → `previewSegmentMask`（split → 对每段调 `previewSegmentPart` → join）或 `previewSimpleMask`（旧逻辑）
   - `resolveMaskChar(rule)`：兼容（两变体都有 maskChar）

5. **`frontend/src/components/panels/MaskPanel.jsx`** — 模板类型切换 + Segment 参数 UI：
   - import 新增 `EMPTY_SEGMENT_TEMPLATE`、`TEMPLATE_TYPE_SEGMENT`、`TEMPLATE_TYPE_SIMPLE`、`isSegmentTemplate`
   - 新增 `handleTemplateTypeChange`（Simple↔Segment 切换时用对应空模板初始化）、`handleSegmentFieldChange`（编辑段配置单字段）、`handleAddSegment`（新增段配置，index 自增）、`handleRemoveSegment`（删除段配置）
   - `handleReset` 适配：保留当前模板类型，清空参数到对应空模板
   - general-mask 区域新增「模板类型」Select + Segment 模式下渲染分隔符 Input / 掩码字符 Input / 段配置列表（每行 = 段索引 + keepPrefix + keepSuffix + maskMinLen + 删除按钮 + 底部添加按钮）；Simple 模式下保留旧 6 字段 + 预设下拉

6. **`frontend/src/components/panels/RulesPanel.jsx`** — 镜像 MaskPanel 的模板类型切换 + Segment 参数 UI：
   - import 同 MaskPanel
   - 新增 `handleTemplateTypeChange` / `handleSegmentFieldChange` / `handleAddSegment` / `handleRemoveSegment`（操作 draftTemplate）
   - general-mask 可填参数卡片新增「模板类型」Select + Segment/Simple 分支 UI（与 MaskPanel 一致）

### 文档（3 文件）

7. `docs/versions/1.1.3/更新日志.md` — 追加 T52 进度行 + E20~E28 验收 + 关键设计决策（untagged enum + apply_segment_template + DB 无需迁移）+ 已知边界（Segment 不内置预设 / 无 guard / 连续分隔符空段）
8. `handoff/TASK-BOARD.md` — 追加 T52 任务行 + DAG + E20~E28 验收
9. `handoff/TASK-T52-HANDOFF.md`（本文件）+ `handoff/TASK-T52-REPORT.md`

## 3. 关键设计决策

### 3.1 untagged enum + deny_unknown_fields（向后兼容）

`TemplateParams` 从 flat struct 改为 `#[serde(untagged)]` enum。untagged serde 会先尝试 Simple（所有字段 Option，旧 JSON 全部命中），再尝试 Segment（需要 `delimiter` 字段）。

**关键坑**：SimpleTemplate 全字段 Option，会"吞掉"任何 JSON 字段（包括 delimiter/segments），导致 Segment JSON 永远被反序列化为 Simple。解决：给 SimpleTemplate 加 `#[serde(deny_unknown_fields)]`，serde 尝试 Simple 时遇到 delimiter/segments 未知字段会失败，再尝试 Segment 变体成功。由 `old_flat_json_deserializes_to_simple` + `segment_template_serde_roundtrip` 两个测试覆盖。

### 3.2 apply_segment_template + apply_segment_part（无 guard）

`apply_segment_template` 按 `delimiter` 拆分输入（`String::split`），遍历每段，命中 `segments` 中 `index` 的段调 `apply_segment_part`，未命中的段原样保留，最后 join 回 delimiter。

`apply_segment_part` 复用 `apply_template` 的保留段逻辑（kp/ks/mml），但**不带 min/max guard**——guard 是整段模板的语义（值总长度区间），分段模板对每段独立脱敏，每段长度由该段的 kp/ks/mml 决定。保留段重叠时仅输出 `mask_min_len` 个 mask_char（与 apply_template 一致）。

### 3.3 DB 无需迁移

`template TEXT` JSON 列存 `TemplateParams` 序列化串。untagged enum 透明序列化：旧 DB 存的 flat JSON（无 delimiter）→ 反序列化为 Simple（向后兼容）；新 JSON（含 delimiter + segments）→ Segment。`SCHEMA_VERSION` 保持 4。`mask_column` / `update_rule_template` IPC 签名不变（TemplateParams 仍是同一类型 enum，serde 自动处理 untagged）。

### 3.4 mask_char 方法访问

TemplateParams 是 enum 后，原来直接访问 `t.mask_char` 字段失效（不同变体字段不同）。改为 `t.mask_char()` 方法：Simple → `self.0.mask_char`；Segment → `self.0.mask_char`。两变体都有 mask_char 字段，方法统一返回 `Option<char>`。优先级不变：`replacement > template.mask_char() > 默认 *`。

### 3.5 前端 isSegmentTemplate 判定

前端 `normalizeTemplate` / `buildTemplateForRun` / `previewMask` / `detectPreset` 都按 `isSegmentTemplate(tpl)`（`typeof tpl.delimiter === "string"`）分流。后端 untagged enum 不序列化 `type` 字段（untagged 无 tag），前端靠 `delimiter` 字段是否存在判断变体。Segment 模板的 `detectPreset` 永远返回 "custom"（不内置分段预设）。

## 4. 验收结果

- [x] `cargo fmt --all -- --check`：通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings`：通过（无 warning，含 db/mod.rs 测试适配）
- [x] `cargo test --workspace`：全绿（96 个单测，含 4 个 rules.rs segment 测试 + 9 个 masker.rs segment 测试）
- [x] `pnpm --prefix frontend build`：通过（3082 modules transformed，无 lint/构建错误）

## 5. 安全约束

- 不涉及 DB schema 迁移、不涉及新 SQL（template TEXT 列 + serde_json 透明序列化，复用 T48/T49 的参数绑定）
- TemplateParams 序列化用 serde，不手动拼 JSON
- 不含可用凭据字面量
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）

## 6. 已知边界

- `Segment` 模板不内置预设（邮箱/IP 等场景由用户自行配置分隔符 + 段配置）；`MASK_PRESETS` 仍只针对 Simple 模板
- `Segment` 模板的 `apply_segment_part` 不带 min/max guard（guard 是整段模板的语义），每段独立脱敏；保留段重叠时仅输出 `mask_min_len` 个 mask_char
- `Segment` 模板按 `String::split` 拆分，连续分隔符会产生空段（如 `a,,b` 按 `,` 拆 → `["a","","b"]`）；空段在 `apply_segment_part` 中 n=0 → 输出 `mask_min_len` 个 mask_char
- 前端 `previewMask` 的 Segment 分支是后端 `apply_segment_template` + `apply_segment_part` 的等价移植（纯前端逻辑，不写 DB），用于规则管理面板内联测试预览；实际脱敏仍由后端 `mask_column` 执行（结果一致）
- 旧 DB（flat template JSON）仍能正确加载为 Simple 模板，行为不变（由 `old_flat_json_deserializes_to_simple` 测试覆盖）
