```yaml
task_id: T46
goal: |
  把 Base64 编解码 UI 从 ColumnOpsPanel 抽离到独立的「加解密」侧边栏能力面板
  (CryptoPanel)，在 TopToolbar 新增 crypto 能力按钮（KeyOutlined 图标），
  作为后续所有加解密/哈希/编解码类操作的统一入口。ColumnOpsPanel 仅保留
  JSON 解析。
in_scope:
  - frontend/src/components/panels/CryptoPanel.jsx（新建 — 搬迁 Base64 Form + handleBase64）
  - frontend/src/components/panels/ColumnOpsPanel.jsx（删除 Base64 相关 state/handler/JSX/import）
  - frontend/src/components/layout/TopToolbar.jsx（CAPABILITIES 新增 crypto + import KeyOutlined）
  - frontend/src/components/layout/SidePanel.jsx（PANELS 注册 crypto: CryptoPanel）
out_of_scope:
  - 不改 base64_column IPC / tauri.js base64Column 封装 / DB 方法 / Rust 命令
  - 不改 App.jsx 路由（crypto 走默认 Layout + SidePanel 分支）
  - 不改 state/reducer.js（SET_ACTIVE_CAPABILITY XOR 逻辑通用）
  - 不改 constants.js（无新增 ACTION）
  - 不改版本号（v1.1.2 范围内 UI 重组）
acceptance_criteria:
  - 工具栏出现「加解密」按钮（KeyOutlined 图标），点击后左侧 260px 侧栏展开 CryptoPanel
  - 再次点击「加解密」按钮收起侧栏（XOR 切换）
  - CryptoPanel 内：目标列 Select（来自当前 sheet headers）+ 模式 Select(编码/解码，初值 encode) + 执行按钮
  - 执行 Base64 编/解码后：当前页数据刷新 + 撤销栈刷新 + 成功提示（行为与原 ColumnOpsPanel 内 Base64 完全一致）
  - ColumnOpsPanel 仅剩 JSON 解析，无 Base64 残留（无 Divider、无 base64Form、无 handleBase64、无 base64Column import）
  - pnpm --prefix frontend build 通过
  - 版本号仍 1.1.2
verification_commands:
  - pnpm --prefix frontend build
files_changed:
  - frontend/src/components/panels/CryptoPanel.jsx（新建）
  - frontend/src/components/panels/ColumnOpsPanel.jsx（瘦身）
  - frontend/src/components/layout/TopToolbar.jsx（加 crypto 按钮）
  - frontend/src/components/layout/SidePanel.jsx（注册 CryptoPanel）
risks:
  - 纯前端 UI 重组，无后端改动，风险极低
  - Base64 行为不变（handleBase64 逻辑原样搬迁，IPC/DB 方法不动）
depends_on: [T38]
status: verified_complete
```

## 实现指引

### 1. 新建 `frontend/src/components/panels/CryptoPanel.jsx`

参考 ColumnOpsPanel 原 Base64 块（机械搬迁），结构：

- `useAppContext()` 取 `state/dispatch`
- `Form` + `Select`(目标列) + `Select`(模式 encode/decode，初值 encode) + 执行按钮
- `handleBase64()` 逻辑原样搬迁：`base64Column(sheet.id, column, mode)` → `getSheetData` + `SET_SHEET_DATA` → `listUndoableOperations` + `SET_UNDO_STACK` → `message.success`
- 顶部 `<Typography.Title level={5}>加解密</Typography.Title>` 作为多操作入口标题
- 注释说明：本面板是加解密操作入口，v1.1.2 承载 Base64，后续扩展 URL-safe Base64 / MD5 / SHA / AES

### 2. 修改 `ColumnOpsPanel.jsx`

- 删除 `base64Form`/`base64ing` state
- 删除 `handleBase64` 函数
- 删除 Base64 Form JSX 块 + 其前的 `<Divider>`
- 删除 `base64Column` + `listUndoableOperations` import（JSON 解析只用 `getSheetData` + `parseColumnAsJson`）
- 更新文件顶部注释

### 3. 修改 `TopToolbar.jsx`

- import 加 `KeyOutlined`
- `CAPABILITIES` 数组在 `columnOps` 后、`rules` 前插入：
  ```js
  { id: "crypto", label: "加解密", icon: <KeyOutlined /> },
  ```

### 4. 修改 `SidePanel.jsx`

- import `CryptoPanel`
- `PANELS` 加 `crypto: CryptoPanel,`
- 注释更新

### 验证

```bash
pnpm --prefix frontend build
```

预期：✓ built，无 ESLint / 构建错误。
