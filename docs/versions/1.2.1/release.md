# RuT0DataKit v1.2.1

## Update

- **大 TXT 文件按行分块解析**：TxtReader 不再将整个文件塞入单一 cell，改用 `chunk_string` + `lines()` 分块，8.5 MB 单行 TXT 可正常分页浏览
- **第三轮架构优化**：Rust 后端表驱动去重（builtins.rs 397→62 行）+ `OnceLock<Regex>` 正则缓存 + `conn()`/`map_row_to_cell`/`log_column_op` 共享 helper 收敛；前端 `useActiveSheet`/`useStaleGuard` hooks + AppContext 精简（195→32 行）+ 3 个 god-file 拆分（DataTable 455→287 / ValidatePanel 412→177 / ExportModal 378→246）

## Fix

- **手机号空名单误过滤 7xx 号段**：`check_phone_prefix(s, &[])` 从 `starts_with('1')` 改为 `true`（空=不过滤）；PhonePrefix 分支统一走 `is_valid_phone` 完整校验（11 位+全数字+前缀）
- **DataTable 超长单元格冻结**：`highlightCell` 对超阈值 cell 跳过 `TextEncoder.encode`，antd Table 启用 `ellipsis` 截断
- **ExtractPanel 主线程同步冻结**：超大输入跳过 `[...matchAll(re)]` 同步正则，提示改用「提取并校验到新 Tab」
- **文档漂移**：清除 `docs/02` + `docs/03` 中所有 `func_validator`/`extractor.rs`/`validate_column`/`extract_column` 过时引用

## 下载

| 平台 | 文件 |
|---|---|
| macOS arm64 | RuT0DataKit_1.2.1_aarch64.dmg |
| macOS x64 | RuT0DataKit_1.2.1_x64.dmg |

每个安装包均附带 .sig 签名文件，供 updater 验签。

## 升级

v1.2.0 用户可直接升级：DB schema 不变（`SCHEMA_VERSION=5`），无需数据迁移。所有 IPC 命令签名向后兼容，旧前端 JSON 无新字段时走默认值。

完整技术文档见 docs/versions/1.2.1/。
