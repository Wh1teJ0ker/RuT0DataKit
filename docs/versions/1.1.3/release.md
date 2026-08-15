# RuT0DataKit v1.1.3

## Update

### 脱敏能力增强（T48-T54）

- **通用模板脱敏算子（TemplateParams，T48）**：`Rule` 新增 `template: Option<TemplateParams>` 字段（JSON 存储），参数包含 `keep_prefix`（保留前 N 字符）、`keep_suffix`（保留后 N 字符）、`mask_char`（掩码字符，默认 `*`）、`mask_min_len`（掩码最少字符数）、`min_len` / `max_len`（输入长度 guard，不在区间原样返回）。`SimpleMasker` 在 `rule.kind == Mask` 分支优先检查 `template`：空模板（所有字段 None）→ 不脱敏（透传）；非空模板 → 走模板逻辑（head + mask + tail，重叠时全掩码）；无 template → 走旧逻辑（保留首尾各 1），完全向后兼容。mask_char 优先级为 `replacement（临时覆盖）> template.mask_char > 默认 *`
- **DB schema v3→v4（T48）**：`rules` 表新增 `template TEXT` 列存 `TemplateParams` JSON。`migrate_v3_to_v4` 用 `PRAGMA table_info` 检查列存在性再 ALTER，幂等；`migrate()` 新增链式分支 `Some(2) => v2→v3→v4` 和 `Some(3) => v3→v4`
- **seed_builtin_rules 改为按 id upsert 缺失规则（T48）**：从"只在 DB 无规则时 seed"改为"遍历内置规则集，对每条 id 不存在的规则 upsert"。老用户升级 → 启动自动补新规则，已存在规则（含用户修改过参数的）不动
- **4 个脱敏预设（T49）**：身份证（keep 6/4, mask_min_len 8, len 18 → `110101********1234`）、手机（keep 3/4, len 11 → `138****5678`）、出生日期（keep 8/0, len 10 → `1990-01-**`）、银行卡（keep 4/4 → `6222***********0123`）。预设是 `TemplateParams` 常量构造器（非 DB Rule 条目），前端选预设即填充 6 个可编辑参数框，可继续修改
- **脱敏面板子规则下拉 + 6 个可编辑参数框（T49/T50/T51）**：选脱敏规则 → 显示「子规则（预设）」下拉 + 6 个可编辑参数框（保留前缀/后缀字符数、掩码字符、最少掩码字符数、值长度下限/上限）。抽取 `maskTemplate.js` 共享模块（MASK_PRESETS / buildTemplateForRun / previewMask 等），MaskPanel 与 RulesPanel 共用。`mask_column` 新增 `template` 临时参数（透传执行，不写回 DB）；`update_rule_template` IPC 持久化 template 到 DB（SQL 参数绑定）。RulesPanel 移除「目标列」显示行；`cleanup_deprecated_rules()` 清理 v1.1.3 T48 遗留的 4 条独立脱敏规则 id（幂等）
- **分段脱敏模板（T52）**：`TemplateParams` 由 flat struct 改为 untagged enum（`Simple`/`Segment`）。`Segment` 变体含 `delimiter` + `segments: Vec<SegmentMask>`（每段 `index`/`keep_prefix`/`keep_suffix`/`mask_min_len`）。新增 `apply_segment_template` + `apply_segment_part`。`#[serde(deny_unknown_fields)]` on `SimpleTemplate` 确保 untagged enum 分流正确。DB 无需迁移（旧 flat JSON → Simple）。前端模板类型切换 + Segment 参数 UI
- **反向脱敏模板（T53）**：`SimpleTemplate` 加 `reverse: Option<bool>` 字段。`reverse=true` → 反向：前 `keep_prefix` + 后 `keep_suffix` 字符替换为 mask_char，中间保留（如 `13812345678` + kp=3/ks=4/reverse=true → `***1234****`）。`apply_template` 加反向分支（含重叠全脱码 + guard 仍生效）。DB 无需迁移（旧 JSON 无 reverse → None → 正向）。前端加「反向脱敏」Switch + 预览反向分支
- **拆分整段脱敏与分段脱敏为两条独立规则（T54）**：删除 `general-mask` → 新建 `simple-mask`（整段脱敏，持 Simple 空模板）+ `segment-mask`（分段脱敏，持 Segment 空模板）。`with_defaults()` 5 条（3 name + simple-mask + segment-mask）。旧 `general-mask` id 加入 `cleanup_deprecated_rules()` 删除列表。前端删除「模板类型」Select，按 `selected.id` 渲染对应参数区

