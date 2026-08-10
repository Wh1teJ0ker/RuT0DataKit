# RuT0DataKit v1.1.3

> Git tag：`v1.1.3`（待推送）
> 状态：qa_passed（T48~T57 全部完成 + R1 Release QA 审计通过；R2 架构/性能/安全审计 T59~T66 全部 verified_complete + E2E 全绿，见 [`docs/qa/versions/1.1.3/QA-审计报告.md`](../../qa/versions/1.1.3/QA-审计报告.md) §R2）
> 前置：v1.1.2 已发布 tag `v1.1.2`

## 这是什么

RuT0DataKit v1.1.3 在 v1.1.2 的 `SimpleMasker`（仅"保留首尾各 1 字符"）之上，引入**通用模板脱敏算子**（参考 v0.8.0 `TemplateOp`：keep_prefix / keep_suffix / mask_char / mask_min_len / min_len / max_len），并落地**通用脱敏规则** `general-mask` + 4 个预设（身份证 / 手机 / 出生日期 / 银行卡）。4 条预设作为 `general-mask` 的子规则，前端选预设即填充 6 个可编辑参数框，用户可继续修改；空模板（不选预设）= 不脱敏（透传）。DB schema 从 v3 升级到 v4（`rules` 表加 `template TEXT` 列），老用户升级自动迁移 + 自动补 seed `general-mask`，已存在规则参数不丢。

## 新增

- **通用模板脱敏算子（TemplateParams）**：`Rule` 新增 `template: Option<TemplateParams>` 字段（JSON 存储），参数包含 `keep_prefix`（保留前 N 字符）、`keep_suffix`（保留后 N 字符）、`mask_char`（掩码字符，默认 `*`）、`mask_min_len`（掩码最少字符数）、`min_len` / `max_len`（输入长度 guard，不在区间原样返回）。`SimpleMasker` 在 `rule.kind == Mask` 分支优先检查 `template`：空模板（所有字段 None）→ 不脱敏（透传）；非空模板 → 走模板逻辑（head + mask + tail，重叠时全掩码）；无 template → 走旧逻辑（保留首尾各 1），完全向后兼容。mask_char 优先级为 `replacement（临时覆盖）> template.mask_char > 默认 *`
- **通用脱敏规则 general-mask + 4 个预设（T49 子规则化）**：

  | 预设 | 名称 | 模板参数 | 示例 |
  |---|---|---|---|
  | 身份证号 | `idcard_preset()` | keep 6/4, mask_min_len 8, len 18 | `110101199001011234` → `110101********1234` |
  | 手机号 | `phone_preset()` | keep 3/4, mask_min_len 4, len 11 | `13812345678` → `138****5678` |
  | 出生日期 | `birthdate_preset()` | keep 8/0, mask_min_len 2, len 10 | `1990-01-15` → `1990-01-**` |
  | 银行卡号 | `bankcard_preset()` | keep 4/4, mask_min_len 1 | `6222021234567890123` → `6222***********0123` |

  4 个预设是 `TemplateParams` 常量构造器（非 DB Rule 条目），作为 `general-mask` 规则的子规则。`general-mask` 默认持空模板（`TemplateParams::default()`，所有字段 None）→ 不脱敏（透传）。身份证/手机/出生日期预设有 `min_len=max_len` guard，长度不匹配的输入原样返回；银行卡预设无 len guard。`RuleRegistry::with_defaults()` 现注册 4 条规则（3 name + general-mask）

