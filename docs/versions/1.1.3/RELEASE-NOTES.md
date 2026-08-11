# RuT0DataKit v1.1.3

> Git tag：`v1.1.3`
> 状态：qa_passed（T48~T57 全部完成 + R1 Release QA 审计通过；R2 架构/性能/安全审计 T59~T66 全部 verified_complete + E2E 全绿 + Mimosa 密封扫描重跑 0 findings，见 [`docs/qa/versions/1.1.3/QA-审计报告.md`](../../qa/versions/1.1.3/QA-审计报告.md) §R2）
> 前置：v1.1.2 已发布 tag `v1.1.2`

## 这是什么

RuT0DataKit v1.1.3 在 v1.1.2 基础上完成三大能力跃升：

1. **通用模板脱敏算子**（T48-T54）：在 `SimpleMasker` 之上引入 `TemplateParams`（keep_prefix / keep_suffix / mask_char / mask_min_len / min_len / max_len），落地 `simple-mask`（整段脱敏，含 4 个预设：身份证 / 手机 / 出生日期 / 银行卡）+ `segment-mask`（分段脱敏，按分隔符拆分每段保留首尾）+ 反向脱敏（掩码首尾、保留中间）。DB schema v3→v4（`rules` 表加 `template TEXT` 列）。
2. **数据提取与校验**（T55-T57）：新增 `ExtractParams` typed enum + `func_validator.rs` 模块（Luhn / IPv4 / IPv6 / 身份证 GB 11643 校验码 / 性别推断 / 手机前缀白名单）+ 5 条提取规则（phone/bankcard/ip4/ip6/idcard）+ 批量多规则提取（两列 [类型, 数据值] 输出）+ 行级多字段校验（7 字段 + 跨字段联合 → 双 Tab 分流）。DB schema v4→v5（`rules` 表加 `params TEXT` 列）。
3. **性能与架构优化**（T59-T65）：搜索行级 N+1 消除、导入单次解析、分页 total 语义统一、翻页状态保留、陈旧响应隔离、前端 O(C²) 消除。

## Update

### 脱敏能力增强（T48-T54）

- **通用模板脱敏算子（TemplateParams，T48）**：`Rule` 新增 `template: Option<TemplateParams>` 字段（JSON 存储），参数包含 `keep_prefix`（保留前 N 字符）、`keep_suffix`（保留后 N 字符）、`mask_char`（掩码字符，默认 `*`）、`mask_min_len`（掩码最少字符数）、`min_len` / `max_len`（输入长度 guard，不在区间原样返回）。`SimpleMasker` 在 `rule.kind == Mask` 分支优先检查 `template`：空模板（所有字段 None）→ 不脱敏（透传）；非空模板 → 走模板逻辑（head + mask + tail，重叠时全掩码）；无 template → 走旧逻辑（保留首尾各 1），完全向后兼容。mask_char 优先级为 `replacement（临时覆盖）> template.mask_char > 默认 *`
- **DB schema v3→v4（T48）**：`rules` 表新增 `template TEXT` 列存 `TemplateParams` JSON。`migrate_v3_to_v4` 用 `PRAGMA table_info` 检查列存在性再 ALTER，幂等；`migrate()` 新增链式分支 `Some(2) => v2→v3→v4` 和 `Some(3) => v3→v4`
- **seed_builtin_rules 改为按 id upsert 缺失规则（T48）**：从"只在 DB 无规则时 seed"改为"遍历内置规则集，对每条 id 不存在的规则 upsert"。老用户升级 → 启动自动补新规则，已存在规则（含用户修改过参数的）不动
- **4 个脱敏预设（T49）**：

  | 预设 | 名称 | 模板参数 | 示例 |
  |---|---|---|---|
  | 身份证号 | `idcard_preset()` | keep 6/4, mask_min_len 8, len 18 | `110101199001011234` → `110101********1234` |
  | 手机号 | `phone_preset()` | keep 3/4, mask_min_len 4, len 11 | `13812345678` → `138****5678` |
  | 出生日期 | `birthdate_preset()` | keep 8/0, mask_min_len 2, len 10 | `1990-01-15` → `1990-01-**` |
  | 银行卡号 | `bankcard_preset()` | keep 4/4, mask_min_len 1 | `6222021234567890123` → `6222***********0123` |

  4 个预设是 `TemplateParams` 常量构造器（非 DB Rule 条目），前端选预设即填充 6 个可编辑参数框，可继续修改
