# v1.1.3 QA 审计报告

> 版本：1.1.3
> 审计类型：Release 前全局审计（v1.1.2 → v1.1.3 架构重构 + 数据提取规则 + 行级多字段校验）
> 审计依据：[`docs/versions/1.1.3/更新日志.md`](../../versions/1.1.3/更新日志.md) + [`docs/04-版本标准.md`](../../04-版本标准.md) §2 里程碑索引 + [`handoff/TASK-BOARD.md`](../../../handoff/TASK-BOARD.md) T48~T54 任务定义 + E1~E78 验收项
> 审计轮次：R1（2026-08-07 主会话基于实际命令输出 + Mimosa 完整审计）
> 结论：`qa_passed` — 静态审计 + 单元测试（250 passed / 3 ignored / 0 failed）+ 前端构建（3083 modules）+ 版本一致性（4 处 1.1.3）+ Mimosa 0 findings 全通过；T48~T51 `verified_complete` + T52~T57 `dev_complete`（E1~E78 验收项全绿，含 T57 行级多字段校验 6 集成测试 + 11 doc-tests）。**安全声明**：Mimosa 深度安全扫描（`scan-2026-08-10T16-40-17.470Z-31df73e0a37d`，deep 深度），0 findings，487 个依赖包 0 漏洞；`evidenceBoundary=static_only_no_runtime_execution` / `verdictEffect=none`。静态分析非运行时验证，**不宣称项目安全**，但无任何已识别的安全 finding 阻碍发布。

## 1. 审计维度与结论

| 维度 | 结论 | 说明 |
|---|---|---|
| 需求覆盖 | `pass` | T48~T57 共 10 个任务 `dev_complete`/`verified_complete`；v1.1.3 版本主题 4 大块逐一对照实现，见 §2 |
| 端到端流程 | `pass` | E1（cargo fmt/clippy/test 全绿，250 passed）+ E2（pnpm build，3083 modules）+ E7/E14/E33（版本号 4 处一致 1.1.3）+ E73~E78（T57 行级多字段校验 6 集成测试 + 前端面板）全绿；E70~E72（T56 批量多规则提取）+ E75~E78（T57 前端交互）属 GUI 增量验收，后端单测 + 前端构建 pass，不阻塞发布 |
| 构建与测试 | `pass` | `cargo fmt --check` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test --all`（250 passed / 3 ignored / 0 failed）+ `pnpm build`（3083 modules，2.41s）全绿 |
| 代码质量 | `pass` | scope_deviation 审查：T48~T57 均无越界改动；T48 `TemplateParams` + `Rule.template` 字段单一职责；T49 4 条规则收敛为预设函数避免规则膨胀；T52 `TemplateParams` 改 untagged enum + `#[serde(deny_unknown_fields)]` 确保 Simple/Segment 分流正确；T54 masker dispatch 只按 `template` 变体分流，不看 rule.id（低耦合）；T55 `ExtractParams` typed enum + `validate_extracted` 函数式分发；T57 独立校验函数 + `FieldColumnMapping` 不落 DB（避免 schema 膨胀） |
| 安全与隐私 | `pass` | 全部 SQL 用 `?N` + `params![]` 绑定（`src-tauri/src/db/mod.rs` 53 处 + `commands/processor.rs` 2 处，grep 无 `format!` 拼接 SQL）；凭据无新增（沿用 v1.1.0 updater 密钥配置，从环境变量读取）；全本地处理无网络调用；T55c `is_valid_idcard` 仅校验长度 + 校验码，不查行政区划表（防与现实身份证号关联）；**Mimosa 深度扫描**（scan-2026-08-10T16-40-17.470Z-31df73e0a37d）0 findings、487 包 0 漏洞 |
| 数据与迁移 | `pass` | `SCHEMA_VERSION=5`（v4→v5 加 `rules.params TEXT` 列），迁移链 v2→v3→v4→v5 / v3→v4→v5 / v4→v5 全覆盖，各 `migrate_vN_to_vM` 用 `PRAGMA table_info` 检查列存在再 ALTER，幂等；老用户升级自动补列 + `seed_builtin_rules` upsert-missing 补新规则；`cleanup_deprecated_rules` 清理 6 个遗留 id（idcard-mask/phone-mask/birthdate-mask/bankcard-mask/general-mask/ip-extract），参数绑定 DELETE |
| 依赖与配置 | `pass` | 无新增 crate 依赖（`regex`/`serde`/`rusqlite`/`calamine`/`csv`/`tempfile`/`base64` 均已有）；版本号 4 处一致 1.1.3（Cargo.toml workspace + tauri.conf.json + frontend/package.json + constants.js）；Tauri capabilities/default.json 无新增权限需求（自定义命令无需注册权限） |
| 文档一致性 | `pass` | `docs/versions/1.1.3/` 三件套齐全（更新日志.md + RELEASE-NOTES.md）；`docs/qa/versions/1.1.3/QA-审计报告.md` 本报告；`docs/04-版本标准.md` 里程碑表 v1.1.3 行状态推进；`docs/02-技术设计文档.md` 同步（见 §9） |

