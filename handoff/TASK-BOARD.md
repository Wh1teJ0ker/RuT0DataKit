# v0.4.0 TASK-BOARD

> 主会话维护。任务 DAG、状态总表、E2E 与 Release QA 门禁。
> 版本：v0.4.0 — 架构性完整重构（7 界面）。

## 版本目标

从「按数据源分片（csv/log/pcap）」演进为「按功能分界面（7 个主界面）」的统一架构。
所有数据源（csv/xlsx/sql/pcap/log/json）统一进入数据预处理界面归一化为 `Records`，
后续功能界面（规则管理/搜索/脱敏/校验/导出/Tools）共享预处理产物。

## 任务 DAG

```text
T5-1 (规则引擎 tags)  ─┬─> T5-4 (GUI 规则管理重构)
                        ├─> T5-7 (脱敏/校验界面改造)
                        ├─> T5-9 (Tools SQL 解析后端)
                        └─> T5-2 (预处理归一化，规则用 tags)

T5-2 (预处理归一化)  ─┬─> T5-3 (GUI 预处理界面 + Sidebar 7 项)
                      ├─> T5-5 (搜索后端)
                      ├─> T5-7 (脱敏/校验改造)
                      └─> T5-8 (导出扩展 json)

T5-3 ─> T5-6 (GUI 搜索界面)        [T5-3 提供 Sidebar/跳转底座]
T5-5 ─> T5-6
T5-5 ─> T5-6 (依赖搜索后端)

T5-9  ─> T5-10 (GUI SQL 解析界面)
T5-11 (正则解析后端，无依赖) ─> T5-12 (GUI 正则解析界面)

T5-3, T5-4, T5-6, T5-7, T5-8, T5-10, T5-12 ─> T5-13 (端到端 + fixture)
T5-13 ─> T5-14 (收尾：版本号 + 文档 + QA + tag)
```

## 状态总表

| ID | 标题 | 依赖 | 状态 | HANDOFF |
|----|------|------|------|---------|
| T5-1 | 规则引擎重构：FieldRule/MaskRule 加 tags + RuleSet::by_tag + presets 分组 | - | verified_complete | (cleaned) |
| T5-2 | 数据预处理归一化：sql/json reader + pcap/log reader 产出 Records + 统一导入命令 | T5-1 | verified_complete | (cleaned) |
| T5-3 | GUI 数据预处理界面（PreprocessView）+ Sidebar 重构为 7 项 | T5-2 | verified_complete | (cleaned) |
| T5-4 | GUI 规则管理界面重构：多标签编辑 + 按标签过滤 | T5-1 | verified_complete | (cleaned) |
| T5-5 | 搜索后端：倒排索引 + 关键词/正则/精确匹配 + 性能基线 | T5-2 | verified_complete | (cleaned) |
| T5-6 | GUI 搜索界面（SearchView）：结果表 + 片段高亮 + 跳转 | T5-3, T5-5 | verified_complete | (cleaned) |
| T5-7 | 脱敏/校验界面改造：数据源改为消费预处理 Records + 按 mask/validate 标签筛选规则 | T5-2, T5-1 | verified_complete | (cleaned) |
| T5-8 | 数据导出界面扩展：新增 json 格式 + 数据源 Select 接预处理/脱敏/校验产物 | T5-2 | verified_complete | (cleaned) |
| T5-9 | Tools SQL 解析后端：tools::sql_parse 模块（盲注全类型 + 数据库还原，独立于 LogEntry） | T5-1 | verified_complete | (cleaned) |
| T5-10 | GUI Tools SQL 解析界面：输入 + 解析按钮 + 还原数据库 Table + 探针明细表 | T5-9 | verified_complete | (cleaned) |
| T5-11 | Tools 正则解析后端：regex_explain（token 解释）+ regex_template（模板生成） | - | verified_complete | (cleaned) |
| T5-12 | GUI Tools 正则解析界面：解析 Tab + 模板 Tab | T5-11 | verified_complete | (cleaned) |
| T5-13 | 端到端集成 + fixture 扩充（6 源预处理 + 搜索大文件 + SQL 解析探针序列） | T5-3, T5-4, T5-6, T5-7, T5-8, T5-10, T5-12 | verified_complete | (cleaned) |
| T5-14 | 收尾：版本号 0.3.0 → 0.4.0（4 处）+ 文档同步 + Phase 7-9 + git tag v0.4.0 + push | T5-13 | verified_complete | (cleaned) |

## E2E 验收项（Phase 7）

1. 6 源预处理：csv/xlsx/sql/pcap/log/json 各导入一张表，PreprocessView 预览 headers/rows 一致。
2. 7 界面全 active 无 disabled。
3. 规则多标签：一条规则挂 mask+sensitive 双标签，RulesView/MaskView 都能选到。
4. 搜索：≥10w 行 fixture 倒排索引构建 < 5s，单次查询 < 200ms。
5. 数据脱敏/校验/导出数据源切换不丢数据。
6. 数据导出 json 格式行数 = 原始行数，key = headers。
7. Tools SQL 解析：v0.2.4 fixture 探针序列还原出 schema=person / 1 表 person_data / 7 列。
8. Tools 正则解析：`^1[3-9]\d{9}$` 解释正确；邮箱模板生成可跑通。
9. `cargo test --workspace` 全绿；npm build；tauri build 产出 v0.4.0 dmg。

## E2E 验证命令

- `cargo build --workspace`
- `cargo test --workspace`
- `cd frontend && npm run build`
- `cd src-tauri && npx @tauri-apps/cli@latest build`

## Release QA 门禁

- required: true
- report: docs/qa/versions/0.4.0/QA-审计报告.md
- audit_scope:
  - 需求覆盖
  - 端到端流程
  - 构建与测试
  - 代码质量
  - 安全与隐私
  - 数据与迁移
  - 依赖与配置
  - 文档一致性

## 并行调度策略

- T5-1（规则 tags）与 T5-11（正则解析后端）无依赖 → 可并行派。
- T5-2 依赖 T5-1 → T5-1 verified_complete 后派。
- T5-3/T5-4/T5-5/T5-7/T5-8/T5-9 在 T5-1/T5-2 通过后可按文件不重叠原则串行+准备重叠派。
- 默认串行单 coder，避免多 coder 改同一片代码冲突。
