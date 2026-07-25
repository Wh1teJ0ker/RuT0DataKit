# v0.6.5 Release QA 审计报告

> 字段批量重命名 + 数据流数据源指示版本发布前全局 QA 审计。依据 `orchestrator-workflow` 技能「发布前 QA 门禁」要求，覆盖需求覆盖、端到端流程、构建与测试、代码质量、文档一致性五维度。
>
> 本版本范围：① PreprocessView 预览表头可编辑（单字段 ✏ + 批量重命名 Modal）；② 真实改写 `records.headers` 并级联 re-key 所有 header-keyed 状态切片；③ 预处理流与提取流加数据源指示 Tag，明确两流独立、不共享 records。

## §0 审计结论

`qa_passed` — 2 项任务（T20-1/T20-2）全部落地。字段批量重命名、数据源指示两处用户诉求全部满足，cargo build 0 error，cargo test 全绿（含 `search_big_file` 本轮通过），npm build 绿（3009 modules），4 处 manifest 版本号同步 0.6.5，docs 一致，grep 验证三处关键点到位，无阻塞项。

## §1 任务对齐

| # | 任务 | 优先级 | 落地证据 | 状态 |
|---|------|--------|----------|------|
| 1 | T20-1 字段重命名 state 级联 + PreprocessView 表头编辑 UI + 数据源指示 | P0 | state.js `COLUMN_RENAME_SET` action + 级联 re-key；PreprocessView ✏ 单字段重命名 + 批量重命名 Modal + 数据源 Tag；ExtractView 数据源 Tag；docs 补充交互规范 | ✅ |
| 2 | T20-2 版本号 + docs | P1 | 4 manifest 0.6.5 + 更新日志 + QA 报告 + 04-版本标准里程碑行 | ✅ |

## §2 代码审计

### §2.1 T20-1 COLUMN_RENAME_SET state 级联（state.js）

| 检查项 | 结果 |
|--------|------|
| ACTION 对象新增 `COLUMN_RENAME_SET: "SET_COLUMN_RENAME"` | ✅（state.js:22） |
| `fileDomain` 新增 `case ACTION.COLUMN_RENAME_SET` | ✅（state.js:290） |
| 载荷 `renames: [{ oldName, newName }]` 支持批量 | ✅（state.js:291） |
| 构建 `Map<oldName, newName>`，过滤空值与同名 | ✅（state.js:296 `filter(([o,n]) => o && n && o !== n)`） |
| `map.size === 0` 时返回原 state（无操作） | ✅（state.js:297） |
| 真实改写 `state.records.headers`（`map(remapKey)`） | ✅（state.js:308） |
| 级联 re-key `selectedColumns` / `columnOrder` / `exportColumns`（数组 remap） | ✅（state.js:313-315 `remapArr`） |
| 级联 re-key `maskOverrides` / `validateOverrides`（key + value.field remap） | ✅（state.js:300-307 `remapOverrides`） |
| 清空 `maskedRows` / `maskedSummary` / `validateResult` | ✅（state.js:318-320） |
| 全局 `rules.validators/maskers[].field` 不改 | ✅（reducer 不触碰 rules 字段） |
| `!state.records` 守卫（无 records 时返回原 state） | ✅（state.js:293） |

### §2.2 T20-1 PreprocessView 表头可编辑 + 批量重命名

| 检查项 | 结果 |
|--------|------|
| 预览列 title 渲染为 `<Space><span>{h}</span><Button icon={<EditOutlined/>} onClick={handleRenameOne(h)} /></Space>` | ✅（PreprocessView.jsx:150-161） |
| `handleRenameOne`：`Modal.confirm` + `Input`（defaultValue=oldName），确认后 dispatch `SET_COLUMN_RENAME` | ✅（PreprocessView.jsx:76-109） |
| `onOk` 守卫：`!v \|\| v === oldName` 时不 dispatch | ✅（PreprocessView.jsx:100-101） |
| 成功 message 提示「脱敏/校验结果已清空，请重新运行」 | ✅（PreprocessView.jsx:106） |
| 批量重命名按钮在预览 Card `extra`（`DiffOutlined`，无 records disabled） | ✅（PreprocessView.jsx:247-255） |
| `openBatchRename`：从 `records.headers` 初始化 `renameMap` | ✅（PreprocessView.jsx:113-119） |
| `submitBatchRename`：filter 改了的字段 → dispatch `SET_COLUMN_RENAME { renames }` | ✅（PreprocessView.jsx:121-133） |
| 批量 Modal JSX：左 disabled 原名 + 右可编辑新名 Input | ✅（PreprocessView.jsx:301-335） |
| 数据源 Tag「数据源：预处理流」在导入 Card | ✅（PreprocessView.jsx:208-211） |

### §2.3 T20-1 ExtractView 数据源指示

| 检查项 | 结果 |
|--------|------|
| 输入 Card 顶部加 `Tag color="geekblue"` 文案「数据源：提取流（与预处理流独立，不共享 records）」 | ✅（ExtractView.jsx:235-238） |

### §2.4 安全合规

