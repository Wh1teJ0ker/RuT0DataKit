# v1.2.2 E2E 验收报告

> 验收时间：2026-08-19
> 结论：`done_e2e` — 4/4 任务 verified_complete，全部端到端验证命令通过。

## 端到端验收项

| # | 验收项 | 证据 | 结果 |
|---|---|---|---|
| 1 | MySQL dump 和 SQLite SQL 均可由核心数据源测试覆盖 | cargo test datasource::sql: 6/6 通过；mysql_dump: 端到端测试通过 | ✓ |
| 2 | 地址参数从 UI 契约传递到后端并保持向后兼容 | 旧 {"validator":"address"} 可反序列化；英文地址失败；号/室范围在后端、规则管理、行级校验和预览中一致生效 | ✓ |
| 3 | 出生日期格式可持久化；身份证首位为零开关仅运行时覆盖不持久化 | extract_ops 测试 25/25 通过；idcard_allow_leading_zero 测试覆盖 | ✓ |
| 4 | 列操作和加解密面板不会因未声明 Form/state 变量而渲染失败 | 前端 build 通过，3124 modules，0 error | ✓ |
| 5 | 版本源均为 1.2.2 | Cargo.toml / package.json / tauri.conf.json 均为 1.2.2 | ✓ |
| 6 | 未触碰 CTF answer 文件或恢复用户删除的测试样例 | 已删除测试文件仍为 D 状态，未恢复 | ✓ |

## 端到端验证命令

| 命令 | 结果 |
|---|---|
| cargo test -p ruT0-data-kit-core | 188 passed + 1 integration + 15 doc-tests, 0 failed |
| cargo test -p ruT0-data-kit | 141 passed, 0 failed |
| cargo check --workspace | ✓ Finished dev profile, 0 error |
| pnpm --prefix frontend build | ✓ 3124 modules, built in 2.80s |

## 任务状态

| 任务 | 标题 | 状态 |
|---|---|---|
| T1 | MySQL dump SQL 导入兼容 | verified_complete |
| T2 | 地址规则参数化与前后端一致性 | verified_complete |
| T3 | 出生日期与身份证提取参数一致性 | verified_complete |
| T4 | 工具面板运行时初始化与版本收口 | verified_complete |