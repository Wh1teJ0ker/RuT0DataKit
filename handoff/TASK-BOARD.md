# v1.1.3 TASK-BOARD

> 版本：v1.1.3
> 状态：qa_passed（T48~T51 verified_complete + T52~T57 dev_complete + E2E 全绿 + Release QA 审计通过，结论 qa_passed，见 `docs/qa/versions/1.1.3/QA-审计报告.md`）
> 前置：v1.1.2 `qa_passed` + tag `v1.1.2` 已发布

## 任务 DAG

```
T48 通用模板脱敏算子（TemplateParams + DB schema v4 + seed upsert-missing）
   ├─ core: rules.rs TemplateParams + Rule.template 字段
   ├─ core: masker.rs template 分支 + apply_template + mask_char 优先级
   ├─ tauri: schema.rs SCHEMA_VERSION=4 + template 列
   ├─ tauri: migrate.rs migrate_v3_to_v4 + v2→v4 链式
   ├─ tauri: db/mod.rs row_to_rule/upsert_rule/list/get + seed upsert-missing
   └─ 测试数据: tests/脱敏 + tests/提取 + tests/校验
       ↓
T49 4 条脱敏规则收敛为通用脱敏的子规则（预设）
   ├─ core: rules.rs 删除 4 独立规则 + general_mask_rule + 4 预设函数 + is_empty
   ├─ core: masker.rs 空模板 no-op 检测
   ├─ tauri: db/mod.rs update_rule_template 方法
   ├─ tauri: processor.rs mask_column 加 template 参数 + update_rule_template IPC
   ├─ tauri: lib.rs 注册 update_rule_template
   ├─ frontend: tauri.js maskColumn 加 template + updateRuleTemplate
   ├─ frontend: MaskPanel.jsx 子规则下拉 + 6 参数框 + 预设填充
   └─ 文档: 更新日志 + RELEASE-NOTES + 技术设计 + tests README
       ↓
T50 清理遗留规则 + 移除前端「目标列」显示
   ├─ tauri: db/mod.rs seed_builtin_rules 加 cleanup_deprecated_rules
   └─ frontend: RulesPanel.jsx 删除「目标列」显示行
       ↓
T51 规则管理面板补齐通用脱敏模板参数（6 字段 + 预设 + 模板测试）
   ├─ frontend: maskTemplate.js 抽取共享模块（MASK_PRESETS/buildTemplateForRun/previewMask/...）
   ├─ frontend: MaskPanel.jsx 改用共享模块（去重）
   ├─ frontend: RulesPanel.jsx general-mask 暴露 6 模板参数 + 预设下拉
   │            + 保存分支（updateRuleTemplate）+ 重置 + 模板测试（previewMask）
   └─ 文档: 更新日志 + TASK-BOARD + T51 交接三件套
       ↓
T52 分段脱敏模板（按分隔符拆分 + 每段保留首尾）
   ├─ core: rules.rs TemplateParams 改 untagged enum（Simple/Segment）
   │        + SegmentMask/SegmentTemplate + builder + is_empty 适配
   ├─ core: masker.rs mask 分支 match enum + apply_segment_template/apply_segment_part
   ├─ frontend: maskTemplate.js EMPTY_SEGMENT_TEMPLATE + normalizeTemplate/
   │            buildTemplateForRun/previewMask 分段分支 + isSegmentTemplate
   ├─ frontend: MaskPanel.jsx + RulesPanel.jsx 模板类型切换 + Segment 参数 UI
   └─ 文档: 更新日志 + TASK-BOARD + T52 交接三件套
```

T53 反向脱敏模板（掩码首尾、保留中间）
   ├─ core: rules.rs SimpleTemplate + reverse: Option<bool> + with_reverse
   ├─ core: masker.rs apply_template 反向分支（重叠全脱码 + guard 仍生效）
   ├─ frontend: maskTemplate.js reverse 分支 + MaskPanel/RulesPanel 反向 Switch
   └─ 文档: 更新日志 + TASK-BOARD + T53 交接三件套
       ↓
T54 拆分整段脱敏与分段脱敏为两条独立规则
   ├─ core: rules.rs 删 general_mask_rule → simple_mask_rule + segment_mask_rule
   │        + with_defaults 5 条 + 测试拆分
   ├─ core: masker.rs 测试辅助重命名 + 测试改用新构造函数（dispatch 不变）
   ├─ tauri: db/mod.rs cleanup_deprecated_rules 加 general-mask + seed 5 条 + 测试
   ├─ tauri: processor.rs doc 注释更新（签名不变）
   ├─ frontend: maskTemplate.js doc 注释更新（逻辑不变）
   ├─ frontend: MaskPanel.jsx + RulesPanel.jsx 删模板类型 Select + id 判断改
   │            simple-mask/segment-mask
   ├─ frontend: tauri.js JSDoc 更新
   └─ 文档: 更新日志 + TASK-BOARD + T54 交接三件套
