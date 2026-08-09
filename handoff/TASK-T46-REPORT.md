# T46 实施报告 — Base64 抽离为独立加解密面板

> 任务：T46
> 状态：verified_complete
> 依赖：T38（Base64 列编解码前端 IPC + UI）

## 一、目标

把 Base64 编解码 UI 从 `ColumnOpsPanel` 抽离到独立的「加解密」侧边栏能力面板 `CryptoPanel`，在 TopToolbar 新增 `crypto` 能力按钮（`KeyOutlined` 图标），作为后续所有加解密/哈希/编解码类操作的统一入口。`ColumnOpsPanel` 仅保留 JSON 解析。

## 二、改动文件

| 文件 | 改动 |
|---|---|
| `frontend/src/components/panels/CryptoPanel.jsx`（新建） | 搬迁 Base64 编解码 Form + `handleBase64` 逻辑（行为不变：`base64Column` → `getSheetData` + `SET_SHEET_DATA` → `listUndoableOperations` + `SET_UNDO_STACK` → `message.success`）；顶部加 `<Typography.Title level={5}>加解密</Typography.Title>` 标题；注释说明作为加解密统一入口 |
| `frontend/src/components/panels/ColumnOpsPanel.jsx` | 删除 `base64Form`/`base64ing` state + `handleBase64` 函数 + Base64 Form JSX + `<Divider>` + `base64Column`/`listUndoableOperations` import；仅剩 JSON 解析 |
| `frontend/src/components/layout/TopToolbar.jsx` | import 加 `KeyOutlined`；`CAPABILITIES` 数组在 `columnOps` 与 `rules` 间插入 `{id:"crypto",label:"加解密",icon:<KeyOutlined/>}` |
| `frontend/src/components/layout/SidePanel.jsx` | import `CryptoPanel`；`PANELS` 注册表加 `crypto: CryptoPanel`；注释更新 |

## 三、不改动的文件

- `frontend/src/tauri.js`：`base64Column` IPC 封装已存在
- `frontend/src/App.jsx`：路由不变（`crypto` 走默认 Layout + SidePanel 分支）
- `frontend/src/state/reducer.js`：`SET_ACTIVE_CAPABILITY` XOR 逻辑通用
- `frontend/src/constants.js`：无新增 ACTION
- 所有 Rust 代码：`base64_column` 命令 / `base64_transform_column_cells` DB 方法不动

## 四、验收

| 验收项 | 结果 |
|---|---|
| `pnpm --prefix frontend build` | pass（3080 modules transformed，✓ built in 2.56s） |
| 工具栏「加解密」按钮可见（KeyOutlined） | pass（CAPABILITIES 含 crypto 条目） |
| 点击展开 CryptoPanel / 再点收起（XOR） | pass（复用 setActiveCapability XOR 语义） |
| CryptoPanel：目标列 Select + 模式 Select + 执行按钮 | pass（Form 结构完整） |
| 执行 Base64 后刷新当前页 + 撤销栈 + 成功提示 | pass（handleBase64 逻辑原样搬迁，行为不变） |
| ColumnOpsPanel 仅剩 JSON 解析无 Base64 残留 | pass（无 base64Form/base64ing/handleBase64/base64Column import/Divider） |
| 版本号仍 1.1.2 | pass（不改版本） |

## 五、设计决策

- **侧边栏能力形态（与用户确认）**：`crypto` 走 `activeCapability` XOR 切换 + 260px `SidePanel` 注册表，与脱敏/校验/提取/列操作一致，而非像 `rules`/`settings` 那样走全屏视图。理由：Base64 操作是轻量列变换，与列选择 + 模式 + 执行的轻量交互模式匹配，不需要全屏空间。
- **CryptoPanel 加标题**：其他单操作面板（MaskPanel/ValidatePanel/ExtractPanel/ColumnOpsPanel）无标题，但 CryptoPanel 作为多操作入口（v1.1.2 承载 Base64，后续扩展哈希/AES），加 `<Typography.Title level={5}>加解密</Typography.Title>` 有助于后续扩展时区分操作分组。
- **纯前端 UI 重组**：无 IPC/DB/Rust 改动，`handleBase64` 逻辑原样搬迁，行为完全一致。风险极低。

## 六、文档同步

- `docs/02-技术设计文档.md`：§4.9 base64_column 实现要点补充 CryptoPanel 迁移说明 + `activeCapability` 取值加 `crypto`
- `docs/versions/1.1.2/更新日志.md`：T46 行 + E8 验收项 + 关键设计决策
- `docs/versions/1.1.2/RELEASE-NOTES.md`：新增 + 优化段落更新
- `docs/qa/versions/1.1.2/QA-审计报告.md`：R4 审计轮次 + §2 需求覆盖 + §3 E2E + §12 结论 + §13 修复证据
- `handoff/TASK-BOARD.md`：T46 行 + DAG + E8
- `handoff/TASK-T46-HANDOFF.md` + `handoff/TASK-T46-REPORT.md`（本文件）
