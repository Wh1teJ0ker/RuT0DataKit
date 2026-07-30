# TASK-T3-HANDOFF — Sheet/Tab 多页 + antd Table

## task_id
T3

## goal
实现中央 Workbench 的 Sheet/Tab 多页（新建/切换/关闭/重命名）+ antd Table（行复选 + 区间选择 / 列 checkbox 显隐 / 列拖拽排序 / 状态高亮 className 枚举），v1.0.0 状态高亮仅 `default` 样式；用 mock 数据跑通交互（真实导入流 T5 接管）。

## in_scope
允许新增/修改的文件：

- `frontend/src/state.js`：**扩展**（不覆盖 T2 的 `activeCapability`/`currentView`/`aiPanel`），新增 `sheets: []`、`activeSheetId: null`；Sheet 对象结构 `{ id, sessionId, name, headers, rows, total, page, pageSize, columnOrder, columnVisibility, selection, statusHighlights }`。action 枚举新增：`ADD_SHEET`/`CLOSE_SHEET`/`SET_ACTIVE_SHEET`/`RENAME_SHEET`/`SET_SELECTION`/`REORDER_COLUMNS`/`SET_COLUMN_VISIBILITY`/`SET_PAGE`。初始 mock 数据可在 reducer 初始化时塞 1~2 个 mock Sheet（含假 headers/rows）便于交互演示，或提供 `loadMockSheet` action——**本任务用 mock 数据**，T5 接管真实数据后移除 mock。
- `frontend/src/components/SheetTabs.jsx`：Sheet/Tab 多页组件。用 antd `Tabs`（`type="editable-card"`）或自实现 Tab 条。支持：新建 Tab（`+` 按钮或工具栏触发，新建一个空 mock Sheet 或带示例数据的 Sheet）、切换（点击 Tab 切 `activeSheetId`）、关闭（Tab 上的 `×`，关闭后激活相邻 Tab）、重命名（双击 Tab 标题进入 `Input` 编辑，回车确认 dispatch `RENAME_SHEET`）。
- `frontend/src/components/DataTable.jsx`：antd `Table` 封装。支持：
  - `rowSelection`（`type="checkbox"`）：行复选 + 区间选择（Shift 点击选区间——可自实现 `onSelect` 逻辑记录 lastSelectedIndex，Shift 时选 [last, current] 区间）。
  - 列 checkbox 显隐：Table 上方或列头提供 `Checkbox.Group` 或 `Dropdown` 切换 `columnVisibility`，隐藏列从 `columns` 过滤。
  - 列拖拽排序：用 `react-resizable` 实现 `onHeaderCell` 的 `onMouseDown` 拖拽，或用 antd `Table` 的 `components.header.cell` 配合 `react-dnd`（若 `react-dnd` 未装，可用 `react-resizable` 的 `resizable` 标签实现列宽调整 + 自实现拖拽列序——列序拖拽可用 `react-dnd` HTML5 backend，需装 `react-dnd` + `react-dnd-html5-backend`，或用更轻量的 `@dnd-kit/core`）。**列序拖拽是硬需求**，不许降级为仅列宽调整。
  - 状态高亮：行 `className` 枚举 `default`/`invalid`/`masked`/`hit`，通过 `rowClassName={(record) => record.status || 'default'}` 注入；v1.0.0 仅 `default` 有样式（正常行），其余枚举值在 CSS 占位但 v1.0.0 不触发（mock 数据所有行 `status='default'`）。CSS 可写在 `DataTable.css` 或用 `className` + 全局样式。
  - 分页：antd `Table.pagination`，`pageSize=50`，`total=rows.length`（mock），`onChange` dispatch `SET_PAGE`。v1.0.0 mock 全量数据前端分页即可（T5 后端分页接管后改 `total` 来自后端）。
- `frontend/src/components/Workbench.jsx`：**扩展** T2 的 Workbench 占位——当 `activeSheetId !== null` 时渲染 `SheetTabs` + `DataTable`；`activeSheetId === null` 且 `sheets` 为空时显示空态「导入数据后在此展示工作台」（保留 T2 空态）。`currentView === 'settings'` 时仍由 T8 设置页覆盖（本任务不实现设置页）。
- 可新增 `frontend/src/components/table/index.js`（barrel，可选）、`frontend/src/components/DataTable.css`（状态高亮样式）。
- `frontend/package.json`：若需新增 `react-dnd`/`react-dnd-html5-backend` 或 `@dnd-kit/core`/`react-resizable` 依赖，允许添加（列拖拽排序必需）。

## out_of_scope
明确不许动的：

