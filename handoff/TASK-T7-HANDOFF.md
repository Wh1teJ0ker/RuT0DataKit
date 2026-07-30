# TASK-T7-HANDOFF — AI 占位 IPC 契约

## task_id
T7

## goal
落地 `ai_suggest` / `invoke_ai_op` 两个 `#[command]` 空实现 IPC 契约（返回 `v1.1+` 错误），定义 `AiContext`/`AiSuggestion` 类型（嵌套 struct 带 camelCase），注册到 `generate_handler!`，并提供前端 `tauri.js` 封装（`aiSuggest`/`invokeAiOp`）。

## in_scope
允许新增/修改的文件：

- `src-tauri/src/commands.rs`（或拆 `commands/ai.rs`）：新增类型与命令：
  - `AiContext` struct：`#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]`，字段可参考 02 文档 §4.4（如 `sheet_id: Option<i64>`/`selection: Option<Selection>`/`prompt: Option<String>`——字段可精简占位，关键是 struct 存在且 camelCase）。
  - `AiSuggestion` struct：同上 serde 派生 + camelCase，字段如 `suggestion: String`/`confidence: f64`（占位）。
  - `ai_suggest(context: AiContext) -> Result<AiSuggestion, String>`：返回 `Err("ai_suggest not implemented until v1.1+".into())`。
  - `invoke_ai_op(op: String, params: serde_json::Value) -> Result<serde_json::Value, String>`：返回 `Err("invoke_ai_op not implemented until v1.1+".into())`。
  - 命令在 `lib.rs` 的 `generate_handler!` 注册。
- `frontend/src/tauri.js`（新增，T1 未建此文件）：前端 invoke 封装层，对齐 IPC 契约。导出 `aiSuggest(context)`、`invokeAiOp(op, params)`（`invoke('ai_suggest', { context })` / `invoke('invoke_ai_op', { op, params })`）。也可顺带占位导出 `importFile`/`getSheetData`/`checkUpdate` 等封装（供 T5/T8 调用），但**本任务只实现 `aiSuggest`/`invokeAiOp` 封装**，其它封装留给对应任务（或占位 stub，以不阻塞为准）。
- `frontend/src/components/AiPanel.jsx`：**扩展** T2 的 AiPanel 占位——加一个「调用 ai_suggest 测试」`Button`（或 AiPanel 加载时自动调一次 `aiSuggest`），调用 `.catch` 错误并展示「AI 能力 v1.1+ 释放」文案，UI 不崩溃。本任务确保 AiPanel 能触发调用并优雅处理 `v1.1+` 错误。
- `src-tauri/src/lib.rs`：**扩展** `generate_handler!` 注册 `ai_suggest`/`invoke_ai_op`（若 T4/T6 已拆 `commands/` 目录，本任务在对应 `commands/ai.rs` 加；若 `commands.rs` 单文件，加在该文件）。

## out_of_scope
明确不许动的：

- **不许实现真实 AI 逻辑**（v1.4+）。
- **不许动 DB 层**（T4）。
- **不许动 updater**（T6）。
- **不许动布局组件 TopToolbar/SidePanel/Workbench/panels**（T2/T3 产物，本任务只扩展 AiPanel 的调用逻辑）。
- **不许实现导入**（T5）。
- **不许动 docs/**、**不许动 handoff/ 其它任务文件**。
- `tauri.js` 中不许实现 `importFile`/`getSheetData`/`checkUpdate` 的真实封装（可占位 stub 但不要写 invoke 调用逻辑——留给 T5/T8；本任务只实现 `aiSuggest`/`invokeAiOp`）。

## acceptance_criteria
1. `src-tauri/src/commands.rs`（或 `commands/ai.rs`）定义 `AiContext`/`AiSuggestion` struct，均 `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]`。
2. `ai_suggest` 命令返回 `Err("ai_suggest not implemented until v1.1+")`。
3. `invoke_ai_op` 命令返回 `Err("invoke_ai_op not implemented until v1.1+")`，接受任意 `op: String` + `params: Value`。
4. 两个命令注册到 `lib.rs` 的 `generate_handler!`。
5. `frontend/src/tauri.js` 导出 `aiSuggest(context)` 与 `invokeAiOp(op, params)`，调 `invoke('ai_suggest', { context })` / `invoke('invoke_ai_op', { op, params })`。
6. `frontend/src/components/AiPanel.jsx` 有调用 `aiSuggest` 的入口（按钮或加载触发），`.catch` 后展示「AI 能力 v1.1+ 释放」文案，UI 不崩溃。
7. `cargo check --workspace` + `pnpm --prefix frontend run build` 通过。
8. `grep -n 'rename_all' src-tauri/src/commands.rs src-tauri/src/commands/ai.rs 2>/dev/null` 命中 `AiContext`/`AiSuggestion`。

## verification_commands
```sh
# 1. 编译
cargo check --workspace

# 2. 前端构建
pnpm --prefix frontend run build

# 3. 命令与类型核对
grep -n 'ai_suggest\|invoke_ai_op\|AiContext\|AiSuggestion' src-tauri/src/commands.rs src-tauri/src/commands/ai.rs 2>/dev/null
grep -n 'generate_handler' src-tauri/src/lib.rs

# 4. serde camelCase
grep -n 'rename_all' src-tauri/src/commands.rs src-tauri/src/commands/ai.rs 2>/dev/null

# 5. 前端封装
grep -n 'aiSuggest\|invokeAiOp\|invoke' frontend/src/tauri.js
```

GUI 调用核验（AiPanel 触发 `aiSuggest` 得到 `v1.1+` 文案）由主会话环境补；coder 至少编译 + 前端构建通过并描述调用与 catch 逻辑。

## files_likely_to_change
- `src-tauri/src/commands.rs`（或 `commands/ai.rs`）
- `src-tauri/src/lib.rs`（generate_handler 扩展）
- `frontend/src/tauri.js`（新建）
- `frontend/src/components/AiPanel.jsx`（扩展调用逻辑）

## risks
- **`invoke_ai_op` 的 `params: Value`**：`serde_json::Value` 接受任意 JSON，命令签名 `invoke_ai_op(op: String, params: serde_json::Value) -> Result<serde_json::Value, String>`；注意 Tauri v2 命令参数从 `invoke` 的 `args` 对象按 key 取，前端 `invoke('invoke_ai_op', { op, params })` 对齐。
- **AiPanel 调用时机**：若加载时自动调 `aiSuggest`，每次切到 AiPanel 都会触发一次空错误——可接受（v1.0.0 占位），或加按钮手动触发更干净。
- **`tauri.js` 文件首次创建**：T1 未建此文件，本任务新建；后续 T5/T8 会扩展。建议 `tauri.js` 用 `import { invoke } from '@tauri-apps/api/core'`（Tauri v2 的 invoke 路径是 `@tauri-apps/api/core`，非 v1 的 `@tauri-apps/api/tauri`）。

## depends_on
[T1]

## status
planned
