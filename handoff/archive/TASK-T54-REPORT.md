# T54 — 拆分整段脱敏与分段脱敏为两条独立规则 REPORT

> 版本：v1.1.3
> 任务：T54
> 状态：dev_complete（待 reviewer 审核）
> 依赖：T53（dev_complete）

## 1. 执行摘要

将原 `general-mask` 一条规则（用 `TemplateParams` untagged enum 承载 Simple + Segment 两种变体 + 前端「模板类型」Select 切换）拆分为两条独立规则：

- `simple-mask`（整段脱敏）：持 `TemplateParams::Simple(SimpleTemplate::default())` 空模板
- `segment-mask`（分段脱敏）：持 `TemplateParams::Segment(SegmentTemplate::default())` 空模板

旧 `general-mask` id 加入 `cleanup_deprecated_rules()` 删除列表。v1.1.3 未发布、dev 库无生产数据，可安全改名。

**核心洞察**：masker dispatch 只按 `template` 变体分流（`match tpl { Simple(s) => apply_template(...), Segment(s) => apply_segment_template(...) }`），与 rule.id 无关。因此拆分是 seeding + UI 层面的改动，masker 算子零改动。

## 2. 改动统计

| 层 | 文件 | 改动类型 | 行数变化 |
|----|------|----------|----------|
| core | `crates/core/src/processor/rules.rs` | 删 1 构造函数 + 加 2 构造函数 + 测试拆分 + doc | ~+30 |
| core | `crates/core/src/processor/masker.rs` | 测试辅助重命名 + 测试改用新构造函数 + doc | ~+10 |
| tauri | `src-tauri/src/db/mod.rs` | cleanup 加 general-mask + seed 5 条 + 测试 + doc | ~+20 |
| tauri | `src-tauri/src/commands/processor.rs` | doc 注释更新 | ~+2 |
| frontend | `frontend/src/components/panels/maskTemplate.js` | doc 注释更新 | ~+4 |
| frontend | `frontend/src/components/panels/MaskPanel.jsx` | 删类型切换 + id 判断改 + JSX 重构 | ~-15 |
| frontend | `frontend/src/components/panels/RulesPanel.jsx` | 镜像 MaskPanel | ~-10 |
| frontend | `frontend/src/tauri.js` | JSDoc 更新 | ~+2 |
| docs | `docs/versions/1.1.3/更新日志.md` | 追加 T54 进度 + E38~E44 + 设计决策 + 边界 | ~+30 |
| docs | `handoff/TASK-BOARD.md` | 追加 T54 DAG + 任务行 + 验收 | ~+25 |
| docs | `handoff/TASK-T54-HANDOFF.md` | 新建 | ~+90 |
| docs | `handoff/TASK-T54-REPORT.md` | 新建（本文件）| ~+60 |

## 3. 验收结果

### 3.1 构建与测试

```
cargo fmt --all -- --check          → 通过
cargo clippy --workspace --all-targets -- -D warnings  → 通过
cargo test --workspace              → 184 个单测全绿
  - core lib: 83 passed
  - core (含 masker/rules): 101 passed
  - tauri db: 含 seed/cleanup/update_rule_template 测试
pnpm --prefix frontend build        → 通过（3082 modules）
```

### 3.2 关键测试

| 测试 | 验证点 |
|------|--------|
| `with_defaults_loads_five_rules` | 5 条规则（3 name + simple-mask + segment-mask），mask_count==3 |
| `simple_mask_rule_has_empty_template` | simple-mask 持 `TemplateParams::Simple` 空模板 |
| `segment_mask_rule_has_empty_template` | segment-mask 持 `TemplateParams::Segment` 空模板 |
| `get_by_id_works` | `get("simple-mask")` + `get("segment-mask")` 存在，`get("general-mask")` 为 None |
| `register_overwrites_same_id` | len==5 |
| `seed_builtin_rules_inserts_five_when_empty` | seed 后 count==5，重复 seed 幂等 |
| `seed_builtin_rules_upserts_missing_on_existing_db` | simple-mask/segment-mask 被 upsert 补 seed，general-mask 不存在 |
| `cleanup_deprecated_rules_removes_legacy_ids` | 5 个遗留 id（含 general-mask）全删，5 条内置规则保留 |
| `update_rule_template_persists_and_reads_back` | simple-mask template 持久化 + 读回 |
| `rule_kind_round_trip_through_db` | list.len()==5 |

