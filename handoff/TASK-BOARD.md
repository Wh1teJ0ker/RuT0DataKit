# v1.1.3 TASK-BOARD

> 版本：v1.1.3
> 状态：done_e2e（T48 + T49 + T50 + T51 verified_complete + E2E 全绿）
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
```

## 任务清单

| 任务 | 标题 | 状态 | 依赖 | 交接文件 |
|------|------|------|------|----------|
| T48 | 通用模板脱敏算子（TemplateParams + DB schema v4 + seed upsert-missing） | verified_complete | — | handoff/TASK-T48-HANDOFF.md |
| T49 | 4 条脱敏规则收敛为通用脱敏的子规则（预设） | verified_complete | T48 | handoff/TASK-T49-HANDOFF.md |
| T50 | 清理遗留规则 + 移除前端「目标列」显示 | verified_complete | T49 | handoff/TASK-T50-HANDOFF.md |
| T51 | 规则管理面板补齐通用脱敏模板参数 | verified_complete | T50 | handoff/TASK-T51-HANDOFF.md |

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

## Release QA 门禁

- required: true（待 Release QA 审计）
- report: docs/qa/versions/1.1.3/QA-审计报告.md（待生成）
- audit_scope: 需求覆盖 / 端到端流程 / 构建与测试 / 代码质量 / 安全与隐私 / 数据与迁移 / 依赖与配置 / 文档一致性
- conclusion: 待审计
- 安全约束：Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）