```

## 任务清单

| 任务 | 标题 | 状态 | 依赖 | 交接文件 |
|------|------|------|------|----------|
| T48 | 通用模板脱敏算子（TemplateParams + DB schema v4 + seed upsert-missing） | verified_complete | — | handoff/TASK-T48-HANDOFF.md |
| T49 | 4 条脱敏规则收敛为通用脱敏的子规则（预设） | verified_complete | T48 | handoff/TASK-T49-HANDOFF.md |
| T50 | 清理遗留规则 + 移除前端「目标列」显示 | verified_complete | T49 | handoff/TASK-T50-HANDOFF.md |
| T51 | 规则管理面板补齐通用脱敏模板参数 | verified_complete | T50 | handoff/TASK-T51-HANDOFF.md |
| T52 | 分段脱敏模板（按分隔符拆分 + 每段保留首尾） | dev_complete | T51 | handoff/TASK-T52-HANDOFF.md |
| T53 | 反向脱敏模板（掩码首尾、保留中间） | dev_complete | T51 | handoff/TASK-T53-HANDOFF.md |
| T54 | 拆分整段脱敏与分段脱敏为两条独立规则 | dev_complete | T53 | handoff/TASK-T54-HANDOFF.md |

## 端到端验收项

- [x] E1: `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace` 全绿（83 个单测，含 masker 模板分支 + 预设参数 + 空模板透传 + DB update_rule_template 持久化 + cleanup_deprecated_rules 清理）
- [x] E2: `pnpm --prefix frontend build` 通过（T51 新增 maskTemplate.js 共享模块 + RulesPanel 6 模板参数，无 lint/构建错误）
- [x] E3: 规则管理面板可见 4 条规则（3 name + 1 general-mask），不再显示「目标列」行
- [x] E4: 脱敏面板下拉可选 2 条 mask 规则（name-mask + general-mask）
- [x] E5: general-mask 选身份证预设 → `110101********1234`；15 位原样（guard）
- [x] E6: general-mask 选手机预设 → `138****5678`；10 位原样
- [x] E7: general-mask 选出生日期预设 → `1990-01-**`
- [x] E8: general-mask 选银行卡预设 → `6222***********0123`
- [x] E9: 姓名脱敏仍走旧逻辑（`张三丰` → `张*丰`），向后兼容
- [x] E10: general-mask 不选预设（空模板）→ 原样返回（不脱敏）
- [x] E11: 掩码字符输入 `#` → 临时覆盖（身份证预设 → `110101########1234`）
- [x] E12: 6 个参数框可手动编辑（选预设填充后可继续修改）
- [x] E13: 保存设置 → general-mask 持久化 template（update_rule_template）；name-mask 持久化 replacement（update_rule_params）
- [x] E14: 版本号 3 处一致 1.1.3
- [x] E15（T51）：规则管理面板选 general-mask → 可填参数卡片显示子规则下拉 + 6 个模板参数框
- [x] E16（T51）：规则管理面板选身份证预设 → 内联测试 `110101199001011234` → `110101********1234`
- [x] E17（T51）：规则管理面板保存参数 → 持久化 template 到 DB（updateRuleTemplate）；切回规则后草稿从 DB 回显
- [x] E18（T51）：规则管理面板选 name-mask → 仍只显示掩码字符输入（旧逻辑不变）
- [x] E19（T51）：MaskPanel 与 RulesPanel 共用 maskTemplate.js（无重复定义）
- [x] E20（T52）：cargo fmt/clippy/test 全绿（96 个单测，含 segment 模板 serde roundtrip + 旧 flat JSON 兼容 Simple + email/ip 分段脱敏 + 空 delimiter/segments 透传 + 未知段索引不动）
- [x] E21（T52）：pnpm build 通过（maskTemplate.js 分段分支 + MaskPanel/RulesPanel 模板类型切换 + Segment 参数 UI）
- [x] E22（T52）：后端 segment_email_mask → `zhangsan@example.com` → `z******n@example.com`
- [x] E23（T52）：后端 segment_ip_mask → `192.168.11.1` → `192.168.**.1`
- [x] E24（T52）：后端 segment_multiple_segments_masked → `192.168.11.1` → `192.168.**.**`
- [x] E25（T52）：后端 segment_empty_delimiter_passthrough + segment_no_segments_passthrough → 透传
- [x] E26（T52）：后端 segment_unknown_index_untouched → 段索引超出范围，该段不动
- [x] E27（T52）：后端 old_flat_json_deserializes_to_simple → 旧 DB flat JSON 仍能反序列化为 Simple（向后兼容）
- [x] E28（T52）：前端 MaskPanel + RulesPanel 均可切换 Simple/Segment 模板类型 + 编辑 Segment 参数
- [x] E29（T53）：cargo fmt/clippy/test 全绿（100 个单测，含 reverse 反向脱码首尾 + 保留中间 + 重叠全脱码 + mask_char + guard 仍生效 + 默认正向 + reverse serde roundtrip + 旧 JSON 兼容）
- [x] E30（T53）：pnpm build 通过（maskTemplate.js reverse 分支 + MaskPanel/RulesPanel 反向脱敏 Switch）
- [x] E31（T53）：后端 `template_reverse_mask_head_tail` → `13812345678` + kp=3/ks=4/reverse=true → `***1234****`
- [x] E32（T53）：后端 `template_reverse_keeps_middle` → `abcdef` + kp=2/ks=2/reverse=true → `**cd**`
- [x] E33（T53）：后端 `template_reverse_full_mask_on_overlap` → kp=10/ks=10/n=11/reverse=true → 11 个 `*`（重叠全脱码）
- [x] E34（T53）：后端 `template_reverse_with_mask_char` → + mask_char='#' → `###1234####`
- [x] E35（T53）：后端 `template_reverse_guard_still_works` → + min_len=18 → 15 位输入原样返回（guard 仍生效）
- [x] E36（T53）：后端 `template_reverse_default_is_forward` → reverse=None → 正向 `138****5678`
- [x] E37（T53）：前端 MaskPanel + RulesPanel 均有「反向脱敏」Switch，开启后预览 `13812345678` → `***1234****`
- [x] E38（T54）：cargo fmt/clippy/test 全绿（184 个单测，含 simple_mask_rule/segment_mask_rule 空模板 + with_defaults 5 条 + cleanup 含 general-mask + seed 5 条 idempotent + upsert-missing + get_by_id simple-mask/segment-mask）
- [x] E39（T54）：pnpm build 通过（MaskPanel/RulesPanel 删除模板类型 Select + handleTemplateTypeChange，按 selected.id 分流渲染 simple-mask/segment-mask 参数区）
- [x] E40（T54）：后端 `with_defaults_loads_five_rules` → 5 条（3 name + simple-mask + segment-mask）；`get_by_id_works` → `get("simple-mask")` + `get("segment-mask")` 存在、`get("general-mask")` 为 None
- [x] E41（T54）：`cleanup_deprecated_rules_removes_legacy_ids` → 含 general-mask 在内的 5 个遗留 id 全部删除，5 条内置规则保留
- [x] E42（T54）：`seed_builtin_rules_inserts_five_when_empty` + 重复 seed 幂等；`upserts_missing_on_existing_db` → simple-mask/segment-mask 被 upsert 补 seed，general-mask 不存在
- [x] E43（T54）：前端 MaskPanel 脱敏规则下拉 3 条 mask 规则（name-mask + simple-mask + segment-mask）；选 simple-mask → 预设下拉 + 7 个 Simple 参数框（含反向脱敏 Switch），无「模板类型」Select
- [x] E44（T54）：前端 RulesPanel 选 segment-mask → 显示分隔符 + 段配置，无「模板类型」Select，无 Simple 预设；重置按钮按 selected.id 回退到对应空模板

## Release QA 门禁

- required: true（已完成 Release QA 审计）
- report: docs/qa/versions/1.1.3/QA-审计报告.md（已生成，结论 qa_passed）
- audit_scope: 需求覆盖 / 端到端流程 / 构建与测试 / 代码质量 / 安全与隐私 / 数据与迁移 / 依赖与配置 / 文档一致性 / 回归检查 / 发布门禁
- conclusion: qa_passed（250 单测 + 11 doc-tests + 3083 modules 构建 + 4 处版本一致 + SCHEMA_VERSION=5 幂等迁移 + 55 处 SQL 参数绑定 + Mimosa 0 findings）
- 安全约束：Mimosa 深度扫描已完成（scan-2026-08-10T16-40-17.470Z-31df73e0a37d，0 findings / 487 包 0 漏洞）；静态分析非运行时验证，不宣称项目安全，但无已识别 finding 阻碍发布
