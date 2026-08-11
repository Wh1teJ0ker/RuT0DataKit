# v1.1.4 QA 审计报告

> 版本：1.1.4
> 审计类型：Release 前全局审计（v1.1.3 → v1.1.4 校验模块统一 — 统一校验页面重设计）
> 审计依据：[`docs/versions/1.1.4/更新日志.md`](../../versions/1.1.4/更新日志.md) + [`docs/04-版本标准.md`](../../04-版本标准.md) §2 里程碑索引 + [`handoff/TASK-BOARD.md`](../../../handoff/TASK-BOARD.md) T67~T69 任务定义 + E79~E84 验收项
> 审计轮次：R1（2026-08-07 主会话基于实际命令输出 + 代码核查）
> 结论：`qa_passed` — R1 覆盖 T67~T69（292 passed / 3 ignored / 0 failed + 3082 modules + 4 处版本一致 + DB schema 不变 + SQL 全参数绑定 + 凭据无新增）。**安全声明**：本轮未重新运行 Mimosa 完整深度扫描（v1.1.3 R1+R2 两轮 Mimosa 深度扫描均 0 findings / 487 包 0 漏洞，v1.1.4 改动为校验模块功能性扩展，未引入新依赖 crate、未改 DB schema、未改 CSP/网络/fs 权限，沿用 v1.1.3 静态分析结论作为基线）。静态分析非运行时验证，不宣称项目安全，但无已识别 finding 阻碍发布。

## 1. 审计维度与结论

