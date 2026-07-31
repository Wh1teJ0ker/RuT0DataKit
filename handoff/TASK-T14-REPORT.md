---
id: T14
title: 前端配置集中 + IPC 收口 + docs/02 同步
type: refactor
reported_status: verified_complete
---

# TASK-T14-REPORT — 前端组件抽取 + 配置集中 + IPC 收口

## goal

在 T13 基础上完成 v1.0.0 架构审计剩余前端项：集中 PAGE_SIZE、把 UpdateCard 的 raw `invoke()` 收口到 `tauri.js`、同步 docs/02 对 `state/` 的描述。纯重构，不改外部行为。

## implemented_changes

- `frontend/src/constants.js`
  - 新增 `export const PAGE_SIZE = 50;`（含 JSDoc 说明用途与集中化原则），位置在 `DEV_STATUS` 之后。未动 `APP_VERSION` / `DEV_STATUS`。
- `frontend/src/App.jsx`
  - 删除顶层 `const PAGE_SIZE = 50;`，改为 `import { PAGE_SIZE } from "./constants";`（紧随 `./tauri` import 之后）。`:31` / `:55` 的 `PAGE_SIZE` 引用变量名不变。
- `frontend/src/state/factory.js`
  - 顶部加 `import { PAGE_SIZE } from "../constants";`。
  - `createEmptySheet` 与 `createSheetFromImport` 两处 `pageSize: 50` 改为 `pageSize: PAGE_SIZE`。
- `frontend/src/components/DataTable.jsx`
  - 顶部加 `import { PAGE_SIZE } from "../constants";`。
  - `:65` 注释 `pageSize=50` → `pageSize=PAGE_SIZE`。
  - `:208` `sheet.pageSize || 50` → `sheet.pageSize || PAGE_SIZE`。
- `frontend/src/tauri.js`
  - 顶部模块说明新增 T14 行（checkUpdate / installUpdate 收口）。
  - 新增 `export function checkUpdate()` + `export function installUpdate()`，含 JSDoc，风格对齐 `detectTshark` / `saveTsharkPath` 等。未改其他导出。
- `frontend/src/components/settings/cards/UpdateCard.jsx`
  - 删除 `import { invoke } from "@tauri-apps/api/core";`，改为 `import { checkUpdate, installUpdate } from "../../../tauri";`。
  - `handleCheck` 内 `await invoke("check_update")` → `await checkUpdate()`。
  - `handleInstall` 内 `await invoke("install_update")` → `await installUpdate()`。
  - 删除「直接内联 invoke 避免 T7 覆盖」过时注释，改为说明已收口到 tauri.js。
  - `commands.rs` 注释路径改为 `commands/update.rs`。
- `docs/02-技术设计文档.md`
  - `:107` 目录树 `state.js` 行改为「薄 barrel」描述，并在其下补 `state/` 子模块四行（constants/factory/reducer/AppContext）。
  - `:355` 「`state.js` 顶层 `useReducer`」改为「`state/AppContext.jsx` 的 `AppProvider` 内 `useReducer`，`state.js` 现为薄 barrel」。
  - `:362` `aiPanel: { visible: true, ... }` → `{ visible: false, ... }`。

## verification_run

```
pnpm --prefix frontend install --frozen-lockfile
pnpm --prefix frontend build
```

附加验收 grep（acceptance_criteria §2/§3）：

```
grep -rn "PAGE_SIZE = 50\|pageSize: 50\||| 50" frontend/src/
grep -rn "from \"@tauri-apps/api/core\".*invoke\|import.*invoke.*core" frontend/src/components/
```

## verification_results

`pnpm --prefix frontend install --frozen-lockfile`：

```
Lockfile is up to date, resolution step is skipped
Already up to date

[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: esbuild@0.19.3

Run "pnpm approve-builds" to pick which dependencies should be allowed to run scripts.
```

> `ERR_PNPM_IGNORED_BUILDS` 仅是 esbuild 构建脚本审批提示，非锁定/安装失败，依赖已就绪。属既有环境状态，非本轮引入。

`pnpm --prefix frontend build`：

```
vite v5.0.0 building for production...
transforming...
✓ 3078 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                     0.40 kB │ gzip:   0.27 kB
dist/assets/index-fDToV_Mp.css      0.28 kB │ gzip:   0.16 kB
dist/assets/index-CI8UO9pY.js       7.45 kB │ gzip:   1.98 kB
dist/assets/index-D4FV0K4h.js   1,092.66 kB │ gzip: 348.17 kB

(!) Some chunks are larger than 500 kB after minification. Consider:
- Using dynamic import() to code-split the application
- Use build.rollupOptions.output.manualChunks to improve chunking
- Adjust chunk size limit via build.chunkSizeWarningLimit
✓ built in 2.44s
```

> chunk > 500kB 警告为既有大型依赖（antd 等）所致，非本轮引入，out_of_scope。

grep `PAGE_SIZE = 50\|pageSize: 50\||| 50`：仅命中 `frontend/src/constants.js:19` 一处定义。其余引用均改为 `PAGE_SIZE` 常量。

grep raw `invoke` import in `frontend/src/components/`：零命中（UpdateCard 不再直接 import invoke）。

## docs_updated

- `docs/02-技术设计文档.md`（目录树 + `useReducer` 描述 + `aiPanel.visible` 三处同步，使文档与 T13 后的 `state/` 子模块结构一致）。

## scope_deviation

none。仅在 in_scope 7 项内改动，未抽 DataTable selection / TopToolbar handleImport / ExportModal，未改 state/ 子模块内部逻辑、Rust 后端、tauri.conf.json/package.json/capabilities，未补新功能。

## reported_status

verified_complete（建议状态，最终由主会话判定）
