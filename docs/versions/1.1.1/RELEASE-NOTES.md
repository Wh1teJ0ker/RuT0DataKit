# v1.1.1 Release Notes

> Git tag：`v1.1.1`（待发布）
> 状态：待发布（T29~T35 verified_complete；T36 文档收口 + Release QA 进行中）
> 前置：v1.1.0 已 `qa_passed` 并发布 tag `v1.1.0`

## 这是什么

RuT0DataKit v1.1.1 在 v1.1.0（脱敏 / 校验 / 提取原型 + 规则管理持久化）之上，交付**撤销 / 重做 + 列操作 + 搜索**三大功能，并补齐 commands 层测试。数据全程在本地处理，不外发。

## 更新了什么

### 新增

- **撤销 / 重做**：`mask_column` / `replace_in_column` / `replace_all` 三类就地变更操作现可撤销可重做；`mask_column` 执行时抓取 before/after 快照（仅变更列 cells）存入 `operations` 表的 `before_snapshot_json` / `result_snapshot_json`；撤销 = 回写 before 快照，重做 = 回写 after 快照；撤销范围明确限定为就地变更类（import / parse_json / validate / extract 不入撤销栈）
- **列操作 — JSON 解析为新 Tab**：选列 → `parse_column_as_json` 把每行当 JSON 对象解析 → 收集所有 key 作新表头 → 创建新 sheet（`{column}_json`）写入展开后的列；解析失败的行跳过并报告 `skipped` 计数
- **列操作 — 列内批量替换**：选列 + from/to + useRegex → `replace_in_column` 单事务列内替换，before/after 快照存入 `operations` 供撤销
- **搜索 — 关键字 + 正则**：搜索栏输入关键字（`LIKE '%kw%' ESCAPE '\'`）或正则模式（SQL LIKE 预筛 + Rust `regex` 精确匹配）；可选指定列或搜全表；服务端分页（`LIMIT/OFFSET`）+ `total` 总数；命中单元格用 `<mark>` 高亮匹配区间（背景 #fff48f）
- **搜索 — 全局替换**：全局替换 Modal（from/to/useRegex）→ `replace_all` 全表搜索替换（单事务），before/after 快照存入 `operations` 供撤销
- **搜索索引优化**：新增 `idx_cells_sheet_col` 复合索引（`cells(sheet_id, col_idx)`）+ 服务端分页，避免大文件搜索卡顿
- **commands 层测试**：首个 commands 层测试套件——undo/redo（5 tests）+ search（15 tests）+ columns（7 tests）；DB 层扩展至 39 tests；workspace 全量 66 passed
- **版本号**：1.1.0 → 1.1.1

### 明确不做（推迟至 v1.2+）

- FTS5 全文搜索（LIKE + 复合索引 + 分页已满足性能）
- import / parse_json / validate / extract 纳入撤销栈（仅就地变更操作可撤销）
- 嵌套 JSON 递归展平（`a.b` 列名）
- 多字节字符搜索高亮精确边界（v1.1.1 字节偏移切片，纯 ASCII 安全）
- 规则引擎完整化 / Tools / tshark / PCAP / 真实 AI

## 验证

- `cargo fmt --check`：通过
- `cargo clippy --workspace -- -D warnings`：通过
- `cargo test --workspace`：66 passed / 0 failed / 0 ignored（含 commands 层 undo/redo 5 + search 15 + columns 7 + DB 39）
- `pnpm --prefix frontend install --frozen-lockfile`：通过
- `pnpm --prefix frontend build`：通过（3079 modules transformed，2.31s）

## 已知限制

- 撤销范围仅限 mask / replace_in_column / replace_all 三类就地变更操作；import / parse_json（新增 sheet）/ validate / extract 不可撤销（parse_json 产生的 Tab 用关闭 Tab 移除）
- 搜索高亮匹配区间按**字节偏移**切片（`str::find` / `regex::Match`），纯 ASCII 安全，多字节字符（中文）边界可能错位——已知简化，留待 v1.2+ 优化
- 正则模式 `total` 取全量命中数（无法用 SQL COUNT），超大表性能待后续版本优化
- `parse_column_as_json` 仅展平一层 JSON 对象，嵌套对象的 value 用 `to_string()` 序列化；递归展平（`a.b` 列名）推迟 v1.2+
- T34 review 标记 3 项 minor 非阻塞问题：全局替换后未刷新 undoStack（建议手动切换 Tab 触发刷新）/ 空 sheet 调 listUndoableOperations 噪音（空数组，无副作用）/ ColumnOpsPanel 空 sheet sessionId 守卫

## 升级

v1.1.0 用户可直接升级：DB schema 从 `SCHEMA_VERSION=2` 增量迁移到 `3`（`operations` 表加 `before_snapshot_json` 列 + `idx_cells_sheet_col` 索引，`PRAGMA table_info` 检查列存在再 ALTER + `IF NOT EXISTS` 幂等，不触发备份重建），历史数据完整保留。启动后历史 v1.1.0 数据无需转换即可使用新搜索与撤销功能（历史 operations 行的 `before_snapshot_json` 为 NULL，表示不可撤销，仅新产生的 mask/replace 操作可撤销）。
