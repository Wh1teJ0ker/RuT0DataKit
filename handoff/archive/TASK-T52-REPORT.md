# T52 — 分段脱敏模板（按分隔符拆分 + 每段保留首尾）REPORT

> 版本：v1.1.3
> 任务：T52
> 状态：dev_complete
> 依赖：T51（已 verified_complete）

## 1. 需求回顾

用户要求新增一种脱敏模板：**按某个分隔符把值拆成多段，对指定段做脱敏（保留段内首尾字符）**。

示例：
- `zhangsan@example.com` 按 `@` 分隔 → 第 0 段 `zhangsan` 保留首尾各 1（`z******n`），第 1 段 `example.com` 不动 → `z******n@example.com`
- `192.168.11.1` 按 `.` 分隔 → 第 2 段 `11` 整段脱敏为 `**` → `192.168.**.1`

**用户决策**（已通过 AskUserQuestion 确认）：
1. 建模方式：扩展 TemplateParams 为 untagged enum（Simple / Segment 两种变体），复用现有 `general-mask` 规则、DB `template` 列、IPC，无需 DB 迁移、无需新规则。
2. 段配置粒度：按段索引 + 每段 keepPrefix/keepSuffix/maskMinLen。不在列表中的段原样保留。
3. 不内置 email/ip 预设——只提供自定义分段配置 UI。

## 2. 文件级改动

### 2.1 `crates/core/src/processor/rules.rs`（改，核心）