- **脱敏面板子规则下拉 + 6 个可编辑参数框（T49/T50/T51）**：选脱敏规则 → 显示「子规则（预设）」下拉 + 6 个可编辑参数框（保留前缀/后缀字符数、掩码字符、最少掩码字符数、值长度下限/上限）。抽取 `maskTemplate.js` 共享模块（MASK_PRESETS / buildTemplateForRun / previewMask 等），MaskPanel 与 RulesPanel 共用。`mask_column` 新增 `template` 临时参数（透传执行，不写回 DB）；`update_rule_template` IPC 持久化 template 到 DB（SQL 参数绑定）。RulesPanel 移除「目标列」显示行；`cleanup_deprecated_rules()` 清理 v1.1.3 T48 遗留的 4 条独立脱敏规则 id（幂等）
- **分段脱敏模板（T52）**：`TemplateParams` 由 flat struct 改为 untagged enum（`Simple`/`Segment`）。`Segment` 变体含 `delimiter` + `segments: Vec<SegmentMask>`（每段 `index`/`keep_prefix`/`keep_suffix`/`mask_min_len`）。新增 `apply_segment_template` + `apply_segment_part`。`#[serde(deny_unknown_fields)]` on `SimpleTemplate` 确保 untagged enum 分流正确。DB 无需迁移（旧 flat JSON → Simple）。前端模板类型切换 + Segment 参数 UI
- **反向脱敏模板（T53）**：`SimpleTemplate` 加 `reverse: Option<bool>` 字段。`reverse=true` → 反向：前 `keep_prefix` + 后 `keep_suffix` 字符替换为 mask_char，中间保留（如 `13812345678` + kp=3/ks=4/reverse=true → `***1234****`）。`apply_template` 加反向分支（含重叠全脱码 + guard 仍生效）。DB 无需迁移（旧 JSON 无 reverse → None → 正向）。前端加「反向脱敏」Switch + 预览反向分支
- **拆分整段脱敏与分段脱敏为两条独立规则（T54）**：删除 `general-mask` → 新建 `simple-mask`（整段脱敏，持 Simple 空模板）+ `segment-mask`（分段脱敏，持 Segment 空模板）。`with_defaults()` 5 条（3 name + simple-mask + segment-mask）。旧 `general-mask` id 加入 `cleanup_deprecated_rules()` 删除列表。前端删除「模板类型」Select，按 `selected.id` 渲染对应参数区

### 数据提取与校验（T55-T57）

- **数据提取规则 + 函数式严格校验（T55/T55b/T55c）**：新增 `ExtractParams` typed enum（`PhonePrefix`/`Luhn`/`Ipv4`/`Ipv6`/`IdCard`，serde `tag="validator"`）+ `Rule.params: Option<ExtractParams>` 字段。新增 `func_validator.rs` 模块：
  - `luhn_check`：银行卡 Luhn 校验（右起偶数位×2，>9 则数位和，总和 %10==0）
  - `is_valid_ipv4`：4 段 0-255，禁前导零（`"0"` 单独允许，`"01"`/`"00"` 拒）
  - `is_valid_ipv6`：RFC 4291 严格解析（`std::net::Ipv6Addr::from_str`）
  - `is_valid_idcard`：GB 11643-1999 校验码（18 位 + 前 17 纯数字 + 末位数字或 X/x + 加权系数 `[7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2]` mod 11 查表 `[1,0,X,9,8,7,6,5,4,3,2]`）
  - `idcard_gender`：第 17 位奇=男/偶=女
  - `check_phone_prefix`：前缀白名单（空列表=默认 1 开头通过）
  - `validate_extracted`：按 `rule.params` 分发（`None` → `(true, "")` 向后兼容）

  5 条提取规则：

  | 规则 id | 名称 | 正则 | 校验器 |
  |---|---|---|---|
  | `phone-extract` | 手机号提取 | `\b1\d{10}\b` | PhonePrefix（可选前缀白名单） |
  | `bankcard-extract` | 银行卡号提取 | `\b[1-9]\d{12,18}\b` | Luhn |
  | `ip4-extract` | IPv4地址提取 | `\b(?:\d{1,3}\.){3}\d{1,3}\b` | Ipv4 |
  | `ip6-extract` | IPv6地址提取 | `(?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}` | Ipv6 |
  | `idcard-extract` | 身份证号提取 | `\b\d{17}[\dXx]\b` | IdCard（校验码 + 性别联合校验） |

  提取正则宽松（召回优先），严格校验交给 `validate_extracted`。`with_defaults()` 5→10 条