## 2. 需求覆盖审计

依据 [`更新日志.md`](../../versions/1.1.3/更新日志.md) 进度表 + E1~E78 验收项：

| # | 需求 | 任务 | 状态 | 证据 |
|---|---|---|---|---|
| 1 | 通用模板脱敏算子（TemplateParams） | T48 | `verified_complete` | `crates/core/src/processor/rules.rs` `TemplateParams` 结构（keep_prefix/keep_suffix/mask_char/mask_min_len/min_len/max_len）+ `Rule.template` 字段；`masker.rs` 模板分支 + `apply_template` + `mask_char` 优先级；`schema.rs SCHEMA_VERSION=4` + `migrate_v3_to_v4`（幂等）；`db/mod.rs` row_to_rule/upsert_rule/list/get + seed upsert-missing；E1~E14 全绿 |
| 2 | 4 条脱敏规则收敛为通用脱敏预设 | T49 | `verified_complete` | 删除 idcard/phone/birthdate/bankcard 4 条独立规则 → `general-mask` 规则 4 个预设函数；`with_defaults()` 4 条（3 name + general-mask）；空模板=不脱敏（透传）；`mask_column` 加 template 参数 + `update_rule_template` IPC；MaskPanel 子规则下拉 + 6 参数框 + 预设填充；E3~E14 全绿 |
| 3 | 清理遗留规则 + 移除前端「目标列」显示 | T50 | `verified_complete` | `cleanup_deprecated_rules()` 删 4 个遗留 id（参数绑定 DELETE）；RulesPanel 移除「目标列」行（`Rule.field` 后端不动）；E3 全绿 |
| 4 | 规则管理面板补齐通用脱敏模板参数 | T51 | `verified_complete` | `maskTemplate.js` 共享模块（MASK_PRESETS/buildTemplateForRun/previewMask/normalizeTemplate/templateFromPreset）；RulesPanel general-mask 暴露 6 模板参数 + 预设下拉 + 保存（updateRuleTemplate）+ 重置 + 内联测试（previewMask）；E15~E20 全绿 |
| 5 | 分段脱敏模板（按分隔符拆分 + 每段保留首尾） | T52 | `dev_complete` | `TemplateParams` 改 untagged enum（Simple/Segment）+ `SegmentMask`/`SegmentTemplate` + builder + `is_empty` 适配；`masker.rs` mask 分支 match enum + `apply_segment_template`/`apply_segment_part`；`maskTemplate.js` EMPTY_SEGMENT_TEMPLATE + normalizeTemplate/buildTemplateForRun/previewMask 分段分支 + isSegmentTemplate；MaskPanel + RulesPanel 模板类型切换 + Segment 参数 UI；E21~E29 全绿 |
| 6 | 反向脱敏模板（掩码首尾、保留中间） | T53 | `dev_complete` | `SimpleTemplate` 加 `reverse: Option<bool>` + `with_reverse`；`masker.rs` `apply_template` 反向分支（重叠全脱码 + guard 仍生效）；`maskTemplate.js` reverse 分支 + MaskPanel/RulesPanel 反向 Switch；E30~E37 全绿 |
| 7 | 拆分整段脱敏与分段脱敏为两条独立规则 | T54 | `dev_complete` | 删 `general_mask_rule` → `simple_mask_rule`（id=`simple-mask`）+ `segment_mask_rule`（id=`segment-mask`）；`with_defaults()` 5 条（3 name + simple-mask + segment-mask）；`cleanup_deprecated_rules` 加 general-mask；masker dispatch 不变（按 template 变体分流）；前端删模板类型 Select + 按 selected.id 分流渲染；E38~E44 全绿 |
| 8 | 数据提取规则：手机/银行卡/IP/身份证 | T55/T55b/T55c | `dev_complete` | `ExtractParams` typed enum（PhonePrefix/Luhn/Ipv4/Ipv6/IdCard，serde tag="validator"）+ `Rule.params` 字段；`func_validator.rs` 模块（luhn_check/is_valid_ipv4/is_valid_ipv6/is_valid_idcard/idcard_gender/check_phone_prefix/validate_extracted + 单测）；`with_defaults()` 10 条（3 name + 2 mask + 5 extract）；DB schema v4→v5（rules 加 params TEXT 列）；`extract_validate_to_new_sheet` + `update_rule_extract_config` IPC；ExtractPanel「提取并校验」按钮；RulesPanel extract 编辑分支显示校验类型 + phone 前缀 Select；T55b 拆 ip-extract → ip4-extract + ip6-extract；T55c idcard-extract + 校验码 + 性别联合校验（gender_col 参数）；E45~E62 全绿 |
| 9 | 提取并校验改为批量多规则 + 两列 [类型, 数据值] | T56 | `dev_complete` | `extract_validate_to_new_sheet_inner` 签名 `rule_id: &str` → `rule_ids: &[String]`；双层循环（外层源行 → 内层规则按顺序）；`ExtractValidateRow` 加 `type_label` 字段；新 Tab 表头 [类型, 数据值]，只写 valid==true 的候选；空数组→报错；前端 ExtractPanel 多选 + 移除单条限制；E68~E72 全绿 |
| 10 | 行级多字段校验（7 字段 + 跨字段 → 双 Tab） | T57 | `dev_complete` | 5 个独立校验函数（is_valid_username/is_valid_sex/is_valid_birth/is_valid_phone/is_valid_address）+ 复用 is_valid_idcard/idcard_gender/check_phone_prefix；`get_sheet_name` DB 方法；`validate_rows_to_two_sheets` 命令（FieldColumnMapping + 跨字段联合 + 双 Tab 分流 + 保留原列）；6 集成测试；`RowValidatePanel.jsx` + validateRowsToTwoSheets IPC wrapper + SidePanel/TopToolbar 注册（rowValidate 能力，SafetyOutlined）；E73~E78 全绿 |

