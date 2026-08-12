```yaml
task_id: T40
goal: |
  将设置入口从 AiPanel 底部迁移到 TopToolbar 右端：
  AiPanel 移除两处设置按钮（折叠态 + 展开态），TopToolbar 右端新增 SettingOutlined 按钮。
in_scope:
  - frontend/src/components/AiPanel.jsx（移除折叠态底部设置按钮 + 展开态底部设置按钮）
  - frontend/src/components/layout/TopToolbar.jsx（右端新增设置按钮）
  - frontend/src/App.jsx（移除传给 AiPanel 的 onSettings prop）
out_of_scope:
  - 不改 SettingsView.jsx 本身
  - 不改 state/reducer（setView 已存在）
  - 不改后端
acceptance_criteria:
  - AiPanel 折叠态不再有设置图标按钮
  - AiPanel 展开态不再有底部「设置」按钮
  - TopToolbar 右端有 SettingOutlined 按钮，点击进入设置页
  - 设置页「返回」按钮仍能回到 workbench
  - pnpm build 通过
verification_commands:
  - pnpm --prefix frontend build
files_likely_to_change:
  - frontend/src/components/AiPanel.jsx
  - frontend/src/components/layout/TopToolbar.jsx
  - frontend/src/App.jsx
risks:
  - AiPanel 移除设置按钮后底部留白，需调整 flex 布局
depends_on: []
status: planned
```

## 实现指引

### AiPanel.jsx

移除两处设置入口：

1. 折叠态（!visible 分支）：删除底部的 `<Button type="text" icon={<SettingOutlined />} onClick={onSettings} ... />`。
   保留 `<div style={{ flex: 1 }} />` 占位。
2. 展开态（visible 分支）：删除底部 footer 的整个 `<div style={{ borderTop... }}>` 块（含「设置」按钮）。
3. 移除 `onSettings` prop（不再需要）。
4. 移除 `SettingOutlined` import（如不再使用）。

组件签名改为 `export default function AiPanel()`（无 props）。

### TopToolbar.jsx

在 return 的最外层 div 末尾（ExportModal 之前）新增：

```jsx
<div style={{ flex: 1 }} />
<Button
  type="text"
  icon={<SettingOutlined />}
  onClick={() => setView("settings")}
/>
```

需要：
- import `SettingOutlined` from `@ant-design/icons`
- 从 `useAppContext()` 解构 `setView`（已有 `state, setActiveCapability`，加 `setView`）
- `<div style={{ flex: 1 }} />` 把设置按钮推到右端（flex 布局）

### App.jsx

移除传给 AiPanel 的 onSettings prop：

```jsx
// 之前
<AiPanel onSettings={() => setView("settings")} />
// 之后
<AiPanel />
```

`setView` 仍需保留在 useAppContext 解构中（TopToolbar 现在用它）。
