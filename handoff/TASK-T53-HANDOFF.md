# T53 — 反向脱敏模板（掩码首尾、保留中间）HANDOFF

> 版本：v1.1.3
> 任务：T53
> 状态：dev_complete
> 依赖：T51（已 verified_complete，T53 可独立推进）

## 1. 需求

用户原话：
> 数据脱敏通用模版-反向，可以保留中间，对前多少位和后多少位脱敏，这个再通用模版的基础上优化即可实现功能，另外，是否值长度下限（guard，空=不限）这个可以删除呢

两个子请求：
- **(a) 反向脱敏模板**：当前 `Simple` 模板是正向（保留首尾、掩码中间），用户要一种反向模式——保留中间、对前 N 位和后 N 位脱敏。例：`13812345678` + 前 3 后 4 脱敏 → `***1234****`。用户明确说"再通用模版的基础上优化即可实现功能" → 扩展现有 `SimpleTemplate`，不新增枚举变体。
- **(b) min_len 是否可删除**：用户询问「值长度下限（guard，空=不限）」能否删除。

**决定**（已通过 ExitPlanMode 审批）：
1. 反向脱敏：在 `SimpleTemplate` 加 `reverse: Option<bool>` 标志位（1 个字段），`reverse=true` 走反向分支，`None/false` 走正向。
2. min_len/max_len：**保留，不删除**。理由：3 个预设（身份证/手机/出生日期）依赖 `min_len==max_len` 作精确长度 guard，删除会让 15 位身份证被按 6+4 脱敏产生异常结果；字段已是 `Option`（空=不限），对自定义场景无负担。

## 2. 改动文件清单

### 后端核心（2 文件，改）

1. **`crates/core/src/processor/rules.rs`** — `SimpleTemplate` 加 reverse 字段：
   - 新增字段 `pub reverse: Option<bool>`（doc 注释说明 T53 语义：`None`/`false` = 正向；`true` = 反向，掩码首尾、保留中间；反向时 keep_prefix/keep_suffix 语义变为「首部脱码位数」/「尾部脱码位数」）
   - `TemplateParams::new` / `SimpleTemplate::new` 初始化 `reverse: None`
   - 新增链式方法 `SimpleTemplate::with_reverse(mut self, reverse: bool) -> Self`
   - `is_empty()` **不检查 reverse**（因 `reverse=true` 单独存在 + kp=ks=0 = 透传，等价空模板）
   - 4 个预设不加 reverse（保持正向）
   - 4 个预设测试（idcard/phone/birthdate/bankcard）加 `assert_eq!(s.reverse, None)`
   - 新增测试 `reverse_template_serde_roundtrip`（验证 reverse=true 序列化含 `"reverse":true` + 反序列化回环）
   - `old_flat_json_deserializes_to_simple` 测试加 `assert_eq!(s.reverse, None)`（旧 JSON 无 reverse → None → 正向）

2. **`crates/core/src/processor/masker.rs`** — `apply_template` 加反向分支 + 6 个测试：
   - 文件头 doc 注释追加 T53 反向语义说明
   - `apply_template` 在 guard 检查后、正向逻辑前加反向分支：
     - `reverse=true` 且 n=0 → 空串
     - `head_mask_end = kp.min(n)`、`tail_mask_start = n.saturating_sub(ks)`
     - `tail_mask_start <= head_mask_end`（kp+ks >= n 重叠）→ 整段脱敏，输出 `max(n, mml)` 个 mask_char
     - 否则 → head_mask（kp 个）+ middle（保留）+ tail_mask（ks 个）
   - 新增 6 个测试：`template_reverse_mask_head_tail`（`13812345678` → `***1234****`）、`template_reverse_keeps_middle`（`abcdef` → `**cd**`）、`template_reverse_full_mask_on_overlap`（kp=10/ks=10/n=11 → 11 个 `*`）、`template_reverse_with_mask_char`（mask_char='#' → `###1234####`）、`template_reverse_guard_still_works`（min_len=18 → 15 位透传）、`template_reverse_default_is_forward`（reverse=None → 正向）

### 前端（3 文件，改）

3. **`frontend/src/components/panels/maskTemplate.js`** — 共享模块补 reverse：
   - 模块头注释追加 T53 说明
   - `EMPTY_TEMPLATE` 加 `reverse: null`
   - `normalizeTemplate` Simple 分支补 `reverse: tpl.reverse === true ? true : null`
   - `buildSimpleForRun`：`template.reverse === true` 时输出 `tpl.reverse = true`（false/null 不输出，省字段）
   - `previewSimpleMask` 加反向分支（镜像后端 `apply_template` 反向逻辑，含重叠全脱码）

4. **`frontend/src/components/panels/MaskPanel.jsx`** — Simple 区加「反向脱敏」Switch：
   - import 新增 `Switch`
   - `maskMinLen` 之后新增「反向脱敏」Form.Item + Switch（`checked={template.reverse === true}`，onChange 传 `true` 或 `null`）