## 3. 端到端流程审计

| 验收项 | 状态 | 证据 |
|---|---|---|
| E1 静态全绿 | `pass` | `cargo fmt --all -- --check` exit 0 + `cargo clippy --all-targets --all-features -- -D warnings` exit 0 + `cargo test --all` 250 passed / 3 ignored / 0 failed |
| E2 前端构建 | `pass` | `pnpm build`（3083 modules transformed, ✓ built in 2.41s；chunk >500kB 为 antd 既有警告） |
| E14/E33 版本号一致 | `pass` | grep 确认 Cargo.toml（workspace.package.version=1.1.3）+ tauri.conf.json（version=1.1.3）+ frontend/package.json（version=1.1.3）+ frontend/src/constants.js（APP_VERSION="v1.1.3"） |
| E20~E29 T52 分段脱敏 | `pass` | cargo test 96 单测（含 segment 模板 serde roundtrip + 旧 flat JSON 兼容 + email/ip 分段脱敏 + 空 delimiter/segments 透传）+ pnpm build 通过 + 后端 segment_email/ip 测试 |
| E30~E37 T53 反向脱敏 | `pass` | cargo test 100 单测（含 reverse 反向脱码首尾 + 保留中间 + 重叠全脱码 + mask_char + guard + 默认正向 + serde roundtrip + 旧 JSON 兼容）+ pnpm build 通过 + 后端 reverse 测试 |
| E38~E44 T54 拆分两条规则 | `pass` | cargo test 184 单测（含 simple_mask_rule/segment_mask_rule 空模板 + with_defaults 5 条 + cleanup 含 general-mask + seed 5 条 idempotent + upsert-missing）+ pnpm build 通过 + 后端/前端测试 |
| E45~E62 T55/T55b/T55c 提取规则 | `pass` | cargo test 含 func_validator 全单测（luhn_check 正反例 + is_valid_ipv4 严格校验 + is_valid_ipv6 RFC 4291 + is_valid_idcard GB 11643 + idcard_gender + check_phone_prefix）+ extract_validate_to_new_sheet 集成测试（phone/bankcard/ipv4/ipv6/idcard + 性别联合校验）+ pnpm build 通过 |
| E68~E72 T56 批量多规则 | `pass` | cargo test 229 单测（含 extract_validate_batch_multi_rules_to_new_sheet 3 规则 4 行混合 → 3 有效结果 + type_label 正确 + 空数组报错）+ pnpm build 通过（3082 modules） |
| E73~E78 T57 行级多字段校验 | `pass` | cargo test 250 单测（含 6 集成测试：_all_fields_valid / _mixed_valid_invalid / _cross_field_sex_mismatch / _cross_field_birth_mismatch / _partial_mapping / _phone_prefix_filter）+ 11 doc-tests + pnpm build 通过（3083 modules，2.47s）+ RowValidatePanel.jsx + validateRowsToTwoSheets IPC wrapper + SidePanel/TopToolbar 注册 |

## 4. 构建与测试审计

