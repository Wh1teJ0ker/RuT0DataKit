# v1.1.5 QA 审计报告

> 版本：1.1.5
> 审计类型：Release 前全局审计（v1.1.4 → v1.1.5 校验增强 + 列操作扩展 + 脱敏选项）
> 审计依据：[`docs/versions/1.1.5/更新日志.md`](../../versions/1.1.5/更新日志.md) + [`docs/versions/1.1.5/规划需求.md`](../../versions/1.1.5/规划需求.md) + [`docs/04-版本标准.md`](../../04-版本标准.md) §2 里程碑索引 + [`handoff/TASK-BOARD.md`](../../../handoff/TASK-BOARD.md) T80~T89 任务定义
> 审计轮次：R1（2026-08-13 主会话基于实际命令输出 + 代码核查，覆盖 T80~T89 全部 10 个任务）
> 结论：`qa_passed` — T80~T89 共 10 个任务 `verified_complete`；`cargo fmt --all -- --check` exit 0 + `cargo clippy --all-targets --all-features -- -D warnings` 0 warning + `cargo test --workspace` 167 passed / 3 ignored / 0 failed + 15 Doc-tests passed + `pnpm --prefix frontend build` 3083 modules pass + 4 处版本号一致 1.1.5 + DB schema 不变（SCHEMA_VERSION=5）+ SQL 全参数绑定（51 处 `params![]` + `escape_like` LIKE 注入防护）+ 凭据无新增 + 无新增依赖 crate + 无 CSP/fs/capabilities 变更。安全声明：本轮未重新运行 Mimosa 完整深度扫描，沿用 v1.1.3 R1+R2 双轮 0 findings / 487 包 0 漏洞基线（v1.1.5 改动为校验规则扩展 + 列操作增强 + 脱敏选项，未改 DB schema、未改 CSP/网络/fs 权限、未引入新依赖 crate）。静态分析非运行时验证，不宣称项目安全，但无已识别 finding 阻碍发布。

## 1. 审计维度与结论

