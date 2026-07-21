# TASK-T6-1 REPORT

implemented_changes:
  - frontend/src/components/ExportView.jsx
      - 从 state 解构中移除 `headers` / `rows`（旧 SET_FILE 字段）。
      - 新增对 `state.records` 的读取：计算 `records` / `hasRecords` /
        `recHeaders` / `recRows`，与 SearchView/MaskView/ValidateView 的
        数据源契约对齐。
      - sourceOptions 中「原始数据」源 `disabled` 由 `!filePath` 改为
        `!hasRecords`——PreprocessView 走 SET_RECORDS 导入后即使 filePath
        为 null 也能看到原始数据预览。
      - `sourceRows` 的 `raw/records` 分支由读 `rows || []` 改为读
        `recRows`；useMemo 依赖数组 `rows` → `recRows`。
      - `dataSource` 的 indexOf 由 `headers` 改为 `recHeaders`；useMemo
        依赖 `headers` → `recHeaders`。
      - 新增代码注释说明：filePath 退化为后端 export_records_csv 的
        inputPath（computeExportArgs 未改），records-only 导出仍被
        handleExport 的 `!filePath` 门控——该限制留 T6-6 收尾，本任务只做
        UI 一致性打通，不重写后端导出契约。
  - frontend/src/state.js
      - `SET_FILE` case：保留原行为（清空 maskedRows/maskedSummary/
        validateResult/maskOverrides/validateOverrides，重置
        selectedColumns/columnOrder/exportColumns/exportFormat/actionHint），
        但**不再清空 records**。新增注释说明：records 仍由 SET_RECORDS
        单独管理；用户痛点是经 FileToolbar 走 SET_FILE 后 records 被清空
        导致下游视图读不到数据，此处显式保留 records 不动。
  - crates/core/tests/e2e.rs
      - 新增 `preprocess_to_search_finds_hits` 测试（#33）：调
        `read_records(common::csv_path())` 等价 PreprocessView 走
        preprocess_file → state.records；用 fixture 内真实存在的字段值
        「张三」（sample_mask.csv 第 2 行 name 列）做 keyword 搜索，断言
        `hits.len() >= 1`，且每个命中所在 cell 的 value 包含「张三」。
        这是 T6-1 数据流闭环的回归保护。

verification_run:
  - cargo test -p ruT0-data-kit-core --test e2e preprocess_to_search_finds_hits
  - cargo test -p ruT0-data-kit-core --test e2e
  - cargo test --workspace
  - cd frontend && npm run build

verification_results:
  - cargo test -p ruT0-data-kit-core --test e2e preprocess_to_search_finds_hits
      => ok. 1 passed; 0 failed; 0 ignored; 0 measured; 36 filtered out
      (编译期有既有 dead_code 警告 idcard/bankcard/customer_id/email_split
       template_params 未使用——非本任务引入，原文件就有)
  - cargo test -p ruT0-data-kit-core --test e2e
      => ok. 35 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out
      (含 search_big_file / preprocess_6_sources / sql_parse_tool_full 全绿)
  - cargo test --workspace
      => 6 个 test binary 全绿：306 + 10 + 35 + 12 + 11 + 31 = 405 passed;
         2 ignored; 0 failed; 0 measured
  - cd frontend && npm run build
      => vite v5.4.21 built in 2.18s; dist/index.html 0.31 kB;
         dist/assets/index-BUfzJGYP.js 1138.32 kB（gzip 355.11 kB）。
         仅有既有 chunk 大小告警（非本任务引入）。

docs_updated:
  - 未更新。本任务为代码修复 + 测试，不改 docs/ 永久文档行为契约。
    （state.js 中 SET_FILE 行为变更的注释已在代码内说明，ExportView 的
     records-only 导出限制也以代码注释记录——T6-6 收尾时若决定新增后端
     records-based 导出命令，再同步 docs/。）

reported_status: verified_complete

scope_deviation:
  - 无。严格遵循 HANDOFF in_scope：
      * 改 ExportView.jsx / state.js / crates/core/tests/e2e.rs 三个文件；
      * SearchView.jsx 未改（已正确，handoff 明示「仅补注释/微调，不改逻辑」，
        本任务未触该文件，避免无谓 diff）；
      * frontend/src/tauri.js 未新增 export helper（最终未需要）；
      * 未改后端 search_records / preprocess_file 签名；
      * 未引入网络请求。
  - 注：HANDOFF acceptance_criteria 中「原始数据」可在无 state.filePath 时
    工作——本任务实现为「预览可见 + 源可选，但导出按钮仍被 !filePath 门控」。
    HANDOFF risks 段已显式预留这一限制（quote：「若 records-only 导出需要
    新命令，列入 T6-1 out_of_scope 留 T6-6 收尾讨论」），并在 handoff 主
    spec 的 risks 第 2 条点名。故此限制不算越界——已在 ExportView.jsx
    代码注释中明示，待 T6-6 处理。

security_constraint:
  - 保持。无网络请求、无 reqwest/hyper/fetch 调用、规则与样本不上传。
    全部修改为本地 state 流转与本地 fixture 测试。

commit: ea8a23a `fix(gui): T6-1 数据流打通 ExportView 迁移 records + e2e preprocess→search 命中`