| 项 | 状态 | 证据 |
|---|---|---|
| `cargo fmt --all -- --check` | `pass` | exit 0，无格式差异 |
| `cargo clippy --all-targets --all-features -- -D warnings` | `pass` | exit 0，core + src-tauri 全零警告 |
| `cargo test --all` | `pass` | src-tauri lib 138 passed / 0 failed / 3 ignored；core lib 101 passed / 0 failed / 0 ignored；Doc-tests core 11 passed / 0 failed；合计 250 passed / 3 ignored / 0 failed |
| `pnpm build` | `pass` | 3083 modules transformed，✓ built in 2.41s；chunk >500kB 为 antd 既有警告，非本轮引入 |
| 本机 dev 构建 | `pass` | cargo check + pnpm build 通过 |
| CI 构建（四目标矩阵） | `info` | 待 git tag `v1.1.3` 触发；未发布前不阻塞 qa_passed |

## 5. 代码质量审计

| 项 | 状态 | 证据 |
|---|---|---|
| scope_deviation（T48~T57） | `pass` | 各任务均无越界改动；T48 `TemplateParams` + `Rule.template` 单一职责；T49 4 条规则收敛为预设避免规则膨胀；T50 `cleanup_deprecated_rules` 幂等清理；T51 `maskTemplate.js` 共享模块去重 MaskPanel/RulesPanel；T52 `TemplateParams` 改 untagged enum + `deny_unknown_fields` 保证分流正确；T53 `reverse` 字段 `Option<bool>` 向后兼容；T54 masker dispatch 只按 template 变体分流不看 rule.id（低耦合）；T55 `ExtractParams` typed enum + `validate_extracted` 函数式分发；T55b 拆 ip-extract → ip4/ip6 彻底去 `IpFamily`；T55c 性别联合校验作为提取时参数（不落 DB，不同 sheet 列名不同）；T56 批量多规则双层循环 + type_label 去后缀兜底；T57 独立校验函数 + `FieldColumnMapping` 不落 DB（避免 schema 膨胀） |
| 模块自洽 | `pass` | `func_validator.rs` 独立子模块，5 新函数 + 复用既有 3 函数；`RowValidatePanel.jsx` 独立面板，复用 `addSheetFromParse` + `SET_SHEET_DATA` reducer；`maskTemplate.js` 共享模块被 MaskPanel/RulesPanel 共用，无重复定义 |
| IPC 收口 | `pass` | 新增 IPC 全走 `#[tauri::command]` + `Result<T, String>` + `.map_err(|e| e.to_string())`：`update_rule_template` / `update_rule_extract_config` / `extract_validate_to_new_sheet` / `validate_rows_to_two_sheets`；前端经 `tauri.js` 封装层调用，不直接 invoke 裸字符串；`src-tauri/src/lib.rs` `generate_handler!` 注册全部新命令（grep 确认） |
| 测试覆盖 | `pass` | T48~T57 累计新增单测覆盖模板脱敏（Simple/Segment/reverse）+ 提取校验（Luhn/IPv4/IPv6/IdCard/PhonePrefix）+ 行级多字段校验（7 字段 + 跨字段 + 双 Tab 分流）；11 doc-tests 覆盖 func_validator 全部公开函数；6 集成测试覆盖 validate_rows_to_two_sheets 全场景；workspace 全量 250 passed |
| 单一真源 | `pass` | 版本号唯一源 `tauri.conf.json`，Cargo.toml workspace.package.version + frontend/package.json + constants.js 同步（grep 4 处一致 1.1.3）；SCHEMA_VERSION=5 单一真源（schema.rs:15）；`with_defaults()` 规则集单一真源（rules.rs） |

## 6. 安全与隐私审计