- 重命名纯前端 state 操作（`dispatch SET_COLUMN_RENAME` → reducer 改 `state.records.headers`），不涉及网络/文件 IO。与 `docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」约束一致。
- 数据源指示 Tag 纯 UI 标注，无逻辑。
- 无新增网络调用 / 加密 / 外发逻辑。

### §2.5 scope 合规

- T20-1 改 `frontend/src/state.js` + `frontend/src/components/PreprocessView.jsx` + `frontend/src/components/ExtractView.jsx` + `docs/01-页面与交互说明.md`。
- T20-2 改 4 manifest + docs。
- 未越界触及 core / mask pipeline / validate pipeline / 加密模块 / RulesView / RegexTool / SearchView / ExportView（ExportView 自动消费 remap 后的 `exportColumns`，无需改动）。

## §3 测试审计

```
$ cargo test
test result: ok. 395 passed; 0 failed; 3 ignored   (lib unittests)
test result: ok. 10 passed; 0 failed; 0 ignored     (integration log_test)
test result: ok. 34 passed; 0 failed; 2 ignored     (integration log_scan_test)
test result: ok. 12 passed; 0 failed; 0 ignored     (integration blind_aggregator_test)
test result: ok. 11 passed; 0 failed; 0 ignored     (integration logsign_test)
test result: ok. 31 passed; 0 failed; 0 ignored     (integration e2e, search_big_file 通过)
test result: ok. 0 passed; 0 failed; 0 ignored      (Doc-tests)
```

合计 **482 passed, 0 failed, 5 ignored**。

本版本无 Rust 业务逻辑变更（仅前端 state + UI + docs），测试全绿。`search_big_file` 本轮通过（6.98s 段内含编译，实际运行未超阈值）。

## §4 构建审计

| 构建目标 | 命令 | 结果 |
|----------|------|------|
| core lib + src-tauri | `cargo build` | 0 error，1 warning（历史遗留 crate 名 `ruT0_data_kit_core should have a snake case name`，与 v0.6.4 一致） |
| frontend | `npm --prefix frontend run build` | vite build 3009 modules（与 v0.6.4 一致），0 error，2.27s |

## §5 文档一致性

| 检查项 | 结果 |
|--------|------|
| 4 处 manifest 版本号 | 全部 0.6.5 ✅ |
| `docs/04-版本标准.md` 里程碑行 | v0.6.5 状态 `release_complete` ✅ |
| `docs/versions/0.6.5/更新日志.md` | 回填完毕 ✅ |
| QA 报告 | 本文件 ✅ |
| `handoff/TASK-BOARD.md` | v0.6.5 DAG 回填 ✅ |
| 安全约束 §6 | 「不外发数据：全本地处理」保留，v0.6.5 不变 ✅ |
| `docs/01-页面与交互说明.md` | PreprocessView 表头可编辑/批量重命名/级联影响 + ExtractView 数据源 Tag ✅ |

## §6 grep 关键符号验证

```
=== grep 1: COLUMN_RENAME_SET action + 级联 (state.js) ===
22:  COLUMN_RENAME_SET: "SET_COLUMN_RENAME",
290:    case ACTION.COLUMN_RENAME_SET: {
296:      const map = new Map(renames.map((r) => [r.oldName, r.newName]).filter(([o, n]) => o && n && o !== n));
308:      const headers = state.records.headers.map(remapKey);
313:        selectedColumns: remapArr(state.selectedColumns),
314:        columnOrder: remapArr(state.columnOrder),
315:        exportColumns: remapArr(state.exportColumns),
316:        maskOverrides: remapOverrides(state.maskOverrides),
317:        validateOverrides: remapOverrides(state.validateOverrides),
318:        maskedRows: null,
319:        maskedSummary: null,
320:        validateResult: null,

=== grep 2: PreprocessView 表头可编辑 + 批量重命名 + 数据源 Tag ===
20:  EditOutlined,
21:  DiffOutlined,
103:          type: "SET_COLUMN_RENAME",
130:    dispatch({ type: "SET_COLUMN_RENAME", renames });
156:            icon={<EditOutlined />}
210:              数据源：预处理流
249:                icon={<DiffOutlined />

=== grep 3: ExtractView 数据源 Tag ===
235:        {/* v0.6.5 T20-1：数据源指示——提取流（与预处理流独立，不共享 records） */}
238:            数据源：提取流（与预处理流独立，不共享 records）

=== grep 4: 4 manifest 版本号 0.6.5 ===
Cargo.toml:8:version = "0.6.5"
src-tauri/Cargo.toml:3:version = "0.6.5"
src-tauri/tauri.conf.json:4:  "version": "0.6.5",
frontend/package.json:4:  "version": "0.6.5",
```

四处关键验证（state 级联 / PreprocessView UI / ExtractView Tag / 版本号）全部到位。

## §7 端到端流程验证

用户预期流程：

1. **单字段重命名**：PreprocessView → 导入文件 → 预览表头 `name` 旁点 ✏ → Modal 输入「名字」→ 确认 → 表头变为「名字」，脱敏/校验结果清空（需重新运行） ✅
2. **批量重命名**：预览 Card 右上「批量重命名」→ Modal 列出所有 headers → 改 `name`→名字 / `phone`→电话 → 「应用重命名」→ 多个表头一次性更新 ✅
3. **级联 re-key**：重命名后切到 ExportView → 导出列表显示新表头（非旧表头） ✅
4. **全局规则不破**：重命名后切到 MaskView → 若某 preset 的 `field` 仍是旧名则不匹配（需重新应用 preset） ✅
5. **数据源指示**：PreprocessView 导入区显示「数据源：预处理流」Tag；ExtractView 输入区显示「数据源：提取流（与预处理流独立，不共享 records）」Tag ✅

## §8 阻塞项

无。全部任务落地，构建/test/grep/docs 全绿。

## §9 结论

`qa_passed` — 可进入 `release_complete`。