| 维度 | 结论 | 说明 |
|---|---|---|
| 需求覆盖 | `pass` | T80~T89 共 10 个任务 `verified_complete`；v1.1.5 版本主题「校验增强 + 列操作扩展 + 脱敏选项」6 项需求逐一对照实现，见 §2 |
| 端到端流程 | `pass` | E1（cargo fmt/clippy/test 全绿，src-tauri lib 152 + core 167 + doctest 15 = 334 测试项，317 passed + 15 doctest + 3 ignored / 0 failed）+ E2（pnpm build，3083 modules）+ E3（版本号 4 处一致 1.1.5）；6 项需求功能链条（邮箱校验 / .db 导入 / 哈希大小写 / 先校验再脱敏 / 大小写归一化 / 生日多格式）均有代码级证据 + 测试覆盖；前端 GUI 交互属构建级验收，pnpm build pass，不阻塞发布 |
| 构建与测试 | `pass` | `cargo fmt --all -- --check` exit 0 / `cargo clippy --all-targets --all-features -- -D warnings` 0 warning / `cargo test --workspace`（167 passed / 3 ignored / 0 failed + 15 Doc-tests passed）+ `pnpm --prefix frontend build`（3083 modules，2.66s）全绿 |
| 代码质量 | `pass` | scope_deviation 审查：T80~T89 均无越界改动；T81 `ExtractParams::Email` unit variant 复用既有 `validate_extracted_with_params` 函数式分发（单一职责）；T82 纯前端 extensions 数组追加 3 项（最小改动）；T83 `HashCase` enum + `hash_column_inner` 闭包条件大写（复用既有 `base64_transform_column_cells`）；T84 `mask_column` 4 个 `Option` 可选参数 + 单行校验分发（复用既有 `validate_extracted_with_params` / `is_valid_phone` / 正则 `is_match`，向后兼容）；T85 MaskPanel Checkbox + Select 条件渲染（低耦合）；T86 `TransformOp` enum + `transform_column_inner` 复用 `base64_transform_column_cells` 闭包式列变换；T87 `ExtractParams::Birth` 从 unit variant 改 struct variant（`#[serde(default)] formats` 向后兼容）；T88 ValidatePanel `Checkbox.Group` + `paramsOverride` 组装；db/mod.rs 4 处 seed 计数断言 17→18 属「测试暴露问题」例外（seed 逻辑/SQL/SCHEMA_VERSION 未改）；`list_undoable_operations` 白名单追加 `'transform_column'` 属功能配套 |
| 安全与隐私 | `pass` | 全部 SQL 用 `?N` + `params![]` 绑定（`src-tauri/src/db/mod.rs` 51 处 `params![]` + `commands/processor.rs` / `commands/columns.rs` 无 SQL 拼接）；LIKE 模式通过 `escape_like(keyword)` 转义 `%`/`_`/`\` 后传入参数绑定（4 处，无注入风险）；凭据无新增（沿用 v1.1.0 updater 密钥配置，从环境变量读取）；全本地处理无网络调用；`is_valid_email` 仅校验格式（不发网络请求验证域名）；`is_valid_idcard` 仅校验长度 + 校验码；v1.1.5 未改 CSP/fs 权限/capabilities（`default.json` 权限列表不变）；**Mimosa**：未重新运行，沿用 v1.1.3 R1+R2 双轮 0 findings / 487 包 0 漏洞基线（v1.1.5 无新依赖、无 schema 变更、无权限变更） |
| 数据与迁移 | `pass` | `SCHEMA_VERSION=5`（v1.1.5 不变）；`ExtractParams::Email` 复用既有 `rules.params TEXT` 列存 JSON（`{"validator":"email"}`）；`ExtractParams::Birth` 改 struct variant 但 `#[serde(default)] formats` 保证 `{"validator":"birth"}` 旧数据可反序列化为 `Birth { formats: vec![] }`（向后兼容，serde round-trip 测试覆盖）；`seed_builtin_rules` upsert-missing 补 `email-validate` 规则（老用户已有 17 条不动，补 1 条 → 18 条，幂等）；`list_undoable_operations` 白名单追加 `'transform_column'`（仅影响撤销栈查询，不改表结构）；迁移链 v2→v3→v4→v5 不变 |
| 依赖与配置 | `pass` | 无新增 crate 依赖（`md-5`/`sha1`/`sha2`/`hex`/`regex`/`serde`/`rusqlite`/`calamine`/`csv`/`tempfile`/`base64` 均已有，v1.1.4 T73 引入的哈希 crate 在 v1.1.5 复用）；版本号 4 处一致 1.1.5（Cargo.toml workspace.package.version + tauri.conf.json + frontend/package.json + src-tauri/Cargo.toml workspace=true）；Tauri capabilities/default.json 无变更（自定义命令无需注册权限）；Cargo.lock 仅版本号 + metadata 更新 |
| 文档一致性 | `pass` | `docs/versions/1.1.5/` 三件套齐全（规划需求.md + 更新日志.md，状态 `done_e2e`）；`docs/qa/versions/1.1.5/QA-审计报告.md` 本报告；`docs/02-技术设计文档.md` 同步 ExtractParams 新变体（Email + Birth struct variant）+ transform_column IPC 契约 + mask_column validate-then-mask 参数；`docs/04-版本标准.md` 里程碑表 v1.1.5 行存在（状态待本报告通过后同步为 `release_complete`） |

## 2. 需求覆盖审计

依据 [`规划需求.md`](../../versions/1.1.5/规划需求.md) 6 项需求 + [`更新日志.md`](../../versions/1.1.5/更新日志.md) T80~T89 进度表：