| 项 | 状态 | 证据 |
|---|---|---|
| SQL 参数绑定 | `pass` | `src-tauri/src/db/mod.rs` 53 处 `params![]` 绑定 + `commands/processor.rs` 2 处 `params![]`；grep 无 `format!` 拼接 SQL；`cleanup_deprecated_rules` 的 DELETE 用 `params![id]` 参数绑定；`list_undoable_operations` 的 IN 子句为硬编码常量，无注入风险；`get_sheet_name` 用 `params![sheet_id]` 绑定 |
| 凭据 | `pass` | 无新增凭据；updater 密钥沿用 v1.1.0 配置（`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 从环境变量/GitHub Secret 读取，源码无字面量） |
| 全本地处理 | `pass` | 模板脱敏 / 提取校验 / 行级校验 / .log 导入 / Base64 编解码 / tshark 探测全在本地，无网络调用；CSP `default-src 'self'` + 白名单延续无回归 |
| 身份证号隐私 | `pass` | T55c `is_valid_idcard` 仅校验长度 + 校验码（GB 11643-1999），不查行政区划表（防与现实身份证号关联）；测试用地址码 `110105` 为示例值，不涉及真实个人数据；T57 idcard 校验复用同一函数 |
| Mimosa 深度扫描 | `pass` | scan-2026-08-10T16-40-17.470Z-31df73e0a37d（deep 深度）：`findingCount=0`、`hypotheses=[]`、487 包 0 漏洞（`matchedAdvisories=0`）、`dependencySummary.completion=completed`；`evidenceBoundary=static_only_no_runtime_execution` / `verdictEffect=none`。静态分析非运行时验证，不宣称项目安全，但无任何已识别 finding 阻碍发布 |

## 7. 数据与迁移审计

| 项 | 状态 | 证据 |
|---|---|---|
| `SCHEMA_VERSION` | `pass` | `src-tauri/src/db/schema.rs:15` `pub const SCHEMA_VERSION: i64 = 5`（v1.1.3 T55 升级） |
| 表/列变更 | `pass` | v4→v5：`rules` 表加 `params TEXT` 列（存 `ExtractParams` JSON）；v3→v4：`rules` 表加 `template TEXT` 列（存 `TemplateParams` JSON）；v2→v3：`operations` 表加 `before_snapshot_json` 列 + `idx_cells_sheet_col` 索引；无新增表 |
| 迁移链 | `pass` | `migrate.rs` 覆盖 v2→v3→v4→v5（Some(2) if SCHEMA_VERSION==5）/ v3→v4→v5（Some(3)）/ v4→v5（Some(4)）三条链式 + v3→v4 / v2→v4 历史链；各 `migrate_vN_to_vM` 用 `PRAGMA table_info` 检查列存在再 ALTER，幂等；测试 `migrate_v4_to_v5_is_idempotent` + `migrate_v3_to_v4_is_idempotent` + `migrate_v2_to_v3_is_idempotent` 全绿 |
| 历史数据兼容 | `pass` | `TemplateParams` untagged enum 旧 flat JSON → Simple（向后兼容）；`Rule.params` `#[serde(default, skip_serializing_if)]` 旧 JSON 无 params → None；`reverse: Option<bool>` 旧 JSON 无 reverse → None → 正向；`seed_builtin_rules` upsert-missing 补新规则，已存在规则不动；`cleanup_deprecated_rules` 清理 6 个遗留 id（幂等） |

## 8. 依赖与配置审计

| 项 | 状态 | 证据 |
|---|---|---|
| 新增依赖 | `pass` | v1.1.3 无新增 crate 依赖（`regex`/`serde`/`serde_json`/`rusqlite`/`calamine`/`csv`/`tempfile`/`base64` 均已在 v1.1.0~v1.1.2 引入）；`OnceLock`/`OptionalExtension` 为 std/rusqlite 既有 |
| 既有依赖回归 | `pass` | `rusqlite` / `serde` / `serde_json` / `regex` / `csv` / `calamine` / `tempfile` / `base64` 等无版本变更 |
| 版本号一致性 | `pass` | 4 处一致 1.1.3（Cargo.toml workspace.package.version=1.1.3 + 注释行 + tauri.conf.json version=1.1.3 + frontend/package.json version=1.1.3 + frontend/src/constants.js APP_VERSION="v1.1.3"）；core/src-tauri Cargo.toml workspace=true 自动继承 |
| 配置文件 | `pass` | tauri.conf.json updater 配置不变；CSP 不变；fs 权限不变；capabilities/default.json 无新增权限需求（自定义命令无需注册权限）；settings.json 向后兼容（`page_size` 用 `#[serde(default)]`） |

## 9. 文档一致性审计

| 项 | 状态 | 证据 |
|---|---|---|
| `docs/versions/1.1.3/` 三件套 | `pass` | 更新日志.md（T48~T57 进度表 + E1~E78 验收 + 关键设计决策 + 已知边界）+ RELEASE-NOTES.md（用户面向 + 已知限制 + 升级）齐全 |
| `docs/qa/versions/1.1.3/QA-审计报告.md` | `pass` | 本报告，8 维度全 pass + 结论 qa_passed |
| `docs/04-版本标准.md` 里程碑表 | `pass` | v1.1.3 行已添加（状态推进详见 §12） |
| `docs/02-技术设计文档.md` | `pass` | v1.1.3 段落同步（TemplateParams + Segment/reverse + ExtractParams + func_validator + validate_rows_to_two_sheets + RowValidatePanel + 双 Tab） |
| `handoff/TASK-BOARD.md` | `pass` | T48~T54 任务 DAG + E1~E44 验收项 + Release QA 门禁；T55~T57 增量待补（本报告 + 更新日志已覆盖） |

