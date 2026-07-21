```yaml
task_id: T6-1
goal: |
  修复「数据预处理导入后搜索界面搜不到数据」的根因并打通全流程：让 ExportView
  切换为消费 state.records（与 SearchView/MaskView/ValidateView 一致），移除对
  state.filePath 的硬依赖；并补一条端到端测试断言「preprocess → search 命中 ≥1」。

in_scope:
  - frontend/src/components/ExportView.jsx          # 数据源 Select 改读 records；filePath 退化为可选
  - frontend/src/state.js                           # SET_FILE 不再清空 records（导入新文件路径与 records 解耦，避免误清空）
  - frontend/src/components/SearchView.jsx           # 仅补注释/微调，不改逻辑（已正确读 records）
  - crates/core/tests/e2e.rs                        # 新增 e2e：preprocess→search 命中≥1（csv 源）
  - frontend/src/tauri.js                           # 若需要新增 export helper（如 exportRecordsFromRecords），仅此处

out_of_scope:
  - 不移除 FileToolbar（由 T6-2 负责）
  - 不改 ToolsView/RegexTool/SqlParseTool（由 T6-3/T6-4/T6-5 负责）
  - 不改后端 search_records / preprocess_file Rust 命令签名
  - 不外发数据：不引入网络请求、不调用 reqwest/hyper/fetch，规则与样本不上传

acceptance_criteria:
  - ExportView 数据源 Select「原始数据」可在无 state.filePath 时工作（仅 records 也能导出）
  - ExportView「原始数据」源消费 state.records.rows（不再依赖 SET_FILE 路径）
  - SET_FILE 被 FileToolbar 触发时不清空已有 records（避免误清空用户已导入数据）
  - 新增 e2e 测试 preprocess_to_search_finds_hits：用 csv fixture 调 read_records → search_records(Keyword "Alice") → 断言 hits ≥1
  - 既有 e2e search_big_file / preprocess_6_sources / sql_parse_tool_full 全绿不破
  - cargo test --workspace 全绿；cd frontend && npm run build 通过

verification_commands:
  - cargo test --workspace
  - cargo test -p ruT0-data-kit-core --test e2e
  - cd frontend && npm run build

files_likely_to_change:
  - frontend/src/components/ExportView.jsx
  - frontend/src/state.js
  - crates/core/tests/e2e.rs

risks:
  - ExportView 当前 sourceOptions 用 `disabled: !filePath` 禁用「原始数据」，迁移时要改为 `disabled: !hasRecords`，否则用户感知不到能导出。
  - computeExportArgs 用 filePath 作为后端 export_records_csv 的 inputPath；若 records-only 路径要导出，需要后端新增 records-based 导出命令或前端先写临时文件。本任务范围保守：保留 filePath 路径作主路径，但不再因 filePath 缺失就禁用 UI；若完全 records-only 导出需要新命令，列入 T6-1 out_of_scope 留 T6-6 收尾讨论。
  - SET_FILE 当前会清空 maskedRows/validateResult 但不清 records，迁移时不要把 records 也清空（用户痛点就是 records 被清空）。

depends_on: []
status: planned
```

## 上下文（coder 阅读用）

**用户原话**：「现在数据流没有打通，严格检查是否遵守从数据预处理到之后的工作流，我数据预处理导入后，在搜索界面无法搜到到」。

**已排查结论**（主会话 Phase 1 调查，coder 不必重复）：

- `PreprocessView.jsx` 正确 dispatch `SET_RECORDS`，写入 `state.records = { headers, rows, rowCount, sourceType }`。
- `SearchView.jsx` 正确读 `state.records`，调 `searchRecords(headers, rows, queryJson)`。前端契约正确。
- 后端 `search_records` 命令签名 `(headers, rows, query_json)` 正确，core `search::search_records` 逻辑正确，e2e `search_big_file` 已证明后端可工作。
- **真正的断点不在 SearchView**，而在 **ExportView**：它读 `state.filePath / state.headers / state.rows`（来自 `SET_FILE`），而不是 `state.records`；当用户从 PreprocessView 跳过来时，`filePath` 为空，ExportView 把「原始数据」option 禁用，给用户「数据没打通」的观感。此外 FileToolbar（顶部常驻导入按钮）走 `SET_FILE` 旧路径，会在导入新文件时重置 maskedRows/validateResult，但不重置 records——这个不一致也是观感问题。
- 因此本任务 scope 是：让 ExportView 也走 records 路径（与 Mask/Validate/Search 对齐），并加 e2e 断言「preprocess→search 命中」防回归。

**安全约束（来自 docs/00 §6，必须遵守）**：不外发数据：全本地处理；规则与样本不上传。
