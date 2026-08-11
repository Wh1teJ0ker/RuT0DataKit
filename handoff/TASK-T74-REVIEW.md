# T74 REVIEW — CryptoPanel 哈希 UI

```yaml
task_id: T74
reviewer_verdict: review_passed
reviewed_at: 2026-08-11
commits_reviewed: [12669e5]
```

## 验收对照

| 验收项 | 结果 | 证据 |
|---|---|---|
| CryptoPanel 显示操作类型选择（Base64 编/解码 / MD5 / SHA1 / SHA256） | PASS | `frontend/src/components/panels/CryptoPanel.jsx`（commit 12669e5）`<Form.Item label="操作类型" name="op">` options 含 5 项：`base64_encode / base64_decode / md5 / sha1 / sha256`，`initialValue="base64_encode"` |
| 选 Base64 时按 op 分发 mode；选哈希时调 hashColumn | PASS | `handleExecute`：`op.startsWith("base64_")` → `base64Column(sheet.id, column, mode)`（mode = encode/decode）；否则 `hashColumn(sheet.id, column, op)`。哈希分支不传 mode，符合"哈希不可逆无 mode" |
| algorithm 字符串与后端 serde lowercase 对齐 | PASS | 前端传 `op` 本身（`"md5"/"sha1"/"sha256"`）；后端 `HashAlgorithm` `#[serde(rename_all = "lowercase")]`（`src-tauri/src/commands/columns.rs:345-351`），枚举 MD5/SHA1/SHA256 → 反序列化为 lowercase。字符串一一对应 |
| hashColumn wrapper 在 tauri.js 导出，invoke "hash_column"，参数 { sheetId, column, algorithm } | PASS | `frontend/src/tauri.js`（commit 12669e5，紧邻 `base64Column`）`export function hashColumn(sheetId, column, algorithm) { return invoke("hash_column", { sheetId, column, algorithm }); }`，附 JSDoc 注明 HashResult camelCase 返回 |
| 执行成功后刷新当前页数据（getSheetData + SET_SHEET_DATA） | PASS | `handleExecute` 成功分支：`const data = await getSheetData(sheet.id, { page, pageSize }); dispatch({ type: "SET_SHEET_DATA", payload: data });` |
| 执行成功后刷新 undo 栈（listUndoableOperations + SET_UNDO_STACK） | PASS | 同分支：`const ops = await listUndoableOperations(sheet.id); dispatch({ type: "SET_UNDO_STACK", payload: ops });` |
| message.success 显示 affected/skipped 数量 | PASS | `message.success(\`${label}完成：${res.affected} 行，跳过 ${res.skipped ?? 0}\`)`；label 按 op 动态生成（Base64 编码/解码 / MD5 哈希 / SHA1 哈希 / SHA256 哈希） |
| 哈希操作 UI 提示不可逆但可撤销 | PASS | `const isHashOp = currentOp === "md5" || "sha1" || "sha256"`（`Form.useWatch("op", form)` 驱动）；`isHashOp && <Alert type="warning" showIcon message="哈希不可逆" description="哈希操作无法解码还原，但可通过撤销恢复原文。" />` |
| docstring 更新反映 MD5/SHA1/SHA256 已实现 | PASS | header 注释（L14-23）改写为"当前承载：Base64 编/解码 + MD5/SHA1/SHA256 哈希（就地变换，可撤销；哈希不可逆，但通过 before_snapshot 回写可撤销恢复原文）"，并标注操作模式为二级选择 |
| pnpm --prefix frontend build exit 0 | PASS | 复跑：`vite v5.0.0 building... ✓ 3083 modules transformed... ✓ built in 2.41s`，EXIT=0 |

## 验证命令执行结果

```
$ pnpm --prefix frontend build
> rut0-data-kit-frontend@1.1.4 build
> vite build
vite v5.0.0 building for production...
✓ 3083 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                     0.40 kB │ gzip:   0.27 kB
dist/assets/index-fDToV_Mp.css      0.28 kB │ gzip:   0.16 kB
dist/assets/index-DyjYciMA.js       7.45 kB │ gzip:   1.98 kB
dist/assets/index-DbE0L0NV.js   1,196.91 kB │ gzip: 379.95 kB
✓ built in 2.41s
EXIT=0
```