### 3.3 端到端验收

- [x] E38: cargo fmt/clippy/test 全绿（184 个单测）
- [x] E39: pnpm build 通过
- [x] E40: with_defaults 5 条 + get_by_id simple-mask/segment-mask 存在、general-mask 为 None
- [x] E41: cleanup 含 general-mask 的 5 个遗留 id 全删
- [x] E42: seed 5 条 idempotent + upsert-missing 补 simple-mask/segment-mask
- [x] E43: 前端 MaskPanel 3 条 mask 规则 + simple-mask 显示预设 + 7 参数框 + 反向 Switch，无模板类型 Select
- [x] E44: 前端 RulesPanel segment-mask 显示分隔符 + 段配置，无模板类型 Select，重置按 id 回退

## 4. 设计决策

1. **彻底改名**（vs 保留 general-mask）：新建 `simple-mask` + `segment-mask`，旧 `general-mask` 删除。v1.1.3 未发布，dev 库无生产数据，无需保留向后兼容。
2. **TemplateParams 结构不变**：untagged enum 保留，DB JSON 向后兼容，SCHEMA_VERSION 保持 4。每条规则固定一种变体（simple-mask → Simple，segment-mask → Segment），但 enum 本身仍支持两种。
3. **masker dispatch 不看 rule.id**：`match tpl { Simple(s) => ..., Segment(s) => ... }` 只按 template 变体分流。拆分是 seeding + UI 改动，算子零改动。
4. **前端删「模板类型」Select**：每条规则固定一种模板类型，无需切换。`handleTemplateTypeChange` 删除，渲染按 `selected.id` 分流。`TEMPLATE_TYPE_*` 常量在 maskTemplate.js 保留（不删除，避免破坏外部 import）。
5. **cleanup_deprecated_rules 加 general-mask**：dev 库升级时旧 general-mask 被 `DELETE FROM rules WHERE id = ?1` 删除（参数绑定），simple-mask/segment-mask 被 upsert-missing 补 seed。

## 5. 已知边界

- 旧 general-mask 规则的 DB 行（含用户编辑过的 template JSON）在升级时被删除——v1.1.3 未发布，dev 库无生产数据，可接受；若用户在旧 general-mask 上保存过自定义参数，需在 simple-mask 或 segment-mask 重新配置。
- `maskTemplate.js` 仍保留 `TEMPLATE_TYPE_SIMPLE`/`TEMPLATE_TYPE_SEGMENT` 常量导出（未删除），MaskPanel/RulesPanel 不再 import 它们。
- masker dispatch 只按 template 变体分流，若用户把 simple-mask 规则的 template 改为 Segment JSON（理论上可通过直接调 `update_rule_template` IPC 实现），masker 仍会走分段脱敏逻辑——规则 id 与 template 变体之间无强校验（前端 UI 已限制每条规则只能编辑对应类型参数）。

## 6. 安全约束遵守

- ✅ 不涉及 DB schema 迁移（SCHEMA_VERSION 保持 4）
- ✅ `cleanup_deprecated_rules` 用参数绑定 `DELETE FROM rules WHERE id = ?1`
- ✅ TemplateParams 序列化用 serde，不手动拼 JSON
- ✅ 不含可用凭据字面量
- ⚠️ Mimosa 深度扫描需在 commit 前重跑完整审计（不宣称项目安全）
- ✅ 不创建标签（v1.1.3 未完成）

## 7. 后续

- 待 reviewer 审核 T54（交接文件：TASK-T54-HANDOFF.md）
- 审核通过后 T54 状态 → verified_complete
- v1.1.3 全部任务（T48~T54）verified_complete 后进入 Release QA 审计
