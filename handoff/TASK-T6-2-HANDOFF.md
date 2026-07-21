```yaml
task_id: T6-2
goal: |
  移除各功能界面（rules/search/mask/validate/export/tools）顶部常驻的「导入文件」
  按钮（FileToolbar），让数据导入仅作为 PreprocessView 内置入口存在；App.jsx 不
  再为任何 view 渲染 FileToolbar。

in_scope:
  - frontend/src/App.jsx                              # 移除 FileToolbar import + 渲染分支 + NO_TOOLBAR_VIEWS 集合
  - frontend/src/components/FileToolbar.jsx           # 删除文件（其能力已并入 PreprocessView 内置导入按钮）
  - frontend/src/components/ExportView.jsx            # 移除「请先导入文件」对 filePath 的依赖，改为读 records（与 T6-1 配合）
  - frontend/src/components/MaskView.jsx              # 仅文案/Empty 兜底对齐（如需要）
  - frontend/src/components/ValidateView.jsx          # 仅文案/Empty 兜底对齐（如需要）

out_of_scope:
  - 不改 PreprocessView 内的导入按钮（用户要求保留）
  - 不改后端命令
  - 不改 SET_FILE reducer 行为（T6-1 已处理）
  - 不外发数据：不引入网络请求

acceptance_criteria:
  - App.jsx 不再 import FileToolbar，不再渲染顶部常驻导入条
  - FileToolbar.jsx 文件已删除
  - rules/search/mask/validate/export/tools 6 个 view 顶部均无「导入文件」按钮
  - PreprocessView 顶部「导入文件」按钮保留且可用
  - 无 records 时各 view 仍显示 Empty「请先到数据预处理导入文件」引导
  - cd frontend && npm run build 通过，无 import 残留报错

verification_commands:
  - cd frontend && npm run build
  - grep -rn "FileToolbar" frontend/src/   # 应 0 命中

files_likely_to_change:
  - frontend/src/App.jsx
  - frontend/src/components/FileToolbar.jsx   # 删除
  - frontend/src/components/ExportView.jsx

risks:
  - ExportView 当前用 filePath 决定能否选「原始数据」源；移除 FileToolbar 后 filePath 永远为空，必须改为读 records（依赖 T6-1 verified）。
  - App.jsx 顶部留白会改变视觉布局；保留 Content padding 24 + gap 16 即可，不需新增占位。

depends_on: [T6-1]
status: planned
```

## 上下文

**用户原话**：「每一个界面上方不应该有导入文件的按钮」。

当前实现：`App.jsx:43` 为非 `NO_TOOLBAR_VIEWS` 的 view 渲染 `<FileToolbar/>`，而 `NO_TOOLBAR_VIEWS = { preprocess, rules, mask, validate }`——意味着 `search / export / tools` 这 3 个 view 顶部都有「导入文件」按钮。用户要求全部移除，导入只保留在 PreprocessView。

**安全约束**：不外发数据；全本地处理；规则与样本不上传。
