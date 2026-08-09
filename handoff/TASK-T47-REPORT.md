# T47 实施报告 — 设置页全局每页行数（PAGE_SIZE 持久化）

> 任务：T47
> 状态：verified_complete
> 依赖：无（v1.1.2 收尾功能，独立任务）

## 一、目标

把当前硬编码的 `PAGE_SIZE = 50`（`frontend/src/constants.js`）改成可在「设置」页配置的全局默认值，持久化到 `settings.json`（与 tshark 路径同文件、同模式）。用户在设置页选择每页行数后，所有 Sheet 的分页、翻页、搜索翻页立即按新值生效。这是 v1.1.2 的收尾功能。

## 二、改动文件

| 文件 | 改动 |
|---|---|
| `src-tauri/src/commands/settings.rs` | `TsharkSettings` → `AppSettings` + 新增 `#[serde(default)] page_size: Option<u32>` 字段 + `load_page_size` / `save_page_size` 两个 `#[tauri::command]` |
| `src-tauri/src/lib.rs` | `generate_handler!` 注册 `load_page_size` + `save_page_size`；模块文档注释加 v1.1.2 说明 |
| `frontend/src/state/constants.js` | `initialState.pageSize = 50`（顶层全局默认值）+ `ACTION.SET_PAGE_SIZE` + `activeCapability` 注释加 `'crypto'` |
| `frontend/src/state/factory.js` | 3 工厂增加可选 `pageSize = PAGE_SIZE` 参数：`createEmptySheet` / `createSheetFromImport` / `createSheetFromParse` |
| `frontend/src/state/reducer.js` | 新增 `SET_PAGE_SIZE` case（合并全局 + Sheet 同步：`sheets.map(s => ({...s, pageSize, page: 1}))`）+ ADD_SHEET/IMPORT_SUCCESS/ADD_SHEET_FROM_PARSE 传 `state.pageSize` + SET_SHEET_DATA 的 toRowObjects 传 `action.payload.pageSize ?? s.pageSize` |
| `frontend/src/state/AppContext.jsx` | 新增 `setPageSize` dispatcher（`useCallback` + dispatch `SET_PAGE_SIZE`）+ 加入 value useMemo 与 deps |
| `frontend/src/tauri.js` | 新增 `loadPageSize` / `savePageSize` IPC 封装 + `toRowObjects` 增加 `pageSize` 参数（`base = (page-1) * pageSize`）消除硬编码 `PAGE_SIZE` 行号计算 |
| `frontend/src/components/settings/cards/PageSizeCard.jsx`（新建） | `useAppState` + antd Form/Select（20/50/100/200）+ Save 按钮；`handleSave`：`savePageSize` → dispatch `SET_PAGE_SIZE` → 刷新当前激活 Sheet 首页 → `message.success` |
| `frontend/src/components/settings/SettingsView.jsx` | import PageSizeCard + 插入到 TsharkPathCard 与 DbPathCard 之间 |
| `frontend/src/App.jsx` | 启动 `useEffect` 调 `loadPageSize` → dispatch `SET_PAGE_SIZE`；导入首页 `getSheetData(id, 1, state.pageSize)`（原 `PAGE_SIZE`）；deps 加 `state.pageSize` |
| `frontend/src/components/DataTable.jsx` | 搜索 L241/269 改 `sheet.pageSize || PAGE_SIZE`（修既有不一致：搜索用 raw `PAGE_SIZE`，分页用 `sheet.pageSize || PAGE_SIZE`） |

## 三、不改动的文件

- `frontend/src/constants.js`：`PAGE_SIZE = 50` 常量保留（作为编译期兜底默认值）
- 后端 `get_sheet_data` SQL/分页逻辑：已接收 `page_size` 参数，前端传新值即可
- DB schema：`SCHEMA_VERSION=3` 不变，无新增表/列/索引
- 既有撤销/重做/搜索/列操作/Base64 逻辑：不受影响
- 版本号：4 处仍 1.1.2（v1.1.2 范围内功能）

