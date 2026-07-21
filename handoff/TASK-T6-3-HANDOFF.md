```yaml
task_id: T6-3
goal: |
  把 ToolsView 顶部从 antd Tabs（横版标签）改为 antd Select 下拉栏，下拉两项
  「SQL 解析」「正则解析」；选中项写入 state.toolsActiveTab，切 view 不重置。

in_scope:
  - frontend/src/components/ToolsView.jsx            # Tabs → Select

out_of_scope:
  - 不改 SqlParseTool / RegexTool 内部布局
  - 不改 state.toolsActiveTab 字段名/语义
  - 不改后端命令
  - 不外发数据

acceptance_criteria:
  - ToolsView 顶部为 antd Select（非 Tabs），options = [{value:"sql",label:"SQL 解析"},{value:"regex",label:"正则解析"}]
  - 选中 sql 渲染 <SqlParseTool/>；选中 regex 渲染 <RegexTool/>
  - 切到其他 view 再切回 tools，选中项保留
  - 不引入原生 <select>（用 antd Select）
  - cd frontend && npm run build 通过

verification_commands:
  - cd frontend && npm run build
  - grep -n "Tabs" frontend/src/components/ToolsView.jsx   # 应 0 命中

files_likely_to_change:
  - frontend/src/components/ToolsView.jsx

risks:
  - antd Select 默认下拉宽度自适应，需配 style={{ width: 220 }} 避免过窄。
  - Select onChange 与 Tabs onChange 签名一致（都是 key/value），迁移简单。

depends_on: [T6-1]
status: planned
```

## 上下文

**用户原话**：「Tools页面的应该是一个下拉栏，不是横版标签」。

当前 `ToolsView.jsx:14-33` 用 `antd Tabs` 渲染两个 Tab。改为 antd `Select` 单选下拉即可，选中项 dispatch `SET_TOOLS_ACTIVE_TAB`。state 字段已存在（`toolsActiveTab: "sql"`），无需改 state.js。

**安全约束**：不外发数据；全本地处理；规则与样本不上传。
