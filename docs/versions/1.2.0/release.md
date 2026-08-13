# RuT0DataKit v1.2.0

> 发布日期：2026-08-13
> QA 报告：[QA-审计报告.md](../../qa/versions/1.2.0/QA-审计报告.md)

## Update

- **后端模块拆分**：3 个过大 Rust 文件拆为 11 个职责子模块 — `db/mod.rs` → cells/sheets/rules/search/operations（5 子模块）；`commands/processor.rs` → rules_ops/mask_ops/extract_ops/undo_ops（4 子模块）；`core/processor/rules.rs` → template/extract_params（2 子模块）。纯结构拆分，不改外部行为。
- **前端共享抽象**：提取 `useBreakpoint` / `useRules` / `useSheetOps` 三个共享 hooks 和 `ColumnSelect` / `PhonePrefixSelect` / `TemplateEditor` 三个共享 UI 原语，消除 MaskPanel ↔ RulesPanel 间的模板编辑器重复代码。
- **前端高适应性升级**：基于 antd `Grid.useBreakpoint()` 断点感知驱动全布局响应 — SidePanel/AiPanel 可折叠为 48px 图标条，窄窗口自动折叠；新增「固定」按钮（PushpinOutlined/PushpinFilled），pinned 时窄屏不自动折叠；TopToolbar wrap 换行 + icon-only 模式；RulesPanel 自适应堆叠；根布局 dvh 兼容；散落硬编码像素宽度收敛为 flex/vh 响应式值。
- **UI 文案精简**：所有面板描述精简为简洁一句话，移除冗余解释和行话堆砌。

## Fix

- **RulesPanel 详情滚动修复**：antd `Spin` 包装器的 `spin-container` div 会破坏 `height: 100%` 链导致右侧详情区无法滚动。移除 Spin 包装改为 absolute overlay 遮罩方案，恢复 flex/height/overflow 链完整性。
- **内联测试结果展示增强**：验证器返回的 `note` 字段（如身份证性别推断）现通过 Alert description 展示；测试结果容器和命中列表添加 `maxHeight` + `overflow: auto` 防止长结果撑爆布局。

## Downloads

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | RuT0DataKit_1.2.0_aarch64.dmg |
| macOS (Intel) | RuT0DataKit_1.2.0_x64.dmg |

> 按实际构建产物填写。如启用签名，附签名文件。

## 升级

- **数据迁移**：无。`SCHEMA_VERSION=5` 未变，数据库完全兼容。
- **IPC 命令**：32 个 `generate_handler!` 命令列表不变，前后端 IPC 接口完全兼容。
- **配置变更**：无新增或废弃配置项。

## 已知问题

- vite build 有 `>500kB chunk` 体积警告，是 antd 全量引入的已知行为（v1.0.0 起一直存在），不影响功能。

> 完整技术文档见 `docs/versions/1.2.0/`。
