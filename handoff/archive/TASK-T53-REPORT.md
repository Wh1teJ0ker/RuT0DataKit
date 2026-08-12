# T53 — 反向脱敏模板（掩码首尾、保留中间）REPORT

> 版本：v1.1.3
> 任务：T53
> 状态：dev_complete
> 依赖：T51（已 verified_complete，T53 可独立推进）

## 1. 需求回顾

用户要求两个子请求：
- **(a) 反向脱敏模板**：在通用脱敏模板基础上扩展一种反向模式——保留中间、对前 N 位和后 N 位脱敏。例：`13812345678` + 前 3 后 4 脱敏 → `***1234****`。用户明确说"再通用模版的基础上优化即可实现功能" → 扩展现有 `SimpleTemplate`，不新增枚举变体。
- **(b) min_len 是否可删除**：用户询问「值长度下限（guard，空=不限）」能否删除。

**用户决策**（已通过 ExitPlanMode 审批）：
1. 反向脱敏：在 `SimpleTemplate` 加 `reverse: Option<bool>` 标志位（1 个字段），`reverse=true` 走反向分支，`None/false` 走正向。
2. min_len/max_len：**保留，不删除**（3 个预设依赖精确长度 guard；字段已是 Option，对自定义无负担）。

## 2. 文件级改动

### 2.1 `crates/core/src/processor/rules.rs`（改，核心）

`SimpleTemplate` 新增 `reverse: Option<bool>` 字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimpleTemplate {
    pub keep_prefix: Option<usize>,
    pub keep_suffix: Option<usize>,
    pub mask_char: Option<char>,
    pub mask_min_len: Option<usize>,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    /// 反向脱敏标志（T53）。None/false = 正向（保留首尾、掩码中间）；
    /// true = 反向（掩码首尾、保留中间）。反向时 keep_prefix/keep_suffix
    /// 语义变为「首部脱码位数」/「尾部脱码位数」。旧 JSON 无此字段 → None → 正向，向后兼容。
    pub reverse: Option<bool>,
}
```

**关键点**：
- `reverse` 是 `Option<bool>`，`#[serde(deny_unknown_fields)]` 保留，`reverse` 在字段白名单内
- 旧 DB JSON 无 reverse 字段 → `None` → 正向（向后兼容）
- `TemplateParams::new` / `SimpleTemplate::new` 初始化 `reverse: None`
- 新增链式方法 `with_reverse(mut self, reverse: bool) -> Self`
- `is_empty()` **不检查 reverse**（因 `reverse=true` 单独存在 + kp=ks=0 = 透传，等价空模板；`reverse=false` = 正向 = 默认）
- 4 个预设不加 reverse（保持正向）
- 测试：4 个预设测试加 `assert_eq!(s.reverse, None)`；新增 `reverse_template_serde_roundtrip`；`old_flat_json_deserializes_to_simple` 加 `assert_eq!(s.reverse, None)`

### 2.2 `crates/core/src/processor/masker.rs`（改，核心）

`apply_template` 在 guard 检查后、正向逻辑前加反向分支：

```rust
// T53：reverse=true → 反向脱敏（掩码首尾，保留中间）
if tpl.reverse.unwrap_or(false) {
    if n == 0 {
        return MaskResult { output: String::new(), rule_id: rule_id.to_string() };
    }
    let head_mask_end = kp.min(n);
    let tail_mask_start = n.saturating_sub(ks);
    if tail_mask_start <= head_mask_end {
        // kp+ks >= n → 整段脱敏，至少 mml 个 mask_char
        let mask_len = n.max(mml);
        return MaskResult { output: mask_char.to_string().repeat(mask_len), rule_id: rule_id.to_string() };
    }
    let head_mask = mask_char.to_string().repeat(head_mask_end);
    let middle: String = chars[head_mask_end..tail_mask_start].iter().collect();
    let tail_mask = mask_char.to_string().repeat(n - tail_mask_start);
    return MaskResult { output: format!("{head_mask}{middle}{tail_mask}"), rule_id: rule_id.to_string() };
}
```

**关键点**：
- 反向分支在 guard（min_len/max_len）检查后执行 → guard 仍生效（反向 + min_len=18 → 15 位透传）
- 重叠处理（`kp+ks >= n`）→ 整段脱敏，输出 `max(n, mml)` 个 mask_char（与正向重叠处理一致，避免 11 位值只输出 1 个 `*`）
- mask_char 优先级不变：`replacement > template.mask_char > 默认 *`
- 新增 6 个测试：`template_reverse_mask_head_tail`、`template_reverse_keeps_middle`、`template_reverse_full_mask_on_overlap`、`template_reverse_with_mask_char`、`template_reverse_guard_still_works`、`template_reverse_default_is_forward`

### 2.3 `frontend/src/components/panels/maskTemplate.js`（改，共享模块补 reverse）