`TemplateParams` 从 flat struct 改为 `#[serde(untagged)]` enum：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum TemplateParams {
    Simple(SimpleTemplate),
    Segment(SegmentTemplate),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimpleTemplate {
    pub keep_prefix: Option<usize>,
    pub keep_suffix: Option<usize>,
    pub mask_char: Option<char>,
    pub mask_min_len: Option<usize>,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentTemplate {
    pub mask_char: Option<char>,
    pub delimiter: String,
    pub segments: Vec<SegmentMask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentMask {
    pub index: usize,
    pub keep_prefix: Option<usize>,
    pub keep_suffix: Option<usize>,
    pub mask_min_len: Option<usize>,
}
```

**关键点**：`SimpleTemplate` 加 `#[serde(deny_unknown_fields)]`——serde untagged 会先尝试 Simple，因 SimpleTemplate 全字段 Option 会"吞掉"任何 JSON 字段（包括 delimiter/segments），导致 Segment JSON 永远被反序列化为 Simple。加 `deny_unknown_fields` 后，serde 尝试 Simple 时遇到 delimiter/segments 未知字段会失败，再尝试 Segment 变体成功。由 `old_flat_json_deserializes_to_simple` + `segment_template_serde_roundtrip` 两个测试覆盖。

- impl TemplateParams：`default()` → `Simple(SimpleTemplate::default())`；`new()` 返回 `Simple`；`with_mask_char/with_len_range` 只影响 Simple（Segment 变体返回 self 不变）；`is_empty()` 按变体判定（Simple → 6 字段全 None；Segment → delimiter 空或 segments 空）；新增 `mask_char()` 方法返回 `Option<char>`（两变体都有 mask_char 字段）
- impl SegmentTemplate：`new(delimiter)` + `with_mask_char(c)` + `with_segment(index, kp, ks, mml)`
- 4 个 preset 函数（idcard/phone/birthdate/bankcard）仍调 `TemplateParams::new()` 返回 Simple，不变
- 新增测试：`segment_template_serde_roundtrip`、`old_flat_json_deserializes_to_simple`、`segment_is_empty_when_no_segments`、`segment_is_empty_when_empty_delimiter`

### 2.2 `crates/core/src/processor/masker.rs`（改，核心）

- import 新增 `SegmentMask, SegmentTemplate, SimpleTemplate`
- `mask()` 内 `t.mask_char` → `t.mask_char()`（方法访问）；`apply_template` 调用改为 match：
  ```rust
  return Ok(match tpl {
      TemplateParams::Simple(s) => apply_template(s, &chars, n, mask_char, &r.id),
      TemplateParams::Segment(s) => apply_segment_template(s, input, mask_char, &r.id),
  });
  ```
- `apply_template` 签名改为 `tpl: &SimpleTemplate`（原 `tpl: &TemplateParams`）
- 新增 `apply_segment_template(tpl, input, mask_char, rule_id)`：split by delimiter → 遍历段，命中 segments 中 index 的段调 `apply_segment_part`，未命中段原样保留 → join 回 delimiter。空 delimiter / 空 segments → 透传
- 新增 `apply_segment_part(part, cfg, mask_char)`：单段保留首尾 + 中间替换 mask_char，无 min/max guard（guard 是整段模板的语义），保留段重叠时仅输出 `mask_min_len` 个 mask_char
- 新增 9 个测试：`segment_email_mask`（`zhangsan@example.com` → `z******n@example.com`）、`segment_email_short_local_part`（`user@qq.com` → `u**r@qq.com`）、`segment_ip_mask`（`192.168.11.1` → `192.168.**.1`）、`segment_multiple_segments_masked`（`192.168.11.1` → `192.168.**.**`）、`segment_empty_delimiter_passthrough`、`segment_no_segments_passthrough`、`segment_unknown_index_untouched`、`segment_mask_char_from_template`、`segment_replacement_overrides_mask_char`

### 2.3 `src-tauri/src/db/mod.rs`（改测试）

`update_rule_template_persists_and_reads_back` 测试适配 enum：从 `got.keep_prefix` 等直接字段访问改为先 `match got { TemplateParams::Simple(s) => s, _ => panic }` 取出 Simple 变体再断言。import 不变（TemplateParams 仍是同一类型）。无生产代码改动。

### 2.4 `frontend/src/components/panels/maskTemplate.js`（改，共享模块补充分段分支）

- 新增常量：`TEMPLATE_TYPE_SIMPLE = "simple"`、`TEMPLATE_TYPE_SEGMENT = "segment"`、`EMPTY_SEGMENT_TEMPLATE = { maskChar: null, delimiter: "", segments: [] }`
- 新增工具函数：`isSegmentTemplate(tpl)`（`typeof tpl.delimiter === "string"`）、`normalizeSegment(seg)`
- `normalizeTemplate(tpl)`：按是否含 `delimiter` 字段分流 → Segment（maskChar/delimiter/segments）或 Simple（旧 6 字段）
- `detectPreset(t)`：Segment 模板 → 永远 "custom"（不内置分段预设）
- `buildTemplateForRun(template)`：按 isSegmentTemplate 分流 → `buildSegmentForRun`（delimiter 空/segments 空 → null）或 `buildSimpleForRun`（旧逻辑）
- `previewMask(template, input, fallbackMaskChar)`：分流 → `previewSegmentMask`（split → 对每段调 `previewSegmentPart` → join）或 `previewSimpleMask`（旧逻辑）
- `resolveMaskChar(rule)`：兼容（两变体都有 maskChar）

### 2.5 `frontend/src/components/panels/MaskPanel.jsx`（改，UI）

- import 新增 `EMPTY_SEGMENT_TEMPLATE`、`TEMPLATE_TYPE_SEGMENT`、`TEMPLATE_TYPE_SIMPLE`、`isSegmentTemplate`
- 新增 `handleTemplateTypeChange`（Simple↔Segment 切换时用对应空模板初始化）、`handleSegmentFieldChange`（编辑段配置单字段）、`handleAddSegment`（新增段配置，index 自增）、`handleRemoveSegment`（删除段配置）
- `handleReset` 适配：保留当前模板类型，清空参数到对应空模板
- general-mask 区域新增「模板类型」Select + Segment 模式下渲染分隔符 Input / 掩码字符 Input / 段配置列表（每行 = 段索引 + keepPrefix + keepSuffix + maskMinLen + 删除按钮 + 底部添加按钮）；Simple 模式下保留旧 6 字段 + 预设下拉

### 2.6 `frontend/src/components/panels/RulesPanel.jsx`（改，UI）

镜像 MaskPanel 的模板类型切换 + Segment 参数 UI（操作 draftTemplate）。general-mask 可填参数卡片新增「模板类型」Select + Segment/Simple 分支 UI（与 MaskPanel 一致）。

### 2.7 文档

- `docs/versions/1.1.3/更新日志.md`：追加 T52 进度行 + E20~E28 验收 + 关键设计决策（untagged enum + apply_segment_template + DB 无需迁移）+ 已知边界
- `handoff/TASK-BOARD.md`：追加 T52 任务行 + DAG + E20~E28 验收
- `handoff/TASK-T52-HANDOFF.md` + `handoff/TASK-T52-REPORT.md`（本文件）

## 3. 验收清单

- [x] `cargo fmt --all -- --check`：通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings`：通过（无 warning，含 db/mod.rs 测试适配）
- [x] `cargo test --workspace`：全绿（96 个单测，含 4 个 rules.rs segment 测试 + 9 个 masker.rs segment 测试）
- [x] `pnpm --prefix frontend build`：通过（3082 modules transformed）
- [x] E20：cargo fmt/clippy/test 全绿（96 个单测）
- [x] E21：pnpm build 通过
- [x] E22：后端 segment_email_mask → `zhangsan@example.com` → `z******n@example.com`
- [x] E23：后端 segment_ip_mask → `192.168.11.1` → `192.168.**.1`
- [x] E24：后端 segment_multiple_segments_masked → `192.168.11.1` → `192.168.**.**`
- [x] E25：后端 segment_empty_delimiter_passthrough + segment_no_segments_passthrough → 透传
- [x] E26：后端 segment_unknown_index_untouched → 段索引超出范围，该段不动
- [x] E27：后端 old_flat_json_deserializes_to_simple → 旧 DB flat JSON 仍能反序列化为 Simple（向后兼容）
- [x] E28：前端 MaskPanel + RulesPanel 均可切换 Simple/Segment 模板类型 + 编辑 Segment 参数

## 4. 安全合规

- 不涉及 DB schema 迁移、不涉及新 SQL（template TEXT 列 + serde_json 透明序列化，复用 T48/T49 的参数绑定）
- TemplateParams 序列化用 serde，不手动拼 JSON
- 不含可用凭据字面量
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）— **不要宣称项目安全**

## 5. 已知边界

- `Segment` 模板不内置预设（邮箱/IP 等场景由用户自行配置分隔符 + 段配置）；`MASK_PRESETS` 仍只针对 Simple 模板
- `Segment` 模板的 `apply_segment_part` 不带 min/max guard（guard 是整段模板的语义），每段独立脱敏；保留段重叠时仅输出 `mask_min_len` 个 mask_char
- `Segment` 模板按 `String::split` 拆分，连续分隔符会产生空段（如 `a,,b` 按 `,` 拆 → `["a","","b"]`）；空段在 `apply_segment_part` 中 n=0 → 输出 `mask_min_len` 个 mask_char
- 前端 `previewMask` 的 Segment 分支是后端 `apply_segment_template` + `apply_segment_part` 的等价移植（纯前端逻辑，不写 DB），用于规则管理面板内联测试预览；实际脱敏仍由后端 `mask_column` 执行（结果一致）
- 旧 DB（flat template JSON）仍能正确加载为 Simple 模板，行为不变（由 `old_flat_json_deserializes_to_simple` 测试覆盖）

## 6. 待办（T52 dev_complete → verified_complete）

- [ ] Reviewer 审查 T52 交接三件套（HANDOFF + REPORT + 代码改动）
- [ ] 审查通过后状态升 verified_complete，TASK-BOARD / 更新日志同步
- [ ] v1.1.3 整体 Release QA 审计（待 v1.1.3 全部任务 verified_complete 后触发）
