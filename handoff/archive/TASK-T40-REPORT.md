```yaml
implemented_changes:
  - frontend/src/components/AiPanel.jsx
      - 移除 SettingOutlined import（不再使用）
      - 组件签名从 AiPanel({ onSettings }) 改为 AiPanel()（无 props）
      - 折叠态：删除底部设置按钮，保留 flex:1 占位
      - 展开态：删除底部 footer（含「设置」按钮的 borderTop div）
      - 更新顶部注释：设置入口已迁移到 TopToolbar 右端（T40）
  - frontend/src/components/layout/TopToolbar.jsx
      - import 增加 SettingOutlined
      - useAppContext 解构增加 setView
      - 在 CAPABILITIES 的 Space 之后、ExportModal 之前插入
        <div style={{ flex: 1 }} /> 占位 + SettingOutlined 按钮（onClick 调 setView("settings")）
  - frontend/src/App.jsx
      - 移除 <AiPanel onSettings={() => setView("settings")} /> 上的 onSettings prop
      - 改为 <AiPanel />；setView 仍在 useAppContext 解构中（TopToolbar 现在用它）
verification_run:
  - pnpm --prefix frontend build
verification_results:
  - pnpm --prefix frontend build: 通过（✓ built in 2.32s，3079 modules transformed）
    仅有一条与本次改动无关的 chunk 大小警告（antd 体积大，既有问题）
docs_updated:
  - none（无 docs/ 行为变更，设置入口位置不涉及产品文档描述差异）
commit_summary:
  - none（按要求未 commit）
reported_status:
  - verified_complete
scope_deviation:
  - none（仅改动 in_scope 的 3 个文件；工作树中其他文件如 pcap/Cargo/settings cards 属于并行任务，非本任务改动）
```

## 说明

- 验收标准自检：
  - AiPanel 折叠态不再有设置图标按钮：是
  - AiPanel 展开态不再有底部「设置」按钮：是
  - TopToolbar 右端有 SettingOutlined 按钮，点击调 setView("settings") 进入设置页：是
  - 设置页「返回」按钮仍回到 workbench：未改动 SettingsView.jsx / App.jsx 的 SettingsView 渲染分支，行为保持
  - pnpm build 通过：是
- 未 commit / 未 push（按要求）。
