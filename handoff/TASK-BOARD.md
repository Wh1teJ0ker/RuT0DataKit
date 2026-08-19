goal: |
  完成 v1.2.2：将现有 SQL 导入、规则参数及工具面板修复纳入可验证的版本闭环，
  完成构建、测试、版本号同步和自动提交；不创建 tag。
tasks:
  - id: T1
    title: MySQL dump SQL 导入兼容
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T1-HANDOFF.md
  - id: T2
    title: 地址规则参数化与前后端一致性
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T2-HANDOFF.md
  - id: T3
    title: 出生日期与身份证提取参数一致性
    depends_on: []
    status: verified_complete
    handoff: handoff/TASK-T3-HANDOFF.md
  - id: T4
    title: 工具面板运行时初始化与版本收口
    depends_on: [T1, T2, T3]
    status: verified_complete
    handoff: handoff/TASK-T4-HANDOFF.md
e2e_acceptance:
  - MySQL dump 和 SQLite SQL 均可由核心数据源测试覆盖。
  - 地址、出生日期、身份证参数从 UI 契约传递到后端并保持向后兼容。
  - 列操作和加解密面板不会因未声明 Form/state 变量而渲染失败。
  - 版本源均为 1.2.2，未触碰 CTF answer 文件或恢复用户删除的测试样例。
e2e_verification:
  - cargo test -p ruT0-data-kit-core
  - cargo test -p ruT0-data-kit
  - cargo check --workspace
  - pnpm --prefix frontend build
release_qa:
  required: true
  report: docs/qa/versions/1.2.2/QA-审计报告.md
  audit_scope:
    - 需求覆盖
    - 端到端流程
    - 构建与测试
    - 代码质量
    - 安全与隐私
    - 数据与迁移
    - 依赖与配置
    - 文档一致性