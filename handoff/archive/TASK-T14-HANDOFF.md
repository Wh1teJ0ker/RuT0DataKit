---
id: T14
title: 前端组件抽取 + 配置集中 + IPC 收口
depends_on: [T13]
version: 1.0.0
type: refactor
status: planned
---

# TASK-T14-HANDOFF — 前端组件抽取 + 配置集中 + IPC 收口

## 目标（goal）

在 T13 已建立的 `state/` 模块化 + `AppContext` 基础上，完成 v1.0.0 架构审计剩余前端项：**集中 PAGE_SIZE 配置、把 UpdateCard 的 raw `invoke()` 旁路收口到 `tauri.js`、同步 docs/02 对 `state/` 子模块的描述**。纯重构，不改任何外部行为。

## 背景与审计结论（background）

T13 落地后，主会话审计确认以下前端遗留项（coder 无需重新发现）：

1. **`PAGE_SIZE = 50` 在 4 处重复硬编码**：
   - `frontend/src/App.jsx:13` 顶层 `const PAGE_SIZE = 50;`
   - `frontend/src/state/factory.js:28` + `:48` Sheet 工厂 `pageSize: 50`
   - `frontend/src/components/DataTable.jsx:208` `pageSize: sheet.pageSize || 50`
   - 审计结论：应集中到 `frontend/src/constants.js`（已有 `APP_VERSION`/`DEV_STATUS`，天然是配置集中点），各处改为 `import { PAGE_SIZE } from "../constants"`。
2. **`UpdateCard.jsx` raw `invoke()` 旁路**：
   - `frontend/src/components/settings/cards/UpdateCard.jsx:4` `import { invoke } from "@tauri-apps/api/core";`
   - `:24` `await invoke("check_update")` / `:38` `await invoke("install_update")`
   - 审计结论：违反「前端唯一 IPC 出口是 `tauri.js`」原则（docs/02 明确）。应在 `tauri.js` 新增 `checkUpdate()` / `installUpdate()` 封装，UpdateCard 改为 import 调用。旁路原注释「避免与 T7 的 aiSuggest/invokeAiOp 封装产生文件覆盖」已过时（T7 早已落库，无覆盖风险）。
3. **docs/02 对 `state.js` 的描述过时**：
   - `docs/02-技术设计文档.md:107` 目录树 `state.js # useReducer 全局 state（单一真相源）` —— T13 后实际是 `state.js`（薄 barrel）+ `state/{constants,factory,reducer,AppContext}.jsx` 子模块。
   - `docs/02:355` 「`state.js` 顶层 `useReducer`」—— 实际 `useReducer` 在 `AppContext.jsx` Provider 内。
   - `docs/02:362` `aiPanel: { visible: true, ... }` —— 实际 T13 后 `visible: false`（前序会话有意改动，T13 已裁定保留）。
   - 审计结论：T14 同步 docs/02 这三处描述，使文档与 `state/` 结构一致。

## 范围（scope）

### in_scope

1. **`frontend/src/constants.js` 新增 `PAGE_SIZE`**：
   - 在现有 `APP_VERSION` / `DEV_STATUS` 之后新增 `export const PAGE_SIZE = 50;`（含 JSDoc 注释说明用途）。
   - 不改 `APP_VERSION` / `DEV_STATUS` 现有值。
2. **`frontend/src/App.jsx` 改用集中常量**：
   - 删 `:13` 的 `const PAGE_SIZE = 50;`，改为 `import { PAGE_SIZE } from "./constants";`。
   - `:31` / `:55` 引用不变（变量名仍是 `PAGE_SIZE`，来源改为 import）。
3. **`frontend/src/state/factory.js` 改用集中常量**：
   - `:28` + `:48` 的 `pageSize: 50` 改为 `pageSize: PAGE_SIZE`，顶部加 `import { PAGE_SIZE } from "../constants";`。
4. **`frontend/src/components/DataTable.jsx` 改用集中常量**：
   - `:208` `pageSize: sheet.pageSize || 50` 改为 `sheet.pageSize || PAGE_SIZE`，顶部加 `import { PAGE_SIZE } from "../constants";`。
   - `:65` 注释 `pageSize=50` 改为 `pageSize=PAGE_SIZE`（注释对齐）。
5. **`frontend/src/tauri.js` 新增 `checkUpdate()` / `installUpdate()`**：
   - 新增 `export function checkUpdate() { return invoke("check_update"); }`（含 JSDoc，对齐现有 `detectTshark` 等风格）。
   - 新增 `export function installUpdate() { return invoke("install_update"); }`。
   - 不改现有导出函数。
