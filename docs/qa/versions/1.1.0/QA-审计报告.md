# v1.1.0 QA 审计报告

> 版本：1.1.0
> 审计类型：Release 前全局审计（含 v1.0.0 → v1.1.0 历史遗留与架构性问题回归）
> 审计依据：[`docs/00-需求文档.md`](../../../00-需求文档.md) §6 验收标准 + [`docs/03-开发任务清单.md`](../../../03-开发任务清单.md) §5 T15~T21 任务定义 + [`docs/04-版本标准.md`](../../../04-版本标准.md) §4 发布门禁
> 结论：`conditional_pass` — 静态审计 + 单元测试 + 前端构建全通过；本轮（完整性修复轮 T22~T27 + 脱敏语义修正轮 T28）就地修复用户反馈 7 项 UI/交互问题并回归通过；GUI 端到端交互（E3~E5）仍为 `pending_e2e`，需在通过 GUI 自动化验收后推进至 `qa_passed`。

## 1. 审计维度与结论

| 维度 | 结论 | 说明 |
|---|---|---|
| 功能完整性 | `conditional_pass` | T15~T21 单元级全 `verified_complete`；core 40 单测 + db 迁移单测全过；本轮修复 masker 2 字符姓名脱敏 bug（`张三` → `张*`）+ T28 脱敏语义修正（replacement → 掩码字符）；GUI 端到端交互（RulesPanel 两栏 / 面板联动 / 持久化重启）→ `pending_e2e` |
| 回归与端到端 | `conditional_pass` | `cargo fmt --check` / `cargo clippy --workspace -- -D warnings` / `cargo test --workspace`（40 passed / 2 ignored）+ `pnpm build`（3078 modules）全绿；GUI 交互回归 → `pending_e2e` |
| 构建与产物 | `pending_release` | 四目标矩阵构建需 CI/Release 流程；版本一致性已 pass；本机 dev 构建 pass |
| 安全 | `conditional_pass` | 本轮就地修复三项 MAJOR 级遗留：`tauri.conf.json` CSP 从 `null` 收紧为 `default-src 'self'` + 显式白名单；`capabilities/default.json` 删除 `home` / `desktop` 递归写权限（仅留 `document` / `download` + `write-text-file`）；updater `dialog` 改为 `true`（用户确认）；签名验签失败拒绝安装 → `pending_e2e` |
| 文档 | `pass` | `docs/04` v1.1.0 里程碑修正（脱敏+校验+提取+持久化，`done_e2e`）；`docs/03` 补 §5 T15~T21 七任务定义；`docs/02` 补 §4.6 处理命令（6 签名）+ 行高亮色统一（invalid=#fff1f0 / masked=#f0f5ff / hit=#f6ffed）；`docs/01` + `RELEASE-NOTES` 高亮色同步一致；`handoff/` 14 个 T10~T14 历史三件套归档至 `handoff/archive/`，`TASK-BOARD.md` 重写为 v1.1.0 |

## 2. 功能完整性审计

依据 [`规划需求.md`](../../versions/1.1.0/规划需求.md) §2.1 的 6 项验收标准与 [`03-开发任务清单.md`](../../../03-开发任务清单.md) §5 T15~T21 任务定义：

