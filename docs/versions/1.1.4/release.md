# RuT0DataKit v1.1.4

## Update

- 统一校验页面：重做 `ValidatePanel` 为单一表单（antd `Form.List` 动态增删「目标列 + 校验规则」行），一个「校验」按钮把通过 / 失败的行分流到两个新 Tab；新增 `validate_multi_rules_to_two_sheets` IPC（多规则列式校验 + 身份证跨字段比对 → 双 Tab 输出 + 失败原因汇总）
- 通用校验规则：新增 `Generic` 变体（`allow_digits` / `allow_letters` / `allow_special_chars` / `min_len` / `max_len`）+ `generic-validate` 内置规则（17 条）；`is_valid_birth` 支持清理分隔符后校验；`is_valid_address` 放宽为结构化校验
- 哈希函数：新增 `hash_column` IPC（MD5 / SHA1 / SHA256 列式哈希，hex 小写，不可逆，可撤销）；`CryptoPanel` 扩展算法选择
- DB 文件解析：新增 `DbReader` 数据源，打开外部 `.db` / `.sqlite` / `.sqlite3` 文件，多表联合输出（`__table` 首列标识来源 + 缺列补空 + 跳过内部表）
- 特殊符号白名单 + 前缀 UX 统一 + 名称排序：`allow_special: bool` → `allow_special_chars: String` 白名单语义；手机号前缀配置 UX 统一（ValidatePanel / ExtractPanel 内嵌前缀 Select）；规则列表按 name Unicode 码点排序
- 版本号 1.1.3 → 1.1.4

## Fix

- 修复 v1.1.3 行级校验与单列校验割裂问题：统一校验页面取代独立「行级校验」入口
- 修复 `is_valid_birth` 不支持分隔符日期格式（如 `1949-12-31`）的问题
- 修复规则列表排序不稳定问题：从 kebab-case id 排序改为 name Unicode 码点排序

## 下载

| 平台 | 文件 |
|---|---|
| Windows x64 | RuT0DataKit_1.1.4_x64-setup.exe / RuT0DataKit_1.1.4_x64_en-US.msi |
| macOS arm64 | RuT0DataKit_1.1.4_aarch64.dmg / RuT0DataKit_aarch64.app.tar.gz |
| Linux x64 | RuT0DataKit_1.1.4_amd64.deb / RuT0DataKit-1.1.4-1.x86_64.rpm / RuT0DataKit_1.1.4_amd64.AppImage |

每个安装包均附带 .sig 签名文件，供 updater 验签。macOS Intel (x86_64) 不在本版构建矩阵中（仅 aarch64-apple-darwin），Intel Mac 用户可通过 Rosetta 运行 ARM 版本。

## 升级

v1.1.3 用户可直接升级：DB schema 不变（`SCHEMA_VERSION=5`），无需迁移。`ExtractParams::PhonePrefix` 同时服务于 phone-extract 和 phone-validate；`MultiRuleValidation.params_override` 使用 `#[serde(default)]` 向后兼容；既有 `validate_column` / `validate_rows_to_two_sheets` IPC 与 wrapper 保留。

完整技术文档见 docs/versions/1.1.4/。