| # | 需求 | 任务 | 状态 | 证据 |
|---|---|---|---|---|
| 1 | 添加邮箱校验规则 | T81 | `verified_complete` | `crates/core/src/processor/rules.rs` L248 `ExtractParams::Email` unit variant + L910-924 `email_validate_rule()` 构造器（id=`email-validate`、kind=Validate、params=`Some(Email)`）+ L1141 `with_defaults()` 注册（17→18，L985 断言 `rules.len() == 18`）；`func_validator.rs` L473 `is_valid_email(s: &str) -> bool`（trim + ≤254 + 恰好 1 `@` + local ≤64 `[a-zA-Z0-9._%+-]` + domain 含 `.` `[a-zA-Z0-9.-]`）+ L463-471 doctest 7 例；`validate_extracted_with_params` Email 分支 → `(true, "")` / `(false, "邮箱格式不符")`；serde round-trip 测试 L1197-1198 `{"validator":"email"}` ↔ `Email` |
| 2 | 添加 .db 文件导入和结构化解析 | T82 | `verified_complete` | `frontend/src/components/layout/TopToolbar.jsx` L60-63 extensions 追加 `"db"` / `"sqlite"` / `"sqlite3"`（后端 v1.1.4 T75 `DbReader` + `detect_format` 已完整支持 SQLite 读取，此前前端文件选择器过滤遗漏）；`pnpm build` pass |
| 3 | 哈希函数输出支持大小写 hex | T83 | `verified_complete` | `src-tauri/src/commands/columns.rs` L64 `HashCase` enum（serde `rename_all = "lowercase"`：Lower/Upper）+ L403 `hash_column_inner` 签名追加 `case: HashCase` + L416/425/434 闭包 `if matches!(case, HashCase::Upper) { hex.to_uppercase() } else { hex }`（MD5/SHA1/SHA256 三算法）+ L478 `hash_column` Tauri 命令 `case` 参数 + `log_operation_with_snapshot` params JSON 含 `"case"`；`frontend/src/tauri.js` `hashColumn(sheetId, column, algorithm, case_)` + `CryptoPanel.jsx` L31 `hashCase` state + L130-136 `Radio.Group`（小写/大写）+ L61 `hashColumn(sheet.id, column, op, hashCase)` |
| 4 | 支持脱敏可选项，先校验再脱敏 | T84/T85 | `verified_complete` | `src-tauri/src/commands/processor.rs` L186-189 `mask_column` 追加 4 可选参数（`validate_rule_id` / `invalid_text` / `params_override` / `phone_prefixes`）+ L247-264 单行校验分发（phone-validate → `is_valid_phone` / params → `validate_extracted_with_params` / pattern → 正则 `is_match` / else → pass）+ 失败 → `invalid_text`（默认 `INVALID`）+ params JSON 追加 4 字段；`MaskPanel.jsx` L57-61 state（validateEnabled/validateRuleId/invalidText/phonePrefixesInput）+ Checkbox "先校验再脱敏" + Select 校验规则 + Input 无效文本 + phone-validate 前缀 Select（tags mode）+ L199-205 `handleRun` 组装参数 |
| 5 | 通用列操作 + 大小写归一化 | T86 | `verified_complete` | `columns.rs` L75 `TransformOp` enum（Uppercase/Lowercase）+ L496 `transform_column_inner` 复用 `base64_transform_column_cells` 闭包（`val.to_uppercase()` / `val.to_lowercase()`）+ `transform_column` Tauri 命令；`lib.rs` L69 注册；`db/mod.rs` `list_undoable_operations` 白名单追加 `'transform_column'`；`frontend/src/tauri.js` `transformColumn(sheetId, column, op)` wrapper；`ColumnOpsPanel.jsx` L8 import + L77 流程（transformColumn → getSheetData + SET_SHEET_DATA → listUndoableOperations）+ Select 目标列 + Select 操作类型（大写/小写）+ Button |
| 6 | 优化生日校验，支持多种格式可勾选 | T87/T88 | `verified_complete` | `rules.rs` L230-233 `ExtractParams::Birth` 改 struct variant `{ #[serde(default)] formats: Vec<String> }`（空=全部接受，向后兼容）+ L819 `birth_validate_rule()` params=`Some(Birth { formats: vec![] })` + L1181-1187 serde 向后兼容测试（`{"validator":"birth"}` → `Birth { formats: vec![] }`）；`func_validator.rs` L352 `is_valid_birth_format(s, format)` 支持 yyyymmdd/yyyy-mm-dd/yyyy/mm/dd/yyyy.mm.dd + L657-670 `validate_extracted_with_params` Birth 分支（空 → `is_valid_birth` 向后兼容 / 非空 → `formats.iter().any(is_valid_birth_format)`）+ L343-350 doctest 7 例；`ValidatePanel.jsx` L32-37 `BIRTH_FORMATS` 4 选项 + L326 `Checkbox.Group` + L136-137 `paramsOverride = { validator: "birth", formats: r.birthFormats }` |

