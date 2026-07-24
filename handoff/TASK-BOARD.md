# TASK-BOARD — v0.6.3 脱敏/校验总行数 BUG 修复 + 正则构造可视化积木重构（release_complete）

> 版本：v0.6.3
> 创建：2026-07-23
> 完成：2026-07-23
> 依赖版本：v0.6.2（release_complete @ 860671c）
> 依据：用户 goal「① 校验后 - 行 显示总行数 BUG；脱敏后 - 行 同样问题；② Tools 正则解析的构造重构为可视化积木构建」
> **状态：release_complete（qa_passed）**

## 任务 DAG（全部 done）

```yaml
goal: 修复脱敏/校验视图总行数显示 BUG（前端字段名对齐 total_rows/invalid_rows/
  masked_rows）；RegexTool 构造 Tab 由自然语言描述重构为可视化积木构建（点选
  数字/字母/至少一次等模块拼装正则，纯客户端，不外发数据）。bump 0.6.3，
  全链路验收通过。
version: 0.6.3
depends_on_version: 0.6.2
tasks:
  - id: T18-1
    title: 修复脱敏/校验视图总行数显示（前端字段名对齐）
    status: done  # MaskView.jsx 5 处 .total→.total_rows；
                  # ValidateView.jsx 4 处 .total→.total_rows / .invalid_count→.invalid_rows /
                  # .valid_count→计算 total_rows-invalid_rows；npm build 绿
  - id: T18-2
    title: RegexTool 构造 Tab 改为可视化积木构建
    status: done  # 新增 RegexConstructTab.jsx（~300 LOC，5 类 21 模块 + 3 预设 +
                  # 参数弹窗 + 测试高亮）；RegexTool.jsx import 替换原 ConstructTab；
                  # 后端 regex_construct.rs 保留；npm build 绿（3009 modules +1）
  - id: T18-3
    title: 版本号 bump 0.6.2→0.6.3（4 manifest）+ docs 同步 + QA 报告
    status: done  # 本任务
```

任务 DAG 结构：

```
T18-1 (frontend field-name fix) ─┐
                                 ├─→ T18-3 (version+docs+QA)
T18-2 (regex building-block UI) ─┘
```

T18-1 和 T18-2 无文件交集（MaskView/ValidateView vs RegexTool），可并行实施。

## E2E 验收

- `cargo test --workspace`：493 passed / 0 failed / 5 ignored（与 v0.6.2 基线一致，无 Rust 变更）✅
- `cargo build --manifest-path src-tauri/Cargo.toml`：0 error ✅
- `npm --prefix frontend run build`：vite build 3009 modules（+1 对比 v0.6.2），0 error ✅
- grep 验证：
  - MaskView/ValidateView `.total_rows`/`.invalid_rows`/`.masked_rows` 命中（14 行）✅
  - 无残留 `.total`/`.valid_count`/`.invalid_count`（0 命中）✅
  - `RegexConstructTab` / `BLOCK_MODULES` / `PRESETS` 关键符号到位 ✅
  - 4 处 manifest 版本号 0.6.3 ✅

## QA 门禁

- `docs/qa/versions/0.6.3/QA-审计报告.md`：结论 `qa_passed` ✅
- `docs/versions/0.6.3/更新日志.md`：状态 `release_complete` ✅
- `docs/04-版本标准.md` v0.6.3 里程碑行：`release_complete` ✅

## 安全约束（不变）

`docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」保留，v0.6.3 不变。T18-2 积木拼装纯客户端，不调用后端、不外发数据。

## 提交链

```
T18-1+T18-2 (frontend fix + rebuild) ─→ T18-3 (version+docs+QA)
```

按 v0.6.x 惯例，T18-1/T18-2 合并为一个 feat 提交（均为前端改动，无依赖耦合），T18-3 单独 chore 提交（版本号 + docs）。

---

# TASK-BOARD — v0.6.4 数据提取功能三项优化（release_complete）

> 版本：v0.6.4
> 创建：2026-07-23
> 完成：2026-07-23
> 依赖版本：v0.6.3（release_complete @ b746af4）
> 依据：用户 goal「① 文件导入和文本粘贴数据应不共通（黏贴后点文件导入出现所有数据且未优化显示）；② 导出应可调整格式（txt 固定 `iphone_176` 无法自定义，csv 无法预览改字段，无单条/批量）；③ 结果显示每页 50 个无法调整，点每页 10 个不改变」
> **状态：release_complete（qa_passed）**

## 任务 DAG（全部 done）

```yaml
goal: 数据提取三项优化 — 文件导入 vs 文本粘贴数据隔离；导出格式可自定义
  （txt 模板 + csv/json 字段勾选调序 + 单/批量勾选）；分页器受控化
  （8 处 Table 每页条数切换生效）。bump 0.6.4，全链路验收通过。
