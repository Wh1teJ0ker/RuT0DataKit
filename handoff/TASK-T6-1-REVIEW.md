# TASK-T6-1 REVIEW

reviewer: reviewer subagent
task_id: T6-1
commit_reviewed: ea8a23a
date: 2026-07-21

## §1 Verdict

**pass_with_notes**

goal 满足、acceptance_criteria 全部满足、scope 无越界、安全约束保持、
验证独立复跑全绿。仅有一处 minor 文档/注释一致性问题，不阻塞验收。

## §2 Acceptance criteria checklist

| # | criterion | result | evidence |
|---|---|---|---|
| 1 | ExportView「原始数据」源可在无 state.filePath 时工作（UI 不再被禁用） | ✓ | `frontend/src/components/ExportView.jsx:66` `disabled: !hasRecords`；`ExportView.jsx:47-59` 新增 `records/hasRecords/recHeaders/recRows`，不再读 `state.headers/state.rows` |
| 2 | ExportView「原始数据」源消费 state.records.rows | ✓ | `ExportView.jsx:109` `out = recRows`（= `records.rows`）；useMemo dep 改 `recRows` (`:140`)；`ExportView.jsx:156` `c = recHeaders.indexOf(h)` dep 改 `recHeaders` (`:161`) |
| 3 | SET_FILE 不清空 records | ✓ | `frontend/src/state.js:101-129`，SET_FILE 的返回对象未出现 `records:` 赋值，仅清空 maskedRows/maskedSummary/validateResult/maskOverrides/validateOverrides 等；`:123-128` 注释显式说明保留 records |
| 4 | 新增 e2e `preprocess_to_search_finds_hits`：csv fixture → read_records → search_records(Keyword) 断言 hits≥1 | ✓ | `crates/core/tests/e2e.rs:2505-2575`：`read_records(common::csv_path())` → `search_records(&records, &SearchQuery::Keyword { terms: vec!["张三"], mode: SearchMode::Or })` → `assert!(!res.hits.is_empty())`，并断言每条 hit 的 cell value 包含「张三」 |
| 5 | 既有 e2e search_big_file / preprocess_6_sources / sql_parse_tool_full 仍绿 | ✓ | `crates/core/tests/e2e.rs:2051 / :2208 / :2282` 三个测试函数仍在；本次 `cargo test -p ruT0-data-kit-core --test e2e` 全绿 35 passed |
| 6 | cargo test --workspace 全绿；npm build 通过 | ✓ | 见 §5 实跑输出：workspace 6 个 binary 共 405 passed / 0 failed / 2 ignored；npm run build `✓ built in 2.19s` |

## §3 Scope adherence

in_scope（HANDOFF 列出）：
- frontend/src/components/ExportView.jsx  ✓ 已改
- frontend/src/state.js                   ✓ 已改（仅 +6 行注释，无逻辑改动）
- frontend/src/components/SearchView.jsx  — 未触（HANDOFF 注「仅补注释/微调，不改逻辑」；coder 在 REPORT 中说明 SearchView 已正确故未触，避免无谓 diff，合理）
- crates/core/tests/e2e.rs                ✓ 新增 test #33
- frontend/src/tauri.js                   — 未触（HANDOFF「若需要」最终未需要，合理）

实际改动文件（`git show --stat ea8a23a`）：
- crates/core/tests/e2e.rs               +71
- frontend/src/components/ExportView.jsx +33 / -7
- frontend/src/state.js                  +6

**无越界**。后端 `search_records` / `preprocess_file` 签名未改；FileToolbar 未移除（属 T6-2）；ToolsView/RegexTool/SqlParseTool 未触（T6-3/T6-4/T6-5）。

## §4 Security constraint audit

约束原文：`不外发数据：全本地处理；规则与样本不上传`。

