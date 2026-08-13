# RuT0DataKit v1.1.5

## Update

- 邮箱校验规则：新增 `ExtractParams::Email` 变体 + `is_valid_email` 函数式校验器（trim + ≤254 字符 + 恰好 1 个 `@` + local ≤64 + domain 含 `.`）；新增 `email-validate` 内置规则（17 → 18 条），前端自动从 DB 加载
- 哈希大小写 hex 选项：新增 `HashCase` enum（`lower` / `upper`），`hash_column` 命令追加 `case` 参数；前端 `CryptoPanel` 新增 `Radio.Group`（小写 / 大写）
- 先校验再脱敏：`mask_column` 追加 4 个可选参数（`validate_rule_id` / `invalid_text` / `params_override` / `phone_prefixes`），通过校验 → 正常脱敏，不通过 → 输出 `INVALID`（可自定义）；前端 `MaskPanel` 新增 Checkbox + 校验规则 Select + 无效文本 Input
- 列操作大小写归一化：新增 `TransformOp` enum（`uppercase` / `lowercase`）+ `transform_column` IPC（可撤销）；前端 `ColumnOpsPanel` 新增列变换区块
- 生日校验多格式可勾选：`ExtractParams::Birth` 改为 struct variant（`formats: Vec<String>`），空 = 全部接受，非空 = 仅接受指定格式（`yyyymmdd` / `yyyy-mm-dd` / `yyyy/mm/dd` / `yyyy.mm.dd`）；前端 `ValidatePanel` birth-validate 行展开 `Checkbox.Group`
- 版本号 1.1.4 → 1.1.5

## Fix

- 修复前端文件选择器无法选中 `.db` / `.sqlite` / `.sqlite3` 文件的问题（extensions 过滤遗漏，后端 v1.1.4 T75 已支持）
- 优化生日校验：从统一清理分隔符后 8 位校验改为支持按格式勾选，用户可限定仅接受特定日期格式

## 下载

| 平台 | 文件 |
|---|---|
| Windows x64 | RuT0DataKit_1.1.5_x64-setup.exe / RuT0DataKit_1.1.5_x64_en-US.msi |
| macOS arm64 | RuT0DataKit_1.1.5_aarch64.dmg / RuT0DataKit_aarch64.app.tar.gz |
| Linux x64 | RuT0DataKit_1.1.5_amd64.deb / RuT0DataKit-1.1.5-1.x86_64.rpm / RuT0DataKit_1.1.5_amd64.AppImage |

每个安装包均附带 .sig 签名文件，供 updater 验签。macOS Intel (x86_64) 不在本版构建矩阵中（仅 aarch64-apple-darwin），Intel Mac 用户可通过 Rosetta 运行 ARM 版本。

## 升级

v1.1.4 用户可直接升级：DB schema 不变（`SCHEMA_VERSION=5`），无需迁移。`email-validate` 规则通过 `seed_builtin_rules` upsert-missing 自动补入（老用户 17 条不动，补 1 条 → 18 条）。`mask_column` 4 个新参数均为 `Option`，既有调用方不传则行为不变；`ExtractParams::Birth` 使用 `#[serde(default)] formats`，旧数据可反序列化为空 formats（全部接受，与 v1.1.4 一致）。

完整技术文档见 docs/versions/1.1.5/。
