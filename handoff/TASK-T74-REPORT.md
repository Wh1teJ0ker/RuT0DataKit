# T74 REPORT — CryptoPanel 哈希 UI

task_id: T74
reported_status: implemented_and_verified
commits: [12669e5]

## implemented_changes
- `frontend/src/tauri.js`：紧邻 `base64Column` 新增 `hashColumn(sheetId, column, algorithm)` wrapper，invoke `"hash_column"`，参数 `{ sheetId, column, algorithm }`，algorithm 取值 `"md5"/"sha1"/"sha256"`（与后端 HashAlgorithm serde lowercase 对齐）。附 JSDoc 注明 HashResult camelCase 返回。
- `frontend/src/components/panels/CryptoPanel.jsx`：
  - 操作模式从「单选 Base64 编/解码」重构为「目标列 + 操作类型(op)」二级选择，op options = `base64_encode / base64_decode / md5 / sha1 / sha256`，统一「执行」按钮按 op 分发。
  - `handleExecute`：`op` 以 `base64_` 开头 → `base64Column(..., mode)`；md5/sha1/sha256 → `hashColumn(..., op)`。成功后 `getSheetData + SET_SHEET_DATA` 刷新当前页 + `listUndoableOperations + SET_UNDO_STACK` 刷新撤销栈 + `message.success` 显示 affected/skipped。
  - 哈希操作动态显示 antd `Alert` 提示「哈希不可逆，但可通过撤销恢复原文」。
  - 同步 header docstring：反映 MD5/SHA1/SHA256 已落地，标注哈希可撤销机制。
- 复用既有 `base64Column / getSheetData / listUndoableOperations` wrapper 与 `SET_SHEET_DATA / SET_UNDO_STACK` action；未改后端、reducer、capability 注册。

## verification_run
```
$ pnpm --prefix frontend build
> rut0-data-kit-frontend@1.1.4 build /Users/joker/code/RuT0DataKit/frontend
> vite build
vite v5.0.0 building for production...
transforming...
✓ 3083 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                     0.40 kB │ gzip:   0.27 kB
dist/assets/index-fDToV_Mp.css      0.28 kB │ gzip:   0.16 kB
dist/assets/index-DyjYciMA.js       7.45 kB │ gzip:   1.98 kB
dist/assets/index-DbE0L0NV.js   1,196.91 kB │ gzip: 379.95 kB
✓ built in 2.66s
```
lint：package.json 未配置 `lint` script，未运行。

## verification_results
- pnpm --prefix frontend build: pass（exit 0，✓ built in 2.66s）
- pnpm --prefix frontend lint: 未运行（前端未配置 lint script）

## scope_deviation
none

## commit_summary
- feat(frontend): T74 CryptoPanel 哈希算法 UI + hashColumn IPC wrapper （12669e5）

## 自验结论
状态：implemented_and_verified（最终 verified_complete 判定权在主会话）
