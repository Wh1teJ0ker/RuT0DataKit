```yaml
task_id: T6-2
```

## implemented_changes
- `frontend/src/App.jsx`：删除 `FileToolbar` import；删除 `NO_TOOLBAR_VIEWS` 集合；删除 `{!NO_TOOLBAR_VIEWS.has(view) && <FileToolbar ... />}` 渲染分支；顶部注释更新为「数据导入仅作为 PreprocessView 内置入口存在」。
- `frontend/src/components/FileToolbar.jsx`：删除整个文件。
- `frontend/src/components/ExportView.jsx`：`handleExport` 顶部 gate 从 `!filePath` 改为 `!hasRecords`，文案改为「请先到数据预处理导入文件」；底部 Table `locale.emptyText` 同步改为「请先到数据预处理导入文件并选择源数据」。`computeExportArgs` 仍沿用 filePath 作为后端 inputPath（T6-6 cleanup item，未动）。
- `frontend/src/components/LogView.jsx`：仅注释文案对齐（删除对 FileToolbar 的引用）。
- `frontend/src/state.js`：仅注释文案对齐（SET_FILE case 顶部注释）。
- `docs/02-技术设计文档.md`：§3.1 关键数据流图从「FileToolbar 导入 CSV/XLSX」改为「PreprocessView 内置导入入口」；state 字段示例从 `filePath/headers/rows` 更新为 `records`；补充 T6-2 顶部导入条移除说明。

## verification_run
- `cd frontend && npm run build`
- `grep -rn "FileToolbar" frontend/src/`

## verification_results
- `npm run build`：通过。输出 `✓ 3008 modules transformed` / `✓ built in 2.16s`，仅有 chunk size > 500kB 警告（与 T6-2 无关，既有警告）。
- `grep -rn "FileToolbar" frontend/src/`：0 命中（exit 1，即无匹配）。

## docs_updated
- `docs/02-技术设计文档.md`（§3.1 数据流图与跨视图 state 说明）

## reported_status
implemented_pending_review

## scope_deviation
none

- in_scope 全部覆盖：App.jsx / FileToolbar.jsx 删除 / ExportView.jsx gate 改 records / MaskView.jsx / ValidateView.jsx 已是「请先到数据预处理导入文件」口径（T6-1 已落地，本次仅复核确认无需改动）。
- out_of_scope 严格守住：未改 PreprocessView 导入按钮、未改后端命令、未改 SET_FILE reducer 行为、未引入任何网络请求。
- commit hash：`e79593f`