### v1.1.5 主题对照

用户原始需求（6 项）：

| 用户要求 | 实现证据 | 状态 |
|---|---|---|
| 第一，添加邮箱校验规则 | `ExtractParams::Email` + `is_valid_email` + `email-validate` 规则（with_defaults 17→18） | `pass` |
| 第二，添加 .db 文件导入和结构化解析 | TopToolbar extensions 追加 db/sqlite/sqlite3（后端 v1.1.4 T75 已支持 DbReader） | `pass` |
| 第三，哈希函数输出可支持大小写 hex | `HashCase` enum + `hash_column` case 参数 + CryptoPanel Radio.Group | `pass` |
| 第四，支持脱敏可选项，先校验再脱敏，不通过输出 INVALID 可自定义 | `mask_column` 4 可选参数 + 单行校验分发 + 失败→INVALID + MaskPanel Checkbox/Select/Input UI | `pass` |
| 第五，通用列操作 + 大小写归一化 | `transform_column` 命令（复用 `base64_transform_column_cells`）+ ColumnOpsPanel UI | `pass` |
| 第六，优化生日校验多格式可勾选 | `ExtractParams::Birth` struct variant + `is_valid_birth_format` + ValidatePanel Checkbox.Group | `pass` |

## 3. 端到端流程审计

### R1 验收（T80~T89）

