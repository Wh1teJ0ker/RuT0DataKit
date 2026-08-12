```yaml
task_id: T47
goal: |
  把当前硬编码的 PAGE_SIZE = 50 改成可在「设置」页配置的全局默认值，
  持久化到 settings.json（与 tshark 路径同文件、同模式）。用户在设置页
  选择每页行数后，所有 Sheet 的分页、翻页、搜索翻页立即按新值生效。
  这是 v1.1.2 的收尾功能。
in_scope:
  - src-tauri/src/commands/settings.rs（TsharkSettings→AppSettings + page_size 字段 + load_page_size/save_page_size 命令）
  - src-tauri/src/lib.rs（generate_handler! 注册 2 新命令）
  - frontend/src/state/constants.js（initialState.pageSize + ACTION.SET_PAGE_SIZE）
  - frontend/src/state/factory.js（3 工厂增加可选 pageSize 参数）
  - frontend/src/state/reducer.js（SET_PAGE_SIZE case + ADD_SHEET/IMPORT_SUCCESS/ADD_SHEET_FROM_PARSE 传 state.pageSize）
  - frontend/src/state/AppContext.jsx（setPageSize dispatcher）
  - frontend/src/tauri.js（loadPageSize/savePageSize + toRowObjects 增加 pageSize 参数）
  - frontend/src/components/settings/cards/PageSizeCard.jsx（新建）
  - frontend/src/components/settings/SettingsView.jsx（注册 PageSizeCard）
  - frontend/src/App.jsx（启动加载 pageSize + 导入首页用 state.pageSize）
  - frontend/src/components/DataTable.jsx（搜索 L241/269 一致性修复）
out_of_scope:
  - 不改 DB schema（SCHEMA_VERSION=3 不变，无新增表/列/索引）
  - 不改后端 get_sheet_data SQL/分页逻辑（已接收 page_size 参数，前端传新值即可）
  - 不改版本号（v1.1.2 范围内功能，4 处仍 1.1.2）
  - 不改既有撤销/重做/搜索/列操作/Base64 逻辑
  - 不改 PAGE_SIZE 常量（保留作为编译期兜底默认值）
acceptance_criteria:
  - 设置页出现「每页行数」卡片，Select 默认显示当前值（首次为 50）
  - 可选档位 20 / 50 / 100 / 200
  - 选择 100 → 保存 → 当前 Sheet 立即按 100 行/页重渲染（首页刷新，page 重置 1），翻页/搜索翻页均用新值
  - 切换到其他 Sheet → 也按 100 行/页（全局生效，SET_PAGE_SIZE 合并全局 + Sheet 同步）
  - 重启应用 → 设置仍为 100（settings.json 持久化）
  - 新导入的文件 / 新建 Sheet / JSON 解析新 Tab → pageSize 继承全局值
  - 旧 settings.json（仅 tshark_path）升级后不报错，page_size 回退 50（#[serde(default)]）
  - cargo clippy -- -D warnings 通过（无警告）
  - pnpm --prefix frontend build 通过
  - 版本号仍 1.1.2
verification_commands:
  - cd src-tauri && cargo clippy -- -D warnings
  - pnpm --prefix frontend build
files_changed:
  - src-tauri/src/commands/settings.rs（TsharkSettings→AppSettings + page_size + 2 命令）
  - src-tauri/src/lib.rs（generate_handler! 注册）
  - frontend/src/state/constants.js（initialState.pageSize + SET_PAGE_SIZE）
  - frontend/src/state/factory.js（3 工厂 pageSize 参数）
  - frontend/src/state/reducer.js（SET_PAGE_SIZE case + 3 处传 state.pageSize）
  - frontend/src/state/AppContext.jsx（setPageSize dispatcher）
  - frontend/src/tauri.js（loadPageSize/savePageSize + toRowObjects pageSize 参数）
  - frontend/src/components/settings/cards/PageSizeCard.jsx（新建）
  - frontend/src/components/settings/SettingsView.jsx（注册 PageSizeCard）
  - frontend/src/App.jsx（启动加载 + 导入首页用 state.pageSize）
  - frontend/src/components/DataTable.jsx（搜索一致性修复）
risks:
  - SET_PAGE_SIZE reducer 合并全局 + Sheet 同步（page 重置 1），避免页码越界
  - toRowObjects 增加 pageSize 参数消除硬编码行号，需确认所有调用方传正确值
  - 旧 settings.json 向后兼容（#[serde(default)] page_size 取 None）
depends_on: []
status: verified_complete
```

## 实现指引

### 1. Rust：`src-tauri/src/commands/settings.rs`