- **DB schema v4→v5（T55）**：`rules` 表新增 `params TEXT` 列存 `ExtractParams` JSON。`migrate_v4_to_v5` 用 `PRAGMA table_info` 检查列存在性再 ALTER，幂等；`migrate()` 链式分支 `Some(2) => v2→v3→v4→v5`、`Some(3) => v3→v4→v5`、`Some(4) => v4→v5`。`SCHEMA_VERSION` 4→5。`row_to_rule` 读 `params` 列 → `serde_json::from_str` 反序列化；`upsert_rule` SQL + params 加 `params` 列（None → NULL）。`update_rule_extract_config` 方法单独更新 `pattern` + `params` 列（参数绑定，不拼接 SQL）
- **提取并校验到新 Tab（T55）**：新增 `extract_validate_to_new_sheet` IPC（提取候选 → `validate_extracted` → 新 Tab）+ `update_rule_extract_config` IPC（持久化 pattern + params，SQL 参数绑定）。前端 ExtractPanel 加「提取并校验到新 Tab」按钮；RulesPanel extract 编辑分支显示校验类型只读 Tag（手机前缀/Luhn/IPv4/IPv6/身份证）+ phone-extract 允许前缀 `Select mode="tags"`。idcard-extract 支持性别列联合校验（提取时选性别列，比对第 17 位推断性别与性别列值，矛盾→无效，`normalize_gender` 接受中英文）
- **批量多规则提取 + 两列输出（T56）**：`extract_validate_to_new_sheet` 从单规则改为批量多规则（`rule_id: &str` → `rule_ids: &[String]`）。新 Tab 输出从 4 列 [源行号, 提取值, 有效, 说明] 改为 2 列 [类型, 数据值]，**只写有效候选**（无效候选不写入 sheet，但仍保留在返回的 `Vec<ExtractValidateRow>` 供测试断言）。类型标签 = 规则名去掉「提取」后缀（如「身份证号提取」→「身份证号」、「姓名提取」→「姓名」，结果为空用原名兜底）。同一行多规则命中 → 多条输出。前端 ExtractPanel 多选规则 → 批量提取，无「请只选一条」限制；`isIdcardExtract` 改 `.some()` 批量判定
- **行级多字段校验 → 双 Tab 输出（T57）**：对结构化人员数据完成 7 条校验规则：

  | 字段 | 校验规则 | 示例 |
  |---|---|---|
  | username | 纯字母数字 `^[a-zA-Z0-9]+$` | `admin` ✓ / `ab.cd` ✗ |
  | name | 2-4 位中文 `^[\u4e00-\u9fa5]{2,4}$` | `张三` ✓ / `张3` ✗ |
  | sex | 仅"男"/"女" + 跨字段（与 idcard 第 17 位一致） | `男` ✓ / `male` ✗ |
  | birth | 8 位数字 + 跨字段（与 idcard 第 7-14 位一致） | `19491231` ✓ |
  | idcard | 18 位 GB 11643-1999 校验码 | `11010519491231002X` ✓ |
  | phone | 11 位 1 开头 + 可选前缀白名单 | `13412345678` ✓ |
  | address | 全中文 + 号(1-1500) + 室(101-999) | `内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室` ✓ |

  新增 5 个独立校验函数（`is_valid_username`/`is_valid_sex`/`is_valid_birth`/`is_valid_phone`/`is_valid_address`，`OnceLock<Regex>` 缓存），复用已有 `is_valid_idcard`/`idcard_gender`/`check_phone_prefix`/`normalize_gender`。**跨字段联合校验**：sex vs idcard 第 17 位性别推断、birth vs idcard 第 7-14 位出生日期码（仅当 idcard 本身有效 + 相关字段都映射时生效）。新增 `validate_rows_to_two_sheets` 命令：接受 `FieldColumnMapping`（7 个 `Option<String>` 字段→列名映射，未映射=不校验）+ `phone_prefixes` 白名单 → 逐行逐字段校验 + 跨字段联合 → 整行按通过/失败分流到两个新 Tab（`{源sheet名}_校验通过` / `{源sheet名}_校验失败`，保留原列不新增列，列顺序与源 sheet 一致）。新增 `get_sheet_name(sheet_id)` DB 方法（用于双 Tab 命名）。前端新增 `RowValidatePanel.jsx`（7 字段列映射 Select + 手机前缀编辑 + 触发按钮）+ `validateRowsToTwoSheets` IPC wrapper + SidePanel/TopToolbar 注册（`rowValidate` 能力，`SafetyOutlined` 图标）

### 性能与架构优化（T59-T65）