- **不许实现真实导入流**：不调 `import_file`/`get_sheet_data`/`@tauri-apps/plugin-dialog`；数据用 mock。
- **不许动 src-tauri/**、**不许动 crates/core/**。
- **不许实现状态高亮 `invalid`/`masked`/`hit` 的触发逻辑**：v1.0.0 只定义 className 枚举 + `default` 样式，mock 数据全部 `default`；触发逻辑是 v1.2+。
- **不许实现设置页**（T8）。
- **不许动 TopToolbar/SidePanel/AiPanel/panels**（T2 产物，本任务只扩展 Workbench）。
- **不许动 docs/**、**不许动 handoff/ 其它任务文件**。
- state.js 扩展时不许删除或覆盖 T2 已有的 `activeCapability`/`currentView`/`aiPanel` 字段与 action。

## acceptance_criteria
1. state.js 含 `sheets: []`、`activeSheetId: null` 与 Sheet 对象结构（含 `id`/`name`/`headers`/`rows`/`total`/`page`/`pageSize`/`columnOrder`/`columnVisibility`/`selection`/`statusHighlights` 字段定义），以及 `ADD_SHEET`/`CLOSE_SHEET`/`SET_ACTIVE_SHEET`/`RENAME_SHEET`/`SET_SELECTION`/`REORDER_COLUMNS`/`SET_COLUMN_VISIBILITY`/`SET_PAGE` action。
2. SheetTabs 支持新建 Tab（产生一个 mock Sheet，headers 如 `["id","name","value"]`、rows 50 行假数据）、切换、关闭（关闭后激活相邻）、重命名（双击编辑）。
3. DataTable 支持行复选（checkbox 单选）+ Shift 区间选择（点第一行后 Shift 点第五行，1~5 行全选）。
4. DataTable 支持列 checkbox 显隐（至少能隐藏/显示 1 列，隐藏后该列不渲染）。
5. DataTable 支持列拖拽排序（拖拽列头改变 `columnOrder`，Table 列序随之变化）。
6. DataTable 行 `className` 含 `default`/`invalid`/`masked`/`hit` 枚举（通过 `rowClassName` 注入），mock 数据全部 `default`，其余 className 在 CSS 占位但不触发。
7. DataTable 分页控件渲染，`pageSize=50`，mock 100 行数据可翻页。
8. Workbench 在 `activeSheetId !== null` 时渲染 SheetTabs + DataTable；空态显示「导入数据后在此展示工作台」。
9. `pnpm --prefix frontend run build` 通过。
10. 新增依赖（若装了 `react-dnd` 等）在 `package.json` + `pnpm-lock.yaml` 同步。

## verification_commands
```sh
# 1. 前端构建
pnpm --prefix frontend run build

# 2. state.js 字段与 action
grep -n 'sheets\|activeSheetId\|ADD_SHEET\|CLOSE_SHEET\|SET_ACTIVE_SHEET\|RENAME_SHEET\|SET_SELECTION\|REORDER_COLUMNS\|SET_COLUMN_VISIBILITY\|SET_PAGE' frontend/src/state.js

# 3. 组件存在
ls frontend/src/components/SheetTabs.jsx frontend/src/components/DataTable.jsx

# 4. 依赖核对（若装了拖拽库）
grep -n 'react-dnd\|@dnd-kit\|react-resizable' frontend/package.json
```

GUI 交互核验（新建/切换/关闭/重命名 Tab、Shift 区间选、列拖拽、分页翻页）由主会话环境补核验；coder 至少 `pnpm build` 通过并描述实现逻辑。

## files_likely_to_change
- `frontend/src/state.js`（扩展）
- `frontend/src/components/SheetTabs.jsx`、`DataTable.jsx`
- `frontend/src/components/Workbench.jsx`（扩展）
- `frontend/src/components/DataTable.css`（可选）
- `frontend/package.json` + `frontend/pnpm-lock.yaml`（若加拖拽依赖）

## risks
- **列拖拽排序库选型**：`react-dnd` 较重但生态成熟；`@dnd-kit` 更现代轻量；`react-resizable` 主要管列宽。建议 `@dnd-kit/core` + `@dnd-kit/sortable`（列序）+ `react-resizable`（列宽）或纯 `react-dnd`。无论选哪个，必须能拖列序。
- **Shift 区间选择与 antd rowSelection 兼容**：antd `rowSelection.onSelect` 单个勾选回调，Shift 需自记录 `lastSelectedIndex` 并在 `onSelect` 判断 `e.shiftKey`，手动批量 `setSelectedRowKeys`。
- **重命名双击编辑**：antd `Tabs` 的 `tabBarStyle` 自定义或用 `onEdit` + `TabPane` 标题包 `Input`；双击进入编辑态需自管 `editing` state。
- **mock 数据与 T5 切换**：mock 数据写在 reducer 初始化或 `loadMockSheet` action，T5 接管后需移除——在代码注释标 `// TODO(T5): replace mock with real import`。

## depends_on
[T1]

## status
planned