- 模块头注释追加 T53 说明
- `EMPTY_TEMPLATE` 加 `reverse: null`
- `normalizeTemplate` Simple 分支补 `reverse: tpl.reverse === true ? true : null`
- `buildSimpleForRun`：`template.reverse === true` 时输出 `tpl.reverse = true`（false/null 不输出，省字段）
- `previewSimpleMask` 加反向分支（镜像后端 `apply_template` 反向逻辑，含重叠全脱码）：
  ```js
  if (built.reverse === true) {
    if (n === 0) return { output: "", skipped: false };
    const headMaskEnd = Math.min(kp, n);
    const tailMaskStart = Math.max(0, n - ks);
    if (tailMaskStart <= headMaskEnd) {
      return { output: maskChar.repeat(Math.max(n, mml)), skipped: false };
    }
    const headMask = maskChar.repeat(headMaskEnd);
    const middle = chars.slice(headMaskEnd, tailMaskStart).join("");
    const tailMask = maskChar.repeat(n - tailMaskStart);
    return { output: `${headMask}${middle}${tailMask}`, skipped: false };
  }
  ```

### 2.4 `frontend/src/components/panels/MaskPanel.jsx`（改，UI）

- import 新增 `Switch`
- `maskMinLen` Form.Item 之后新增「反向脱敏」Form.Item + Switch：
  ```jsx
  <Form.Item label="反向脱敏" extra="开启后保留中间，对首 N 位和后 N 位脱敏（上方前后缀位数变为首尾脱码位数）">
    <Switch
      size="small"
      checked={template.reverse === true}
      onChange={(checked) => handleTemplateFieldChange("reverse", checked || null)}
    />
  </Form.Item>
  ```
- `checked={template.reverse === true}`（null/false 都显示关闭）
- onChange 传 `true` 或 `null`（false 归一为 null，避免存 `{reverse:false}`）

### 2.5 `frontend/src/components/panels/RulesPanel.jsx`（改，UI）

镜像 MaskPanel 的反向脱敏 Switch（操作 draftTemplate，Switch 已在 import 中）。位置在 `maskMinLen` 之后、`minLen` 之前。

### 2.6 文档

- `docs/versions/1.1.3/更新日志.md`：追加 T53 进度行 + E29~E37 验收 + 关键设计决策（reverse 标志位 + min_len 保留理由）+ 已知边界（反向重叠全脱码 + reverse 向后兼容）
- `handoff/TASK-BOARD.md`：追加 T53 任务行 + 状态行同步
- `handoff/TASK-T53-HANDOFF.md` + `handoff/TASK-T53-REPORT.md`（本文件）

## 3. 验收清单

- [x] `cargo fmt --all -- --check`：通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings`：通过（无 warning）
- [x] `cargo test --workspace`：全绿（100 个单测，含 1 个 rules.rs reverse serde roundtrip + 6 个 masker.rs reverse 测试）
- [x] `pnpm --prefix frontend build`：通过（3082 modules transformed）
- [x] E29：cargo fmt/clippy/test 全绿（100 个单测）
- [x] E30：pnpm build 通过
- [x] E31：后端 `template_reverse_mask_head_tail` → `13812345678` + kp=3/ks=4/reverse=true → `***1234****`
- [x] E32：后端 `template_reverse_keeps_middle` → `abcdef` + kp=2/ks=2/reverse=true → `**cd**`
- [x] E33：后端 `template_reverse_full_mask_on_overlap` → kp=10/ks=10/n=11/reverse=true → 11 个 `*`
- [x] E34：后端 `template_reverse_with_mask_char` → + mask_char='#' → `###1234####`
- [x] E35：后端 `template_reverse_guard_still_works` → + min_len=18 → 15 位输入原样返回
- [x] E36：后端 `template_reverse_default_is_forward` → reverse=None → 正向 `138****5678`
- [x] E37：前端 MaskPanel + RulesPanel 均有「反向脱敏」Switch，开启后预览 `13812345678` → `***1234****`

## 4. 安全合规

- 不涉及 DB schema 迁移、不涉及新 SQL（reverse 是 SimpleTemplate 的 Option 字段，serde 透明序列化，复用 T48/T49 的参数绑定）
- TemplateParams 序列化用 serde，不手动拼 JSON
- 不含可用凭据字面量
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）— **不要宣称项目安全**

## 5. 已知边界

- 反向脱敏（`reverse=true`）的 `keep_prefix`/`keep_suffix` 语义变为「首部脱码位数」/「尾部脱码位数」；当 `kp+ks >= n`（重叠）→ 整段脱敏，输出 `max(n, mml)` 个 mask_char
- `reverse` 字段是 `Option<bool>`，旧 DB JSON 无此字段 → `None` → 正向（向后兼容）；4 个预设不加 `reverse`（保持正向）；`is_empty()` 不检查 `reverse`
- 保留 `min_len`/`max_len`（不删除）：3 个预设依赖精确长度 guard，字段已是 Option 对自定义无负担
- 前端 `previewSimpleMask` 反向分支是后端 `apply_template` 反向逻辑的等价移植（纯前端逻辑，不写 DB），用于内联测试预览；实际脱敏仍由后端 `mask_column` 执行（结果一致）
- 旧 DB（无 reverse 字段的 template JSON）仍能正确加载为正向 Simple 模板，行为不变（由 `old_flat_json_deserializes_to_simple` + `template_reverse_default_is_forward` 测试覆盖）

## 6. 待办（T53 dev_complete → verified_complete）

- [ ] Reviewer 审查 T53 交接三件套（HANDOFF + REPORT + 代码改动）
- [ ] 审查通过后状态升 verified_complete，TASK-BOARD / 更新日志同步
- [ ] v1.1.3 整体 Release QA 审计（待 v1.1.3 全部任务 verified_complete 后触发）
