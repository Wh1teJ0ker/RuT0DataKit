# RuT0DataKit v1.1.4

> Git tag：`v1.1.4`（待推送）
> 状态：开发中
> 前置：v1.1.3 已发布 tag `v1.1.3`

## 概要

v1.1.4 是一个聚焦体验优化的版本：将 v1.1.3 作为独立能力入口的「行级校验」合并回「校验」模块，与单列校验并列为同一面板的两个 Tab，消除用户在两个按钮间切换的割裂感。本版为纯前端 UI 合并，后端校验命令与实现不变。

## 改动

### 校验模块统一（T67）

**问题**：v1.1.3 把行级多字段校验（T57）做成了独立能力（TopToolbar 单独按钮「行级校验」+ SidePanel 单独面板入口），与单列校验割裂。用户需在「校验」与「行级校验」两个按钮间切换，且两者本质都是「校验」能力。

**方案**：在 `ValidatePanel` 内用 antd `Tabs` 提供两个页签：

- **单列校验**：选列 + 规则 → 原位高亮无效行（`validateColumn` IPC，行为不变）
- **行级校验**：7 字段（username/name/sex/birth/idcard/phone/address）→列映射 + 手机前缀白名单 → 通过/失败的行分别写入两个新 Tab（`validateRowsToTwoSheets` IPC，行为不变）

两个 Tab 各持独立 `Form.useForm()` 实例，切 Tab 不丢数据。

**入口收编**：

- TopToolbar 移除独立「行级校验」按钮，能力按钮从 7 个回到 6 个（脱敏 / 校验 / 提取 / 列操作 / 加解密 / 规则管理）
- SidePanel 移除 `rowValidate` 面板入口与 `RowValidatePanel.jsx`（逻辑已合并入 ValidatePanel）

**版本号 bump**：1.1.3 → 1.1.4（`Cargo.toml` workspace / `tauri.conf.json` / `frontend/package.json` / `frontend/src/constants.js` 四处同步）

## 不变项

- 后端 `validate_rows_to_two_sheets` / `validate_column` IPC 命令、实现、测试均不变
- `frontend/src/tauri.js` wrapper 保留
- `frontend/src/App.jsx` 路由不变
- DB schema / capabilities/default.json 不变
- v1.1.3 已发布文档与 GitHub Release 正文不改

## 验收

- TopToolbar 不再有「行级校验」按钮（6 个能力按钮）
- 点「校验」→ SidePanel 渲染 ValidatePanel，内含「单列校验」「行级校验」两个 Tab
- 单列校验 Tab 行为与 v1.1.3 一致（原位高亮无效行）
- 行级校验 Tab 行为与 v1.1.3 `RowValidatePanel` 一致（7 字段映射 + 跨字段联合 + 双 Tab 落地）
- `RowValidatePanel.jsx` 已删除，构建无未解析导入
- 版本号 4 处统一为 1.1.4
- `pnpm --prefix frontend build` 通过
- `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all` 通过（后端未改，预期全绿）

## 安全说明

- SQL 全部参数化绑定，禁拼接（后端未改，保持）
- 无凭据字面量
- Mimosa 完整审计未拿到结论前不宣称安全；本版未重新运行完整审计

## 详细文档

- 更新日志：[`docs/versions/1.1.4/更新日志.md`](./更新日志.md)
- v1.1.3 基线：[`docs/versions/1.1.3/RELEASE-NOTES.md`](../1.1.3/RELEASE-NOTES.md)
