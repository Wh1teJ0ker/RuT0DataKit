# TASK-T13-REPORT — 前端 state 模块化 + Context

```yaml
implemented_changes:
  - 新建 frontend/src/state/constants.js：ACTION 常量 + initialState（含尾部冗余 sheets/activeSheetId 重赋值，按 handoff 要求保留不清理）。
  - 新建 frontend/src/state/factory.js：createEmptySheet（模块私有 sheetSeq 闭包计数器）+ createSheetFromImport，返回结构字节级不变。
  - 新建 frontend/src/state/reducer.js：reducer + patchActiveSheet，所有分支逻辑、action 语义不变；仅 import 调整为从 ./constants 与 ./factory 取。
  - 新建 frontend/src/state/AppContext.jsx：AppContext + AppProvider + useAppContext hook。Provider value 用 useMemo 包裹（state + dispatch + 14 个 useCallback dispatcher，依赖数组完整稳定），避免整树重渲染。useAppContext 在无 Provider 时抛错（Rules of Hooks 兜底）。
    - 注：因含 JSX（<AppContext.Provider>），文件必须用 .jsx 扩展名，否则 vite build 的 import-analysis 解析失败（首次构建已踩坑并修正）。
  - frontend/src/state.js 变薄 barrel：重导出 ACTION / initialState / createEmptySheet / createSheetFromImport / reducer / patchActiveSheet / AppContext / AppProvider / useAppContext；并保留 useAppState 别名（= useAppContext）以兼容 settings/cards/TsharkPathCard.jsx 现有 import，不改其内部逻辑（out_of_scope）。
  - frontend/src/App.jsx：拆为 App（仅 <AppProvider><AppShell/></AppProvider>）+ AppShell（原布局 + handleImport/handleSetPage）。顶层包裹 AppProvider，state/dispatch/dispatcher 经 Context 下发。AppShell 仅消费 useAppContext；Workbench/TopToolbar/AiPanel 不再接收 state/dispatch/setter props，仅保留真正跨组件回调（onImport、setPage=handleSetPage、onSettings）。
  - frontend/src/components/layout/TopToolbar.jsx：改 useAppContext 取 activeCapability/setActiveCapability + 从 state.sheets 推导 activeSheet；移除 activeCapability/setActiveCapability/activeSheet props；保留 onImport 回调。ExportModal 接口不变。
  - frontend/src/components/SheetTabs.jsx：改 useAppContext 取 sheets/activeSheetId/addSheet/setActiveSheet/closeSheet/renameSheet；移除全部 state/dispatcher props，无 props 接口。
  - frontend/src/components/Workbench.jsx：改 useAppContext 取 sheets/activeSheetId；仅保留 setPage prop（App 注入的翻页拉数据流回调）。SheetTabs 不再传 props；DataTable 仍传 sheet + onSetPage。
  - frontend/src/components/DataTable.jsx：改 useAppContext 取 setSelection/reorderColumns/setColumnVisibility；移除对应 on* props；保留 sheet（Workbench 注入）+ onSetPage（跨组件拉数据回调）。handleSelect/handleSelectAll/handleDragEnd 内部调用从 onSetSelection→setSelection、onReorderColumns→reorderColumns、onSetColumnVisibility→setColumnVisibility。
  - frontend/src/components/AiPanel.jsx：改 useAppContext 取 state.aiPanel.visible + setAiPanelVisible；移除 visible/setVisible props；保留 onSettings 跨组件回调。
verification_run:
  - cd frontend && pnpm install
  - cd frontend && pnpm build
  - cd frontend && pnpm lint  （项目 package.json 无 lint script；系统 PATH 中的 lint 是 Android lint 工具，需 JDK17，本机仅 JDK1.8，与前端无关 → 未运行）
verification_results:
  - pnpm install：通过。Lockfile is up to date, Already up to date。仅 esbuild build script 被忽略（与 T13 无关）。
  - pnpm build：通过。首次失败（AppContext.js 含 JSX 但用 .js 扩展名，vite import-analysis 报 invalid JS syntax），改为 AppContext.jsx 后通过：✓ 3078 modules transformed，✓ built in 2.28s。无 React key / hook 顺序警告。chunk size > 500kB 警告为既有现状（与 T13 无关）。
  - pnpm lint：未运行。frontend/package.json scripts 只有 dev/build/preview，无 lint 脚本；PATH 中 lint 为 Android lint（JDK 不匹配），非前端 ESLint。已在报告中如实标注。
docs_updated:
  - 无。本任务为纯结构调整，不改产品行为，docs/ 下无对应需更新文档。
commit_summary: "e0f4e3f refactor(frontend): T13 split state monolith + introduce AppContext"
reported_status: verified_complete
scope_deviation:
  - 文件命名微调：state/AppContext.js → state/AppContext.jsx（因含 JSX，vite 强制要求 .jsx/.tsx 扩展名）。state.js barrel 中对应 import 路径同步用 ./state/AppContext.jsx。其余拆分文件（constants/factory/reducer）均为纯 JS，保持 .js。此为实现必要的技术约束适配，不改变 handoff 要求的模块职责划分与 API。未触碰 ExportModal/panels/settings 内部逻辑、reducer 分支、action 语义、Sheet 工厂返回结构、tshark 字段。
```

## 备注

- useAppState 别名保留：settings/cards/TsharkPathCard.jsx 现有 `import { useAppState, ACTION } from "../../../state"` 不在 T13 in_scope（不改其内部逻辑），通过 barrel 别名 `useAppContext as useAppState` 保持其工作，无需改动该文件。语义上等价（均需 AppProvider 包裹，App 顶层已包裹）。
- Provider value 的 useMemo 依赖数组包含 state + dispatch + 14 个 dispatcher；dispatch 来自 useReducer 引用稳定，14 个 dispatcher 均为 useCallback([]) 稳定，故 value 仅在 state 变化时重建 —— 符合 handoff risks 要求。
- AppShell 内 handleImport/handleSetPage 仍直接用 dispatch({type: ACTION.IMPORT_SUCCESS/SET_SHEET_DATA})，与原 App 行为一致（未改成 importSuccess/setSheetData dispatcher），保持字节级行为不变。
- handoff 提到的「stale TODOs / dead statusHighlights / dead importSuccess/setSheetData / redundant initialState reassignment」均按 handoff 指示只搬运不清理，留给 T14 或后续。
```