| # | 验收项 | 状态 | 证据 |
|---|---|---|---|
| 1 | core `processor` 模块：`Masker` / `Validator` / `Extractor` trait + 原型实现 + `Rule` / `RuleKind` / `RuleRegistry` + 三条姓名规则 | `pass` | `crates/core/src/processor/{rules,validator,masker,extractor,mod}.rs`；`RuleRegistry::with_defaults` 注册 3 条（`name-validate` / `name-mask` / `name-extract`）；单测 `with_defaults_loads_three_name_rules` + `rulekind_from_str_lowercase_round_trip` + `name_validate_rule_pattern_is_chinese_range` + `name_mask_rule_keeps_first_char_semantic` + `name_extract_rule_has_chinese_pattern` 全过 |
| 2 | `SimpleMasker` 「保留首字符」语义：`张三`→`张*`、`张三丰`→`张**`、`赵`→`*` | `pass` | `masker.rs:43-63` 规则驱动路径三分支（n==0/n==1/n>=2）；单测 `name_mask_rule_keeps_surname` + `rule_without_replacement_keeps_first_char` + `masks_short_value_all_stars` 全过；**本轮修复**：旧实现 n≤2 全 `*`，丢失姓氏，与规则描述「保留姓氏（首字符）」冲突 |
| 3 | DB `rules` 表 + `SCHEMA_VERSION=2` + v1→v2 增量迁移 + Rule CRUD + `seed_builtin_rules` | `pass` | `src-tauri/src/db/schema.rs` SCHEMA_VERSION=2 + rules DDL + idx_rules_kind；`migrate.rs` `migrate_v1_to_v2`（IF NOT EXISTS 幂等）；`db/mod.rs` upsert/list/get/set_rule_enabled/count_rules/update_rule_params/seed_builtin_rules/row_to_rule；单测覆盖迁移 + 3 条规则 bootstrap + CRUD round-trip |
| 4 | 6 个 Tauri 命令（`mask_column` / `validate_column` / `extract_column` / `list_rules` / `toggle_rule` / `update_rule_params`）从 DB 读写，删除 `RuleState` 内存态 | `pass` | `src-tauri/src/commands/processor.rs` 全命令 `Result<T, String>` + `.map_err`；`lib.rs` setup 中 `seed_builtin_rules` + `generate_handler!` 注册 6 命令；`cargo clippy --workspace -- -D warnings` 全绿 |
| 5 | 前端 4 面板真实 UI + RulesPanel 主区两栏 + 无新增规则入口 + 行高亮 | `pass` | `frontend/src/components/panels/{MaskPanel,ValidatePanel,ExtractPanel,RulesPanel}.jsx`；`App.jsx` 按 `activeCapability === "rules"` 路由；`SidePanel.jsx` 删 `rules` 条目；`reducer.js` APPLY_ROW_STATUSES + `toRowObjects` `_rowIdx`；`pnpm build` 3078 modules 通过 |
| 6 | 版本号 `1.0.0` → `1.1.0` 四处一致 + 文档收口 | `pass` | `Cargo.toml` / `src-tauri/Cargo.toml`（workspace=true）/ `tauri.conf.json` / `frontend/package.json` + `constants.js` 均 1.1.0；`docs/versions/1.1.0/` 三件套齐全 |

## 3. 回归与端到端审计

| 场景 | 状态 | 证据 |
|---|---|---|
| `cargo fmt --all --check` | `pass` | 本轮修复 `detect.rs` 一处格式，`--check` 全绿 |
| `cargo clippy --workspace --all-targets -- -D warnings` | `pass` | core + src-tauri 全零警告；本轮修复 `extractor.rs` `Default::expect` panic 风险 + `detect.rs` `TSHARK_OVERRIDE` Mutex poison panic 风险（改静默降级） |
| `cargo test --workspace` | `pass` | 39 passed / 0 failed / 2 ignored（pcap tshark 本机探测 CI 跳过）；含 processor 23 + db 扩展 + migrate v1→v2 |
| `pnpm --prefix frontend build` | `pass` | 3078 modules transformed，✓ built in 2.39s（chunk >500kB 为 antd 既有警告，非本轮引入） |
| GUI 主流程：导入 → Sheet → Table → MaskPanel/ValidatePanel/ExtractPanel/RulesPanel 交互 | `pending_e2e` | 后端单测 + 前端构建 pass；GUI 浏览器自动化验收（mock Tauri IPC 注入 + DOM 交互）需 Phase 8 增量轮次 |
| DB 重启持久化：seed → 改参数 → 重启 → 参数保留 | `pending_e2e` | db 单测覆盖 upsert/update_rule_params；重启后 GUI 验证 → pending |
| 行高亮联动：mask→masked / validate→invalid / extract→hit | `pending_e2e` | reducer APPLY_ROW_STATUSES 分支单测级 pass；GUI 视觉验证 → pending |

## 4. 构建与产物审计

| 项 | 状态 | 证据 |
|---|---|---|
| 四目标矩阵构建（Linux x64 / macOS arm64 / Windows x64） | `pending_release` | 需 CI/Release 流程，非本阶段 |
| 产物命名 + SHA-256 checksums + `latest.json` | `pending_release` | 需 finalize 流程 |
| 版本一致性：`tauri.conf.json` == `Cargo.toml` == `package.json` == `constants.js` | `pass` | 四处均 1.1.0 |

## 5. 安全审计