### 数据提取与校验（T55-T57）

- **数据提取规则 + 函数式严格校验（T55/T55b/T55c）**：新增 `ExtractParams` typed enum（`PhonePrefix`/`Luhn`/`Ipv4`/`Ipv6`/`IdCard`，serde `tag="validator"`）+ `Rule.params: Option<ExtractParams>` 字段。新增 `func_validator.rs` 模块：`luhn_check`（银行卡 Luhn 校验）、`is_valid_ipv4`（4 段 0-255 禁前导零）、`is_valid_ipv6`（RFC 4291 严格解析）、`is_valid_idcard`（GB 11643-1999 校验码：18 位 + 加权系数 `[7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2]` mod 11 查表）、`idcard_gender`（第 17 位奇=男/偶=女）、`check_phone_prefix`（前缀白名单）、`validate_extracted`（按 params 分发）。5 条提取规则：`phone-extract`（11 位 1 开头 + 可选前缀白名单）、`bankcard-extract`（13-19 位 Luhn）、`ip4-extract`（IPv4）、`ip6-extract`（IPv6）、`idcard-extract`（18 位身份证 + 校验码 + 性别联合校验）。提取正则宽松（召回优先），严格校验交给 `validate_extracted`。`with_defaults()` 5→10 条
- **DB schema v4→v5（T55）**：`rules` 表新增 `params TEXT` 列存 `ExtractParams` JSON。`migrate_v4_to_v5` 用 `PRAGMA table_info` 检查列存在性再 ALTER，幂等；`migrate()` 链式分支 `Some(2) => v2→v3→v4→v5`、`Some(3) => v3→v4→v5`、`Some(4) => v4→v5`。`SCHEMA_VERSION` 4→5
- **提取并校验到新 Tab（T55）**：新增 `extract_validate_to_new_sheet` IPC（提取候选 → `validate_extracted` → 新 Tab）+ `update_rule_extract_config` IPC（持久化 pattern + params，SQL 参数绑定）。前端 ExtractPanel 加「提取并校验到新 Tab」按钮；RulesPanel extract 编辑分支显示校验类型只读 Tag（手机前缀/Luhn/IPv4/IPv6/身份证）+ phone-extract 允许前缀 `Select mode="tags"`。idcard-extract 支持性别列联合校验（提取时选性别列，比对第 17 位推断性别与性别列值，矛盾→无效）
- **批量多规则提取 + 两列输出（T56）**：`extract_validate_to_new_sheet` 从单规则改为批量多规则（`rule_id: &str` → `rule_ids: &[String]`）。新 Tab 输出从 4 列 [源行号, 提取值, 有效, 说明] 改为 2 列 [类型, 数据值]，**只写有效候选**。类型标签 = 规则名去掉「提取」后缀（如「身份证号提取」→「身份证号」）。同一行多规则命中 → 多条输出。前端 ExtractPanel 多选规则 → 批量提取，无「请只选一条」限制
- **行级多字段校验 → 双 Tab 输出（T57）**：对结构化人员数据完成 7 条校验规则（username 纯字母数字 / name 2-4 位中文 / sex 男女 / birth 8 位数字 / idcard GB 11643 / phone 11 位 1 开头 + 可选前缀 / address 全中文 + 号 1-1500 + 室 101-999）。新增 5 个独立校验函数（`is_valid_username`/`is_valid_sex`/`is_valid_birth`/`is_valid_phone`/`is_valid_address`，`OnceLock<Regex>` 缓存）。**跨字段联合校验**：sex vs idcard 第 17 位性别推断、birth vs idcard 第 7-14 位出生日期码（仅当 idcard 本身有效 + 相关字段都映射时生效）。新增 `validate_rows_to_two_sheets` 命令：接受 `FieldColumnMapping`（7 个 `Option<String>` 字段→列名映射，未映射=不校验）+ `phone_prefixes` 白名单 → 逐行逐字段校验 + 跨字段联合 → 整行按通过/失败分流到两个新 Tab（`{源sheet名}_校验通过` / `{源sheet名}_校验失败`，保留原列不新增列）。前端新增 `RowValidatePanel.jsx`（7 字段列映射 Select + 手机前缀编辑 + 触发按钮）+ SidePanel/TopToolbar 注册（`rowValidate` 能力）

