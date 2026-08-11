# v1.1.4 TASK-BOARD

> 版本：v1.1.4
> 状态：qa_passed（T67/T68/T69 全部 verified_complete + E2E 全绿 + Release QA 审计通过 — 见 `docs/qa/versions/1.1.4/QA-审计报告.md`）
> 前置：v1.1.3 已发布 tag `v1.1.3`（handoff 旧表归档至 `handoff/archive/TASK-BOARD-v1.1.3-20260807.md`）
> 主题：校验模块重设计 — 统一校验页面（自由选多规则 + 一按钮 + 双 Tab 输出）

## 背景

v1.1.3 把行级多字段校验（T57）做成独立能力。v1.1.4 初版尝试用「单列校验 Tab + 行级校验 Tab」合并入 ValidatePanel。用户反馈：**不应该是两个 Tab，完整的应该是一个统一校验页面** — 自由选取多个规则（每条 = 选一列 + 选一校验规则），一个按钮校验，通过/失败的行分别写入两个新 Tab。

本版重做校验模块为统一页面，并扩展后端规则系统以支持函数式校验规则作为可选 `kind=validate` 规则。

## 用户决策（AskUserQuestion 已确认）

1. **规则集合**：全量保留 — 把 T57 的 6 个函数式校验（username/sex/birth/idcard/phone/address）注册为 DB 可选 `kind=validate` 规则（带 `params` 走 `validate_extracted` 分发）。
2. **跨字段**：以身份证为主 — 选「身份证号校验」规则时，可勾选「对比性别一致性」+「对比出生日期一致性」，并指定性别列/出生日期列。跨字段仅在身份证本身校验通过时生效。

## 已完成的前置工作（v1.1.4 初版，本版复用）

- ✅ 版本号 bump 1.1.3 → 1.1.4（4 处：Cargo.toml / tauri.conf.json / frontend/package.json / frontend/src/constants.js）
- ✅ TopToolbar 移除 `rowValidate` CAPABILITY + SafetyOutlined import
- ✅ SidePanel 移除 `rowValidate` PANELS 键 + RowValidatePanel 导入
- ✅ 删除 RowValidatePanel.jsx
- ✅ docs/versions/1.1.4/ 目录已建（文档内容需重写以反映重设计）

## 任务 DAG

```yaml
goal: |
  重做 v1.1.4 校验模块为统一校验页面：用户自由选取多条「列 + 校验规则」组合，
  身份证规则可勾选跨字段（性别/出生日期）比对，一个按钮校验，通过/失败的行
  分别写入两个新 Tab。后端扩展规则系统支持函数式校验规则作为可选 validate 规则。

tasks:
  - id: T67
    title: 后端规则系统扩展（ExtractParams +4 变体 + validate_extracted 分发 + seed 6 条 validate 规则 + 测试）
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T67-HANDOFF.md（已清理）
  - id: T68
    title: 后端新命令 validate_multi_rules_to_two_sheets（多规则逐行校验 + 身份证跨字段可选 → 双 Tab）+ 集成测试
    depends_on: [T67]
    status: verified_complete
    handoff: handoff/TASK-T68-HANDOFF.md（已清理）
  - id: T69
    title: 前端统一校验页面（重做 ValidatePanel.jsx + tauri.js wrapper + 注册命令 + 版本文档重写）
    depends_on: [T68]
    status: verified_complete
    handoff: handoff/TASK-T69-HANDOFF.md（已清理）

e2e_acceptance:
  - 校验页面是一个统一表单（非两个 Tab），可添加多条「列 + 校验规则」组合
  - 可选校验规则包含 12 条：姓名/用户名/性别/出生日期/身份证/手机号/地址（7 函数式）+ 银行卡/IPv4/IPv6（3 extract 复用）+ 手机号前缀白名单
  - 身份证规则行可勾选「对比性别一致性」+「对比出生日期一致性」，勾选后需指定性别列/出生日期列
  - 一个「校验」按钮 → 通过/失败的行分别写入 {源sheet名}_校验通过 / {源sheet名}_校验失败 两个新 Tab，保留原列不新增列
  - 跨字段校验仅当身份证本身校验通过时生效
  - 旧 validate_column 命令保留（不破坏原位高亮能力，但前端不再走它）
  - 旧 validate_rows_to_two_sheets 命令保留（不删除，避免破坏潜在调用方）

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
- 不创建 tag（v1.1.4 未完成 Release QA 前不打）