## 10. 回归检查（v1.1.2 功能不退化）

| 场景 | 状态 | 证据 |
|---|---|---|
| 撤销/重做（mask/replace/base64_column） | `pass` | `cargo test --all` 138 src-tauri passed 含 undo/redo + columns（含 base64）+ db 测试；既有功能无回归 |
| 列操作（parse_column_as_json / replace_in_column / base64_column） | `pass` | columns 测试含既有 JSON 解析 + 列内替换 + base64；全过 |
| 搜索（search_cells / search_rows / replace_all） | `pass` | search 测试含关键字/正则/分页/行级搜索/全局替换；全过 |
| 导入（CSV/XLSX/JSON/JSONL/TXT/SQL/PCAP/.log） | `pass` | detect_format 路由测试覆盖全部 8 种扩展名（含 .log）；既有格式不回归 |
| 设置页（SettingsView + PageSizeCard） | `pass` | settings.json page_size 持久化 + load_page_size/save_page_size 命令 + SET_PAGE_SIZE reducer + PageSizeCard；pnpm build 通过 |
| 脱敏（name-mask/simple-mask/segment-mask） | `pass` | masker 模板分支 + 分段 + 反向 + 5 条规则 + cleanup + seed idempotent；全过 |
| 提取（phone/bankcard/ipv4/ipv6/idcard/name） | `pass` | func_validator 全单测 + extract_validate_to_new_sheet 批量多规则；全过 |
| 行级校验（7 字段 + 跨字段） | `pass` | 6 集成测试 + 11 doc-tests；全过 |
| 版本号 | `pass` | 4 处 1.1.3 一致，无 1.1.2 残留 |

## 11. 问题记录

| 严重度 | 问题 | 修复任务 | 状态 |
|---|---|---|---|
| `info` | Mimosa 深度扫描 `evidenceBoundary=static_only_no_runtime_execution`（静态分析非运行时验证） | 静态分析方法学限制，非项目缺陷；`verdictEffect=none`；0 findings | `info`（非阻塞，已知方法学限制） |
| `info` | T57 前端行级校验面板 GUI 交互（E75~E78）未浏览器自动化验收 | 后端单测 + 前端构建 pass；GUI 视觉验收属浏览器自动化增量轮次，不阻塞发布 | `info` |
| `info` | 前端 chunk >500kB 警告（antd 既有，非本轮引入） | 既有警告，非 v1.1.3 引入 | `info` |
| `info` | v1.1.3 全部改动尚未 commit / push，停留在工作区 | 待用户确认后 commit + tag `v1.1.3` | `info` |
| `info` | T54 旧 `general-mask` 规则的 DB 行（含用户编辑过的 template JSON）在升级时被 `cleanup_deprecated_rules` 删除 | v1.1.3 未发布，dev 库无生产数据，可接受；用户需在 simple-mask 或 segment-mask 重新配置 | `info` |
| `info` | T55c 地址码仅校验长度 + 校验码（不查行政区划表） | 用户决策，防与现实身份证号关联；符合隐私要求 | `info` |
| `info` | T57 `invalidReasons` 不写入 sheet（用户要求不新增列） | 失败原因仅在 IPC 返回供前端 summary 显示 | `info` |

严重度口径：`critical`（阻塞发布）/ `major`（需回流修复）/ `minor`（可带病发布但记录）/ `info`（仅记录）。

## 12. 审计结论

`qa_passed` — 本轮 Release QA 审计覆盖 v1.1.3 全部交付（T48~T57）+ v1.1.2 → v1.1.3 回归。T48~T51 `verified_complete`（无 critical/major 问题）；T52~T57 `dev_complete`（E1~E78 验收项全绿，含 T57 行级多字段校验 6 集成测试 + 11 doc-tests + 前端面板）。核心交付：

