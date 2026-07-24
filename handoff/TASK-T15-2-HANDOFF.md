```yaml
task_id: T15-2
goal: |
  补完缺失的 T13-3 前端 encrypt 接线：新增 EncryptTool.jsx 子组件
  （算法 Select: AES-CBC/Base64/Hex + 模式切换 加密/解密 + Key 输入 +
  单值试运行区 + 批量列操作区），接线 tauri.js（4 wrapper）、
  state.js（ENCRYPT_* action + toolsDomain cases + initial fields）、
  Sidebar.jsx（tools.encrypt 子项 + handleClick 分支）、ToolsView.jsx
  （第三分支渲染 EncryptTool）。
in_scope:
  - frontend/src/components/EncryptTool.jsx（新建）
  - frontend/src/components/ToolsView.jsx（加 encrypt 分支 + title）
  - frontend/src/components/Sidebar.jsx（加 tools.encrypt 子项 + handleClick 分支）
  - frontend/src/state.js（ENCRYPT_* ACTION 常量 + initial fields + toolsDomain cases）
  - frontend/src/tauri.js（encryptText/decryptText/encryptColumns/decryptColumns wrapper）
out_of_scope:
  - crates/core/* / src-tauri/*（T13-1/T13-2 已完成）
  - docs/*（T15-3 处理）
  - 其他 frontend 组件（RegexTool/SqlParseTool/RulesView/MaskView 等不动）
  - 版本号 bump（T15-3 负责）
acceptance_criteria:
  - tauri.js 导出 encryptText(algo, plaintext, key) / decryptText(algo, ciphertext, key) / encryptColumns(headers, rows, algo, key, selectedColumns) / decryptColumns(headers, rows, algo, key, selectedColumns)
  - state.js 新增 ACTION.ENCRYPT_ALGO_SET / ENCRYPT_MODE_SET / ENCRYPT_KEY_SET / ENCRYPT_INPUT_SET / ENCRYPT_RESULT_SET / ENCRYPT_COLUMNS_RESULT_SET / ENCRYPT_LOADING_SET，initialState 含 encryptAlgo:"aes" / encryptMode:"encrypt" / encryptKey:"" / encryptInput:"" / encryptResult:null / encryptColumnsResult:null / encryptLoading:false
  - toolsDomain 处理上述 7 个 action
  - Sidebar tools SubMenu children 新增 { key:"tools.encrypt", icon:<LockOutlined/>, label:"加密/解密" }，handleClick 加 tools.encrypt 分支
  - ToolsView 加 toolsActiveTab==="encrypt" 分支渲染 <EncryptTool>，title 三元
  - EncryptTool.jsx：算法 Select + 模式 Radio（加密/解密）+ Key Input.Password + 单值 TextArea + 运行按钮 + 结果展示；批量区：从 state.records 取 headers/rows，列勾选 Checkbox.Group，运行后 antd Table 预览 processed_rows（前 50 行）
  - npm --prefix frontend run build 成功
  - AES 模式下 Key 为空时前端 message.warning 提示
verification_commands:
  - npm --prefix frontend run build
  - grep -n 'encryptText\|decryptText\|encryptColumns\|decryptColumns' frontend/src/tauri.js
  - grep -n 'ENCRYPT_' frontend/src/state.js
  - grep -n 'tools.encrypt' frontend/src/components/Sidebar.jsx frontend/src/components/ToolsView.jsx
files_likely_to_change:
  - frontend/src/components/EncryptTool.jsx
  - frontend/src/components/ToolsView.jsx
  - frontend/src/components/Sidebar.jsx
  - frontend/src/state.js
  - frontend/src/tauri.js
risks:
  - Key 为空 AES 会报错：前端先校验再调命令
  - 批量结果可能很长：Table 只预览前 50 行，scroll.y
  - 切 view 不重置 encrypt 子状态（与 sql/regex 一致策略）
  - state.js 已有 SET_RECORDS 级联初始化（T12-7），新增 ENCRYPT_* action 不能破坏现有 case
  - tauri.js 末尾已有 trialMask wrapper，新增 4 个 encrypt wrapper 不要冲突
depends_on: [T15-1]
status: planned
```

## 实施说明

### 1. 参考组件

- `RegexTool.jsx`（Tabs + 试运行模式）
- `MaskView.jsx`（列勾选 + 批量运行）

### 2. EncryptTool 结构

- 顶部：算法 `<Select>`（aes/base64/hex）+ 模式 `<Radio.Group>`（encrypt/decrypt）+ Key `<Input.Password>`（AES 显示，Base64/Hex 隐藏）
- 单值区：`<TextArea>` 输入 + 运行按钮 → `encryptText`/`decryptText` → 结果 `<Paragraph copyable>`
- 批量区：若 `state.records` 存在，显示列勾选 `<Checkbox.Group>`（options=headers）+ 运行按钮 → `encryptColumns`/`decryptColumns` → `<Table>` 预览前 50 行

### 3. state.js

toolsDomain 内新增 7 个 case，initialState 末尾加 7 个字段。
注意不要破坏现有 SET_RECORDS 级联初始化逻辑（T12-7）。

### 4. Sidebar

import `LockOutlined`，children 加第三项，handleClick 加 `tools.encrypt` 分支。

### 5. ToolsView

title 三元 `toolsActiveTab === "encrypt" ? "Tools - 加密/解密" : toolsActiveTab === "regex" ? "Tools - 正则解析" : "Tools - SQL 解析"`，渲染分支同理。

### 6. tauri.js

末尾新增 4 个 wrapper（encryptText/decryptText/encryptColumns/decryptColumns），
invoke 名对齐 src-tauri/src/commands/encrypt.rs 的 4 个 #[tauri::command]：
encrypt_text / decrypt_text / encrypt_columns / decrypt_columns。
参数名用 camelCase（Tauri 自动转 snake_case）。