### 性能与架构优化（T59-T65）

- **搜索行级 N+1 消除（T59）**：`search_rows` 原对每个命中的 row_idx 单独执行 `SELECT`（N+1）+ `count_matched_rows_regex` 通过 `u32::MAX` 范围重扫全表。抽取共享单次扫描核心 `scan_regex_matched_row_ids`，total 与分页结果来自同一次扫描。新增 `query_row_cells_batch`（IN 子句按 500 分块规避 `SQLITE_MAX_VARIABLE_NUMBER`）。净效果 +73 / -152 行，7 个回归测试断言等价
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
- 修复 `settings.json` 写入非原子：写入改用临时文件 + rename 原子化，损坏文件给出明确错误而非静默崩溃（T66）
- 修复 `validate_column` IPC 契约不匹配：前端直接消费 `RowValidation[]`，修正前后端契约不匹配（T61）
- 修复 clippy lint：`unnecessary_get_then_check` / `field_reassign_with_default` ×3 / `type_complexity`

## 下载

| 平台 | 文件 |
|---|---|
| Windows x64 | RuT0DataKit_1.1.3_x64-setup.exe / RuT0DataKit_1.1.3_x64_en-US.msi |
| macOS arm64 | RuT0DataKit_1.1.3_aarch64.dmg / RuT0DataKit_aarch64.app.tar.gz |
| Linux x64 | RuT0DataKit_1.1.3_amd64.deb / RuT0DataKit-1.1.3-1.x86_64.rpm / RuT0DataKit_1.1.3_amd64.AppImage |

每个安装包均附带 .sig 签名文件，供 updater 验签。macOS Intel (x86_64) 不在本版构建矩阵中（仅 aarch64-apple-darwin），Intel Mac 用户可通过 Rosetta 运行 ARM 版本。

## 升级

v1.1.2 用户可直接升级：DB schema 从 v3 链式迁移到 v5（v3→v4→v5：`rules` 表加 `template TEXT` + `params TEXT` 两列，`migrate_v3_to_v4` / `migrate_v4_to_v5` 均幂等），历史数据完整保留。启动时 `seed_builtin_rules` 自动补 seed 新规则（按 id upsert 缺失规则）：simple-mask / segment-mask（脱敏）+ phone-extract / bankcard-extract / ip4-extract / ip6-extract / idcard-extract（提取），已存在的 3 条 name 规则（含用户修改过参数的）不动；旧 `general-mask` / `ip-extract` 等遗留 id 由 `cleanup_deprecated_rules()` 自动删除。启动后即可在脱敏面板选 simple-mask/segment-mask + 预设执行脱敏，在提取面板多选规则批量提取校验，在行级校验面板映射字段分流到双 Tab；既有姓名脱敏、撤销/重做、列操作、搜索、Base64、.log 导入等功能无回归。

完整技术文档见 docs/versions/1.1.3/。