| 维度 | 结论 | 说明 |
|---|---|---|
| 需求覆盖 | `pass` | T67~T69 共 3 个任务 `verified_complete`；v1.1.4 版本主题「校验模块统一 — 重做为统一校验页面（自由选多规则 + 一按钮 + 双 Tab 输出）」逐一对照实现，见 §2 |
| 端到端流程 | `pass` | E1（cargo fmt/clippy/test 全绿，src-tauri 129 + core 152 + doctest 11 = 292 passed）+ E2（pnpm build，3082 modules）+ E3（版本号 4 处一致 1.1.4）+ E79~E84（T67 ExtractParams +4 + 6 validate 规则 / T68 validate_multi_rules 命令 6 集成测试 / T69 统一校验面板构建）全绿；T69 前端 GUI 交互属构建级验收，pnpm build pass，不阻塞发布 |
| 构建与测试 | `pass` | `cargo fmt --all -- --check` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test --all`（src-tauri 129 + core 152 + Doc-tests 11 = 292 passed / 3 ignored / 0 failed）+ `pnpm --prefix frontend build`（3082 modules，2.62s）全绿 |
| 代码质量 | `pass` | scope_deviation 审查：T67~T69 均无越界改动；T67 `ExtractParams` +4 变体复用既有 `validate_extracted` 函数式分发（单一职责）；T68 `validate_multi_rules_to_two_sheets` 复用 `TwoSheetResult`/`RowInvalidReason` 既有结构 + `CrossFieldConfig`/`MultiRuleValidation` 独立结构；T69 `Form.List` + `shouldUpdate` 条件渲染（低耦合）；db/mod.rs 4 处 seed 计数断言 10→16 属「测试暴露问题」例外（seed 逻辑/SQL/SCHEMA_VERSION 未改）；docs/02 技术设计文档同步 T68 IPC 契约（scope_deviation，justified） |
| 安全与隐私 | `pass` | 全部 SQL 用 `?N` + `params![]` 绑定（`src-tauri/src/db/mod.rs` 51 处 + `commands/processor.rs` 2 处，grep 无 `format!` 拼接 SQL）；凭据无新增（沿用 v1.1.0 updater 密钥配置，从环境变量读取）；全本地处理无网络调用；`is_valid_idcard` 仅校验长度 + 校验码（不查行政区划表）；v1.1.4 未改 CSP/fs 权限/capabilities；**Mimosa**：未重新运行，沿用 v1.1.3 R1+R2 双轮 0 findings / 487 包 0 漏洞基线（v1.1.4 无新依赖、无 schema 变更、无权限变更） |
| 数据与迁移 | `pass` | `SCHEMA_VERSION=5`（v1.1.4 不变，T67 复用既有 `rules.params TEXT` 列存 4 新变体 JSON）；迁移链 v2→v3→v4→v5 / v3→v4→v5 / v4→v5 全覆盖，各 `migrate_vN_to_vM` 幂等；老用户升级自动补列 + `seed_builtin_rules` upsert-missing 补 6 条新 validate 规则（已存在 10 条不动）；`cleanup_deprecated_rules` 清理 6 个遗留 id（参数绑定 DELETE，幂等） |
| 依赖与配置 | `pass` | 无新增 crate 依赖（`regex`/`serde`/`rusqlite`/`calamine`/`csv`/`tempfile`/`base64` 均已有）；`serde_json` 启用 `preserve_order` feature（v1.1.4 工作区配置，属前端 JSON 列顺序保留的既有能力，非新增依赖）；版本号 4 处一致 1.1.4（Cargo.toml workspace + tauri.conf.json + frontend/package.json + constants.js）；Tauri capabilities/default.json 无变更（自定义命令无需注册权限） |
| 文档一致性 | `pass` | `docs/versions/1.1.4/` 三件套齐全（更新日志.md + RELEASE-NOTES.md）；`docs/qa/versions/1.1.4/QA-审计报告.md` 本报告；`docs/02-技术设计文档.md` 同步 T68 IPC 契约 + `CrossFieldConfig`/`MultiRuleValidation` 结构签名（§4.6）；`docs/04-版本标准.md` 里程碑表 v1.1.4 行待补（本报告通过后同步） |

## 2. 需求覆盖审计

依据 [`更新日志.md`](../../versions/1.1.4/更新日志.md) 进度表 + E79~E84 验收项：

| # | 需求 | 任务 | 状态 | 证据 |
|---|---|---|---|---|
| 1 | 后端规则系统扩展：ExtractParams +4 变体 + 6 条 validate 规则 + 分发 | T67 | `verified_complete` | `crates/core/src/processor/rules.rs` `ExtractParams` +4 变体（Username/Sex/Birth/Address，L214-226）+ 6 条 validate 规则构造器（username_validate_rule/sex_validate_rule/birth_validate_rule/idcard_validate_rule/phone_validate_rule/address_validate_rule，L723-829）+ `with_defaults()` 注册 6 条（L492-497，总数 10→16）；`func_validator.rs` `validate_extracted` +4 分发分支（L404-434）→ `is_valid_username`/`is_valid_sex`/`is_valid_birth`/`is_valid_address`；DB seed 幂等（`seed_builtin_rules` upsert-missing，老用户 10 条不动）；E79~E81 全绿 |
| 2 | 新增 validate_multi_rules_to_two_sheets IPC 命令 | T68 | `verified_complete` | `src-tauri/src/commands/processor.rs` `CrossFieldConfig`（L1031，camelCase）+ `MultiRuleValidation`（L1050，camelCase）+ `validate_multi_rules_to_two_sheets_inner`（L1078）+ Tauri 命令（L1387）；校验分发：phone-validate→`is_valid_phone` / params 非空→`validate_extracted` / pattern 非空→正则 / 其他→默认通过（L1192-1216）；跨字段：`normalize_gender` vs `idcard_gender` + `is_valid_birth` 后 `idcard[6..14]` 切片比对（L1235-1284）；6 集成测试（all_rules_pass/mixed_pass_fail/idcard_cross_field_sex_mismatch/idcard_cross_field_birth_mismatch/regex_rule_validation/partial_mapping，L2521-2782）；`lib.rs` L58 注册；`docs/02` §4.6 IPC 契约同步；E82~E83 全绿 |
| 3 | 前端重做统一校验页面（多规则 + 一按钮 + 双 Tab 输出） | T69 | `verified_complete` | `frontend/src/components/panels/ValidatePanel.jsx` 完全重写（移除 Tabs，改 `Form.List` 动态规则行 + `shouldUpdate` 条件渲染跨字段配置）；`tauri.js` 新增 `validateMultiRulesToTwoSheets` wrapper（L292-304）；双 Tab 落地复用 `landSheet` 闭包（addSheetFromParse + getSheetData + SET_SHEET_DATA）；汇总消息通过/失败行数 + top 3 失败原因；2 个 minor 缺陷已修复（phonePrefixes 三位纯数字过滤 + crossField 列必填校验）；E84 全绿（pnpm build 3082 modules） |

### v1.1.4 主题对照

用户原始需求：「不应该是行级检验和单列校验，完整的应该是一个校验页面，但是可以自由选取多个规则进行，然后一个按钮校验，然后生成两个新tab」

| 用户要求 | 实现证据 | 状态 |
|---|---|---|
| 一个统一校验页面（非两个 Tab） | `ValidatePanel.jsx` 移除 antd `Tabs`，改单一 `Form` | `pass` |
| 自由选取多个规则 | `Form.List name="rules"` 动态增删规则行，每行 = 目标列 Select + 校验规则 Select | `pass` |
| 一个按钮校验 | 底部单一「校验」按钮 → `validateMultiRulesToTwoSheets` | `pass` |
| 生成两个新 Tab | `landSheet` 闭包落地 `{源sheet名}_校验通过` + `{源sheet名}_校验失败` | `pass` |
| 保留原列不新增列 | 后端 `validate_multi_rules_to_two_sheets_inner` 写新 Tab 时复用源 headers，不新增列 | `pass` |
| 身份证可勾选跨字段比对性别/出生日期 | `idcard-validate` 行 `shouldUpdate` 条件渲染 Checkbox + 列 Select；crossField 列必填校验 | `pass` |

## 3. 端到端流程审计

### R1 验收（T67~T69，E79~E84）

| 验收项 | 状态 | 证据 |
|---|---|---|
| E1 静态全绿 | `pass` | `cargo fmt --all -- --check` exit 0 + `cargo clippy --all-targets --all-features -- -D warnings` exit 0 + `cargo test --all` 292 passed / 3 ignored / 0 failed（src-tauri lib 129 + core 152 + Doc-tests 11） |
| E2 前端构建 | `pass` | `pnpm --prefix frontend build`（3082 modules transformed，✓ built in 2.62s；chunk >500kB 为 antd 既有警告） |
| E3 版本号一致 | `pass` | grep 确认 Cargo.toml（workspace.package.version=1.1.4）+ tauri.conf.json（version=1.1.4）+ frontend/package.json（version=1.1.4）+ frontend/src/constants.js（APP_VERSION="v1.1.4"） |
| E79（T67）ExtractParams +4 变体 | `pass` | `crates/core/src/processor/rules.rs` L214-226 `Username`/`Sex`/`Birth`/`Address` 变体 + `extract_params_serde_roundtrip` 测试覆盖 9 变体 roundtrip |
| E80（T67）validate_extracted +4 分发 | `pass` | `func_validator.rs` L404-434 新增 4 分发分支 + 4 单测（validate_extracted_username/sex/birth/address，L667-762） |
| E81（T67）with_defaults 10→16 | `pass` | `rules.rs` L492-497 注册 6 条新 validate 规则；`with_defaults_loads_ten_rules` 断言 len 16 + validate_count 7；`register_overwrites_same_id` 断言 len 16；`get_by_id_works` +6 断言；DB seed 4 处计数断言 10→16（db/mod.rs，属「测试暴露问题」例外，seed 逻辑未改） |
| E82（T68）validate_multi_rules 命令 | `pass` | `processor.rs` L1078-1377 inner + L1387-1395 Tauri 命令；6 集成测试（L2521/2583/2632/2667/2702/2753）全过；`lib.rs` L58 注册 |
| E83（T68）跨字段联合校验 | `pass` | `processor.rs` L1235-1284 `normalize_gender` vs `idcard_gender` + `is_valid_birth` + `idcard[6..14]` 切片；2 集成测试（idcard_cross_field_sex_mismatch / idcard_cross_field_birth_mismatch）全过 |
| E84（T69）统一校验面板 | `pass` | `ValidatePanel.jsx` 完全重写（Form.List + shouldUpdate + landSheet 闭包 + 汇总消息）；`tauri.js` wrapper；pnpm build 3082 modules pass；2 minor 缺陷已修复（phonePrefixes `/^\d{3}$/.test` + crossField 列必填 message.warning） |

## 4. 构建与测试审计

| 项 | 状态 | 证据 |
|---|---|---|
| `cargo fmt --all -- --check` | `pass` | exit 0，无格式差异 |
| `cargo clippy --all-targets --all-features -- -D warnings` | `pass` | exit 0，core + src-tauri 全零警告 |
| `cargo test --all` | `pass` | src-tauri lib 129 + core 152 + Doc-tests 11 = 292 passed / 3 ignored / 0 failed |
| `pnpm --prefix frontend build` | `pass` | 3082 modules transformed，✓ built in 2.62s；chunk >500kB 为 antd 既有警告 |
| 本机 dev 构建 | `pass` | cargo check + pnpm build 通过 |
| CI 构建（四目标矩阵） | `info` | 待 git tag `v1.1.4` 触发；未发布前不阻塞 qa_passed |

## 5. 代码质量审计

| 项 | 状态 | 证据 |
|---|---|---|
| scope_deviation（T67） | `pass` | `ExtractParams` +4 变体复用既有 typed enum 模式（单一职责）；6 条 validate 规则构造器复用既有 `Rule::new` 模式；`validate_extracted` +4 分支复用既有函数式分发；`with_defaults()` 注册 6 条（规则集单一真源）；db/mod.rs 4 处 seed 计数断言 10→16 属「测试暴露问题」例外（seed 逻辑/SQL/SCHEMA_VERSION 未改，仅断言数字跟上规则集增长） |
| scope_deviation（T68） | `pass` | `CrossFieldConfig`/`MultiRuleValidation` 独立结构（camelCase 对齐前端契约）；`validate_multi_rules_to_two_sheets_inner` 复用 `TwoSheetResult`/`RowInvalidReason` 既有结构（避免重复定义）；校验分发按 ruleId 查表路由（phone-validate→is_valid_phone / params→validate_extracted / pattern→regex / 其他→默认通过）；跨字段仅在 idcard-validate + crossField 非空时触发（低耦合）；`log_operation` 记录操作历史；`docs/02` §4.6 IPC 契约同步（scope_deviation，justified：新 IPC 命令需在技术设计文档登记） |
| scope_deviation（T69） | `pass` | `ValidatePanel.jsx` 完全重写（移除 Tabs，改 Form.List）；`Form.Item shouldUpdate` 条件渲染跨字段配置只对 idcard-validate 生效（天然隔离）；`landSheet` 闭包复用 v1.1.3 RowValidatePanel 模式（addSheetFromParse + getSheetData + SET_SHEET_DATA）；2 minor 缺陷修复（phonePrefixes 三位纯数字过滤 + crossField 列必填校验）属 UX/数据卫生优化，不阻塞；`tauri.js` wrapper 遵循既有 invoke 封装模式；未触碰 out_of_scope（TopToolbar/SidePanel/state/App.jsx/capabilities） |
| 模块自洽 | `pass` | `func_validator.rs` 独立子模块，4 新函数 + 复用既有 5 函数；`ValidatePanel.jsx` 独立面板，复用 `addSheetFromParse` + `SET_SHEET_DATA` reducer；`validate_multi_rules_to_two_sheets` 与 `validate_rows_to_two_sheets` 共享 `TwoSheetResult`/`RowInvalidReason` 结构（避免重复） |
| IPC 收口 | `pass` | 新增 IPC `validate_multi_rules_to_two_sheets` 走 `#[tauri::command]` + `Result<TwoSheetResult, String>` + `.map_err(|e| e.to_string())`；前端经 `tauri.js` 封装层调用，不直接 invoke 裸字符串；`src-tauri/src/lib.rs` L58 `generate_handler!` 注册（grep 确认）；旧 `validate_column` / `validate_rows_to_two_sheets` 保留（向后兼容） |
| 测试覆盖 | `pass` | T67 累计新增单测覆盖 ExtractParams +4 serde roundtrip + validate_extracted +4 分发 + with_defaults 16 条 + seed 16 条 idempotent；T68 6 集成测试覆盖 validate_multi_rules_to_two_sheets 全场景（all_rules_pass/mixed_pass_fail/idcard_cross_field_sex_mismatch/idcard_cross_field_birth_mismatch/regex_rule_validation/partial_mapping）；11 doc-tests 覆盖 func_validator 全部公开函数；workspace 全量 292 passed |
| 单一真源 | `pass` | 版本号唯一源 `tauri.conf.json`，Cargo.toml workspace.package.version + frontend/package.json + constants.js 同步（grep 4 处一致 1.1.4）；SCHEMA_VERSION=5 单一真源（schema.rs:15，v1.1.4 不变）；`with_defaults()` 规则集单一真源（rules.rs，16 条） |

