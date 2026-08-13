# RuT0DataKit v1.1.5

> 发布日期：2026-08-13
> QA 报告：[QA-审计报告.md](../../qa/versions/1.1.5/QA-审计报告.md)
> GitHub Release：<https://github.com/Wh1teJ0ker/RuT0DataKit/releases/tag/v1.1.5>

## Update

### 邮箱校验规则（T81）

- 新增 `ExtractParams::Email` 变体 + `is_valid_email` 函数式校验器（trim + ≤254 字符 + 恰好 1 个 `@` + local ≤64 `[a-zA-Z0-9._%+-]` + domain 含 `.` `[a-zA-Z0-9.-]`）
- 新增 `email-validate` 内置规则（`with_defaults` 17 → 18 条），前端 `ValidatePanel` 自动从 DB 加载，无需前端改动
- `validate_extracted_with_params` 新增 Email 分支：通过 → `(true, "")`，不通过 → `(false, "邮箱格式不符")`

### .db 文件导入修复（T82）

- `TopToolbar` 文件选择器 extensions 追加 `db` / `sqlite` / `sqlite3`（后端 `DbReader` + `detect_format` 在 v1.1.4 T75 已完整支持，此前前端过滤遗漏导致无法选中）

### 哈希大小写 hex 选项（T83）

- 新增 `HashCase` enum（`lower` / `upper`），`hash_column` 命令追加 `case` 参数
- MD5 / SHA1 / SHA256 三算法均支持大写 hex 输出（闭包条件 `to_uppercase()`）
- 前端 `CryptoPanel` 新增 `Radio.Group`（小写 / 大写），选哈希时显示

### 先校验再脱敏（T84/T85）

- `mask_column` 追加 4 个可选参数：`validate_rule_id` / `invalid_text` / `params_override` / `phone_prefixes`
- 勾选后每行先按指定校验规则校验（phone-validate → `is_valid_phone` / params → `validate_extracted_with_params` / pattern → 正则 / else → pass），通过 → 正常脱敏，不通过 → 输出 `INVALID`（可自定义）
- 前端 `MaskPanel` 新增 Checkbox "先校验再脱敏" + 校验规则 Select + 无效文本 Input + phone-validate 前缀 Select（tags mode）

### 列操作大小写归一化（T86）

- 新增 `TransformOp` enum（`uppercase` / `lowercase`）+ `transform_column` IPC 命令
- 复用 `base64_transform_column_cells` 闭包式列变换（单事务 + before/after 快照，可撤销）
- `list_undoable_operations` 撤销白名单追加 `transform_column`
- 前端 `ColumnOpsPanel` 新增列变换区块（目标列 Select + 操作类型 Select + 执行按钮）

### 生日校验多格式可勾选（T87/T88）

- `ExtractParams::Birth` 从 unit variant 改为 struct variant：`Birth { #[serde(default)] formats: Vec<String> }`
- `formats` 空 = 全部接受（向后兼容，走 `is_valid_birth` 清理分隔符后 8 位校验）；非空 = 仅接受指定格式
- 新增 `is_valid_birth_format(s, format)` 支持 4 种格式：`yyyymmdd` / `yyyy-mm-dd` / `yyyy/mm/dd` / `yyyy.mm.dd`
- 前端 `ValidatePanel` birth-validate 行展开 `Checkbox.Group`（4 格式可选），全不选 = 接受所有格式

## Fix

- 修复前端文件选择器无法选中 `.db` / `.sqlite` / `.sqlite3` 文件的问题（extensions 过滤遗漏）
- 优化生日校验：从统一清理分隔符后 8 位校验改为支持按格式勾选，用户可限定仅接受特定日期格式

## Downloads

| 平台 | 文件 |
|------|------|
| Windows x64 (NSIS) | `RuT0DataKit_1.1.5_x64-setup.exe` |
| Windows x64 (MSI) | `RuT0DataKit_1.1.5_x64_en-US.msi` |
| Linux x64 (AppImage) | `RuT0DataKit_1.1.5_amd64.AppImage` |
| Linux x64 (deb) | `RuT0DataKit_1.1.5_amd64.deb` |
| Linux x64 (rpm) | `RuT0DataKit-1.1.5-1.x86_64.rpm` |
| macOS arm64 (DMG) | `RuT0DataKit_1.1.5_aarch64.dmg` |
| macOS arm64 (tar.gz) | `RuT0DataKit_aarch64.app.tar.gz` |

> 所有产物附 Ed25519 签名（`.sig` 文件），updater JSON（`latest.json`）随 Release 上传。

## 升级

- **数据迁移**：DB schema 不变（`SCHEMA_VERSION=5`），从 v1.1.4 升级无需迁移；`email-validate` 规则通过 `seed_builtin_rules` upsert-missing 自动补入（老用户已有 17 条不动，补 1 条 → 18 条）
- **配置变更**：无新增配置项；`HashCase` / `TransformOp` enum 通过 serde lowercase 映射；`mask_column` 4 个新参数均为 `Option`，既有调用方不传则行为不变
- **向后兼容**：`ExtractParams::Birth` 使用 `#[serde(default)] formats`，旧数据 `{"validator":"birth"}` 可反序列化为 `Birth { formats: vec![] }`（空 = 全部接受，与 v1.1.4 行为一致）

## 已知问题

- `pnpm build` 产物 chunk >500kB 警告（antd 既有行为，v1.1.0 起即存在，不影响功能）
- Mimosa 完整深度扫描未重新运行，沿用 v1.1.3 双轮 0 findings 基线（v1.1.5 无新依赖、无 schema 变更、无权限变更）
