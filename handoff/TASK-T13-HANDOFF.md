# TASK-T13-HANDOFF — 前端 state 模块化 + Context

```yaml
task_id: T13
goal: |
  把 frontend/src/state.js（314 行 monolith，含 Sheet 工厂 / reducer / action 常量 / 14 个 dispatcher / useReducer hook）
  按职责拆为独立子模块，并引入 React Context 让深层组件直接消费 state / dispatch，消除 prop drilling。
  纯结构调整，不改任何 reducer 分支逻辑、action 语义、组件现有行为。
in_scope:
  - frontend/src/state.js（拆分后可保留为 barrel / index，或迁移到 state/ 目录）
  - frontend/src/state/ 下的新建子模块文件
  - frontend/src/App.jsx（包裹 AppProvider，仅 Context 注入，不改布局逻辑）
  - frontend/src/components/layout/TopToolbar.jsx（改用 useContext，仅替换数据来源）
  - frontend/src/components/SheetTabs.jsx（改用 useContext，仅替换数据来源）
  - frontend/src/components/Workbench.jsx（改用 useContext，仅替换数据来源）
  - frontend/src/components/DataTable.jsx（改用 useContext，仅替换数据来源）
  - frontend/src/components/AiPanel.jsx（改用 useContext，仅替换数据来源）
  - 其他现有直接调用 useAppState() 的组件文件（按需，仅替换数据来源）
out_of_scope:
  - crates/core/（整个不动）
  - src-tauri/（整个不动）
  - frontend/src/tauri.js（不改 IPC 调用）
  - frontend/src/components/ExportModal.jsx（不改其内部逻辑）
  - frontend/src/components/panels/（不改面板内部逻辑）
  - frontend/src/components/settings/（不改设置卡片内部逻辑）
  - reducer 分支逻辑、action 语义、Sheet 工厂返回结构（保持字节级行为不变）
  - tsharkPath / tsharkDetected / tsharkLoading 字段及其 setter（保持现状，不删不改）
  - statusHighlights 字段（保持现状，虽疑似 dead，但不在本任务清理，留给 T14 或后续）
acceptance_criteria:
  - state.js 不再包含 reducer / dispatcher / 工厂的全部实现体，只做 barrel 重导出（或整体迁移到 state/index.js + 子模块）
  - 至少拆出以下独立子模块（建议命名）：
      - state/constants.js（ACTION 常量 + initialState）
      - state/factory.js（createEmptySheet / createSheetFromImport）
      - state/reducer.js（reducer + patchActiveSheet）
      - state/AppContext.js（Context + Provider + useAppContext hook）
  - App.jsx 顶层包裹 <AppProvider>，内部组件树不再需要逐层传递 state / dispatch / setter props
  - TopToolbar / SheetTabs / Workbench / DataTable / AiPanel 改为 useContext(useAppContext) 取数据，
    不再通过 props 接收 state / dispatch / setter（props 接口仅保留真正跨组件的回调，如 onSettings 导航）
  - 现有应用行为完全不变：四区布局、设置页全屏路由、Sheet/Tab、导入流、导出弹窗、tshark 设置均按现状工作
  - pnpm build 通过，无 ESLint 报错，无 React key / hook 顺序警告
verification_commands:
  - cd frontend && pnpm install
  - cd frontend && pnpm build
  - cd frontend && pnpm lint
files_likely_to_change:
  - frontend/src/state.js（变薄或变 index）
  - frontend/src/state/constants.js
  - frontend/src/state/factory.js
  - frontend/src/state/reducer.js
  - frontend/src/state/AppContext.js
  - frontend/src/App.jsx
  - frontend/src/components/layout/TopToolbar.jsx
  - frontend/src/components/SheetTabs.jsx
  - frontend/src/components/Workbench.jsx
  - frontend/src/components/DataTable.jsx
  - frontend/src/components/AiPanel.jsx
risks:
  - Context 重渲染：若 Provider value 不 memo 化，会引发整树重渲染。value 必须用 useMemo 包裹 state + dispatch + 所有 dispatcher，依赖数组稳定。
  - hook 顺序：useAppContext 必须在组件顶层调用，不能在条件分支内，否则违反 Rules of Hooks。
  - 迁移过程中容易出现「部分组件用 Context、部分仍走 props」的混合态 —— 本任务要求一次性全部迁移受影响组件，不留半态。
  - ExportModal / panels / settings 子组件若当前不直接用 useAppState，不要强行注入（保持其 props 接口不变）。
depends_on: []
status: planned
```

## 背景说明

`frontend/src/state.js` 是 314 行 monolith，把 Sheet 工厂、reducer、14 个 dispatcher、useReducer hook 全塞一个文件。组件通过 `useAppState()` 在顶层取 state/dispatch，再逐层 props 下传到 TopToolbar / SheetTabs / Workbench / DataTable / AiPanel，形成 3 级 prop drilling（审计报告指出）。

本任务做两件事：
1. **state 模块化**：按职责拆 constants / factory / reducer / Context 子模块，state.js 变薄 barrel。
2. **Context 注入**：建 AppContext + Provider，受影响组件改用 `useContext(useAppContext)` 直接取数据，消除 prop drilling。

纯结构调整，reducer 分支、action 语义、Sheet 工厂返回结构、组件行为全部不变。

注意：审计报告指出的「stale TODOs」「dead statusHighlights」「dead importSuccess/setSheetData callbacks」「redundant initialState reassignment」等都是已知现状，**本任务只搬运不清理**，清理留给 T14 或后续议题。