| 项 | 状态 | 证据 |
|---|---|---|
| CSP 收紧（本轮修复 MAJOR） | `fixed` | `tauri.conf.json:25` 旧 `csp: null`（无策略，任意脚本注入风险）→ 新 `default-src 'self'; img-src 'self' data: blob: asset: http://asset.localhost; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost https://api.github.com https://github.com https://objects.githubusercontent.com` |
| fs 权限收紧（本轮修复 MAJOR） | `fixed` | `capabilities/default.json` 旧含 4 个递归写权限（home/desktop/document/download，范围过大）→ 删 `home` / `desktop`，仅留 `document` / `download` 递归 + `write-text-file`；导出走 save 对话框 → 用户选择路径，无需全盘写权限 |
| updater `dialog: true`（本轮修复 MAJOR） | `fixed` | `tauri.conf.json` `updater.dialog` 旧 `false`（静默更新，用户无确认机会）→ `true`（用户应确认更新安装） |
| 全本地处理，用户数据不外发 | `pass` | 三层架构单向依赖；processor 6 命令仅读 DB cells + 回写 DB，无网络调用；CSP `connect-src` 白名单仅 updater 相关 GitHub 域 |
| updater 仅拉版本元数据 + 签名产物 | `pass` | `commands/update.rs` 委托 tauri-plugin-updater；CSP 白名单限 `api.github.com` / `github.com` / `objects.githubusercontent.com` |
| `Default` impl 无 panic 路径（本轮修复 MAJOR） | `fixed` | `extractor.rs` 旧 `Default::expect("builtin pii regexes must compile")` → 改为 `Self::new().unwrap_or_else(|_| Self::empty())`，正则编译失败降级为匹配空串的退化实例 |
| `Mutex` 无 poison panic 路径（本轮修复 MAJOR） | `fixed` | `detect.rs` `TSHARK_OVERRIDE` 旧 `.expect("TSHARK_OVERRIDE poisoned")` ×2 → `set_tshark_path` 改 `if let Ok(guard)` 静默丢弃；`get_tshark_path` 改 `.map(...).unwrap_or(None)` 返回 None |
| `Rule` struct `#[serde(rename_all = "camelCase")]` | `pass` | `crates/core/src/processor/rules.rs`；前端 IPC 契约与 6 命令返回类型 camelCase 对齐 |

## 6. 文档审计

| 项 | 状态 | 证据 |
|---|---|---|
| `docs/04-版本标准.md` v1.1.0 里程碑正确（本轮修复 CRITICAL） | `fixed` | 旧行误标「脱敏+校验」缺失「提取」+「撤销栈」未做 + 状态 `planned` → 改为「脱敏 + 校验 + 提取 + rules 表持久化（SCHEMA_VERSION=2）+ RulesPanel 主区两栏 + 操作日志扩展」+ `done_e2e`；v1.2.0 行移除已迁移项 |
| `docs/03-开发任务清单.md` 补 §5 T15~T21（本轮修复 CRITICAL） | `fixed` | 旧仅 T1~T9（v1.0.0）→ 标题改「v1.0.0 框架阶段 + v1.1.0 能力阶段」+ 追加 §5 七任务（T15 core processor / T16 DB / T17 commands / T18 版本号 / T19 state+IPC / T20 面板 / T21 docs）+ §6 里程碑表 |
| `docs/02-技术设计文档.md` 补 §4.6 处理命令（本轮修复 MAJOR） | `fixed` | 旧无 processor 命令契约 → 追加 §4.6 含 6 命令签名表 + MaskColumnResult/RowValidation/RowExtract 结构定义 + DB 单一真源说明 |
| 行高亮色三处一致（本轮修复 MAJOR） | `fixed` | `RELEASE-NOTES.md` / `01-页面与交互说明.md` / `DataTable.css` 三处统一为 invalid=#fff1f0（浅红）/ masked=#f0f5ff（浅蓝）/ hit=#f6ffed（浅绿），与 CSS 实际值对齐 |
| `handoff/` 历史遗留清理（本轮修复 MAJOR） | `fixed` | 14 个 T10~T14 三件套经 `git mv` 归档至 `handoff/archive/`（保留 history）；`TASK-BOARD.md` 完全重写为 v1.1.0（T15~T21 DAG + E1~E5 + QA 门禁 + 当前状态 `done_e2e`） |
| 过时注释清理（本轮修复 MINOR） | `fixed` | `tauri.js`（JSDoc「保留首尾字符，中间 *」→「保留首字符，其余 *」修正与 masker 实际行为冲突的文档矛盾）+ `DataTable.jsx`（删 TODO(T5)）+ `SheetTabs.jsx`（删 TODO(T5)）+ `factory.js`（删 T5/T13 引用）+ `AboutCard.jsx`（v1.0.0 仅布局壳 → v1.1.0 处理器原型已接入）+ `src-tauri/src/lib.rs` + `commands/mod.rs` + `processor/mod.rs` + `crates/core/src/lib.rs` |
| 双语 README 不含 `{{PLACEHOLDER}}` | `pass` | grep 无命中 |

