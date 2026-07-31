# 设置页全屏化 + AI 面板精简 + 导出格式弹窗

## 需求 1：设置 = 独立全屏页面 + 返回按钮 + 列表式布局

### App.jsx 改造（顶层路由）
当前 `SidePanel + Workbench + AiPanel` 三栏始终并排。改为：`currentView === "settings"` 时渲染一个**独立全屏 SettingsPage**（替换整个三栏布局），不显示 Sidebar / AiPanel / TopToolbar 的数据操作按钮 —— 彻底另一个界面。

```
<Layout>
  <Header><TopToolbar ... /></Header>   ← Header 保留（含返回入口）
  {currentView === "settings"
    ? <SettingsPage onBack={() => setView("workbench")} />
    : <Layout><SidePanel/><Workbench/><AiPanel/></Layout>}
</Layout>
```

### SettingsView.jsx → SettingsPage.jsx 重写
- 顶部 antd `PageHeader`（或自建模态行）：标题「设置」+ `Button` 「← 返回」调用 `onBack` → `setView("workbench")`。
- 主体改为**垂直有序列表**：`antd List` 或 `Space direction="vertical" style={{width:'100%'}}`，每项一个 Card，顺序固定：
  1. 检查更新（UpdateCard）
  2. tshark 路径（TsharkPathCard）
  3. 数据库路径（DbPathCard）
  4. 关于（AboutCard）
- 容器 maxWidth 800px 居中，padding，可滚动。移除原 `Row/Col` 网格。

### 4 张卡片微调
- 各 Card 保持现有内容，仅统一 `size="small"` + `style={{width:'100%'}}`，适配列表项。
- Workbench.jsx 移除 `currentView === "settings"` 分支（设置页不再内嵌 Workbench）。

## 需求 2：AI 助手默认收起 + 删除多余描述

### state.js
`aiPanel: { visible: true }` → `{ visible: false }`（默认折叠为图标条）。

### AiPanel.jsx 精简
删除以下多余描述元素：
- 「预留 IPC 契约：`ai_suggest` / `invoke_ai_op`」Paragraph（lines 94-97）
- 「调用 ai_suggest 测试」Button + onTestAiSuggest 整段逻辑（lines 21-40, 97-105）
- 不可用时的 detail Paragraph（lines 107-114）
保留：展开/折叠按钮 + 一行「AI 能力 开发中」状态文案。

## 需求 3：导出 = 弹窗选格式

### 新建 `frontend/src/components/ExportModal.jsx`
- antd `Modal`，`open` 受控。
- 内含 `Radio.Group` 或 `Select` 选导出格式：CSV / JSON / TXT / XLSX。
- v1.0.0 真实实现：**CSV**（已有 esc 逻辑）、**JSON**（`JSON.stringify(rows, null, 2)`）、**TXT**（Tab 分隔）。
- XLSX 标记「开发中」（需写入库，后续版本）—— Radio 项 disabled + 标注。
- 确定按钮 → 调对应导出函数 → Tauri `save()` 对话框选路径 → 写文件（复用 plugin-fs `writeTextFile`）/ Blob 兜底。

### tauri.js 重构
- 抽出公共 `saveTextFile(filename, content, mimeType)` 工具（Tauri save + writeTextFile + Blob 兜底）。
- `exportSheetToCsv` 改为调用 `saveTextFile`。
- 新增 `exportSheetToJson(sheet)`、`exportSheetToTxt(sheet)`。

### TopToolbar.jsx
`handleExport` 改为 `setExportModalOpen(true)`；`ExportModal` 由 TopToolbar 持有 `open` state，传 `activeSheet`。导出按钮不再直接调 save。

## 影响文件
- `frontend/src/App.jsx`（设置页全屏路由）
- `frontend/src/components/Workbench.jsx`（移除 settings 分支）
- `frontend/src/components/settings/SettingsView.jsx` → 重写为 SettingsPage（列表式 + 返回按钮）
- `frontend/src/state.js`（aiPanel.visible 默认 false）
- `frontend/src/components/AiPanel.jsx`（删除测试按钮 + 多余描述）
- `frontend/src/components/layout/TopToolbar.jsx`（导出按钮改为开弹窗）
- `frontend/src/components/ExportModal.jsx`（新建）
- `frontend/src/tauri.js`（抽 saveTextFile + +JSON/TXT 导出）

## 验证
`cargo tauri dev` → 打开应用：
1. 点设置 → 全屏设置页 + 返回按钮 + 列表式 4 项；点返回回工作台。
2. AI 助手默认折叠为图标条；展开后仅一行状态文案，无测试按钮。
3. 点导出 → 弹窗选 CSV/JSON/TXT → 确定后保存对应格式文件；XLSX 灰显「开发中」。

## 不做
- 不加 XLSX 写入库（开发中占位）。
- 不改 Rust 后端（导出全前端完成，无需新 IPC）。
- 不提交 git（等用户验收）。