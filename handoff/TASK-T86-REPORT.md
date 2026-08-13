# TASK-T86-REPORT — 列操作大小写归一化（后端 + 前端）

```yaml
implemented_changes:
  - src-tauri/src/commands/columns.rs
      - 新增 TransformOp enum（serde lowercase）：Uppercase / Lowercase
      - 新增 transform_column_inner 函数：复用 base64_transform_column_cells 闭包式列变换，
        op=Uppercase → to_uppercase()，op=Lowercase → to_lowercase()；空值行跳过；
        转换后与原值相同的行不计入 affected（与 base64 语义一致）
      - 新增 #[tauri::command] transform_column 命令包装
      - 新增 3 个测试：transform_column_inner_uppercase_lowercase（基本大小写归一化）、
        transform_column_undo_restores_original（before_snapshot + 撤销恢复原文）、
        transform_column_in_undo_list（kind 白名单生效）
  - src-tauri/src/lib.rs
      - invoke_handler 注册 commands::columns::transform_column（紧跟 hash_column 之后）
  - src-tauri/src/db/mod.rs
      - list_undoable_operations 的 SQL IN 白名单追加 'transform_column'
  - frontend/src/tauri.js
      - 新增 transformColumn(sheetId, column, op) wrapper，invoke("transform_column")
  - frontend/src/components/panels/ColumnOpsPanel.jsx
      - 新增第二区块"列变换"：Select 目标列 + Select 操作类型（uppercase/lowercase）+ Button 执行
      - handleTransform 流程：transformColumn → getSheetData + SET_SHEET_DATA 刷新当前页 →
        listUndoableOperations + SET_UNDO_STACK 刷新撤销栈 → message.success
      - 原 JSON 解析区块保持不变，用 Divider + Title 分隔两个区块
  - docs/02-技术设计文档.md
      - commands::columns 行追加 transform_column 命令说明
      - list_undoable_operations 行追加 transform_column 到 kind 集合
      - 新增 transform_column 命令签名行（同时补全 hash_column 的 case 参数签名）
      - 撤销/重做覆盖就地变更类操作说明追加 transform_column
      - 前端 state 注释 undoStack 的 kind 集合追加 transform_column
verification_run:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test -p ruT0-data-kit --lib commands::columns
  - cargo test -p ruT0-data-kit --lib（全量 lib 测试，附加验证未破坏其他模块）
  - pnpm --prefix frontend build
verification_results:
  - cargo fmt --all：通过（rustfmt 重排了 columns.rs 的代码格式，无功能影响）
  - cargo clippy --all-targets --all-features -- -D warnings：通过，Finished `dev` profile，
    无 warning 无 error
  - cargo test -p ruT0-data-kit --lib commands::columns：22 passed; 0 failed; 0 ignored
    （含 3 个新增 transform_column_* 测试全部通过）
  - cargo test -p ruT0-data-kit --lib：143 passed; 0 failed; 0 ignored; 0 measured
    （全量回归无破坏）
  - pnpm --prefix frontend build：通过，vite build 3083 modules transformed，
    built in 2.42s（chunk size 警告为既有项目历史状态，非本任务引入）
docs_updated:
  - docs/02-技术设计文档.md（同步 transform_column IPC 命令、白名单、撤销栈说明）
commit_summary:
  - none（按安全约束：不创建 git 分支，不 commit）
reported_status:
  - verified_complete
scope_deviation:
  - none（严格按 HANDOFF 指定的 5 文件改动 + 1 文档同步，未夹带无关重构 / 未改 DB schema /
    未引入新依赖 / 全部 SQL 参数绑定）
```

## 备注

- HANDOFF 中提示的 `query_column_cells` 已存在并被既有测试 helper `column_values` 复用，无需新增。
- HANDOFF 中提示 `listUndoableOperations` 可能不存在 — 实际已存在于 tauri.js（line 340），无 limit 参数版本（后端硬编码 50），直接复用。
- HANDOFF 示例的测试断言 `cells[0].value.as_deref()` 形式未直接采用；改用既有 `column_values` helper 返回 `Vec<String>`，断言更简洁，与同模块其他 19 个测试保持一致风格。语义完全等价。
- Cargo fmt 重排了 transform_column_inner 中闭包 match arm 的对齐（不影响逻辑）。