- `TsharkSettings` 重命名为 `AppSettings`
- 新增字段：`#[serde(default)] pub(crate) page_size: Option<u32>`
- `read_settings` / `write_settings` 签名改用 `AppSettings`
- 新增两个命令：
  ```rust
  #[tauri::command]
  pub async fn load_page_size(app: AppHandle) -> Result<Option<u32>, String> {
      let settings = read_settings(&app);
      Ok(settings.page_size)
  }

  #[tauri::command]
  pub async fn save_page_size(app: AppHandle, page_size: Option<u32>) -> Result<(), String> {
      let mut settings = read_settings(&app);
      settings.page_size = page_size;
      write_settings(&app, &settings)
  }
  ```

### 2. Rust：`src-tauri/src/lib.rs`

- `generate_handler!` 加 `commands::settings::load_page_size,` + `commands::settings::save_page_size,`

### 3. 前端 state：`constants.js`

- `initialState` 加 `pageSize: 50`（顶层全局默认值，注释说明 v1.1.2 持久化）
- `ACTION` 加 `SET_PAGE_SIZE: "SET_PAGE_SIZE"`（位于 SET_TSHARK_LOADING 之后）
- `activeCapability` 注释加 `'crypto'`

### 4. 前端 state：`factory.js`

- 3 工厂增加可选 `pageSize = PAGE_SIZE` 参数：
  - `createEmptySheet(name, pageSize = PAGE_SIZE)`
  - `createSheetFromImport(result, pageSize = PAGE_SIZE)`
  - `createSheetFromParse(result, pageSize = PAGE_SIZE)`

### 5. 前端 state：`reducer.js`

- ADD_SHEET：`createEmptySheet(name, state.pageSize)`
- IMPORT_SUCCESS：`createSheetFromImport(action.payload, state.pageSize)`
- ADD_SHEET_FROM_PARSE：`createSheetFromParse(action.payload, state.pageSize)`
- SET_SHEET_DATA 的 toRowObjects 调用：加 `action.payload.pageSize ?? s.pageSize` 第 5 参数
- 新增 SET_PAGE_SIZE case（合并全局 + Sheet 同步）：
  ```js
  case ACTION.SET_PAGE_SIZE: {
    const pageSize = action.payload;
    return {
      ...state,
      pageSize,
      sheets: state.sheets.map((s) => ({ ...s, pageSize, page: 1 })),
    };
  }
  ```

### 6. 前端 state：`AppContext.jsx`

- 加 `setPageSize` dispatcher：`useCallback((pageSize) => dispatch({ type: ACTION.SET_PAGE_SIZE, payload: pageSize }), [])`
- 加入 value useMemo 对象 + deps 数组
- 注释更新「20 个 dispatcher（v1.1.2 新增 setPageSize）」

### 7. 前端 IPC：`tauri.js`

- 新增 `loadPageSize()` → `invoke("load_page_size")`
- 新增 `savePageSize(pageSize)` → `invoke("save_page_size", { pageSize })`
- `toRowObjects` 签名加 `pageSize = PAGE_SIZE` 第 5 参数，`base = (page - 1) * pageSize`

### 8. 新建 `PageSizeCard.jsx`

参考 `TsharkPathCard.jsx` 模式：
- `useAppState()` 取 `state/dispatch`
- antd `Card` + `Form` + `Select`（20/50/100/200）+ Save 按钮
- `initialValues={{ pageSize: state.pageSize || PAGE_SIZE }}`
- `handleSave`：`savePageSize(value)` → `dispatch SET_PAGE_SIZE` → 刷新当前激活 Sheet 首页（`getSheetData(id, 1, value)` + `SET_SHEET_DATA`）→ `message.success`
- `PAGE_SIZE_OPTIONS = [20, 50, 100, 200]`

### 9. `SettingsView.jsx`

- import PageSizeCard
- 插入到 TsharkPathCard 与 DbPathCard 之间（顺序：Update → TsharkPath → **PageSize** → DbPath → About）

### 10. `App.jsx`

- import 加 `useEffect` + `loadPageSize`
- 启动 useEffect 加载持久化 pageSize：
  ```js
  useEffect(() => {
    (async () => {
      try {
        const saved = await loadPageSize();
        if (saved && saved > 0) {
          dispatch({ type: ACTION.SET_PAGE_SIZE, payload: saved });
        }
      } catch { /* 静默忽略 */ }
    })();
  }, [dispatch]);
  ```
- handleImport 导入首页：`getSheetData(payload.sheetId, 1, state.pageSize)`（原 `PAGE_SIZE`），deps 加 `state.pageSize`

### 11. `DataTable.jsx`

- 搜索 L241：`PAGE_SIZE` → `sheet.pageSize || PAGE_SIZE`
- 搜索 L269：`PAGE_SIZE` → `sheet.pageSize || PAGE_SIZE`
- 修既有不一致（搜索用 raw PAGE_SIZE，分页用 sheet.pageSize || PAGE_SIZE）

### 验证

```bash
cd src-tauri && cargo clippy -- -D warnings
pnpm --prefix frontend build
```

预期：cargo clippy 0 警告；pnpm build ✓ built，无 ESLint/构建错误。