- **搜索行级 N+1 消除（T59）**：`search_rows` 原对每个命中的 row_idx 单独执行 `SELECT`（N+1）+ `count_matched_rows_regex` 通过 `u32::MAX` 范围重扫全表。抽取共享单次扫描核心 `scan_regex_matched_row_ids`（一次 `LIKE '%' ESCAPE '\' 取全部候选 cell + Rust 侧逐 cell `regex::is_match`），`search_matched_row_ids_regex` / `count_matched_rows_regex` / `search_matched_rows_regex_with_total` 全部复用同一逻辑，total 与分页结果来自同一次扫描。新增 `query_row_cells_batch`（IN 子句按 500 分块规避 `SQLITE_MAX_VARIABLE_NUMBER`）。净效果 +73 / -152 行，7 个回归测试断言等价
- **导入单次解析（T60）**：`Reader::read()` 改为一次性产出 `Dataset { headers, rows }`，消除"先拿 headers 再二次遍历拿 rows"的重复 I/O。CsvReader / XlsxReader / JsonReader / JsonlReader / TxtReader / SqlReader / PcapReader 全部适配
- **分页 total 语义统一（T62）**：`count_rows` SQL 加 `WHERE row_idx > 0`，`query_cells` offset 语义对齐（page=1 从 row_idx=1 起）。分页 total = 纯数据行数（不再含表头行 +1 误差）
- **翻页状态保留（T63）**：`SET_SHEET_DATA` reducer 保留用户拖拽重排的 `columnOrder`（双向 includes + 长度等价判定 set equality）+ `statusHighlights` 按 `${sheetId}-${page}-${i}` key 回填
- **翻页/搜索陈旧响应隔离（T64）**：App.jsx 引入 `pageReqGenRef`（按 sheetId 分桶单调递增 token）+ `isStale` 检查 → 陈旧响应静默丢弃；DataTable.jsx 引入 `searchGenRef`（单一计数器）→ 搜索陈旧结果不覆盖最新 + 不清 loading
- **前端 O(C²) 消除（T65）**：`SET_SHEET_DATA` / `createSheetFromImport` / `createSheetFromParse` 的 `columnVisibility` 构造从 `reduce + [...acc, ...]`（O(C²)）改为单遍 `Object.fromEntries`（O(C)）
- **测试数据三文件夹**：`tests/脱敏/`（idcard/phone/birthdate/bankcard CSV + README）、`tests/提取/`（sample.csv 含手机/邮箱/身份证/银行卡/IP/MAC/地址/密码 token）、`tests/校验/`（sample.csv 含 name/idcard/phone 校验 pass/fail 用例），全部为合成数据
- 版本号 1.1.2 → 1.1.3

## Fix

- 修复 schema 迁移非事务化：`migrate` 全过程包进单事务，任一步骤失败整体回滚，避免半迁移状态（T66）
- 修复 dev server 局域网暴露：Vite dev server 默认从 `host: true`（绑定所有接口）改为 `host: 'localhost'`，消除局域网暴露面；需从其它设备/容器访问时显式设置 `VITE_DEV_HOST=1`（T66）
- 修复 `settings.json` 写入非原子：写入改用临时文件 + rename 原子化（NamedTempFile + persist），损坏文件给出明确错误而非静默崩溃（T66）
- 修复 `validate_column` IPC 契约不匹配：前端直接消费 `RowValidation[]`，修正前后端契约不匹配（T61）
- 修复 clippy lint：`unnecessary_get_then_check` / `field_reassign_with_default` ×3 / `type_complexity`

## 下载

> v1.1.3 Release 由 git tag `v1.1.3` 触发 `release.yml` 工作流，tauri-action 三目标矩阵构建（linux-x86_64 / macos-aarch64 / windows-x86_64），产物自动上传至 GitHub Release。

| 平台 | 安装包 | 校验 |
|---|---|---|
| macOS (Apple Silicon) | `RuT0DataKit_1.1.3_aarch64.dmg` / `.app.tar.gz` | 签名校验（updater 公钥） |
| Windows (x64) | `RuT0DataKit_1.1.3_x64-setup.exe` / `.msi.zip` | 签名校验 |
| Linux (x64) | `RuT0DataKit_1.1.3_amd64.AppImage` / `.deb` | 签名校验 |

每个安装包均附带 .sig 签名文件，供 updater 验签。macOS Intel (x86_64) 不在本版构建矩阵中（仅 aarch64-apple-darwin），Intel Mac 用户可通过 Rosetta 运行 ARM 版本。

## 验证

- `cargo fmt --check`：通过
- `cargo clippy --workspace --all-targets -- -D warnings`：通过（R2 额外修复 5 处 lint：`unnecessary_get_then_check` / `field_reassign_with_default` ×3 / `type_complexity`）
- `cargo test --workspace`：全绿（282 passed / 3 ignored / 0 failed = src-tauri lib 123 + core 148 + Doc-tests 11；含 R2 新增 7 个搜索回归测试 + 2 个 IPC 契约测试 + 8 个 settings 单测 + T55-T57 新增提取/校验/行级校验单测与集成测试）
- `pnpm --prefix frontend build`：通过（3083 modules，2.46s）

## 已知限制

- `simple-mask` 的 4 个预设为固定模板参数（keep_prefix/keep_suffix/mask_min_len 硬编码），但前端 6 个参数框可手动编辑覆盖预设值
- 身份证/手机/出生日期预设有 min_len=max_len guard，长度不匹配的输入原样返回（不脱敏）；银行卡预设无 len guard，任意长度均可脱敏
- 出生日期脱敏按字符串长度处理（keep 8/0 + mask 2），不校验日期合法性
- T49 删除了 v1.1.3 T48 的 4 条独立规则 id（idcard-mask/phone-mask/birthdate-mask/bankcard-mask），T54 删除了 `general-mask`，T55b 删除了 `ip-extract`；升级时 `cleanup_deprecated_rules()` 自动删除这些遗留 id（幂等）。v1.1.3 用户若已保存这些规则的参数，升级后需改用 simple-mask/segment-mask + 预设，或 ip4-extract/ip6-extract
- T55 身份证号提取仅校验长度 + 校验码（不校验真实行政区划），6 位地址码纯数字即可通过（不查 GB/T 2260），符合用户「防止随机生成的身份证号码和现实世界的身份证号码存在联系」的要求
- T56 新 Tab 只写有效候选（无效候选不写入 sheet），用户无法在新 Tab 直接看到无效候选（如需查看可参考返回的 `invalidReasons` summary 消息）
- T57 行级校验的失败原因不写入 sheet（用户要求「保留原有字段，不新增列」），仅在 IPC 返回的 `invalidReasons` 中供前端 summary 消息显示
- T57 address 正则要求全中文前缀 + 号/室，不含字母/标点
- **R2 安全审计 Mimosa 密封扫描重跑（T59~T66）**：R2 commit 前的 Mimosa hook 曾报告不完整结论（`library_source_unavailable` / `callgraph_fact_partial` / `library_source_limit_exceeded`）。已重新运行完整 Mimosa 密封扫描（`scan-2026-08-10T21-49-17.114Z-3e0a1b2d600e`，deep）：0 findings / 487 包 0 漏洞。R1 + R2 双轮 Mimosa 深度扫描均 0 findings（`evidenceBoundary=static_only_no_runtime_execution`，静态分析非运行时验证，不宣称项目安全）
- **R2 开发期 dev server 默认 localhost-only（T66）**：Vite dev server 默认 `host: 'localhost'`（不再 `host: true` 绑定所有接口），消除局域网暴露面；需从其它设备/容器访问时显式设置 `VITE_DEV_HOST=1`

## 升级

v1.1.2 用户可直接升级：DB schema 从 v3 链式迁移到 v5（v3→v4→v5：`rules` 表加 `template TEXT` + `params TEXT` 两列，`migrate_v3_to_v4` / `migrate_v4_to_v5` 均幂等），历史数据完整保留。启动时 `seed_builtin_rules` 自动补 seed 新规则（按 id upsert 缺失规则）：

- **脱敏**：simple-mask（整段脱敏，持 Simple 空模板）+ segment-mask（分段脱敏，持 Segment 空模板）
- **提取**：phone-extract（手机号 + 前缀白名单）+ bankcard-extract（银行卡 Luhn）+ ip4-extract（IPv4）+ ip6-extract（IPv6）+ idcard-extract（身份证 GB 11643 + 性别联合校验）

已存在的 3 条 name 规则（含用户修改过 pattern/replacement 的）不动；旧 `general-mask` / `ip-extract` 等遗留 id 由 `cleanup_deprecated_rules()` 自动删除（幂等）。启动后即可在脱敏面板选 simple-mask/segment-mask + 预设执行脱敏，在提取面板多选规则批量提取校验，在行级校验面板映射字段分流到双 Tab；既有姓名脱敏、撤销/重做、列操作、搜索、Base64、.log 导入等功能无回归。

---

完整更新日志：[`docs/versions/1.1.3/更新日志.md`](更新日志.md)
