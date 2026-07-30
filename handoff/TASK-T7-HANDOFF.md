# TASK-T7-HANDOFF — AI 占位 IPC 契约（主会话 2026-07-30 重派版）

> 本 HANDOFF 已与最新代码基线（T6 verified_complete @ 8a976f6）对齐。T7 的 `src-tauri/src/commands.rs` 修改基线为 T6 产物（`UpdateStatus` + `check_update`/`install_update` 两个命令），请在 T6 产物之上扩展，不要破坏 T6。

## task_id
T7

## goal
落地 `ai_suggest` / `invoke_ai_op` 两个 `#[tauri::command]` 空实现 IPC 契约（返回 `v1.1+` 错误），定义 `AiContext`/`AiSuggestion` 类型（嵌套 struct 带 `camelCase`），注册到 `generate_handler!`，并提供前端 `tauri.js` 封装（`aiSuggest`/`invokeAiOp`）与 `AiPanel.jsx` 调用入口。

## in_scope
允许新增/修改的文件：

- `src-tauri/src/commands.rs`（当前为 T6 产物的单文件，内含 `UpdateStatus` + `check_update`/`install_update`）：新增类型与命令：
  - `AiContext` struct：`#[derive(Debug, Clone, Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]`，字段可精简占位，例如：
    - `sheet_id: Option<i64>`
    - `selection: Option<Vec<i64>>`（选中行 id）
    - `prompt: Option<String>`
    - 关键是 struct 存在且 camelCase。
  - `AiSuggestion` struct：同上 serde 派生 + `#[serde(rename_all = "camelCase")]`，字段如 `suggestion: String`、`confidence: f64`（占位）。
  - `ai_suggest(context: AiContext) -> Result<AiSuggestion, String>`：返回 `Err("ai_suggest not implemented until v1.1+".to_string())`。
  - `invoke_ai_op(op: String, params: serde_json::Value) -> Result<serde_json::Value, String>`：返回 `Err("invoke_ai_op not implemented until v1.1+".to_string())`，接受任意 `op: String` + `params: Value`。
  - 命令在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册（T6 已注册 `check_update`/`install_update`，T7 追加 `ai_suggest`/`invoke_ai_op`，不要移除 T6 注册项）。
- `frontend/src/tauri.js`（新增，T1/T6 均未建此文件）：前端 invoke 封装层，对齐 IPC 契约。导出：
  - `aiSuggest(context)` → `invoke('ai_suggest', { context })`
  - `invokeAiOp(op, params)` → `invoke('invoke_ai_op', { op, params })`
  - **本任务只实现 `aiSuggest`/`invokeAiOp` 封装**，其它封装（`importFile`/`getSheetData`/`checkUpdate`/`installUpdate` 等）留给 T5/T8，本任务不要实现其真实逻辑（可留 TODO 注释，但不要写 invoke 调用）。
  - 使用 `import { invoke } from '@tauri-apps/api/core'`（Tauri v2 的 invoke 路径）。
- `frontend/src/components/AiPanel.jsx`（扩展 T2 的 AiPanel 占位）：加一个「调用 ai_suggest 测试」`Button`（或 AiPanel 加载时 `useEffect` 自动调一次 `aiSuggest`），调用 `.catch` 错误并展示「AI 能力 v1.1+ 释放」文案，UI 不崩溃。保留 T2 已有折叠/展开逻辑与样式，不要破坏 `visible`/`setVisible` 双向绑定。
- `src-tauri/src/lib.rs`：扩展 `generate_handler!` 追加 `commands::ai_suggest`、`commands::invoke_ai_op`。

## out_of_scope
明确不许动的：