- **脱敏面板子规则下拉 + 6 个可编辑参数框**：`MaskPanel` 脱敏规则下拉可选 name-mask（姓名脱敏，旧逻辑）或 general-mask（通用脱敏，模板参数）。选 general-mask → 显示「子规则（预设）」下拉（身份证/手机/出生日期/银行卡/自定义/不脱敏）+ 6 个可编辑参数框（保留前缀/后缀字符数、掩码字符、最少掩码字符数、值长度下限/上限）。选预设即填充参数，用户可继续修改；选「不脱敏」清空所有参数（空模板=透传）；选「自定义」保留当前输入。执行脱敏时把 6 参数组装成 template 透传给 `mask_column`（临时覆盖，不写回 DB）
- **mask_column 新增 template 临时参数 + update_rule_template IPC**：`mask_column(sheet_id, column, rule_id, replacement, template, db)`。前端选预设 → 填充 6 参数 → 透传 template 执行脱敏（临时覆盖规则自身 template，不写回 DB）。`update_rule_template(rule_id, template)` IPC 持久化 template 到 DB `rules.template` 列（SQL 参数绑定，不拼接）。保存设置时：general-mask → update_ruleTemplate 持久化 template；name-mask → updateRuleParams 持久化 replacement
- **DB schema v3→v4**：`rules` 表新增 `template TEXT` 列存 `TemplateParams` JSON。`migrate_v3_to_v4` 用 `PRAGMA table_info` 检查列存在性再 ALTER，幂等；`migrate()` 新增链式分支 `Some(2) => v2→v3→v4` 和 `Some(3) => v3→v4`
- **seed_builtin_rules 改为按 id upsert 缺失规则**：从"只在 DB 无规则时 seed"改为"遍历内置规则集，对每条 id 不存在的规则 upsert"。v1.1.2 老用户升级 → 启动自动补 `general-mask` 规则（持空模板），已存在规则（含用户修改过参数的）不动
- **测试数据三文件夹**：`tests/脱敏/`（idcard/phone/birthdate/bankcard CSV + README）、`tests/提取/`（sample.csv 含手机/邮箱/身份证/银行卡/IP/MAC/地址/密码 token）、`tests/校验/`（sample.csv 含 name/idcard/phone 校验 pass/fail 用例），全部为合成数据

## 优化

