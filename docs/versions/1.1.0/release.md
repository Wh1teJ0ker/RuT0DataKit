# RuT0DataKit v1.1.0

## Update

- 数据脱敏（mask）：选列 + 可填掩码字符（默认 `*`）→ 就地脱敏。≥3 字符保留首尾、中间掩码字符替换（「张三丰」→「张*丰」）；2 字符保留首字符末位替换（「张三」→「张*」）。脱敏结果回写 DB + 行高亮，掩码字符可填、可重置、可保存设置
- 数据校验（validate）：选列 + 选规则 → 正则校验 → 不通过行高亮；内置「姓名校验」规则（2-4 位中文字符）
- 数据提取（extract）：选列 + 多选规则 → 前端正则提取 → 命中行高亮 + PII 列表展示；内置手机号 / 邮箱 / 身份证号 3 正则
- 规则管理（DB 持久化）：内置 3 条姓名相关规则（脱敏 / 校验 / 提取各一），持久化到 SQLite（rules 表，重启后保留）；RulesPanel 改为 Workbench 主区两栏布局（左侧规则列表按类别分组 + 右侧规则详情 + 可填参数 + 内联测试）；可填参数保存后回写 DB
- 行状态高亮：masked（浅蓝）/ invalid（浅红）/ hit（浅绿）三类行高亮样式落地
- 自动更新：启用 createUpdaterArtifacts，构建自动生成 .sig 签名文件 + latest.json 清单，应用内自动更新可用
- 版本号 1.0.0 → 1.1.0

## Fix

- 安全加固：CSP 从 null 收紧为 `default-src 'self'` + 显式白名单；capabilities 删除 home/desktop 递归写权限；updater dialog 改为 true（用户确认）
- CI release workflow 删除 Rename assets 步骤（修复 asset under the same name already exists 冲突）
- CI release workflow 删除 Upload portable exe 步骤（简化 release 流程，避免与 tauri-action 资产上传时序耦合）
- 修复自动更新签名产物缺失（createUpdaterArtifacts 默认 false 导致不生成 .sig + latest.json）

## 下载

| 平台 | 文件 |
|---|---|
| Windows x64 | RuT0DataKit_1.1.0_x64-setup.exe / RuT0DataKit_1.1.0_x64_en-US.msi |
| macOS arm64 | RuT0DataKit_1.1.0_aarch64.dmg / RuT0DataKit_aarch64.app.tar.gz |
| Linux x64 | RuT0DataKit_1.1.0_amd64.deb / RuT0DataKit-1.1.0-1.x86_64.rpm / RuT0DataKit_1.1.0_amd64.AppImage |

每个安装包均附带 .sig 签名文件，供 updater 验签。

## 升级

v1.0.0 用户可直接升级：DB schema 从 SCHEMA_VERSION=1 增量迁移到 2（新增 rules 表 + 索引，IF NOT EXISTS 幂等，不触发备份重建），历史数据兼容。启动时若 rules 表为空，自动 seed 3 条内置规则。

完整技术文档见 docs/versions/1.1.0/。