- **不许实现真实 AI 逻辑**（v1.4+）。
- **不许动 DB 层**（T4，`src-tauri/src/db/` 全部）。
- **不许动 updater**（T6，`commands.rs` 中 `UpdateStatus`/`check_update`/`install_update` 保持原样，不许改字段或行为）。
- **不许动布局组件** `frontend/src/components/layout/TopToolbar.jsx`、`SidePanel.jsx`、`Workbench.jsx`、`panels/*`（T2/T3 产物，本任务只扩展 `AiPanel.jsx` 的调用逻辑）。
- **不许实现导入**（T5，`tauri.js` 中不要写 `importFile`/`getSheetData` 的真实 invoke 调用）。
- **不许动 docs/**、**不许动 handoff/ 其它任务文件**。

## acceptance_criteria
1. `src-tauri/src/commands.rs` 定义 `AiContext`/`AiSuggestion` struct，均 `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]`。
2. `ai_suggest` 命令返回 `Err("ai_suggest not implemented until v1.1+".to_string())`。
3. `invoke_ai_op` 命令返回 `Err("invoke_ai_op not implemented until v1.1+".to_string())`，接受任意 `op: String` + `params: serde_json::Value`。
4. 两个命令注册到 `src-tauri/src/lib.rs` 的 `generate_handler!`，且 T6 的 `check_update`/`install_update` 仍保留。
5. `frontend/src/tauri.js` 导出 `aiSuggest(context)` 与 `invokeAiOp(op, params)`，调 `invoke('ai_suggest', { context })` / `invoke('invoke_ai_op', { op, params })`。
6. `frontend/src/components/AiPanel.jsx` 有调用 `aiSuggest` 的入口（按钮或加载触发），`.catch` 后展示「AI 能力 v1.1+ 释放」文案，UI 不崩溃。
7. `cargo check --workspace` + `pnpm --prefix frontend run build` 通过。
8. `grep -n 'rename_all' src-tauri/src/commands.rs` 命中 `AiContext`/`AiSuggestion` 所在行。
9. T6 产物未被破坏：`grep -n 'check_update\|install_update\|UpdateStatus' src-tauri/src/commands.rs` 仍命中，`grep -n 'check_update\|install_update' src-tauri/src/lib.rs` 的 `generate_handler!` 仍包含两者。

## verification_commands
```sh
# 1. 编译
cargo check --workspace

# 2. 前端构建
pnpm --prefix frontend run build

# 3. 命令与类型核对
grep -n 'ai_suggest\|invoke_ai_op\|AiContext\|AiSuggestion' src-tauri/src/commands.rs
grep -n 'generate_handler' src-tauri/src/lib.rs

# 4. serde camelCase
grep -n 'rename_all' src-tauri/src/commands.rs

# 5. 前端封装
grep -n 'aiSuggest\|invokeAiOp\|invoke' frontend/src/tauri.js

# 6. T6 未被破坏（回归）
grep -n 'check_update\|install_update' src-tauri/src/lib.rs
grep -n 'UpdateStatus\|pub async fn check_update\|pub async fn install_update' src-tauri/src/commands.rs
```

GUI 调用核验（AiPanel 触发 `aiSuggest` 得到 `v1.1+` 文案）由主会话环境补；coder 至少编译 + 前端构建通过并描述调用与 catch 逻辑。

## files_likely_to_change
- `src-tauri/src/commands.rs`（扩展 T6 产物）
- `src-tauri/src/lib.rs`（`generate_handler!` 追加 2 项）
- `frontend/src/tauri.js`（新建）
- `frontend/src/components/AiPanel.jsx`（扩展调用逻辑）

## risks
- **T6 产物保护**：`commands.rs` 当前已有 `UpdateStatus` + `check_update`/`install_update`，T7 新增的 `AiContext`/`AiSuggestion`/`ai_suggest`/`invoke_ai_op` 追加到同一文件，不要删除或修改 T6 代码。`lib.rs` 的 `generate_handler!` 追加两项，不要移除 T6 两项。
- **`invoke_ai_op` 的 `params: Value`**：`serde_json::Value` 接受任意 JSON，命令签名 `invoke_ai_op(op: String, params: serde_json::Value) -> Result<serde_json::Value, String>`；前端 `invoke('invoke_ai_op', { op, params })` 对齐。
- **AiPanel 调用时机**：若加载时自动调 `aiSuggest`，每次切到 AiPanel 都会触发一次空错误——可接受（v1.0.0 占位），或加按钮手动触发更干净。建议加按钮手动触发，避免重复调用。
- **`tauri.js` 文件首次创建**：T1/T6 均未建此文件，本任务新建；后续 T5/T8 会扩展。用 `import { invoke } from '@tauri-apps/api/core'`。

## depends_on
[T1]（T6 已 verified_complete，基线代码已含 updater 命令，T7 在其上扩展）

## status
planned
