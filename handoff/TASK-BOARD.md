# v1.1.4 TASK-BOARD（续轮：通用校验 + 地址放宽 + 生日清理 + 前端统一化）

> 版本：v1.1.4
> 状态：qa_passed（首轮 T67/T68/T69 + 续轮 T70/T71/T72 全部 verified_complete + E2E 全绿 + Release QA 增量审计通过）
> 前置：v1.1.4 首轮 T67/T68/T69 已 qa_passed 并 commit ff0317a（见 `docs/qa/versions/1.1.4/QA-审计报告.md`）
> 主题：v1.1.4 续轮 — 4 项增强（通用校验规则 + 地址校验放宽 + 出生日期清理 + 前端参数 UI 统一化）

## 背景

用户在 v1.1.4 首轮 qa_passed 后追加 4 个需求（仍属 v1.1.4）：

1. **通用校验**：新增一种自由组合字符类（纯数字/纯字母/特殊符号）+ 长度限制的通用校验规则。
2. **地址校验放宽**：当前 `is_valid_address` 严格正则（号1-1500+室101-999）→ 结构化校验（中文+地址关键词，数字不限定）。用户确认"最好是结构化校验"。
3. **出生日期清理**：当前 `is_valid_birth` 只接受 `^\d{8}$` → 先清理分隔符再校验（支持 `20031223` / `2003-12-23` / `2003/12/23` 等）。
4. **前端统一化**：RulesPanel 按规则类型渲染参数 UI（当前 validate 规则只显示空正则输入框）；ValidatePanel 按规则行渲染参数配置。

## 用户决策（AskUserQuestion 已确认）

- **地址校验**：结构化校验（中文 + 地址关键词，数字不限定范围）

## 任务 DAG

```yaml
goal: |
  v1.1.4 续轮：通用校验规则（Generic 变体）+ 地址校验放宽为结构化校验 +
  出生日期先清理后校验 + 前端 RulesPanel/ValidatePanel 按规则类型统一参数 UI。

tasks:
  - id: T70
    title: 后端 — 通用校验 Generic 变体 + 地址放宽 + 生日清理 + params_override 契约 + 测试
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T70-HANDOFF.md
  - id: T71
    title: 前端 — RulesPanel/ValidatePanel 按规则类型统一参数 UI + validateParams.js
    depends_on: [T70]
    status: verified_complete
    handoff: handoff/TASK-T71-HANDOFF.md
  - id: T72
    title: 文档 + E2E 验收 + Release QA 增量审计
    depends_on: [T71]
    status: verified_complete
    handoff: handoff/TASK-T72-HANDOFF.md

e2e_acceptance:
  - 通用校验规则 generic-validate 可在 RulesPanel 编辑字符类 Checkbox + 长度 InputNumber
  - 通用校验规则 generic-validate 可在 ValidatePanel 每行展开参数配置（paramsOverride）
  - 地址校验接受 "北京市朝阳区建国路88号" 等结构化地址（不再要求号1-1500+室101-999）
  - 出生日期校验接受 "20031223" / "2003-12-23" / "2003/12/23" 等格式（先清理分隔符）
  - RulesPanel 选中 birth/address/idcard 等 validate 规则时显示 hint 文案（不显示空正则输入框）
  - ValidatePanel generic-validate 行展开字符类 Checkbox + 长度 InputNumber
  - 跨字段 birth 比对支持分隔符格式（"2003-12-23" 与 idcard[6..14] 一致 → valid）
  - cargo fmt/clippy/test 全绿 + pnpm build 全绿

e2e_verification:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all
  - pnpm --prefix frontend build

release_qa:
  required: true
  report: docs/qa/versions/1.1.4/QA-审计报告.md
  audit_scope:
    - 需求覆盖
    - 端到端流程
    - 构建与测试
    - 代码质量
    - 安全与隐私
    - 数据与迁移
    - 依赖与配置
    - 文档一致性
```

## 安全约束（延续）

- SQL 全部 `?N` + `params![]` 绑定，禁拼接、format、f-string
- 无凭据字面量
- Mimosa 完整审计未拿到结论前不宣称安全
- 不创建 tag（v1.1.4 续轮 Release QA 通过前不打）