5. **`frontend/src/components/panels/RulesPanel.jsx`** — 镜像 MaskPanel 的反向脱敏 Switch（操作 draftTemplate，Switch 已在 import 中）

### 文档（3 文件）

6. `docs/versions/1.1.3/更新日志.md` — 追加 T53 进度行 + E29~E37 验收 + 关键设计决策（reverse 标志位 + min_len 保留理由）+ 已知边界（反向重叠全脱码 + reverse 向后兼容）
7. `handoff/TASK-BOARD.md` — 追加 T53 任务行 + 状态行同步
8. `handoff/TASK-T53-HANDOFF.md`（本文件）+ `handoff/TASK-T53-REPORT.md`

## 3. 关键设计决策

### 3.1 reverse: Option<bool> 标志位（扩展 SimpleTemplate）

在 `SimpleTemplate` 上新增 1 个 `Option<bool>` 字段，而非新增枚举变体或新模板类型。理由：用户明确说"再通用模版的基础上优化即可实现功能"。`Option` 保证向后兼容——旧 DB JSON 无 reverse 字段 → `None` → 正向（行为不变）。`#[serde(deny_unknown_fields)]` 保留，`reverse` 在字段白名单内。

### 3.2 is_empty() 不检查 reverse

`is_empty()` 保持「6 字段全 None 即空」语义，不检查 reverse。理由：
- `reverse=Some(true)` 单独存在（其他全 None）→ kp=ks=0 → 不脱码任何位 → 等价透传
- `reverse=Some(false)` 行为=正向=默认 → 等价空模板
- 两种情况都等价空模板，故 `is_empty` 不加 reverse 检查，保持语义不变

### 3.3 反向分支重叠处理

反向脱敏时 `keep_prefix + keep_suffix >= n`（首尾脱码区重叠）→ 整段脱敏，输出 `max(n, mml)` 个 mask_char。与正向重叠处理一致（正向重叠时输出 `mml` 个 mask_char，但正向重叠意味着 kp+ks>=n 即中间无保留区）。反向重叠时 `max(n, mml)` 保证至少覆盖原值长度（避免 11 位值只输出 1 个 `*` 的异常）。

### 3.4 min_len/max_len 保留（不删除）

用户询问 min_len 是否可删除。决定保留，理由：
1. 3 个预设（身份证/手机/出生日期）依赖 `min_len==max_len` 作精确长度 guard（非 18/11/10 位原样返回）。只删 min_len 会让 15 位身份证被按 6+4 脱敏，产生 `110101********123`（中间只有 5 位却被补成 8 个 `*`）。
2. `bankcard` 预设不设 guard（长度可变），自定义模板也可不填 → 字段已是 `Option`（空=不限），对自定义场景无负担。
3. 删除需改动 3 条 short_passthrough 测试 + 4 个预设 + 前端 UI，收益不明显。

### 3.5 前端 reverse 归一为 true/null

前端 Switch 的 onChange 传 `checked || null`（false 归一为 null）。理由：避免存 `{reverse:false}` 让 JSON 多一个无意义字段；`is_empty` 虽不检查 reverse，但归一为 null 更干净，保持旧 JSON 形态。`buildSimpleForRun` 只在 `reverse===true` 时输出字段。

## 4. 验收结果

- [x] `cargo fmt --all -- --check`：通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings`：通过（无 warning）
- [x] `cargo test --workspace`：全绿（100 个单测，含 1 个 rules.rs reverse serde roundtrip + 6 个 masker.rs reverse 测试）
- [x] `pnpm --prefix frontend build`：通过（3082 modules transformed，无 lint/构建错误）

## 5. 安全约束

- 不涉及 DB schema 迁移、不涉及新 SQL（reverse 是 SimpleTemplate 的 Option 字段，serde 透明序列化，复用 T48/T49 的参数绑定）
- TemplateParams 序列化用 serde，不手动拼 JSON
- 不含可用凭据字面量
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）—— 不宣称项目安全

## 6. 已知边界

- 反向脱敏（`reverse=true`）的 `keep_prefix`/`keep_suffix` 语义变为「首部脱码位数」/「尾部脱码位数」；当 `kp+ks >= n`（重叠）→ 整段脱敏，输出 `max(n, mml)` 个 mask_char
- `reverse` 字段是 `Option<bool>`，旧 DB JSON 无此字段 → `None` → 正向（向后兼容）；4 个预设不加 `reverse`（保持正向）；`is_empty()` 不检查 `reverse`
- 保留 `min_len`/`max_len`（不删除）：3 个预设依赖精确长度 guard，字段已是 Option 对自定义无负担
- 前端 `previewSimpleMask` 反向分支是后端 `apply_template` 反向逻辑的等价移植（纯前端逻辑，不写 DB），用于内联测试预览；实际脱敏仍由后端 `mask_column` 执行（结果一致）
- 旧 DB（无 reverse 字段的 template JSON）仍能正确加载为正向 Simple 模板，行为不变（由 `old_flat_json_deserializes_to_simple` + `template_reverse_default_is_forward` 测试覆盖）
