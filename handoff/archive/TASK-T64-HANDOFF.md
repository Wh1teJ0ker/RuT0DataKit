# T64 — 陈旧响应隔离（generation token）

## 背景

`App.jsx#handleSetPage` 与 `DataTable.jsx#handleSearch/handleSearchPageChange`
均为「发起异步请求 → 等待响应 → dispatch reducer」模式。当用户快速连续操作
（连点分页、连点搜索翻页）时，旧请求的响应可能晚于新请求返回，导致 reducer
写入陈旧数据，UI 闪现错误页/错误搜索结果。

## 方案

引入 generation token（单调递增整数），每个发起点维护一份：

1. **App.jsx 翻页/导入**：`pageReqGenRef = useRef({})`，按 sheetId 维护独立计数器。
   - `nextGen(sheetId)`：自增并返回新 token。
   - `isStale(sheetId, gen)`：响应返回时检查 `pageReqGenRef.current[sheetId] !== gen`。
   - 应用于 `handleImport`（首页拉取）和 `handleSetPage`（翻页拉取）。
   - 按 sheetId 分桶：切换 Sheet 后旧 Sheet 的陈旧响应不影响新 Sheet。

2. **DataTable.jsx 搜索**：`searchGenRef = useRef(0)`，单一计数器（搜索只针对当前 Sheet）。
   - `handleSearch` 与 `handleSearchPageChange` 共用，`++searchGenRef.current`。
   - 响应返回时若 `searchGenRef.current !== gen`，静默丢弃，不 dispatch。
   - `finally` 中仅在非陈旧时 `setSearching(false)`，避免陈旧请求清空 loading 态。

## 验收条件

- [x] `pnpm --dir frontend build` 通过（无 ESLint / 构建错误）。
- [x] 范围合规：仅改 `frontend/src/App.jsx` 与 `frontend/src/components/DataTable.jsx`。
- [x] 不引入新依赖（仅用 React 内置 `useRef` / `useCallback`）。
- [x] 不触碰后端 / reducer / factory / docs。

## 不做的事

- 不引入 AbortController（Tauri IPC invoke 不返回可取消的 Promise）。
- 不在 reducer 层做 generation 校验（在调用方即丢弃，保持 reducer 纯函数语义）。
- 不改 `Workbench.refreshSheetAndUndo`（undo/redo 后刷新单次请求，无并发竞态）。
- 不改 `handleReplace`（替换后刷新当前页，单次请求，且会 clearSearch 退出搜索态）。

## 风险与边界

1. **Sheet 删除后陈旧响应**：`pageReqGenRef.current` 中的 sheetId 条目不会被清理，
   属于内存中的小对象，无实际泄漏风险。
2. **token 溢出**：JS Number 安全整数范围 2^53，单次会话不可能溢出。
3. **首次加载**：`handleImport` 中 `nextGen` 在 IMPORT_SUCCESS dispatch 后调用，
   理论上无竞态（新 Sheet 无并发请求），但保持一致保护模式。

## 涉及文件

- `frontend/src/App.jsx`（+useRef、+nextGen/isStale、handleImport/handleSetPage 改造）
- `frontend/src/components/DataTable.jsx`（+useRef、handleSearch/handleSearchPageChange 改造）