唯一 warning 是 chunk 体积 > 500kB（既有 antd 全量打包，非本次引入，不阻塞）。lint script 未配置（package.json 无 `lint`），HANDOFF `verification_commands` 只要求 build，故不构成缺陷。

## 安全合规

- 无凭据字面量 / 密钥 / token：diff 仅含 UI 表单与 IPC wrapper，无敏感数据。
- 不改后端 IPC 契约：T73 的 `hash_column` Tauri 命令、`HashAlgorithm` 枚举、`HashResult` 结构未在本次 commit 改动。前端 wrapper 调用契约（`hash_column` / `{ sheetId, column, algorithm }` / `{affected, skipped}` camelCase）与后端 `#[tauri::command] hash_column(sheet_id: i64, column: String, algorithm: HashAlgorithm) -> Result<HashResult>` + `HashResult{affected, skipped}`（camelCase 序列化）完全对齐。
- 不创建 tag、不 merge main：`git tag --points-at 12669e5` 空；`git log --merges 12669e5^..12669e5` 空。
- commit 单一逻辑目的，Conventional Commits 格式（`feat(frontend): T74 ...`）。

## Scope 核查

`git show 12669e5 --name-only` 仅 2 文件：
- `frontend/src/components/panels/CryptoPanel.jsx`
- `frontend/src/tauri.js`

无越界：未改 `SidePanel.jsx` / `TopToolbar.jsx` / `reducer.js` / `state/factory.js` / `capabilities/default.json` / 任何后端 `.rs`。crypto capability 已在 v1.1.2 注册，本次无需改。scope_check: none。

## 文档同步核查

`docs/02-技术设计文档.md` 已由 T73 同步覆盖 R3 hash 内容：
- L157：`commands::columns` 表加入 `hash_column`，标注"v1.1.4 R3 新增"
- L384：`list_undoable_operations` kind 白名单含 `hash_column`
- L450：`hash_column` 命令签名 + 委托 `base64_transform_column_cells` + 撤销
- L476：`enum HashAlgorithm { MD5, SHA1, SHA256 }`
- L489：v1.1.4 R3 实现要点段（复用闭包 / hex 小写 / skipped=0 / 依赖声明 / "前端 UI 由 T74 在 CryptoPanel 加入哈希算法选择入口"）

即文档已预告 T74 的前端入口由本任务承接。前端 UI 层的行为变化（操作类型二级选择 + 不可逆 Alert hint）属于面板级交互细节，`02-技术设计文档.md` 的 IPC 契约层文档不要求逐按钮文案同步；`01-页面与交互说明.md` 的 CryptoPanel 描述仍停留在 v1.0.0 占位级（未覆盖 v1.1.2 Base64 迁入），属既存文档缺口，不属 T74 引入的新缺口，不阻塞本任务。docs_check: synced（本任务切片范围内无新缺口）。

## 缺陷清单

无阻塞问题。无 blocker / major / minor 缺陷。

nit（非阻塞，不构成 reject）：
- `01-页面与交互说明.md` 中 CryptoPanel 仍为 v1.0.0 占位描述，未随 v1.1.2 Base64 迁入及 v1.1.4 R3 哈希扩展更新。此为跨版本既存缺口，建议后续版本（如 v1.1.4 R3 文档同步任务或 Release QA 增量）统一补齐面板交互说明，不影响 T74 acceptance。

## 最终意见

T74 忠实实现了 HANDOFF 的 goal：在既有 Base64 编解码 UI 基础上扩展为「操作类型 + 算法」二级选择，新增 `hashColumn` IPC wrapper，执行后刷新当前页 + undo 栈，并提示哈希不可逆但可撤销。全部 9 条 acceptance_criteria 满足；`pnpm --prefix frontend build` exit 0；algorithm 字符串与后端 `HashAlgorithm` serde lowercase 对齐；hashColumn wrapper 签名 / invoke 命令名 / 参数对象 / 返回 camelCase 全部与 T73 后端契约一致；无凭据、无后端改动、无 tag / merge、无 scope 越界；技术设计文档已同步覆盖 R3 hash_column 与前端 T74 入口。

verdict: review_passed