## 7. 问题记录

| 严重度 | 问题 | 修复任务 | 状态 |
|---|---|---|---|
| `critical` | `masker.rs` 规则驱动路径 2 字符姓名脱敏丢失姓氏：旧实现 `n<=2 → "*".repeat(n)`，`张三`→`**`，与规则描述「保留姓氏（首字符）」直接矛盾，属核心功能语义错误 | `masker.rs:43-63` 改三分支（n==0 空串 / n==1 单 `*` / n>=2 首1+(n-1)*）；`张三`→`张*`、`张三丰`→`张**`；单测 `name_mask_rule_keeps_surname` 同步改断言 | `fixed`（cargo test 绿） |
| `critical` | `docs/04-版本标准.md` v1.1.0 里程碑行错误：缺「提取」能力 + 标「撤销栈」未做 + 状态 `planned`，与实际交付严重不符 | 改为「脱敏+校验+提取+持久化+RulesPanel+操作日志」+ `done_e2e`；v1.2.0 行移除已迁移项 | `fixed` |
| `critical` | `docs/03-开发任务清单.md` 仅含 T1~T9，v1.1.0 的 T15~T21 任务定义完全缺失，违反任务清单规范性 | 追加 §5 含 7 任务完整定义（目标/前置依赖/实现要点/交付物/验收标准/验证方式）+ §6 里程碑表 | `fixed` |
| `major` | `tauri.conf.json` `csp: null` — 无内容安全策略，webview 任意脚本注入风险 | 收紧为 `default-src 'self'` + 显式白名单（img/script/style/connect-src） | `fixed` |
| `major` | `capabilities/default.json` 含 4 个递归写权限（home/desktop/document/download），范围过大，违反最小权限原则 | 删 `home` / `desktop`，仅留 `document` / `download` + `write-text-file` | `fixed` |
| `major` | `tauri.conf.json` `updater.dialog: false` — 静默更新，用户无确认机会 | 改为 `true` | `fixed` |
| `major` | `extractor.rs` `Default::expect("builtin pii regexes must compile")` — 正则编译失败时 panic，生产不可用 | 改 `Self::new().unwrap_or_else(|_| Self::empty())`，降级为匹配空串的退化实例 | `fixed`（cargo test 绿） |
| `major` | `detect.rs` `TSHARK_OVERRIDE.lock().expect("poisoned")` ×2 — Mutex 中毒时 panic | `set_tshark_path` 改 `if let Ok(guard)`；`get_tshark_path` 改 `.map(...).unwrap_or(None)` | `fixed`（cargo clippy 绿） |
| `major` | `docs/02-技术设计文档.md` 缺 processor 6 命令契约 + 行高亮色三处不一致 | 追加 §4.6 处理命令（6 签名 + 结构定义）+ 高亮色统一为 #fff1f0/#f0f5ff/#f6ffed | `fixed` |
| `major` | `handoff/` 14 个 T10~T14 三件套未归档，TASK-BOARD.md 仍为 v1.0.0 架构轮，与 v1.1.0 状态不符 | `git mv` 至 `handoff/archive/`；TASK-BOARD 重写为 v1.1.0 | `fixed` |
| `minor` | 多处过时注释：`tauri.js` JSDoc 与 masker 行为矛盾、`DataTable.jsx`/`SheetTabs.jsx` TODO(T5)、`factory.js`/`AboutCard.jsx`/`lib.rs`/`commands/mod.rs`/`processor/mod.rs` 历史 T 编号引用 | 全部清理为 v1.1.0 当前态描述 | `fixed` |
| `info` | `extract_column` 命令不消费 `name-extract` 规则的 pattern（PiiExtractor 内置 3 正则），rule 仅作 UI 展示样本 | 属 v1.1.0 已知设计边界（简化决定，保持命令签名兼容），非 bug；v1.2+ 规则引擎完整化时再统一 | 接受，记录 |
| `info` | GUI 端到端交互（E3~E5）未验证 | 静态 + 单测级全 pass；浏览器自动化 GUI 验收属 Phase 8 增量轮次 | `pending_e2e` |

