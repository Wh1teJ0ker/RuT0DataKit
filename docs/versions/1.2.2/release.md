# v1.2.2 发布说明

> 版本号：v1.2.2（SCHEMA_VERSION=5，无数据库迁移）
> 发布日期：2026-08-19
> 里程碑：MySQL dump 导入兼容 + 规则参数一致性 + 面板修复

## What's New

### MySQL dump SQL 导入兼容
- `SqlReader` 增强：安全处理 MySQL 块注释、版本注释、MySQL-only 语句（CREATE DATABASE/USE/SET/LOCK TABLES）
- `sanitize_create_table`：剥离列级 COMMENT、CHARACTER SET、COLLATE、AUTO_INCREMENT，截断表选项
- `sanitize_backslash_escapes`：将 MySQL 反斜杠转义（`\'`、`\"`、`\\`）转换为 SQLite 兼容格式
- 标准 SQLite 文件不受影响

### 地址规则参数化
- 地址校验增加英文检查（含英文字母的地址不再通过）
- 规则管理、行级校验、本地预览统一支持可选的号（`minHao`/`maxHao`）和室（`minShi`/`maxShi`）数字范围
- 向后兼容：旧 DB 数据 `{"validator":"address"}` 自动反序列化为不限范围

### 出生日期与身份证提取参数一致性
- 出生日期格式选择可保存、重置、传入规则运行预览
- 身份证提取新增「允许首位为 0」开关，仅本次运行时覆盖正则，不持久化规则

### 工具面板修复
- 修复 ColumnOpsPanel 因缺失 `Form.useForm()` 和 `useState` 声明导致的白屏
- 修复 CryptoPanel 因缺失 `Form.useForm()` 和 `useState` 声明导致的白屏

## Fixed

- 块注释内 `*` 字符泄漏到语句缓冲区
- COMMENT 正则大小写不敏感（`(?i)`）
- COMMENT 正则支持转义单引号

## Downloads

通过 GitHub Releases 下载对应平台的安装包。安装后启动即完成升级，无需手动迁移数据。

| 平台 | 架构 | 格式 |
|------|------|------|
| Linux | x86_64 | `.tar.gz` |
| macOS | aarch64 | `.dmg` 或 `.zip` |
| macOS | x86_64 | `.dmg` 或 `.zip` |
| Windows | x86_64 | `.msi` 或 `.zip` |

## 升级说明

- 直接安装覆盖旧版本，应用数据自动继承
- `SCHEMA_VERSION=5` 不变，无数据库迁移