6. **`frontend/src/components/settings/cards/UpdateCard.jsx` 收口**：
   - 删 `:4` `import { invoke } from "@tauri-apps/api/core";`。
   - 加 `import { checkUpdate, installUpdate } from "../../../tauri";`（路径以 cards/ 为基准）。
   - `:24` `await invoke("check_update")` → `await checkUpdate()`。
   - `:38` `await invoke("install_update")` → `await installUpdate()`。
   - 删 `:9-10` 关于「直接内联 invoke 避免 T7 覆盖」的过时注释，改为说明已收口到 tauri.js。
   - `:16` 注释 `见 src-tauri/src/commands.rs UpdateStatus` → 改为 `见 src-tauri/src/commands/update.rs UpdateStatus`（路径已对齐 T11 拆分后结构）。
7. **`docs/02-技术设计文档.md` 同步三处**：
   - `:107` 目录树 `state.js` 行：改为 `state.js`（barrel）+ 下方加 `state/` 子模块四行（constants/factory/reducer/AppContext）。
   - `:355` 「`state.js` 顶层 `useReducer`」改为「`state/AppContext.jsx` 的 `AppProvider` 内 `useReducer`，`state.js` 现为薄 barrel」。
   - `:362` `aiPanel: { visible: true, ... }` → `{ visible: false, ... }`（默认折叠，前序会话有意改动）。
   - **不改 docs/02 其他任何内容**（仅这三处同步）。

### out_of_scope

- **不抽 DataTable 的 selection 逻辑到独立 hook**（224 行可接受，selection 与表格渲染强耦合，抽取收益小于风险；审计结论：保留）。
- **不抽 TopToolbar 的 handleImport 到独立 hook**（import 编排与按钮强耦合，保留）。
- **不重构 ExportModal 的列选择块**（前序会话已做列选择，非重复代码；审计结论：保留）。
- **不改 `state/` 子模块内部逻辑**（T13 已落，本轮只改 docs 描述 + factory.js 一行 import）。
- **不改 Rust 后端**（T12 负责）。
- **不改 `tauri.conf.json` / `package.json` / capabilities**（配置不动）。
- **不补新功能 / 不改行为**（纯集中化 + 收口 + 文档同步）。

## 验收标准（acceptance_criteria）

1. `pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build` 通过。
2. `grep -rn "PAGE_SIZE = 50\|pageSize: 50\||| 50" frontend/src/` 仅命中 `constants.js` 一处定义（其他位置均改为引用 `PAGE_SIZE`）。
3. `grep -rn "from \"@tauri-apps/api/core\".*invoke\|import.*invoke.*core" frontend/src/components/` 零命中（UpdateCard 不再直接 import invoke）。
4. `UpdateCard.jsx` 通过 `tauri.js` 的 `checkUpdate()` / `installUpdate()` 调用，无 raw `invoke()`。
5. `tauri.js` 导出 `checkUpdate` + `installUpdate` 两个函数。
6. docs/02 三处描述与 `state/` 实际结构一致（barrel + 4 子模块 + AppProvider + visible:false）。
7. `App.jsx` / `factory.js` / `DataTable.jsx` / `UpdateCard.jsx` 行为字节级不变（仅 import 来源 + 常量引用方式改变）。

## 验证命令（verification_commands）

```sh
pnpm --prefix frontend install --frozen-lockfile
pnpm --prefix frontend build
```

## 风险与回退（risks）

- **风险**：`PAGE_SIZE` import 路径写错导致 build 失败。
- **缓解**：coder 改完后立即跑 `pnpm --prefix frontend build` 确认；路径以 `constants.js` 相对各文件为准（App.jsx 同级、factory.js 在 state/、DataTable.jsx 在 components/、UpdateCard 在 components/settings/cards/）。
- **风险**：UpdateCard 的 `tauri.js` 相对路径写错（cards/ 嵌套较深）。
- **缓解**：路径为 `../../../tauri`（cards/ → settings/ → components/ → src/）。
- **回退**：`git revert` 本次 commit 即可恢复（纯重构，无行为变更）。

## 交付物（deliverables）

- 修改 `frontend/src/constants.js`（+PAGE_SIZE）。
- 修改 `frontend/src/App.jsx`（import 替换本地 const）。
- 修改 `frontend/src/state/factory.js`（2 处 pageSize 引用）。
- 修改 `frontend/src/components/DataTable.jsx`（1 处 pageSize + 注释）。
- 修改 `frontend/src/tauri.js`（+checkUpdate/installUpdate）。
- 修改 `frontend/src/components/settings/cards/UpdateCard.jsx`（收口 IPC）。
- 修改 `docs/02-技术设计文档.md`（3 处描述同步）。
- `handoff/TASK-T14-REPORT.md`（按 orchestrator-workflow 04-coder-spec 模板）。

## scope_deviation

coder 不得越界。若发现其他 raw `invoke()` 旁路或配置硬编码，**先在 REPORT 记录、不修**，由主会话裁定是否追加 scope。任何「顺手抽组件 / 改文案 / 优化 UI」均视为越界。