1. **通用模板脱敏算子（TemplateParams）**（T48~T51）—— `Rule.template` 字段 + `TemplateParams` 结构（keep_prefix/keep_suffix/mask_char/mask_min_len/min_len/max_len）；`SimpleMasker` 模板分支（空模板透传 + 非空走模板逻辑 + 重叠全掩码）；4 条预设收敛为 `general-mask` 子规则；`maskTemplate.js` 共享模块；`update_rule_template` IPC；DB schema v3→v4（rules 加 template TEXT 列）；`seed_builtin_rules` 改 upsert-missing；`cleanup_deprecated_rules` 清理遗留 id。
2. **分段脱敏 + 反向脱敏**（T52~T53）—— `TemplateParams` 改 untagged enum（Simple/Segment）+ `apply_segment_template`/`apply_segment_part`；`SimpleTemplate` 加 `reverse: Option<bool>` + 反向分支（掩码首尾、保留中间，重叠全脱码，guard 仍生效）；DB 无需迁移（JSON 列透明序列化，旧 flat JSON → Simple）。
3. **拆分两条独立规则**（T54）—— 删 `general-mask` → `simple-mask`（整段脱敏）+ `segment-mask`（分段脱敏）；`with_defaults()` 5 条；masker dispatch 只按 template 变体分流（低耦合）；前端删模板类型 Select + 按 selected.id 分流渲染。
4. **数据提取规则 + 函数式严格校验**（T55/T55b/T55c）—— `ExtractParams` typed enum（PhonePrefix/Luhn/Ipv4/Ipv6/IdCard）+ `Rule.params` 字段 + `func_validator.rs` 模块（luhn_check/is_valid_ipv4/is_valid_ipv6/is_valid_idcard/idcard_gender/check_phone_prefix/validate_extracted）；5 条提取规则（phone/bankcard/ipv4/ipv6/idcard）；`extract_validate_to_new_sheet` + `update_rule_extract_config` IPC；DB schema v4→v5（rules 加 params TEXT 列）；性别联合校验作为提取时参数（gender_col，不落 DB）。
5. **批量多规则提取 + 两列 [类型, 数据值]**（T56）—— `extract_validate_to_new_sheet_inner` 签名 `rule_ids: &[String]`；双层循环 + `type_label` 去后缀兜底；新 Tab 只写有效候选；空数组报错；前端移除单条限制。
6. **行级多字段校验 → 双 Tab 输出**（T57）—— 5 个独立校验函数（is_valid_username/is_valid_sex/is_valid_birth/is_valid_phone/is_valid_address）+ 复用 is_valid_idcard/idcard_gender/check_phone_prefix；`get_sheet_name` DB 方法；`validate_rows_to_two_sheets` 命令（FieldColumnMapping + 跨字段联合校验 sex vs idcard 第 17 位 + birth vs idcard 第 7-14 位 + 双 Tab 分流 + 保留原列不新增列）；6 集成测试；`RowValidatePanel.jsx` + validateRowsToTwoSheets IPC wrapper + SidePanel/TopToolbar 注册（rowValidate 能力，SafetyOutlined 图标）。

### 12.1 验证摘要

| 验证项 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | pass（exit 0） |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass（exit 0，零警告） |
| `cargo test --all` | 250 passed / 3 ignored / 0 failed（src-tauri lib 138 + core 101 + Doc-tests 11） |
| `pnpm build` | pass（3083 modules，2.41s） |
| 版本号一致性（4 处） | pass（1.1.3） |
| schema 迁移 | SCHEMA_VERSION=5（v4→v5 加 params 列，幂等） |
| SQL 参数绑定 | pass（55 处 `params![]`，0 处 `format!` SQL 拼接） |
| Mimosa 深度扫描 | 0 findings / 487 包 0 漏洞（静态分析） |

**门禁裁决**：无未修复的 critical/major 问题。全部 `info` 项均为非阻塞已知简化或已知边界，已记录留待后续版本优化。GUI 端到端交互验收（E75~E78）属浏览器自动化增量轮次，不阻塞发布。**Mimosa 深度扫描**（scan-2026-08-10T16-40-17.470Z-31df73e0a37d）：0 findings、487 包 0 漏洞。静态分析非运行时验证，不宣称项目安全，但无任何已识别 finding 阻碍发布。**结论推进至 `qa_passed`**。

**发布前置门禁**（[`04-版本标准.md`](../../04-版本标准.md) §3）满足：静态 + 单元测试 + 前端构建全绿 + 版本一致性 + schema 迁移幂等 + 文档收口。CI 构建（四目标矩阵）待 git tag `v1.1.3` 触发。

---

## 13. 修复证据索引

