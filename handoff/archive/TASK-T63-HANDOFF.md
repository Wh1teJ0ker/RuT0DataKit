# TASK-T63-HANDOFF — 翻页状态保留（columnOrder + statusHighlights）

## 背景

当前 `SET_SHEET_DATA` action（翻页 / 刷新当前页时触发）有两个状态丢失问题：

1. **columnOrder 重置**：reducer.js:136 `columnOrder: [...headers]` 每次翻页都把列顺序重置为原始 headers 顺序。用户拖拽重排列后，翻一页就丢失重排。
2. **statusHighlights 丢失**：reducer.js:122-128 `toRowObjects` 生成新行时 `status: "default"`，`SET_SHEET_DATA` 全替换 `rows`，新行不读取 `statusHighlights` map。脱敏/校验/提取后翻页再翻回，行高亮消失。`statusHighlights` map 本身通过 `...s` 保留了，但新 rows 不从中回填 `status`。

T63 修复这两个问题，使翻页不丢失列重排和行高亮状态。

## 唯一可信源

- 仓库根：`/Users/joker/code/RuT0DataKit`
- 工作分支：从 `main`（当前 HEAD `752838f`）切 `fix/t63-pagination-state-preservation`
- 基线：`752838f`

## 需求规格

### 1. `frontend/src/state/reducer.js` — `SET_SHEET_DATA` action（约第 113-144 行）

#### columnOrder 保留
- **当前**（第 136 行）：`columnOrder: [...headers]` — 每次翻页重置
- **改为**：如果 `s.columnOrder` 已存在且覆盖当前所有 headers（即 `headers.every(h => s.columnOrder.includes(h))` 且 `s.columnOrder.every(h => headers.includes(h))`），则保留 `s.columnOrder`；否则用 `[...headers]`（首次加载或 headers 变化时）
- 这样用户拖拽重排列后翻页不会丢失，但导入新文件（headers 变化）会正确重置

#### statusHighlights 回填
- **当前**：`toRowObjects` 返回的行 `status: "default"`，不查 `statusHighlights`
- **改为**：在 `SET_SHEET_DATA` 中，`toRowObjects` 生成 rows 后，遍历 rows 查 `s.statusHighlights[row.key]`，如果存在则覆盖 `status`
- 代码示例：
  ```js
  const rows = toRowObjects(...);
  // 回填行状态高亮（翻页后从 statusHighlights map 恢复脱敏/校验/提取标记）
  const rowsWithStatus = rows.map((r) => {
    const savedStatus = s.statusHighlights?.[r.key];
    return savedStatus ? { ...r, status: savedStatus } : r;
  });
  // 使用 rowsWithStatus 替代 rows
  ```

### 2. 不需要修改的文件
- `tauri.js`（`toRowObjects` 保持 `status: "default"`，回填在 reducer 层做）
- `factory.js`（工厂初始化不受影响）
- 任何后端文件
- 任何文档文件

## 验收条件

1. `pnpm --dir frontend build` 成功
2. 如果有前端测试（`pnpm --dir frontend test` 或类似），全部通过
3. Conventional Commits（`fix(state): T63 翻页保留 columnOrder + statusHighlights`）

## 手工验证场景（在 REPORT 中说明如何验证）

1. 导入 CSV → 拖拽重排列 → 翻到第 2 页 → 翻回第 1 页 → 列顺序应保持用户重排
2. 导入 CSV → 校验某列（行高亮为 invalid）→ 翻到第 2 页 → 翻回第 1 页 → 高亮应仍在
3. 导入新文件（不同 headers）→ columnOrder 应重置为新 headers 顺序

## 交付物

1. 工作分支 `fix/t63-pagination-state-preservation` 上的 commit
2. `handoff/TASK-T63-REPORT.md`

## 约束

- 只改 `frontend/src/state/reducer.js`
- 不改后端文件
- 不改 `tauri.js` / `factory.js`
- 不改 `docs/02-技术设计文档.md`（T67 统一同步）
- 不改 `App.jsx` / `DataTable.jsx`（T64 负责）
- Mimosa 安全约束：不硬编码凭据