- `git show ea8a23a` 全 diff 无 `reqwest` / `hyper` / `fetch(` / `http://` / `XMLHttpRequest` / `axios` 等网络调用。
- ExportView.jsx 改动仅本地 state 解构与 useMemo 计算；handleExport 调用本地 tauri 命令（`exportRecordsCsv/Xlsx/Json`），路径参数为本地 `filePath` 与 `outPath`（saveDialog 返回）。
- e2e 测试用本地 fixture `common::csv_path()`（sample_mask.csv）。
- state.js 改动仅注释。

**安全约束保持。**

## §5 Verification re-run output（reviewer 独立实跑）

### cargo test --workspace

```
test result: ok. 306 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 35 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 6.87s   ← e2e binary
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finishing in 0.26s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finishing in 0.09s
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finishing in 0.11s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   (doc-tests)
```

合计 405 passed / 0 failed / 2 ignored（与 coder REPORT 一致）。

### cd frontend && npm run build

```
dist/index.html                    0.31 kB │ gzip:   0.24 kB
dist/assets/index-BUfzJGYP.js  1,138.32 kB │ gzip: 355.11 kB
(!) Some chunks are larger than 500 kB after minification.   ← 既有 chunk-size 告警，非本任务引入
✓ built in 2.19s
```

## §6 Defects found

### defect-1 (minor)
- severity: minor
- file: `frontend/src/components/ExportView.jsx:237-240`
- issue: `handleExport` 仍以 `if (!filePath) { message.warning("请先导入文件"); return; }` 门控导出按钮。当用户经 PreprocessView 走 SET_RECORDS 导入后（filePath=null 但 records 已就绪），UI 上「原始数据」源可选、预览可见，但点导出按钮会被这条 warning 拦下，用户体验上仍像「数据没打通」。
- impact: 不阻断本任务 goal（HANDOFF `risks` 第 2 条已显式把「records-only 导出需新后端命令」列为 out_of_scope，留 T6-6），但用户在 records-only 路径下点导出会收到「请先导入文件」误导性提示。
- fix: 可不改代码（属 T6-6 范围）。建议在 ExportView.jsx:237 上方加一行注释，把「此门控依赖 T6-6 records-based 导出命令落地后才能解除」写明，避免 T6-6 漏掉此处。coder 在 `:53-58` 已有类似注释，但 `:237` 的 warning 文案「请先导入文件」本身对 records-only 用户有误导，T6-6 时应一并改为「records-only 导出待支持」之类的文案。
- 注：HANDOFF `risks` 段已预留此限制，coder 在 REPORT `scope_deviation` 与代码注释中均明示，**不算越界、不算 blocker**。

### defect-2 (minor)
- severity: minor
- file: `frontend/src/state.js:102`（SET_FILE 仍写入 `headers/rows/rowCount`）
- issue: SET_FILE 仍把 `action` 的 `headers/rows/rowCount` 写入 `state.headers/state.rows/state.rowCount`（旧 SET_FILE 字段）。ExportView 已迁出这些字段，但 state 上仍保留，其它视图若仍有读 `state.headers/state.rows` 的残留会读到 SET_FILE 写入的值（而非 records）。本任务 scope 未要求审计其它视图，但这是一致性残留。
- impact: 本任务范围内无实际 bug（ExportView/SearchView/MaskView/ValidateView 均已读 records）。仅是 state schema 上的死字段，T6-2/T6-6 收尾时可清理。
- fix: 不属本任务 scope，留 T6-2（FileToolbar 迁移）或 T6-6。本任务不要求改。

无 blocker / major 缺陷。

## §7 Recommendation

**verified_complete**

理由：
- goal「ExportView 切 records + SET_FILE 不清 records + e2e preprocess→search 命中」全部落地。
- acceptance_criteria 6 条全部 ✓，每条有 path:line 证据。
- scope 严格在 in_scope 内，无越界。
- 安全约束保持，无网络调用。
- reviewer 独立复跑 `cargo test --workspace` 与 `npm run build` 全绿，与 coder REPORT 数字一致。
- 两处 minor 均已被 HANDOFF `risks` 段预留或属后续任务范围，不阻塞本任务验收。

最终 verified_complete 判定权交主会话。