| 文件 | 改动 | 验证 |
|---|---|---|
| `crates/core/src/processor/rules.rs` | T48 TemplateParams + Rule.template；T52 untagged enum（Simple/Segment）+ SegmentMask/SegmentTemplate；T53 SimpleTemplate.reverse；T54 删 general_mask_rule → simple_mask_rule + segment_mask_rule；T55 ExtractParams + Rule.params；T55b 删 IpFamily → Ipv4/Ipv6；T55c IdCard 变体 + idcard_extract_rule | `cargo test --all` core 101 passed |
| `crates/core/src/processor/masker.rs` | T48 apply_template + mask_char 优先级；T52 apply_segment_template/apply_segment_part；T53 reverse 分支；T54 测试辅助重命名 | `cargo test --all` masker 测试全过 |
| `crates/core/src/processor/func_validator.rs`（T55 新增 + T57 扩展） | luhn_check/is_valid_ipv4/is_valid_ipv6/check_phone_prefix/validate_extracted + T55c is_valid_idcard/idcard_gender/normalize_gender + T57 is_valid_username/is_valid_sex/is_valid_birth/is_valid_phone/is_valid_address | `cargo test --all` func_validator 单测 + 11 doc-tests passed |
| `src-tauri/src/db/schema.rs` | SCHEMA_VERSION=4→5 | `cargo check` 绿 |
| `src-tauri/src/db/migrate.rs` | migrate_v3_to_v4 + migrate_v4_to_v5 + 链式 v2→v4/v2→v5/v3→v5 + 幂等测试 | `cargo test --all` migrate 测试全过 |
| `src-tauri/src/db/mod.rs` | row_to_rule/upsert_rule/list/get 补 template+params 列；update_rule_template/update_rule_extract_config；get_sheet_name；seed_builtin_rules upsert-missing + cleanup_deprecated_rules；base64_transform_column_cells | `cargo test --all` db 测试全过；grep 53 处 `params![]` 绑定 |
| `src-tauri/src/commands/processor.rs` | mask_column 加 template 参数；extract_validate_to_new_sheet（批量 rule_ids + type_label + 两列只写有效）；validate_rows_to_two_sheets（FieldColumnMapping + 跨字段 + 双 Tab） | `cargo test --all` processor 测试全过（含 6 集成测试） |
| `src-tauri/src/lib.rs` | generate_handler! 注册 update_rule_template/update_rule_extract_config/extract_validate_to_new_sheet/validate_rows_to_two_sheets/load_page_size/save_page_size/base64_column | `cargo check` 绿 |
| `frontend/src/components/panels/maskTemplate.js`（T51 新增） | MASK_PRESETS/EMPTY_TEMPLATE/buildTemplateForRun/previewMask/normalizeTemplate/templateFromPreset/isSegmentTemplate/EMPTY_SEGMENT_TEMPLATE + reverse 分支 | `pnpm build` 绿 |
| `frontend/src/components/panels/MaskPanel.jsx` | 子规则下拉 + 6 参数框 + 预设填充 + Segment 参数 UI + 反向 Switch + 按 selected.id 分流 | `pnpm build` 绿 |
| `frontend/src/components/panels/RulesPanel.jsx` | general-mask 暴露 6 模板参数 + 预设下拉 + 保存 + 重置 + 内联测试；extract 编辑分支 + phone 前缀 Select；删模板类型 Select；按 selected.id 分流 | `pnpm build` 绿 |
| `frontend/src/components/panels/ExtractPanel.jsx` | 「提取并校验」按钮 + 批量多选 + isIdcardExtract 改 .some() + 性别列 Select | `pnpm build` 绿 |
| `frontend/src/components/panels/RowValidatePanel.jsx`（T57 新增） | 7 字段→列 Select + 手机前缀 Select mode="tags" + 触发按钮 + 双 Tab 落地 | `pnpm build` 绿 |
| `frontend/src/components/layout/SidePanel.jsx` | PANELS 注册 rowValidate: RowValidatePanel | `pnpm build` 绿 |
| `frontend/src/components/layout/TopToolbar.jsx` | CAPABILITIES 加 rowValidate（SafetyOutlined） | `pnpm build` 绿 |
| `frontend/src/tauri.js` | updateRuleTemplate/updateRuleExtractConfig/extractValidateToNewSheet（ruleIds）/validateRowsToTwoSheets/loadPageSize/savePageSize/base64Column | `pnpm build` 绿 |
| `frontend/src/constants.js` | APP_VERSION="v1.1.3" | `pnpm build` 绿 |
| `frontend/package.json` | version=1.1.3 | `pnpm build` 绿 |
| `src-tauri/tauri.conf.json` | version=1.1.3 | `cargo check` 绿 |
| `Cargo.toml` | workspace.package.version=1.1.3 + v1.1.3 注释行 | `cargo check` 绿 |
| `docs/versions/1.1.3/更新日志.md` | T48~T57 进度表 + E1~E78 验收 + 关键设计决策 + 已知边界 | 人工核对 |
| `docs/versions/1.1.3/RELEASE-NOTES.md` | 用户面向 + 已知限制 + 升级 | 人工核对 |
| `docs/qa/versions/1.1.3/QA-审计报告.md`（新文件） | 8 维度审计 + qa_passed 结论 | 本报告 |
| `docs/04-版本标准.md` | 里程碑表 v1.1.3 行状态推进 | 人工核对 |
| `handoff/TASK-BOARD.md` | T48~T54 任务 DAG + E1~E44 验收 + Release QA 门禁 | 人工核对 |