- **mask_char 优先级统一**：`replacement`（脱敏面板输入框）作临时覆盖，`template.maskChar` 作模板内置默认，`*` 作兜底。用户输入 `#` → 身份证脱敏输出 `110101########1234`；不输入则用模板 `*`
- **DB schema 迁移链式**：v2 DB 走 `v2→v3→v4` 链式迁移（先补 `before_snapshot_json` 列 + `idx_cells_sheet_col` 索引，再补 `template` 列），v3 DB 走 `v3→v4` 单步迁移；v1 DB 仍走备份重建
- **R2 搜索行级 N+1 消除（T59）**：`search_rows` 原对每个命中的 row_idx 单独执行 `SELECT`（N+1）+ `count_matched_rows_regex` 通过 `u32::MAX` 范围重扫全表。抽取共享单次扫描核心 `scan_regex_matched_row_ids`（一次 `LIKE '%' ESCAPE '\' 取全部候选 cell + Rust 侧逐 cell `regex::is_match`），`search_matched_row_ids_regex` / `count_matched_rows_regex` / `search_matched_rows_regex_with_total` 全部复用同一逻辑，total 与分页结果来自同一次扫描。新增 `query_row_cells_batch`（IN 子句按 500 分块规避 `SQLITE_MAX_VARIABLE_NUMBER`）。净效果 +73 / -152 行，7 个回归测试断言等价
- **R2 导入单次解析（T60）**：`Reader::read()` 改为一次性产出 `Dataset { headers, rows }`，消除"先拿 headers 再二次遍历拿 rows"的重复 I/O。CsvReader / XlsxReader / JsonReader / JsonlReader / TxtReader / SqlReader / PcapReader 全部适配
- **R2 分页 total 语义统一（T62）**：`count_rows` SQL 加 `WHERE row_idx > 0`，`query_cells` offset 语义对齐（page=1 从 row_idx=1 起）。分页 total = 纯数据行数（不再含表头行 +1 误差）
- **R2 翻页状态保留（T63）**：`SET_SHEET_DATA` reducer 保留用户拖拽重排的 `columnOrder`（双向 includes + 长度等价判定 set equality）+ `statusHighlights` 按 `${sheetId}-${page}-${i}` key 回填
- **R2 翻页/搜索陈旧响应隔离（T64）**：App.jsx 引入 `pageReqGenRef`（按 sheetId 分桶单调递增 token）+ `isStale` 检查 → 陈旧响应静默丢弃；DataTable.jsx 引入 `searchGenRef`（单一计数器）→ 搜索陈旧结果不覆盖最新 + 不清 loading
- **R2 前端 O(C²) 消除（T65）**：`SET_SHEET_DATA` / `createSheetFromImport` / `createSheetFromParse` 的 `columnVisibility` 构造从 `reduce + [...acc, ...]`（O(C²)）改为单遍 `Object.fromEntries`（O(C)）

## 下载

> v1.1.3 Release 由 git tag `v1.1.3` 触发 `release.yml` 工作流，tauri-action 三目标矩阵构建（linux-x86_64 / macos-aarch64 / windows-x86_64），产物自动上传至 GitHub Release。

| 平台 | 安装包 | 校验 |
|---|---|---|
| macOS (Apple Silicon) | `RuT0DataKit_1.1.3_aarch64.dmg` / `.app.tar.gz` | 签名校验（updater 公钥） |
| Windows (x64) | `RuT0DataKit_1.1.3_x64-setup.exe` / `.msi.zip` | 签名校验 |
| Linux (x64) | `RuT0DataKit_1.1.3_amd64.AppImage` / `.deb` | 签名校验 |

## 验证

- `cargo fmt --check`：通过
- `cargo clippy --workspace --all-targets -- -D warnings`：通过（R2 额外修复 5 处 lint：`unnecessary_get_then_check` / `field_reassign_with_default` ×3 / `type_complexity`）
- `cargo test --workspace`：全绿（282 passed / 3 ignored / 0 failed = src-tauri lib 123 + core 148 + Doc-tests 11；含 R2 新增 7 个搜索回归测试 + 2 个 IPC 契约测试 + 8 个 settings 单测）
- `pnpm --prefix frontend build`：通过（3083 modules，2.46s）

## 已知限制

- `general-mask` 的 4 个预设为固定模板参数（keep_prefix/keep_suffix/mask_min_len 硬编码），但前端 6 个参数框可手动编辑覆盖预设值
- 身份证/手机/出生日期预设有 min_len=max_len guard，长度不匹配的输入原样返回（不脱敏）；银行卡预设无 len guard，任意长度均可脱敏
- 出生日期脱敏按字符串长度处理（keep 8/0 + mask 2），不校验日期合法性
- T49 删除了 v1.1.3 T48 的 4 条独立规则 id（idcard-mask/phone-mask/birthdate-mask/bankcard-mask），v1.1.3 用户若已保存这 4 条规则的 template 参数，升级后这 4 条规则会从 DB 消失（seed 不再注册），需改用 general-mask + 对应预设
- **R2 安全审计 Mimosa 结论不完整**：R2 commit 前的 Mimosa hook 多次报告 `library_source_unavailable` / `callgraph_fact_partial` / `library_source_limit_exceeded`（不完整结论）。R1 完整密封扫描 0 findings（scan-2026-08-10T16-40-17.470Z-31df73e0a37d，487 包 0 漏洞），R2 按兼容策略继续合并但**不宣称项目安全**，待重新运行完整 Mimosa 密封扫描（见 QA 报告 §R2-01）
- **R2 开发期 dev server 默认 localhost-only（T66）**：Vite dev server 默认 `host: 'localhost'`（不再 `host: true` 绑定所有接口），消除局域网暴露面；需从其它设备/容器访问时显式设置 `VITE_DEV_HOST=1`

## 升级

v1.1.2 用户可直接升级：DB schema 从 v3 迁移到 v4（`rules` 表加 `template TEXT` 列，`migrate_v3_to_v4` 幂等），历史数据完整保留。启动时 `seed_builtin_rules` 自动补 seed `general-mask` 规则（持空模板，按 id upsert 缺失规则），已存在的 3 条 name 规则（含用户修改过 pattern/replacement 的）不动。启动后即可在脱敏面板选 general-mask + 子规则预设执行脱敏；既有姓名脱敏、撤销/重做、列操作、搜索、Base64、.log 导入等功能无回归。

---

完整更新日志：[`docs/versions/1.1.3/更新日志.md`](更新日志.md)