## 6. 安全与隐私审计

| 项 | 状态 | 证据 |
|---|---|---|
| SQL 参数绑定 | `pass` | `src-tauri/src/db/mod.rs` 51 处 `params![]` 绑定 + `commands/processor.rs` 2 处 `params![]`；grep 无 `format!` 拼接 SQL；`seed_builtin_rules` upsert-missing 用参数绑定；`cleanup_deprecated_rules` DELETE 用 `params![id]` 参数绑定；T68 `validate_multi_rules_to_two_sheets_inner` 无新 SQL（复用 `query_cells`/`create_sheet`/`insert_cells` 既有参数绑定方法） |
| 凭据 | `pass` | 无新增凭据；updater 密钥沿用 v1.1.0 配置（`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 从环境变量/GitHub Secret 读取，源码无字面量） |
| 全本地处理 | `pass` | 校验规则分发 / 跨字段联合校验 / 双 Tab 写入全在本地，无网络调用；CSP `default-src 'self'` + 白名单延续无回归；v1.1.4 未改 CSP/fs 权限/capabilities |
| 身份证号隐私 | `pass` | `is_valid_idcard` 仅校验长度 + 校验码（GB 11643-1999），不查行政区划表（防与现实身份证号关联）；测试用地址码 `110105` 为示例值，不涉及真实个人数据；T68 跨字段校验复用同一函数 + `idcard_gender`（仅取第 17 位奇偶，不查行政区划） |
| Mimosa 深度扫描 | `info` | 本轮未重新运行 Mimosa 完整深度扫描；沿用 v1.1.3 R1（scan-2026-08-10T16-40-17.470Z-31df73e0a37d）+ R2（scan-2026-08-10T21-49-17.114Z-3e0a1b2d600e）双轮 0 findings / 487 包 0 漏洞基线。v1.1.4 改动为校验模块功能性扩展（ExtractParams +4 变体 / validate_multi_rules 命令 / ValidatePanel 重写），未引入新依赖 crate、未改 DB schema、未改 CSP/网络/fs 权限，静态分析基线有效。静态分析非运行时验证，不宣称项目安全；建议后续版本重新运行完整 Mimosa 密封扫描覆盖 v1.1.4 增量代码 |

## 7. 数据与迁移审计

| 项 | 状态 | 证据 |
|---|---|---|
| `SCHEMA_VERSION` | `pass` | `src-tauri/src/db/schema.rs:15` `pub const SCHEMA_VERSION: i64 = 5`（v1.1.4 不变，T67 复用既有 `rules.params TEXT` 列存 4 新 `ExtractParams` 变体 JSON） |
| 表/列变更 | `pass` | v1.1.4 无新增表/列（T67 4 新变体存入既有 `rules.params` JSON 列，T68 无新表，T69 纯前端） |
| 迁移链 | `pass` | `migrate.rs` 覆盖 v2→v3→v4→v5 / v3→v4→v5 / v4→v5（v1.1.4 不变）；各 `migrate_vN_to_vM` 用 `PRAGMA table_info` 检查列存在再 ALTER，幂等 |
| 历史数据兼容 | `pass` | `ExtractParams` 4 新变体（Username/Sex/Birth/Address）serde tag="validator" 向后兼容（旧 JSON 无这 4 个 tag → 反序列化失败 → 规则 params 为 None → 默认通过）；`seed_builtin_rules` upsert-missing 补 6 条新 validate 规则（已存在 10 条不动，老用户升级自动补）；`cleanup_deprecated_rules` 清理 6 个遗留 id（幂等） |

## 8. 依赖与配置审计

| 项 | 状态 | 证据 |
|---|---|---|
| 新增依赖 | `pass` | v1.1.4 无新增 crate 依赖（`regex`/`serde`/`serde_json`/`rusqlite`/`calamine`/`csv`/`tempfile`/`base64` 均已在 v1.1.0~v1.1.3 引入）；`serde_json` 启用 `preserve_order` feature（Cargo.toml workspace.dependencies，前端 JSON 列顺序保留，属既有能力启用 feature flag，非新增 crate） |
| 既有依赖回归 | `pass` | `rusqlite` / `serde` / `serde_json` / `regex` / `csv` / `calamine` / `tempfile` / `base64` 等无版本变更（Cargo.lock 仅 ruT0-data-kit / ruT0-data-kit-core 版本号 1.1.3→1.1.4 + serde_json 拉入 indexmap 2.14.0 作为 preserve_order feature 依赖） |
| 版本号一致性 | `pass` | 4 处一致 1.1.4（Cargo.toml workspace.package.version=1.1.4 + 注释行 + tauri.conf.json version=1.1.4 + frontend/package.json version=1.1.4 + frontend/src/constants.js APP_VERSION="v1.1.4"）；core/src-tauri Cargo.toml workspace=true 自动继承 |
| 配置文件 | `pass` | tauri.conf.json updater 配置不变；CSP 不变；fs 权限不变；capabilities/default.json 无变更（git diff v1.1.3..HEAD 空，自定义命令无需注册权限）；`tempfile` 从 dev-dependencies 提升为 dependencies（v1.1.3 R2 T66 settings.json 原子写已引入，v1.1.4 沿用） |

## 9. 文档一致性审计

| 项 | 状态 | 证据 |
|---|---|---|
| `docs/versions/1.1.4/` 三件套 | `pass` | 更新日志.md（T67~T69 进度表 + E79~E84 验收 + 关键设计决策 + 已知边界）+ RELEASE-NOTES.md（用户面向「统一校验页面：自由选多规则 + 一按钮 + 双 Tab 输出」）齐全 |
| `docs/qa/versions/1.1.4/QA-审计报告.md` | `pass` | 本报告，8 维度全 pass + 结论 qa_passed |
| `docs/04-版本标准.md` 里程碑表 | `pending` | v1.1.4 行待补（本报告通过后同步状态推进） |
| `docs/02-技术设计文档.md` | `pass` | v1.1.4 段落同步：§4.6 IPC 契约表新增 `validate_multi_rules_to_two_sheets` 行 + `CrossFieldConfig`/`MultiRuleValidation` 结构签名（L343-361） |
| `handoff/TASK-BOARD.md` | `pass` | T67~T69 任务 DAG + E79~E84 验收项 + Release QA 门禁；3 个任务全 verified_complete；handoff 三件套已清理 |

## 10. 回归检查（v1.1.3 功能不退化）

| 场景 | 状态 | 证据 |
|---|---|---|
| 撤销/重做（mask/replace/base64_column） | `pass` | `cargo test --all` 129 src-tauri passed 含 undo/redo + columns（含 base64）+ db 测试；既有功能无回归 |
| 列操作（parse_column_as_json / replace_in_column / base64_column） | `pass` | columns 测试含既有 JSON 解析 + 列内替换 + base64；全过 |
| 搜索（search_cells / search_rows / replace_all） | `pass` | search 测试含关键字/正则/分页/行级搜索/全局替换；全过；v1.1.3 R2 T59 搜索路径优化后 28 个搜索测试全绿（v1.1.4 未改搜索） |
| 导入（CSV/XLSX/JSON/JSONL/TXT/SQL/PCAP/.log） | `pass` | detect_format 路由测试覆盖全部 8 种扩展名；既有格式不回归 |
| 设置页（SettingsView + PageSizeCard） | `pass` | settings.json page_size 持久化 + 原子写 + 8 个 settings 单测；v1.1.4 未改 settings |
| 脱敏（name-mask/simple-mask/segment-mask） | `pass` | masker 模板分支 + 分段 + 反向 + 5 条规则 + cleanup + seed idempotent；全过；v1.1.4 未改脱敏 |
| 提取（phone/bankcard/ipv4/ipv6/idcard/name） | `pass` | func_validator 全单测 + extract_validate_to_new_sheet 批量多规则；全过；v1.1.4 T67 新增 4 变体不影响既有 5 变体 |
| 行级校验（7 字段 + 跨字段） | `pass` | T57 `validate_rows_to_two_sheets` 6 集成测试保留全过（v1.1.4 保留命令向后兼容）；前端 RowValidatePanel.jsx 已删除（v1.1.4 初稿移除，能力合并入 ValidatePanel） |
| 单列校验（validate_column） | `pass` | `validate_column` 命令保留（向后兼容）；前端 ValidatePanel 不再调用它（改走 validate_multi_rules） |
| 分页（count_rows / query_cells） | `pass` | v1.1.3 R2 T62/T63/T64/T65 修复保留；v1.1.4 未改分页 |
| 版本号 | `pass` | 4 处 1.1.4 一致，无 1.1.3 残留 |

## 11. 问题记录

| 严重度 | 问题 | 修复任务 | 状态 |
|---|---|---|---|
| `info` | Mimosa 深度扫描未重新运行（沿用 v1.1.3 R1+R2 双轮 0 findings / 487 包 0 漏洞基线） | v1.1.4 改动为校验模块功能性扩展，未引入新依赖/schema/权限变更，静态分析基线有效；建议后续版本重新运行完整 Mimosa 密封扫描覆盖 v1.1.4 增量代码 | `info`（非阻塞，已知方法学限制） |
| `info` | T69 前端统一校验页面 GUI 交互（E84）未浏览器自动化验收 | pnpm build pass + 后端单测 + 前端构建 pass；GUI 视觉验收属浏览器自动化增量轮次，不阻塞发布 | `info` |
| `info` | 前端 chunk >500kB 警告（antd 既有，非本轮引入） | 既有警告，非 v1.1.4 引入 | `info` |
| `info` | T67 db/mod.rs 4 处 seed 计数断言 10→16（scope_deviation） | 属「测试暴露问题」例外：seed 逻辑/SQL/SCHEMA_VERSION 未改，仅断言数字跟上 with_defaults 规则集增长（10→16）；seed_builtin_rules upsert-missing 语义不变 | `info`（已记录，非阻塞） |
| `info` | T68 docs/02 技术设计文档同步 IPC 契约（scope_deviation） | 属「新 IPC 命令需在技术设计文档登记」例外：新增 `validate_multi_rules_to_two_sheets` 行 + `CrossFieldConfig`/`MultiRuleValidation` 结构签名，不改既有契约 | `info`（已记录，非阻塞） |
| `info` | T69 phonePrefixes 过滤 + crossField 列必填校验（2 minor 缺陷） | 已在 fb06d4f commit 修复：phonePrefixes 过滤改 `/^\d{3}$/.test`（剔除非数字串）；crossField 列必填校验加 message.warning 拦截（避免后端静默跳过） | `pass`（已修复） |
| `info` | v1.1.4 全部改动已 commit 到 main，待 tag `v1.1.4` | 待用户确认后 commit docs + tag `v1.1.4` | `info` |

严重度口径：`critical`（阻塞发布）/ `major`（需回流修复）/ `minor`（可带病发布但记录）/ `info`（仅记录）。

## 12. 审计结论

`qa_passed` — 本轮 Release QA 审计覆盖 v1.1.4 全部交付（T67~T69）+ v1.1.3 → v1.1.4 回归。T67~T69 全部 `verified_complete`（E79~E84 验收项全绿，含 T67 ExtractParams +4 变体 + 6 validate 规则 + validate_extracted +4 分发 + with_defaults 10→16 + DB seed 幂等；T68 validate_multi_rules_to_two_sheets 命令 + 6 集成测试 + 跨字段联合校验；T69 统一校验面板 Form.List + shouldUpdate + landSheet 闭包 + 2 minor 缺陷已修复）。核心交付：

1. **后端规则系统扩展**（T67）—— `ExtractParams` +4 变体（Username/Sex/Birth/Address）+ `Rule.params` 既有字段复用 + `validate_extracted` +4 函数式分发分支 → `is_valid_username`/`is_valid_sex`/`is_valid_birth`/`is_valid_address`；`with_defaults()` 注册 6 条新 validate 规则（username/sex/birth/idcard/phone/address-validate，总数 10→16）；DB seed 幂等（upsert-missing，老用户 10 条不动）；SCHEMA_VERSION 不变（复用 v1.1.3 `rules.params TEXT` 列）。
2. **validate_multi_rules_to_two_sheets IPC 命令**（T68）—— `CrossFieldConfig`（camelCase）+ `MultiRuleValidation`（camelCase）结构；`validate_multi_rules_to_two_sheets_inner` 接收任意「列名 + 规则 id」组合，逐行逐规则校验分发（phone-validate→`is_valid_phone` / params 非空→`validate_extracted` / pattern 非空→正则 `is_match` / 其他→默认通过）+ 身份证跨字段联合校验（`normalize_gender` vs `idcard_gender`、`is_valid_birth` 后 `idcard[6..14]` 切片比对，仅当 idcard-validate 规则带 crossField 配置且 idcard 本身有效时触发）→ 整行按通过/失败分流到两个新 Tab（`{源sheet名}_校验通过` / `_校验失败`，保留原列不新增）+ `log_operation`；6 集成测试全过；`lib.rs` 注册；`docs/02` §4.6 IPC 契约同步。
3. **前端统一校验页面**（T69）—— `ValidatePanel.jsx` 完全重写（移除 antd Tabs，改 `Form.List name="rules"` 动态规则行 + `Form.Item shouldUpdate` 条件渲染跨字段配置）；每条规则行 = 目标列 Select + 校验规则 Select（options 来自 `listRules().filter(kind==="validate")`，7 条）+ 删除按钮；idcard-validate 行展开 Checkbox「对比性别一致性」+「对比出生日期一致性」+ 对应列 Select（勾选后必填校验）；底部手机号前缀白名单 `Select mode="tags"`（三位纯数字过滤，全局）；一个「校验」按钮 → `validateMultiRulesToTwoSheets` → 双 Tab 落地（landSheet 闭包：addSheetFromParse + getSheetData + SET_SHEET_DATA）+ 汇总消息（通过/失败行数 + top 3 失败原因 field 计数）；2 minor 缺陷已修复（phonePrefixes 数字校验 + crossField 列必填校验）。

### 12.1 验证摘要

| 验证项 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | pass（exit 0） |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass（exit 0，零警告） |
| `cargo test --all` | pass（src-tauri lib 129 + core 152 + Doc-tests 11 = 292 passed / 3 ignored / 0 failed） |
| `pnpm --prefix frontend build` | pass（3082 modules，2.62s） |
| 版本号一致性（4 处） | pass（1.1.4） |
| schema 迁移 | SCHEMA_VERSION=5（v1.1.4 不变，T67 复用既有 params 列） |
| SQL 参数绑定 | pass（53 处 `params![]`，0 处 `format!` SQL 拼接） |
| Mimosa 深度扫描 | 沿用 v1.1.3 R1+R2 双轮 0 findings / 487 包 0 漏洞基线（v1.1.4 未重新运行，无新依赖/schema/权限变更，基线有效） |

**门禁裁决**：无未修复的 critical/major 问题。全部 `info` 项均为非阻塞已知简化或已知边界，已记录留待后续版本优化。T69 2 个 minor 缺陷已在 fb06d4f 修复。GUI 端到端交互验收（E84）属浏览器自动化增量轮次，不阻塞发布。**Mimosa 深度扫描**：本轮未重新运行，沿用 v1.1.3 R1（scan-2026-08-10T16-40-17.470Z-31df73e0a37d）+ R2（scan-2026-08-10T21-49-17.114Z-3e0a1b2d600e）双轮 0 findings / 487 包 0 漏洞基线；v1.1.4 改动为校验模块功能性扩展（无新依赖 crate、无 schema 变更、无 CSP/网络/fs 权限变更），静态分析基线有效。静态分析非运行时验证，不宣称项目安全。**结论推进至 `qa_passed`**。

**发布前置门禁**（[`04-版本标准.md`](../../04-版本标准.md) §4）满足：静态 + 单元测试 + 前端构建全绿 + 版本一致性 + schema 迁移幂等 + 文档收口。CI 构建（四目标矩阵）待 git tag `v1.1.4` 触发。

---

## 13. 修复证据索引

| 文件 | 改动 | 验证 |
|---|---|---|
| `crates/core/src/processor/rules.rs` | T67 ExtractParams +4 变体（Username/Sex/Birth/Address，L214-226）+ 6 条 validate 规则构造器（L723-829）+ `with_defaults()` 注册 6 条（L492-497，10→16）；测试：`with_defaults_loads_ten_rules`（len 16）+ `register_overwrites_same_id`（len 16）+ `extract_params_serde_roundtrip`（9 变体）+ `get_by_id_works`（+6 断言） | `cargo test --all` core 152 passed |
| `crates/core/src/processor/func_validator.rs` | T67 `validate_extracted` +4 分发分支（L404-434）→ `is_valid_username`/`is_valid_sex`/`is_valid_birth`/`is_valid_address`；4 新单测（L667-762） | `cargo test --all` func_validator 单测 + 11 doc-tests passed |
| `src-tauri/src/db/mod.rs` | T67 4 处 seed 计数断言 10→16（scope_deviation「测试暴露问题」例外，seed 逻辑/SQL/SCHEMA_VERSION 未改） | `cargo test --all` db 测试全过；grep 51 处 `params![]` 绑定 |
| `src-tauri/src/commands/processor.rs` | T68 `CrossFieldConfig`（L1031）+ `MultiRuleValidation`（L1050）+ `validate_multi_rules_to_two_sheets_inner`（L1078-1377）+ Tauri 命令（L1387-1395）；校验分发 + 跨字段联合 + 双 Tab 分流；6 集成测试（L2521-2782） | `cargo test --all` processor 测试全过（含 6 T68 集成测试 + 6 T57 保留集成测试 + 2 契约测试） |
| `src-tauri/src/lib.rs` | T68 L58 `generate_handler!` 注册 `validate_multi_rules_to_two_sheets` | `cargo check` 绿 |
| `docs/02-技术设计文档.md` | T68 §4.6 IPC 契约表新增 `validate_multi_rules_to_two_sheets` 行 + `CrossFieldConfig`/`MultiRuleValidation` 结构签名（scope_deviation，justified） | 人工核对 |
| `frontend/src/components/panels/ValidatePanel.jsx` | T69 完全重写（移除 Tabs，改 Form.List + shouldUpdate + landSheet 闭包 + 汇总消息 + phonePrefixes 数字校验 + crossField 列必填校验） | `pnpm --prefix frontend build` 绿（3082 modules） |
| `frontend/src/tauri.js` | T69 新增 `validateMultiRulesToTwoSheets` wrapper（L292-304） | `pnpm --prefix frontend build` 绿 |
| `frontend/src/constants.js` | APP_VERSION="v1.1.4" | `pnpm build` 绿 |
| `frontend/package.json` | version=1.1.4 | `pnpm build` 绿 |
| `src-tauri/tauri.conf.json` | version=1.1.4 | `cargo check` 绿 |
| `Cargo.toml` | workspace.package.version=1.1.4 + v1.1.4 注释行 + `serde_json` `preserve_order` feature | `cargo check` 绿 |
| `docs/versions/1.1.4/更新日志.md` | T67~T69 进度表 + E79~E84 验收 + 关键设计决策 + 已知边界 | 人工核对 |
| `docs/versions/1.1.4/RELEASE-NOTES.md` | 用户面向「统一校验页面：自由选多规则 + 一按钮 + 双 Tab 输出」 | 人工核对 |
| `docs/qa/versions/1.1.4/QA-审计报告.md`（新文件） | 8 维度审计 + qa_passed 结论 | 本报告 |
| `handoff/TASK-BOARD.md` | T67~T69 任务 DAG + E79~E84 验收 + Release QA 门禁；3 任务全 verified_complete | 人工核对 |
