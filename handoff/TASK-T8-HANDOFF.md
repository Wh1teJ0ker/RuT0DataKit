# TASK-T8-HANDOFF — 设置页 + 能力面板占位提示

> 本 HANDOFF 与最新代码基线（T6 verified_complete @ 8a976f6）对齐。T8 依赖 T2（布局壳 + 能力面板容器）+ T6（updater 检查能力）均 `verified_complete`。T8 与 T7 存在轻微文件重叠（`frontend/src/tauri.js` 可能需 `checkUpdate` 封装），主会话已将 T7 与 T8 并行分派，但通过文件分区避免冲突：**T8 不许动 `src-tauri/`，T7 不许动 `frontend/src/components/settings/`**。`frontend/src/tauri.js` 若两任务都需扩展，T8 只加 `checkUpdate`/`installUpdate` 封装，T7 加 `aiSuggest`/`invokeAiOp`，函数名不冲突；若并行导致 merge 冲突，主会话在 T7 verified 后协调 T8 rebase。

## task_id
T8

## goal
落地设置页（updater 检查 / tshark 路径占位 / DB 路径展示 / 关于）+ 确认 4 项能力面板占位提示统一文案。

## in_scope
允许新增/修改的文件：

- `frontend/src/components/settings/SettingsView.jsx`（新建）：设置页主组件，渲染 4 张卡片：
  - `UpdateCard`：按钮触发 `check_update` + 展示结果（版本号 / 无更新 / 错误降级）。
  - `TsharkPathCard`：输入框占位，v1.0.0 不持久化，提示「v1.3+ 生效」。
  - `DbPathCard`：只读展示 DB 路径 + 「在 Finder 中显示」按钮。
  - `AboutCard`：版本号 v1.0.0 + 链接（GitHub / 文档 / License）。
- `frontend/src/components/settings/cards/UpdateCard.jsx`（新建）：调 `checkUpdate` → 展示 `UpdateStatus{available,version,notes}`。有更新时加「安装更新」按钮调 `installUpdate`。
- `frontend/src/components/settings/cards/TsharkPathCard.jsx`（新建）：输入框 + 「v1.3+ 生效」提示。
- `frontend/src/components/settings/cards/DbPathCard.jsx`（新建）：只读路径 + 「在 Finder 中显示」按钮。**DB 路径获取方式**：v1.0.0 可硬编码展示预期路径（macOS: `~/Library/Application Support/com.rut0.datakit/ruT0datakit.db`）+ 说明，或通过新增 `get_db_path` 命令获取——**但本任务不许动 `src-tauri/`**，所以 v1.0.0 采用前端硬编码展示 + 说明文案。T8 后 T9 可补命令。
- `frontend/src/components/settings/cards/AboutCard.jsx`（新建）：版本号 + 链接。
- `frontend/src/components/Workbench.jsx`（扩展 T3 产物）：`currentView === "settings"` 时渲染 `SettingsView` 而非当前的占位 `Typography.Text`。保留 T3 的 Sheet/Tab + DataTable 分支不变。
- `frontend/src/components/layout/SidePanel.jsx`（扩展 T2 产物，可选）：确认底部「⚙ 设置」按钮已存在（T2 已实现），T8 确认点击切换 `currentView="settings"` 路径工作；设置页返回时恢复 `activeCapability`（T2 的 `SET_VIEW` action 已实现，无需改 state.js）。
- `frontend/src/state.js`（扩展 T3 产物，按需）：若需新增 `dbPath` 等字段可加，但优先复用 `currentView`/`activeCapability`。**不许破坏 T3 的 8 个 action 与 Sheet 结构**。
- `frontend/src/tauri.js`（扩展 T7 产物，若 T7 已 verified）：新增 `checkUpdate()` → `invoke('check_update')`、`installUpdate()` → `invoke('install_update')`。**若 T7 尚未 verified（并行分派中），T8 可本地创建 `tauri.js` 的 `checkUpdate`/`installUpdate` 封装，但需与 T7 的 `aiSuggest`/`invokeAiOp` 协调避免文件覆盖——主会话建议 T8 在 T7 verified 后合并，或 T8 先用内联 `invoke` 调用不依赖 `tauri.js`**。推荐：T8 在 `UpdateCard.jsx` 内直接 `import { invoke } from '@tauri-apps/api/core'` 调 `invoke('check_update')`，不依赖 `tauri.js`，避免与 T7 文件冲突。
- 4 项能力面板（`frontend/src/components/panels/{MaskPanel,ValidatePanel,ExtractPanel,RulesPanel}.jsx`）占位提示文案核对：统一模板「【能力名】能力 v1.1+ 释放」。T2/T3 已实现占位，T8 核对文案一致性，**不许改交互骨架**。

## out_of_scope
明确不许动的：

