# TASK-BOARD — v1.1.5

> 版本：`1.1.5`（校验增强 + 列操作扩展 + 脱敏选项）
> 状态：`done_e2e`
> 创建：2026-08-13

## 任务总览

| 任务 | 标题 | 状态 | 依赖 | 负责 |
|---|---|---|---|---|
| T80 | 版本号升级 + 文档骨架 | verified_complete | — | orchestrator |
| T81 | 邮箱校验规则（后端） | verified_complete | T80 | coder |
| T82 | .db 导入前端修复 | verified_complete | T80 | coder |
| T83 | 哈希大小写 hex（后端+前端） | verified_complete | T80 | coder |
| T84 | 先校验再脱敏 — 后端 | verified_complete | T80 | coder |
| T85 | 先校验再脱敏 — 前端 | verified_complete | T84 | coder |
| T86 | 列操作大小写归一化（后端+前端） | verified_complete | T80 | coder |
| T87 | 生日多格式 — 后端 | verified_complete | T80 | coder |
| T88 | 生日多格式 — 前端 | verified_complete | T87 | coder |
| T89 | E2E + 文档同步 | verified_complete | 全部 | orchestrator |

## 依赖关系

```
T80 ──┬── T81 ──┐
      ├── T82 ──┤
      ├── T83 ──┤
      ├── T84 ──┤── T85
      ├── T86 ──┤
      └── T87 ──┤── T88
                 └── T89
```

## 任务详情

### T80 ✅ verified_complete

- 4 处版本号同步 1.1.4 → 1.1.5
- `docs/versions/1.1.5/规划需求.md` + `更新日志.md` 骨架
- `docs/04-版本标准.md` §2 追加 v1.1.5 行
- `handoff/TASK-BOARD.md` 重建

### T81 邮箱校验规则（后端）

**in_scope：**
- `crates/core/src/processor/rules.rs`：`ExtractParams` enum 追加 `Email` unit variant + `email_validate_rule()` 构造器 + `with_defaults()` 注册（17→18）
- `crates/core/src/processor/func_validator.rs`：`is_valid_email(s: &str) -> bool` + `validate_extracted_with_params` 追加 Email 分支

**out_of_scope：** 前端无需改动（ValidatePanel 从 DB 动态加载 validate 规则）

### T82 .db 导入前端修复

**in_scope：**
- `frontend/src/components/layout/TopToolbar.jsx`：extensions 数组追加 `"db"`, `"sqlite"`, `"sqlite3"`

**out_of_scope：** 后端 DbReader + detect_format 在 v1.1.4 T75 已完整实现

### T83 哈希大小写 hex

**in_scope：**
- `src-tauri/src/commands/columns.rs`：`HashCase` enum + `hash_column_inner` 追加 case 参数 + 闭包条件 to_uppercase + `hash_column` 命令签名追加 case
- `frontend/src/tauri.js`：`hashColumn` 追加 case 参数
- `frontend/src/components/panels/CryptoPanel.jsx`：isHashOp 时显示 Radio.Group（小写/大写）

### T84 先校验再脱敏 — 后端

**in_scope：**
- `src-tauri/src/commands/processor.rs`：`mask_column` 签名追加 `validate_rule_id` / `invalid_text` / `params_override` / `phone_prefixes` 可选参数 + mask 循环条件校验

### T85 先校验再脱敏 — 前端（depends T84）

**in_scope：**
- `frontend/src/tauri.js`：`maskColumn` 追加新参数
- `frontend/src/components/panels/MaskPanel.jsx`：Checkbox + 校验规则 Select + invalid_text Input

### T86 列操作大小写归一化

**in_scope：**
- `src-tauri/src/commands/columns.rs`：`TransformOp` enum + `transform_column_inner` + `transform_column` 命令
- `src-tauri/src/lib.rs`：invoke_handler 注册 transform_column
- `src-tauri/src/db/mod.rs`：list_undoable_operations 白名单追加 transform_column
- `frontend/src/tauri.js`：`transformColumn` wrapper
- `frontend/src/components/panels/ColumnOpsPanel.jsx`：新增列变换区块

### T87 生日多格式 — 后端

**in_scope：**
- `crates/core/src/processor/rules.rs`：`ExtractParams::Birth` unit → struct variant（formats: Vec<String>, serde default）
- `crates/core/src/processor/func_validator.rs`：`is_valid_birth_format(s, format)` + `validate_extracted_with_params` Birth 分支改格式检查

### T88 生日多格式 — 前端（depends T87）

**in_scope：**
- `frontend/src/components/panels/ValidatePanel.jsx`：birth-validate 行展开格式 Checkbox.Group + handleValidate 组装 paramsOverride

### T89 E2E + 文档同步

**验证命令：**
```sh
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
pnpm --prefix frontend build
```

**文档同步：**
- `docs/versions/1.1.5/更新日志.md` — 填充所有任务结果
- `docs/02-技术设计文档.md` — 同步 ExtractParams 新变体 / transform_column / mask validate-then-mask
- `handoff/TASK-BOARD.md` — 所有任务 verified_complete