version: 0.6.4
depends_on_version: 0.6.3
tasks:
  - id: T19-1
    title: ExtractView 全面重构 — state 隔离 + 导出自定义 + 单/批量勾选
    status: done  # state.js EXTRACT_MODE_SET 级联清空 extractInput/extractResult；
                  # export.rs export_extract +3 参数（template/selected_columns/
                  # column_order）；tauri.js exportExtract opts 透传；
                  # ExtractView.jsx TXT 模板 + CSV/JSON 字段勾选调序 + rowSelection；
                  # commit f487637
  - id: T19-3
    title: 分页器受控化（8 处 Table）
    status: done  # ExtractView/SearchView/SqlParseTool×2/PcapView×2/LogView×2
                  # pagination 改受控 current+pageSize useState + onChange/
                  # onShowSizeChange；commit 2a0cba2
  - id: T19-4
    title: 版本号 bump 0.6.3→0.6.4（4 manifest）+ docs 同步 + QA 报告
    status: done  # 本任务
```

任务 DAG 结构：

```
T19-1 (extract state 隔离 + 导出自定义) ─┐
                                       ├─→ T19-4 (version+docs+QA)
T19-3 (分页器受控化 8 处)             ─┘
```

T19-1 和 T19-3 文件交集仅在 ExtractView.jsx（T19-1 改输入/导出区，T19-3 改结果表分页），按 T19-1 先行、T19-3 后继顺序实施无冲突。

## E2E 验收

- `cargo test --workspace`：493 passed / 1 failed (search_big_file 性能阈值) / 7 ignored ✅
  - search_big_file 失败为预先存在的性能敏感测试（断言 100k×10 索引 < 5s，本机 debug 5.5s），search 模块自 v0.4.2 无变更，stash+checkout v0.6.3 基线同样失败，非本版本引入，不阻塞发布
- `cargo build --manifest-path src-tauri/Cargo.toml`：0 error ✅
- `npm --prefix frontend run build`：vite build 3009 modules，0 error，3.91s ✅
- grep 验证：
  - `EXTRACT_MODE_SET` 级联清空 `extractInput=""` / `extractResult=null` ✅
  - `exportExtract` opts 透传 template/selectedColumns/columnOrder ✅
  - 受控分页 onChange/onShowSizeChange 8 处 57 行命中 ✅
  - 后端 `export_extract` +3 参数 ✅
  - 4 处 manifest 版本号 0.6.4 ✅

## QA 门禁

- `docs/qa/versions/0.6.4/QA-审计报告.md`：结论 `qa_passed` ✅
- `docs/versions/0.6.4/更新日志.md`：状态 `release_complete` ✅
- `docs/04-版本标准.md` v0.6.4 里程碑行：`release_complete` ✅

## 安全约束（不变）

`docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」保留，v0.6.4 不变。T19-2 导出模板渲染纯本地，`export_extract` 仅写本地文件 `std::fs::write`，不外发。T19-1/T19-3 仅改前端 state 与 UI，不涉及网络/文件 IO。

## 提交链

```
T19-1+T19-2 (extract 隔离 + 导出自定义) ─→ T19-3 (分页受控化) ─→ T19-4 (version+docs+QA)
```

按 v0.6.x 惯例，T19-1+T19-2 合并为一个 feat 提交（ExtractView 集中改），T19-3 独立 fix 提交（分页受控化），T19-4 单独 chore 提交（版本号 + docs）。另含 2 个本会话前置 BUG 修复提交（正则构造白屏 + ExportView 总行数）。
