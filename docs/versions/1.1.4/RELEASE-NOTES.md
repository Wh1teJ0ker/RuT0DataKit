# RuT0DataKit v1.1.4

> Git tag：`v1.1.4`（待推送）
> 状态：qa_passed（首轮 + 续轮 R2 + R3 续轮 Release QA 增量审计通过）
> 前置：v1.1.3 已发布 tag `v1.1.3`

## 概要

v1.1.4 重做校验模块为**统一校验页面**：用户可动态添加任意多条「目标列 + 校验规则」组合，一个「校验」按钮即把通过 / 失败的行分流到两个新 Tab。身份证规则可勾选跨字段比对性别 / 出生日期。

首轮（T67/T68/T69）完成统一校验页面与后端规则系统扩展；续轮（T70/T71/T72）在此基础上增强 4 项能力：通用校验规则（Generic 变体 + 字符类 / 长度范围参数覆盖）、地址校验放宽为结构化校验（中文 + 地址关键词）、出生日期校验支持分隔符格式（`clean_birth` 归一化）、前端参数 UI 统一化；R3 续轮（T73/T74/T75/T76）新增 2 项能力：CryptoPanel 哈希函数（MD5/SHA1/SHA256 列式变换，可撤销）+ 外部 SQLite 数据库文件解析（`.db`/`.sqlite`/`.sqlite3` 多表联合导入）。

## 改动

### 续轮：通用校验 + 地址放宽 + 生日清理 + 前端参数 UI（T70/T71/T72）

**后端（T70）**：

- `ExtractParams` 新增 `Generic` 变体（`allow_digits` / `allow_letters` / `allow_special` + `min_len` / `max_len`），新增 `generic-validate` 内置规则（`with_defaults()` 16 → 17 条）
- 新增 `is_valid_generic` 函数式校验器（字符类白名单 + 长度范围）；`is_valid_birth` 改为先 `clean_birth` 清理分隔符（`-` / `/` / `.` / 空格）再校验 8 位有效日期；`is_valid_address` 放宽为结构化校验（中文 ≥ 2 + 地址关键词）
- `validate_extracted` 抽取为 `validate_extracted_with_params(params, value)`（不依赖 Rule，便于覆盖参数）；`MultiRuleValidation` 新增 `params_override` 字段（前端可下发参数覆盖 DB 默认值，`#[serde(default)]` 向后兼容）
- 跨字段 birth 比对（`validate_rows_to_two_sheets_inner` + `validate_multi_rules_to_two_sheets_inner`）改用 `clean_birth` 归一化，支持 `1949-12-31` 等分隔符格式

**前端（T71/T72）**：见 T71/T72 任务，统一化校验规则参数 UI。

### R3 续轮：哈希函数 + DB 文件解析（T73/T74/T75）

**后端（T73）**：

- 新增 `hash_column` IPC 命令，对指定列做 MD5 / SHA1 / SHA256 哈希变换（hex 小写，不可逆），复用 v1.1.2 的 `base64_transform_column_cells` 闭包式列变换（单事务 + before/after 快照，可撤销）
- 新增 `HashAlgorithm` enum（`md5` / `sha1` / `sha256`，serde `rename_all = "lowercase"`）；`list_undoable_operations` 撤销栈白名单追加 `hash_column`
- 新增依赖 `md-5 = "0.10"` / `sha1 = "0.10"` / `sha2 = "0.10"` / `hex = "0.4"`（sha2/hex 此前为传递依赖，声明为直接依赖不额外拉取；md-5/sha1 为 Rust 生态标准 crate）；SCHEMA_VERSION 不变（=5，复用既有 `operations` 快照列）

**前端（T74）**：

- `CryptoPanel` 扩展算法选择：Base64 编/解码 + MD5/SHA1/SHA256；选哈希算法时禁用「解码」按钮（哈希不可逆）；执行后刷新数据 + undo 栈
- `tauri.js` 新增 `hashColumn(sheetId, column, algorithm)` IPC wrapper

**后端（T75）**：

- 新增 `DbReader` 数据源（`crates/core/src/datasource/db.rs`），打开外部 `.db` / `.sqlite` / `.sqlite3` 二进制 SQLite 文件，读取全部用户表（跳过 `sqlite_%` 内部表），多表联合输出 `__table` 首列标识来源 + 各表列名按首次出现顺序扩展（缺列补空）
- `detect_format` 工厂新增 3 个 match arm 路由到 `DbReader`，无新依赖（`rusqlite` 已是 core 依赖）
- 表名用 `quote_identifier` 转义（双引号包裹 + 内部双引号翻倍），表名来源为受信系统表 `sqlite_master`，无 SQL 注入面

### 首轮：统一校验页面（T67/T68/T69）

**问题**：v1.1.3 把行级多字段校验（T57）做成了独立能力（TopToolbar 单独按钮「行级校验」+ SidePanel 单独面板入口），与单列校验割裂；v1.1.4 初稿曾以 antd `Tabs` 把「单列校验」与「行级校验」并列入 `ValidatePanel`，但仍需用户在两个 Tab 间手动切换、且行级校验固定 7 字段、无法自由组合规则。

**方案**：把 `ValidatePanel` 完全重写为单一表单（无 Tabs/Segmented 切换）：