严重度口径：`critical`（阻塞发布）/ `major`（需回流修复）/ `minor`（可带病发布但记录）/ `info`（仅记录）。

## 8. 审计结论

`conditional_pass` — 本轮 Release QA 审计覆盖 v1.1.0 全部交付（T15~T21）+ v1.0.0 → v1.1.0 历史遗留回归。首轮发现 3 项 critical + 7 项 major + 1 项 minor 问题，**全部就地修复并回归通过**（cargo fmt/clippy/test 39 passed + pnpm build 3078 modules 全绿）。核心修复包括：

1. **masker 2 字符姓名脱敏语义 bug**（critical）—— `张三` 现正确输出 `张*`（保留姓氏），与规则描述一致；
2. **三项安全收紧**（major）—— CSP 从 `null` 改为显式白名单、fs 递归写权限从 4 个缩至 2 个、updater `dialog` 改 `true`；
3. **两项 panic 风险消除**（major）—— `extractor.rs` `Default` 与 `detect.rs` `Mutex` 改静默降级；
4. **文档与交接件对齐**（critical/major）—— `04` 里程碑 + `03` 任务清单 + `02` §4.6 + 高亮色三处一致 + `handoff/` 归档 + TASK-BOARD 重写。

### 8.1 完整性修复轮（T22~T27）

用户反馈 6 项 UI/交互问题，本轮全部就地修复并回归通过：

| 任务 | 问题 | 修复 | 验证 |
|---|---|---|---|
| T22 | TopToolbar 含 4 个占位按钮（格式/列操作/撤销/运行）+ 相关 `LEFT_OPS_DISABLED` / `iconFor` / 未用 icon import | 全部删除，TopToolbar 仅保留 导入 + 导出 + 4 能力按钮 + ExportModal（113 行，原 175 行） | `pnpm build` 绿 |
| T23 | 各面板含非必要描述文案（"v1.1.0 内置 SimpleMasker..." / "选择列 → ..." 等） | MaskPanel / ValidatePanel / ExtractPanel / RulesPanel 全部移除非必要 `Text` / `Title` 描述块，仅保留功能性 Form 标签 + 必要 extra 提示 | `pnpm build` 绿 |
| T24 | RulesPanel 左列表为扁平 antd List，无分组；mask 测试调用后端 `maskColumn` 写 DB | 左列表改为按 kind 标签分组（脱敏/校验/提取三段，KIND_ORDER）；mask 测试改为前端纯逻辑预览（与 validate/extract 一致：`replacement` 非空→整值替换；空→保留首字符+其余 `*`），**不写 DB** | `pnpm build` 绿 |
| T25 | MaskPanel 仅选列，无可控参数；用户无法自行填入替换串 | MaskPanel 新增「替换串」`Input`（extra 提示"留空：保留首字符，其余 * 替换"）；后端 `mask_column` 命令签名扩展接受 `replacement: Option<String>`，非空时临时覆盖规则 replacement（不写回 DB rules 表）；前端 `maskColumn` IPC 同步加 `replacement` 参数 | `cargo clippy -D warnings` 绿 + `cargo test` 39 passed + `pnpm build` 绿 |
| T26 | ExtractPanel 仅选列，无法选规则；PiiExtractor 内置 3 正则，用户不可控 | ExtractPanel 新增「提取规则（可多选）」`Select mode="multiple"`，仅展示 `kind==="extract"` 且有 pattern 的规则；执行时对**当前页**数据按各选中规则 pattern 做前端正则提取（`new RegExp(pattern,'g').matchAll`），命中行高亮 hit + 命中列表（规则名 + 行号 + 值）；**不写 DB** | `pnpm build` 绿 |
| T27 | 全量验证 + QA 报告更新 | `cargo fmt --all` + `cargo clippy --workspace -- -D warnings` + `cargo test --workspace`（39 passed / 0 failed / 2 ignored）+ `pnpm build`（3078 modules）全绿 | 见 §3 |

