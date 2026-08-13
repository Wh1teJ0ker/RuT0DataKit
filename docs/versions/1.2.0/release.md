# RuT0DataKit v1.2.0

## Update

- 后端模块拆分：3 个过大 Rust 文件拆为 11 个职责子模块 — `db/mod.rs` → cells/sheets/rules/search/operations（5 子模块）；`commands/processor.rs` → rules_ops/mask_ops/extract_ops/undo_ops（4 子模块）；`core/processor/rules.rs` → template/extract_params（2 子模块）。纯结构拆分，不改外部行为
- 前端共享抽象：提取 `useBreakpoint` / `useRules` / `useSheetOps` 三个共享 hooks 和 `ColumnSelect` / `PhonePrefixSelect` / `TemplateEditor` 三个共享 UI 原语，消除 MaskPanel ↔ RulesPanel 间的模板编辑器重复代码
- 前端高适应性升级：基于 antd `Grid.useBreakpoint()` 断点感知驱动全布局响应 — SidePanel/AiPanel 可折叠为 48px 图标条，窄窗口自动折叠；新增「固定」按钮（PushpinOutlined/PushpinFilled），pinned 时窄屏不自动折叠；TopToolbar wrap 换行 + icon-only 模式；RulesPanel 自适应堆叠；根布局 dvh 兼容；散落硬编码像素宽度收敛为 flex/vh 响应式值
- UI 文案精简：所有面板描述精简为简洁一句话，移除冗余解释和行话堆砌
- 版本号 1.1.5 → 1.2.0

## Fix

- RulesPanel 详情滚动修复：antd `Spin` 包装器的 `spin-container` div 破坏 `height: 100%` 链导致右侧详情区无法滚动，移除 Spin 包装改为 absolute overlay 遮罩方案
- 内联测试结果展示增强：验证器返回的 `note` 字段（如身份证性别推断）通过 Alert description 展示；测试结果容器和命中列表添加 `maxHeight` + `overflow: auto` 防止长结果撑爆布局

## 下载

| 平台 | 文件 |
|---|---|
| Windows x64 | RuT0DataKit_1.2.0_x64-setup.exe / RuT0DataKit_1.2.0_x64_en-US.msi |
| macOS arm64 | RuT0DataKit_1.2.0_aarch64.dmg / RuT0DataKit_aarch64.app.tar.gz |
| Linux x64 | RuT0DataKit_1.2.0_amd64.deb / RuT0DataKit-1.2.0-1.x86_64.rpm / RuT0DataKit_1.2.0_amd64.AppImage |

每个安装包均附带 .sig 签名文件，供 updater 验签。macOS Intel (x86_64) 不在本版构建矩阵中（仅 aarch64-apple-darwin），Intel Mac 用户可通过 Rosetta 运行 ARM 版本。

## 升级

v1.1.5 用户可直接升级：DB schema 不变（`SCHEMA_VERSION=5`），无需数据迁移。32 个 `generate_handler!` IPC 命令列表不变，前后端 IPC 接口完全兼容。无新增或废弃配置项，`settings.json` 向后兼容。

完整技术文档见 docs/versions/1.2.0/。
