# RuT0DataKit v1.1.4

> 发布日期：2026-08-12
> QA 报告：[QA-审计报告.md](../../qa/versions/1.1.4/QA-审计报告.md)
> GitHub Release：<https://github.com/Wh1teJ0ker/RuT0DataKit/releases/tag/v1.1.4>

## Update

### 统一校验页面（T67/T68/T69）

- 重做 `ValidatePanel` 为单一表单（移除 Tabs/Segmented 切换）：用 antd `Form.List` 动态增删「目标列 + 校验规则」行，一个「校验」按钮把通过 / 失败的行分流到两个新 Tab（`{源sheet}_校验通过` / `{源sheet}_校验失败`）
- 后端 `ExtractParams` 扩展为 4 变体（mask / extract / validate / 自定义），新增 6 条 `kind=validate` 内置规则（username / sex / birth / idcard / phone / address），加上既有 `name-validate` 共 7 条
- 新增 `validate_multi_rules_to_two_sheets` IPC 命令：多规则列式校验 + 身份证跨字段可选比对性别 / 出生日期 → 双 Tab 输出 + 失败原因汇总

### 通用校验 + 地址放宽 + 生日清理 + 前端统一化（T70/T71/T72）

- 新增 `Generic` 变体（`allow_digits` / `allow_letters` / `allow_special_chars` / `min_len` / `max_len`）+ `generic-validate` 内置规则（with_defaults 16 → 17）
- `is_valid_birth` 改为先 `clean_birth` 清理分隔符（`-` / `/` / `.` / 空格）再校验 8 位有效日期；`is_valid_address` 放宽为结构化校验（中文 ≥ 2 + 地址关键词）
- `MultiRuleValidation` 新增 `params_override` 字段（`#[serde(default)]` 向后兼容），前端可下发参数覆盖 DB 默认值
- 前端 `RulesPanel` / `ValidatePanel` 按规则类型统一参数 UI

### 哈希函数 + DB 文件解析（T73/T74/T75/T76）

- 新增 `hash_column` IPC 命令（MD5 / SHA1 / SHA256 列式哈希，hex 小写，不可逆），复用 v1.1.2 的 `base64_transform_column_cells` 闭包式列变换（单事务 + before/after 快照，可撤销）
- 前端 `CryptoPanel` 扩展算法选择：Base64 编/解码 + MD5/SHA1/SHA256；选哈希时禁用「解码」按钮；新增 `hashColumn` IPC wrapper
- 新增 `DbReader` 数据源：打开外部 `.db` / `.sqlite` / `.sqlite3` 二进制 SQLite 文件，多表联合输出（`__table` 首列标识来源 + 缺列补空 + 跳过 `sqlite_%` 内部表 + `quote_identifier` 表名转义）

### 特殊符号白名单 + 手机号前缀 UX 统一 + 名称排序（T77/T78/T79）

- `generic-validate` 的 `allow_special: bool` → `allow_special_chars: String` 白名单语义（前端 Checkbox.Group → 字符串 Input）
- 手机号前缀配置 UX 统一：`ValidatePanel` phone-validate 每行内嵌前缀 `Select mode="tags"`；`ExtractPanel` phone-extract 新增前缀输入；`extract_validate_to_new_sheet` 新增 `phone_prefixes` 运行时覆盖参数
- `list_rules` SQL 排序 `ORDER BY id ASC` → `ORDER BY name ASC`（Unicode 码点升序），前端各面板自动继承新顺序

## Fix

- 修复 v1.1.3 行级校验与单列校验割裂问题：统一校验页面取代独立「行级校验」入口，用户可自由组合任意规则 + 列
- 修复 `is_valid_birth` 不支持分隔符日期格式（如 `1949-12-31`）的问题
- 修复规则列表排序不稳定问题：从 kebab-case id 排序改为 name Unicode 码点排序

## Downloads

| 平台 | 文件 |
|------|------|
| Windows x64 (NSIS) | `RuT0DataKit_1.1.4_x64-setup.exe` |
| Windows x64 (MSI) | `RuT0DataKit_1.1.4_x64_en-US.msi` |
| Linux x64 (AppImage) | `RuT0DataKit_1.1.4_amd64.AppImage` |
| Linux x64 (deb) | `RuT0DataKit_1.1.4_amd64.deb` |
| Linux x64 (rpm) | `RuT0DataKit-1.1.4-1.x86_64.rpm` |
| macOS arm64 (DMG) | `RuT0DataKit_1.1.4_aarch64.dmg` |
| macOS arm64 (tar.gz) | `RuT0DataKit_aarch64.app.tar.gz` |

> 所有产物附 Ed25519 签名（`.sig` 文件），updater JSON（`latest.json`）随 Release 上传。

## 升级

- **数据迁移**：DB schema 不变（`SCHEMA_VERSION=5`），从 v1.1.3 升级无需迁移；`params` 列复用存 `hash_column` before/after 快照
- **配置变更**：无新增配置项；`ExtractParams::PhonePrefix` 同时服务于 phone-extract 和 phone-validate；`extract_validate_to_new_sheet_inner` 签名新增 `phone_prefixes: &[String]` 末位参数，既有调用方传 `&[]` 保持兼容
- **向后兼容**：`MultiRuleValidation.params_override` 使用 `#[serde(default)]`；`allow_special_chars` 使用 `#[serde(default)]`；既有 `validate_column` / `validate_rows_to_two_sheets` IPC 与 wrapper 保留

## 已知问题

- Mimosa 完整深度审计未在 R3/R4 续轮重新运行（沿用 v1.1.3 R1+R2 双轮 0 findings / 487 包 0 漏洞基线，R3/R4 改动未改 DB schema / CSP / 权限 / 网络，基线有效）
- 静态分析非运行时验证，不宣称项目安全

> 完整技术文档见 `docs/versions/1.1.4/`。