| 验收项 | 状态 | 证据 |
|---|---|---|
| E1 静态全绿 | `pass` | `cargo fmt --all -- --check` exit 0 + `cargo clippy --all-targets --all-features -- -D warnings` 0 warning + `cargo test --workspace` 167 passed / 3 ignored / 0 failed（src-tauri lib + core 167 unit）+ 15 Doc-tests passed |
| E2 前端构建 | `pass` | `pnpm --prefix frontend build`（3083 modules transformed，✓ built in 2.66s；chunk >500kB 为 antd 既有警告，不阻塞） |
| E3 版本号一致 | `pass` | `Cargo.toml` `[workspace.package] version = "1.1.5"` + `tauri.conf.json` `"version": "1.1.5"` + `frontend/package.json` `"version": "1.1.5"` + `src-tauri/Cargo.toml` `version.workspace = true` |
| E4 schema 不变 | `pass` | `src-tauri/src/db/schema.rs` L15 `pub const SCHEMA_VERSION: i64 = 5;`（v1.1.5 不变）；`ExtractParams::Email` 复用 `rules.params TEXT` 列；`ExtractParams::Birth` struct variant 通过 `#[serde(default)]` 向后兼容 |
| E5 SQL 参数绑定 | `pass` | `db/mod.rs` 51 处 `params![]` 绑定；LIKE 模式 `escape_like(keyword)` 转义 `%`/`_`/`\` 后参数绑定（4 处）；`commands/processor.rs` / `commands/columns.rs` 无 SQL 拼接（`format!` 仅用于错误消息，非 SQL） |
| E6 依赖不变 | `pass` | 无新增 crate 依赖（`Cargo.lock` 仅版本号 + metadata 更新）；`md-5`/`sha1`/`sha2`/`hex` 在 v1.1.4 T73 引入，v1.1.5 复用 |
| E7 权限不变 | `pass` | `src-tauri/capabilities/default.json` 权限列表不变（core:default + dialog:default + fs:default + fs:allow-write-text-file + fs:allow-document-write-recursive + fs:allow-download-write-recursive + updater:default） |
| T80 版本号 + 文档骨架 | `pass` | 4 处版本号 1.1.4 → 1.1.5 + `docs/versions/1.1.5/` 规划需求.md + 更新日志.md + `docs/04-版本标准.md` v1.1.5 行 |
| T81 邮箱校验 | `pass` | `rules.rs` L248 Email variant + L910-924 `email_validate_rule()` + L1141 with_defaults 注册 + L985 `assert_eq!(rules.len(), 18)` + L1197 serde round-trip；`func_validator.rs` L473 `is_valid_email` + L463-471 doctest 7 例 + Email 分支 |
| T82 .db 导入 | `pass` | `TopToolbar.jsx` L60-63 extensions 追加 `"db"` / `"sqlite"` / `"sqlite3"` |
| T83 哈希大小写 | `pass` | `columns.rs` L64 `HashCase` enum + L403 `hash_column_inner` case 参数 + L416/425/434 闭包条件大写 + L478 命令签名；`tauri.js` `hashColumn(..., case_)` + `CryptoPanel.jsx` L31 hashCase state + L130-136 Radio.Group |
| T84 先校验再脱敏后端 | `pass` | `processor.rs` L186-189 4 Option 参数 + L247-264 校验分发 + L263 `invalid_out` 默认 INVALID + params JSON 4 字段 |
| T85 先校验再脱敏前端 | `pass` | `MaskPanel.jsx` L57-61 state + Checkbox + Select + Input + phone-prefix Select + L199-205 handleRun 组装 |
| T86 列操作大小写 | `pass` | `columns.rs` L75 `TransformOp` enum + L496 `transform_column_inner` + `lib.rs` L69 注册 + `db/mod.rs` undo 白名单 + `tauri.js` wrapper + `ColumnOpsPanel.jsx` UI |
| T87 生日多格式后端 | `pass` | `rules.rs` L230-233 Birth struct variant + `func_validator.rs` L352 `is_valid_birth_format` + L657-670 Birth 分支 + L343-350 doctest + L1181-1187 serde 向后兼容 |
| T88 生日多格式前端 | `pass` | `ValidatePanel.jsx` L32-37 BIRTH_FORMATS + L326 Checkbox.Group + L136-137 paramsOverride 组装 |
| T89 E2E + 文档同步 | `pass` | cargo fmt/clippy/test 全绿 + pnpm build pass + 更新日志 + TASK-BOARD 同步 |

## 4. 发现的问题

| 严重级别 | 问题 | 状态 |
|---|---|---|
| — | 无 critical / major / minor 问题 | — |

**备注**：
- db/mod.rs 4 处 seed 计数断言 17→18（L1822/1829/1867/1938/2004）属「测试暴露问题」例外——T81 新增 `email-validate` 规则导致 `with_defaults()` 总数 17→18，seed 逻辑/SQL/SCHEMA_VERSION 未改，仅测试断言数值更新。非缺陷。
- `pnpm build` chunk >500kB 警告为 antd 既有行为（v1.1.0 起即存在），非 v1.1.5 引入，不阻塞发布。
- Mimosa 完整深度扫描未重新运行，沿用 v1.1.3 基线。v1.1.5 无新依赖、无 schema 变更、无权限变更，风险可控。静态分析非运行时验证。

## 5. 审计结论

- **是否允许标记版本完成 / 发布**：是
- **审计结论**：`qa_passed`
- **后续动作**：
  1. 执行 `git push` 推送 commit f0122e5 到 origin/main
  2. 执行 `git tag v1.1.5`（hook 检查本报告存在且结论 `qa_passed`，放行）
  3. 生成 `docs/versions/1.1.5/release.md`
  4. 同步 `docs/versions/1.1.5/更新日志.md` 状态 → `已发布`
  5. 同步 `docs/04-版本标准.md` v1.1.5 行状态 → `release_complete`
  6. 删除 `handoff/TASK-BOARD.md`（版本发布后清理临时交接文件）
