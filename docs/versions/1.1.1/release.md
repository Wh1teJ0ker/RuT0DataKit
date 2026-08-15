# RuT0DataKit v1.1.1

## Update

- 撤销 / 重做：`mask_column` / `replace_in_column` / `replace_all` 三类就地变更操作现可撤销可重做；执行时抓取 before/after 快照（仅变更列 cells）存入 `operations` 表；撤销 = 回写 before 快照，重做 = 回写 after 快照；撤销范围明确限定为就地变更类（import / parse_json / validate / extract 不入撤销栈）
- 列操作 — JSON 解析为新 Tab：选列 → `parse_column_as_json` 把每行当 JSON 对象解析 → 收集所有 key 作新表头 → 创建新 sheet（`{column}_json`）写入展开后的列；解析失败的行跳过并报告 `skipped` 计数
- 搜索 — 关键字 + 正则：搜索栏输入关键字（`LIKE '%kw%' ESCAPE '\'`）或正则模式（SQL LIKE 预筛 + Rust `regex` 精确匹配）；可选指定列或搜全表；服务端分页（`LIMIT/OFFSET`）+ `total` 总数；命中单元格用 `<mark>` 高亮匹配区间
- 搜索 — 全局替换：全局替换 Modal（from/to/useRegex）→ `replace_all` 全表搜索替换（单事务），before/after 快照存入 `operations` 供撤销
- 搜索索引优化：新增 `idx_cells_sheet_col` 复合索引（`cells(sheet_id, col_idx)`）+ 服务端分页，避免大文件搜索卡顿
- commands 层测试：首个 commands 层测试套件——undo/redo + search + columns；workspace 全量 72 passed
- 版本号 1.1.0 → 1.1.1

## Fix

- 搜索改为「只保留命中行」模式：新增 `search_rows` 行级搜索命令（按 `distinct row_idx` 分页，返回整行数据 + 命中区间），DataTable 在搜索态渲染 `searchRows` 而非 `sheet.rows`
- 修复搜索 stale highlight bug：第一次搜索的高亮在第二次搜索后仍显示（`APPLY_SEARCH_HITS` 改为先清空再写入）
- 修复搜索高亮多字节字符边界：后端返回字节偏移，前端用 `TextEncoder`/`TextDecoder` 按字节区间还原字符串再包裹 `<mark>`，中英文混合安全
- 修复 CI clippy lint `unnecessary_min_or_max`（rust 1.97.0）

## 下载

| 平台 | 文件 |
|---|---|
| Windows x64 | RuT0DataKit_1.1.1_x64-setup.exe / RuT0DataKit_1.1.1_x64_en-US.msi |
| macOS arm64 | RuT0DataKit_1.1.1_aarch64.dmg / RuT0DataKit_aarch64.app.tar.gz |
| Linux x64 | RuT0DataKit_1.1.1_amd64.deb / RuT0DataKit-1.1.1-1.x86_64.rpm / RuT0DataKit_1.1.1_amd64.AppImage |

每个安装包均附带 .sig 签名文件，供 updater 验签。

## 升级

v1.1.0 用户可直接升级：DB schema 从 `SCHEMA_VERSION=2` 增量迁移到 `3`（`operations` 表加 `before_snapshot_json` 列 + `idx_cells_sheet_col` 索引，`PRAGMA table_info` 检查列存在再 ALTER + `IF NOT EXISTS` 幂等，不触发备份重建），历史数据完整保留。启动后历史 v1.1.0 数据无需转换即可使用新搜索与撤销功能（历史 operations 行的 `before_snapshot_json` 为 NULL，表示不可撤销，仅新产生的 mask/replace 操作可撤销）。

完整技术文档见 docs/versions/1.1.1/。