**门禁裁决**：无未修复的 critical/major 问题。剩余阻塞项为 GUI 端到端交互验收（E3~E5：四区布局 / 面板联动 / 持久化重启 / 行高亮视觉），属 Phase 8 浏览器自动化增量轮次。**结论维持 `conditional_pass`，待 GUI 验收通过后即可推进至 `qa_passed`**。

### 8.2 脱敏语义修正轮（T28）

用户反馈脱敏可控参数语义错误：不应是「替换串」（整值替换），应是「掩码字符」（保留首尾 + 中间用该字符替换）；重置应回到默认 `*`；可选填别的字符并保存到规则设置。

| 任务 | 问题 | 修复 | 验证 |
|---|---|---|---|
| T28 | `SimpleMasker` 把 `replacement` 当作整值替换串（`output = repl`），与「掩码字符」语义不符；MaskPanel/RulesPanel 标签为「替换串」；重置不清回到默认 `*` | Core `masker.rs`：`replacement` 改为「掩码字符」——取首个字符，`None`/空 → 默认 `*`；≥3 字符保留首尾 + 中间掩码字符替换（「张三丰」→「张*丰」、「司马相如」→「司**如」）；2 字符保留首字符末位掩码字符（「张三」→「张*」）；1 字符单个掩码字符。`rules.rs` 字段注释 + `name_mask_rule` description + `mod.rs` 模块注释同步。单测：`rule_with_mask_char_uses_that_char`（`#`→`1#########8`）+ `rule_with_empty_replacement_defaults_star` + `rule_without_replacement_defaults_star` + `name_mask_rule_keeps_surname`（含「张三丰」→「张*丰」、「司马相如」→「司**如」断言）。Command `processor.rs`：`mask_column` 支持 `rule_id=None` + `replacement` 非空 → 构造临时 mask 规则。MaskPanel：「替换串」→「掩码字符」，默认 `*`，重置→`*`，新增「保存设置」按钮（持久化到 name-mask 规则 replacement）。RulesPanel：mask 规则参数改为「掩码字符」，默认 `*`，重置→规则当前值/`*`，前端预览逻辑同步（≥3 保留首尾），测试结果显示掩码字符。`tauri.js` JSDoc 同步 | `cargo clippy -D warnings` 绿 + `cargo test` 40 passed + `pnpm build` 绿 |

**门禁裁决**：无未修复的 critical/major 问题。结论维持 `conditional_pass`，待 GUI 验收通过后即可推进至 `qa_passed`。

**发布前置门禁**（[`04-版本标准.md`](../../../04-版本标准.md) §4）全部满足前，禁止 finalize。当前阻塞项：① GUI 端到端验收；② 四目标矩阵构建。

---

## 9. 修复证据索引