## 四、验收

| 验收项 | 结果 |
|---|---|
| `cargo clippy -- -D warnings` | pass（0 警告） |
| `pnpm --prefix frontend build` | pass（3081 modules transformed，✓ built in 2.59s） |
| 设置页「每页行数」卡片可见 | pass（PageSizeCard 注册于 TsharkPathCard 与 DbPathCard 之间） |
| Select 默认显示当前值（首次 50） | pass（`initialValues={{ pageSize: state.pageSize || PAGE_SIZE }}`） |
| 可选档位 20/50/100/200 | pass（`PAGE_SIZE_OPTIONS = [20, 50, 100, 200]`） |
| 保存后当前 Sheet 首页刷新 + 页码重置 1 | pass（`SET_PAGE_SIZE` reducer `sheets.map(s => ({...s, pageSize, page: 1}))` + PageSizeCard 调 `getSheetData(id, 1, value)`） |
| 切换其他 Sheet 也按新行数 | pass（SET_PAGE_SIZE 同步所有 Sheet） |
| 重启后设置保留 | pass（`settings.json` 持久化，App.jsx 启动 useEffect 加载） |
| 新建/导入 Sheet 继承全局 pageSize | pass（reducer ADD_SHEET/IMPORT_SUCCESS/ADD_SHEET_FROM_PARSE 传 `state.pageSize`） |
| 旧 settings.json 升级不报错 | pass（`#[serde(default)]` page_size 取 None → 前端回退 50） |
| 版本号仍 1.1.2 | pass（不改版本） |

## 五、设计决策

- **沿用 settings.json 持久化模式**：扩展 `TsharkSettings` → `AppSettings`，新增 `page_size` 字段与 tshark 路径同文件同模式；`#[serde(default)]` 保证旧 settings.json 向后兼容（反序列化时取 `None`，前端回退 `PAGE_SIZE=50`）。
- **SET_PAGE_SIZE reducer 合并全局 + Sheet 同步**：一次 dispatch 同时更新 `state.pageSize`（全局）与所有 `sheet.pageSize`（page 重置 1，避免页码越界），`PageSizeCard` 只需 dispatch 一次 + 刷新当前激活 Sheet 首页。
- **toRowObjects 增加 pageSize 参数**：`_rowIdx` 计算从硬编码 `(page-1) * PAGE_SIZE` 改为 `(page-1) * pageSize`，保证切换行数后行号正确。`PAGE_SIZE` 常量保留作为默认参数兜底。
- **DataTable 搜索一致性修复**：搜索原用 raw `PAGE_SIZE`，分页用 `sheet.pageSize || PAGE_SIZE`，现统一为 `sheet.pageSize || PAGE_SIZE`。
- **PAGE_SIZE 常量保留**：作为编译期兜底默认值（state 初始值、factory 兜底、toRowObjects 默认参数、未加载设置时的回退）。

## 六、文档同步

- `docs/02-技术设计文档.md`：§4.11 settings.json page_size 持久化（AppSettings + load_page_size/save_page_size + SET_PAGE_SIZE reducer + PageSizeCard + state.pageSize 字段）
- `docs/versions/1.1.2/更新日志.md`：T47 行 + E9 验收项 + 关键设计决策
- `docs/versions/1.1.2/RELEASE-NOTES.md`：新增段落 + 升级说明（settings.json 向后兼容）
- `docs/qa/versions/1.1.2/QA-审计报告.md`：R5 审计轮次 + §2 需求覆盖 row #10 + §3 E9 + §12 结论 + §13 修复证据
- `handoff/TASK-BOARD.md`：T47 行 + DAG + E9 + R5
- `handoff/TASK-T47-HANDOFF.md` + `handoff/TASK-T47-REPORT.md`（本文件）