- 用 antd `Form.List` 实现动态规则行增删，每行 = 目标列 Select + 校验规则 Select + 删除按钮
- 校验规则 options 来自 `listRules()` 异步加载、`filter kind === "validate"`（含 T67 的 7 条 + T70 新增的 `generic-validate` 共 8 条）
- 当选中规则是 `idcard-validate` 时，用 `Form.Item shouldUpdate` 条件渲染跨字段配置：勾选「对比性别一致性」+ Select 性别列；勾选「对比出生日期一致性」+ Select 出生日期列
- 底部手机号前缀白名单 `Select mode="tags"`（可选，全局应用于 `phone-validate` 规则）
- 一个「校验」按钮 → 调 `validateMultiRulesToTwoSheets` → 通过 / 失败行分别写入两个新 Tab（`{源sheet名}_校验通过` / `{源sheet名}_校验失败`）
- 汇总消息：通过 / 失败行数 + top 失败原因

### 后端规则系统扩展（T67）

- `ExtractParams` 从单一形态扩展为 4 个变体（mask / extract / validate / 自定义）
- 新增 6 条 `kind=validate` 规则（函数式 + 正则），加上既有的 `name-validate`，共 7 条 validate 规则：username-validate / sex-validate / birth-validate / idcard-validate / phone-validate / address-validate / name-validate
- 规则分发逻辑泛化：按 `ruleId` 查表 → 路由到对应校验函数
- DB seed 写入上述 7 条 validate 规则；`idcard-validate` 规则支持 crossField

### 新命令（T68）

- 新增 IPC 命令 `validate_multi_rules_to_two_sheets`，承接多规则列式校验
- 参数：`sheetId / sessionId / rules: Array<{column, ruleId, crossField?, paramsOverride?}> / phonePrefixes?: string[]`
- 返回：`{ validSheet: ParseResult, invalidSheet: ParseResult, invalidReasons: Array<{sourceRow, field, reason}> }`
- `frontend/src/tauri.js` 新增 `validateMultiRulesToTwoSheets` wrapper

## 不变项

- 既有 `validate_column` / `validate_rows_to_two_sheets` IPC 与 wrapper 保留（向后兼容）；`validate_rows_to_two_sheets` 命令签名不变（内部 birth 比对改用 `clean_birth`）
- `extract_validate_to_new_sheet_inner` 不改（`validate_extracted` wrapper 签名保持）
- `App.jsx` 路由不变（validate 走默认 SidePanel 分支；CryptoPanel 入口 v1.1.2 已有）
- DB schema / capabilities/default.json 不变（SCHEMA_VERSION=5，`params` 列复用；`operations` 快照列复用存 `hash_column` before/after）
- `TopToolbar.jsx` / `SidePanel.jsx` 沿用 v1.1.4 初稿已移除 rowValidate 入口的状态
- `base64_column` 命令不变（`hash_column` 是新增独立命令，复用 DB 层闭包式列变换方法）
- `crates/core` 的 processor / datasource trait 不变（`DbReader` 是新增 impl，trait 签名不变）
- `detect_format` 新增 `.db`/`.sqlite`/`.sqlite3` 分支不破坏旧格式（csv/xlsx/json/jsonl/txt/log/sql/pcap/pcapng 分支不变）
- v1.1.3 已发布文档与 GitHub Release 正文不改

## 验收

- `ValidatePanel.jsx` 是单一表单（无 Tabs/Segmented 切换），顶部可动态添加多条规则行
- 每条规则行：目标列 Select + 校验规则 Select + 删除按钮
- 校验规则 options 包含 8 条 validate 规则（含 `generic-validate`）
- 当选中规则是 `idcard-validate` 时展开跨字段配置
- 底部有手机号前缀白名单 `Select mode="tags"`
- 一个「校验」按钮 → 双 Tab 落地 + 汇总消息
- CryptoPanel 可选 Base64 编/解码 + MD5/SHA1/SHA256；哈希执行后可撤销
- `detect_format` 对 `.db`/`.sqlite`/`.sqlite3` 返回 `DbReader`；多表联合输出 `__table` 列
- `cargo fmt --all` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test --all` 全绿（315 passed）
- `pnpm --prefix frontend build` 通过（3083 modules）
- 版本号 4 处一致 1.1.4
- Release QA R3 增量审计通过（见 [`docs/qa/versions/1.1.4/QA-审计报告.md`](../qa/versions/1.1.4/QA-审计报告.md) §16-§17）

## 安全说明

- SQL 全部参数化绑定，禁拼接（后端未改，保持）；DbReader 表名例外：`quote_identifier` 转义 + `sqlite_master` 受信来源，无注入面
- 无凭据字面量
- Mimosa 完整审计未拿到结论前不宣称安全；R3 续轮未重新运行完整审计，沿用 v1.1.3 R1+R2 双轮 0 findings / 487 包 0 漏洞基线（R3 改动为 codec + datasource 功能性增强，未改 DB schema / CSP / 权限 / 网络，基线有效）

## 详细文档

- 更新日志：[`docs/versions/1.1.4/更新日志.md`](./更新日志.md)
- v1.1.3 基线：[`docs/versions/1.1.3/RELEASE-NOTES.md`](../1.1.3/RELEASE-NOTES.md)