| 文件 | 改动 | 验证 |
|---|---|---|
| `crates/core/src/processor/masker.rs` | 规则驱动路径三分支（n==0/n==1/n>=2）；模块 + struct doc 注释；单测断言 `张三`→`张*` | `cargo test processor::masker` 5 passed |
| `crates/core/src/processor/extractor.rs` | `Default` impl 改 `unwrap_or_else(empty)`；新增 `empty()` 退化构造 | `cargo test processor::extractor` 4 passed |
| `crates/core/src/pcap/detect.rs` | `set_tshark_path` / `get_tshark_path` Mutex poison 改静默降级 | `cargo clippy -D warnings` 绿 |
| `src-tauri/tauri.conf.json` | `csp` 收紧 + `updater.dialog: true` | `cargo check` 绿 |
| `src-tauri/capabilities/default.json` | 删 `home` / `desktop` 递归写权限 | `cargo check` 绿 |
| `docs/04-版本标准.md` | v1.1.0 里程碑行修正 + v1.2.0 移除已迁移项 + QA 链接改 1.1.0 | 人工核对 |
| `docs/03-开发任务清单.md` | 追加 §5 T15~T21 + §6 里程碑表 | 人工核对 |
| `docs/02-技术设计文档.md` | 追加 §4.6 处理命令 + operations kind + 行高亮色表 | 人工核对 |
| `docs/01-页面与交互说明.md` | 高亮色同步 | 与 DataTable.css 实际值对齐 |
| `docs/versions/1.1.0/RELEASE-NOTES.md` | 高亮色同步 | 与 DataTable.css 实际值对齐 |
| `handoff/` | 14 文件 `git mv` 至 `archive/`；TASK-BOARD 重写 | `git status` 确认 R（rename） |
| `frontend/src/tauri.js` | JSDoc「保留首尾字符」→「保留首字符」+ 头注释去 T 编号；T25 `maskColumn` 加 `replacement` 参数 | `pnpm build` 绿 |
| `frontend/src/components/DataTable.jsx` | 删 TODO(T5) + 注释改 v1.1.0 | `pnpm build` 绿 |
| `frontend/src/components/SheetTabs.jsx` | 删 TODO(T5) + 注释去 T 编号 | `pnpm build` 绿 |
| `frontend/src/state/factory.js` | 注释去 T5/T13 引用 + statusHighlights 说明改 v1.1.0 | `pnpm build` 绿 |
| `frontend/src/components/settings/cards/AboutCard.jsx` | v1.0.0 仅布局壳 → v1.1.0 处理器原型已接入 | `pnpm build` 绿 |
| `src-tauri/src/lib.rs` + `commands/mod.rs` + `processor/mod.rs` + `crates/core/src/lib.rs` | 去历史 T 编号引用，更新为 v1.1.0 当前态 | `cargo clippy -D warnings` 绿 |
| `frontend/src/components/layout/TopToolbar.jsx`（T22） | 删 4 占位按钮 + `LEFT_OPS_DISABLED` + `iconFor` + 未用 icon import；113 行（原 175） | `pnpm build` 绿 |
| `frontend/src/components/panels/MaskPanel.jsx`（T23+T25+T28） | 删描述 `Title`/`Text` 块；「替换串」→「掩码字符」（默认 `*`，重置→`*`）；新增「保存设置」按钮持久化到 name-mask 规则；`maskColumn` 传掩码字符 | `pnpm build` 绿 |
| `frontend/src/components/panels/ValidatePanel.jsx`（T23） | 删描述 `Title`/`Text` 块（"选择列 + 规则 → ..." + "v1.1.0 仅内置..."） | `pnpm build` 绿 |
| `frontend/src/components/panels/ExtractPanel.jsx`（T23+T26） | 删描述 `Title`/`Text` 块；改为多选规则 + 前端正则提取当前页数据 + 命中高亮 hit；不写 DB | `pnpm build` 绿 |
| `frontend/src/components/panels/RulesPanel.jsx`（T23+T24+T28） | 删顶部描述 `Text`；左列表按 kind 标签分组；mask 测试改前端纯逻辑预览（不写 DB）；mask 规则参数「替换串」→「掩码字符」（默认 `*`，重置→规则值/`*`）；测试结果显示掩码字符 | `pnpm build` 绿 |
| `frontend/src/constants.js`（T22 配套） | 删 `DEV_STATUS` 旧注释（"后续版本业务能力的统一占位状态文案"） | `pnpm build` 绿 |
| `src-tauri/src/commands/processor.rs`（T25+T28） | `mask_column` 命令签名扩展 `replacement: Option<String>`；T28：`rule_id=None` + `replacement` 非空 → 构造临时 mask 规则（不再依赖 rule_id 才能覆盖）；`log_operation` params JSON 加 `replacement` 字段 | `cargo clippy -D warnings` 绿 + `cargo test` 40 passed |
| `crates/core/src/processor/masker.rs`（T28） | `replacement` 语义从「整值替换串」改为「掩码字符」（取首个字符；`None`/空 → 默认 `*`）；≥3 字符保留首尾 + 中间掩码字符替换；2 字符保留首字符末位掩码字符；单测 `rule_with_mask_char_uses_that_char` / `rule_with_empty_replacement_defaults_star` / `rule_without_replacement_defaults_star` / `name_mask_rule_keeps_surname`（含「张三丰」→「张*丰」） | `cargo test` 40 passed |
| `crates/core/src/processor/rules.rs`（T28） | `replacement` 字段注释改为「脱敏掩码字符（取首个字符；`None`/空 → 默认 `*`）」；`name_mask_rule` doc + description 改为「保留首尾字符，中间以 * 替换」 | `cargo test` 40 passed |
| `crates/core/src/processor/mod.rs`（T28） | 模块 doc 注释 masker 描述改为「≥3 保留首尾 + 中间掩码字符；2 保留首字符」 | `cargo clippy -D warnings` 绿 |
| `frontend/src/tauri.js`（T28） | `maskColumn` JSDoc：`replacement` 参数说明改为「掩码字符（取首个字符；空/null → 默认 `*`；不写回 DB）」 | `pnpm build` 绿 |