- **不许动 `src-tauri/` 任何文件**（T4/T6/T7 范围，T8 纯前端任务）。
- **不许动 DB 层**（T4）。
- **不许动布局骨架** `TopToolbar.jsx` 的结构、`SidePanel.jsx` 的四区结构、`Workbench.jsx` 的 Sheet/Tab 分支（本任务只在 `currentView==="settings"` 分支替换占位为 `SettingsView`）。
- **不许动 DataTable/SheetTabs 交互**（T3）。
- **不许实现 tshark 真实路径检测/持久化**（v1.3+）。
- **不许实现导出/格式转换/撤销/列操作真实逻辑**（v1.1+）。
- **不许动 docs/**、**不许动 handoff/ 其它任务文件**。

## acceptance_criteria
1. `frontend/src/components/settings/SettingsView.jsx` 渲染 4 张卡片（UpdateCard/TsharkPathCard/DbPathCard/AboutCard）。
2. `UpdateCard` 按钮触发 `check_update`（或内联 `invoke('check_update')`），结果正确展示（版本号 / 无更新 / 错误降级不崩溃）。
3. `TsharkPathCard` 输入框占位 + 「v1.3+ 生效」提示。
4. `DbPathCard` 展示 DB 路径（硬编码 macOS 路径或说明）+ 「在 Finder 中显示」按钮（可用 `shell` 或 a 标签占位，v1.0.0 可只展示路径 + 按钮 disabled 或 alert 提示）。
5. `AboutCard` 展示版本号 v1.0.0 + 链接。
6. `Workbench.jsx` 的 `currentView==="settings"` 分支渲染 `SettingsView`（替换 T3 占位 `Typography.Text`）。
7. 4 项能力面板占位文案统一「【能力名】能力 v1.1+ 释放」（核对 MaskPanel/ValidatePanel/ExtractPanel/RulesPanel）。
8. 设置页返回（点击非设置能力按钮或 TopToolbar）恢复上次 `activeCapability`（T2 `SET_VIEW` 已实现，T8 核对路径）。
9. `pnpm --prefix frontend run build` 通过。
10. T2/T3/T6/T7 产物未被破坏（`src-tauri/` 未改，前端布局骨架未改）。

## verification_commands
```sh
# 1. 前端构建
pnpm --prefix frontend run build

# 2. 设置页组件存在
ls frontend/src/components/settings/
ls frontend/src/components/settings/cards/

# 3. Workbench 集成
grep -n 'SettingsView\|currentView' frontend/src/components/Workbench.jsx

# 4. 能力面板文案核对
grep -rn 'v1.1+' frontend/src/components/panels/

# 5. src-tauri 未被改动（回归）
git diff --stat src-tauri/  # 应为空
```

GUI 设置页核验（4 卡片渲染 + 检查更新触发 + 能力面板文案）由主会话环境补；coder 至少前端构建通过并描述卡片渲染与 `check_update` 调用逻辑。

## files_likely_to_change
- `frontend/src/components/settings/SettingsView.jsx`（新建）
- `frontend/src/components/settings/cards/{UpdateCard,TsharkPathCard,DbPathCard,AboutCard}.jsx`（新建）
- `frontend/src/components/Workbench.jsx`（settings 分支替换占位）
- `frontend/src/components/panels/*.jsx`（文案核对，可能微调）
- `frontend/src/state.js`（按需，优先复用）

## risks
- **与 T7 的 `tauri.js` 文件冲突**：若 T7 已创建 `tauri.js` 含 `aiSuggest`/`invokeAiOp`，T8 若也需在 `tauri.js` 加 `checkUpdate`/`installUpdate`，并行分派可能覆盖。**缓解**：T8 在 `UpdateCard.jsx` 内直接 `import { invoke } from '@tauri-apps/api/core'` 调 `invoke('check_update')`，不碰 `tauri.js`；或主会话在 T7 verified 后再让 T8 合并 `tauri.js`。推荐前者。
- **DB 路径展示**：v1.0.0 不许动 `src-tauri`，无法新增 `get_db_path` 命令。**缓解**：前端硬编码 macOS 路径 `~/Library/Application Support/com.rut0.datakit/ruT0datakit.db` + 说明文案；T9 可补命令。
- **「在 Finder 中显示」**：Tauri v2 的 `shell` 插件或 `opener` 命令需注册。v1.0.0 若未注册 shell 插件，按钮可 `disabled` 或 `alert` 提示「v1.1+ 完善」。**不要为此加 `src-tauri` 改动**。
- **能力面板文案核对**：T2/T3 已实现 4 个 Panel，T8 只核对/微调文案，不改交互。

## depends_on
[T2, T6]（T2 verified_complete @ ba5e0d0 系列；T6 verified_complete @ 8a976f6）

## status
planned
