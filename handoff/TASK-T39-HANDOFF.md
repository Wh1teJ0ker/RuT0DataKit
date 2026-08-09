```yaml
task_id: T39
goal: |
  精简设置页三张卡片的冗余描述文案：
  - DbPathCard：移除「这是 v1.0.0 在 macOS 上的预期数据库位置；后续版本将提供动态获取命令。」冗余说明
  - AboutCard：移除「数据脱敏 / 校验 / 提取工作台（v1.1.0 处理器原型已接入，业务能力 {DEV_STATUS}）」长文案
  - TsharkPathCard：精简底部「pcap 导入依赖 tshark。未配置时按 PATH 查找；缺失则 pcap 导入禁用。全本地处理，不上传任何 pcap/规则/样本。」为更简短的一句
in_scope:
  - frontend/src/components/settings/cards/DbPathCard.jsx
  - frontend/src/components/settings/cards/AboutCard.jsx
  - frontend/src/components/settings/cards/TsharkPathCard.jsx
out_of_scope:
  - 不改设置页布局（SettingsView.jsx）
  - 不改 UpdateCard
  - 不改后端
  - 不移除功能性按钮或输入框
acceptance_criteria:
  - DbPathCard 不再有「这是 v1.0.0 在 macOS 上的预期数据库位置」描述
  - AboutCard 不再有长段业务能力描述文案
  - TsharkPathCard 底部说明精简为一句话
  - pnpm build 通过
verification_commands:
  - pnpm --prefix frontend build
files_likely_to_change:
  - frontend/src/components/settings/cards/DbPathCard.jsx
  - frontend/src/components/settings/cards/AboutCard.jsx
  - frontend/src/components/settings/cards/TsharkPathCard.jsx
risks: []
depends_on: []
status: planned
```

## 实现指引

### DbPathCard.jsx

移除：
```jsx
<Text type="secondary">
  这是 v1.0.0 在 macOS 上的预期数据库位置；后续版本将提供动态获取命令。
</Text>
```

保留代码路径 Paragraph + 「在 Finder 中显示」按钮。按钮的 `message.info` 文案可精简为「开发中」。

### AboutCard.jsx

移除或精简：
```jsx
<Text type="secondary">
  数据脱敏 / 校验 / 提取工作台（v1.1.0 处理器原型已接入，业务能力 {DEV_STATUS}）
</Text>
```
可改为简短的一句如 `<Text type="secondary">本地数据处理工作台</Text>` 或直接删除。
移除 `DEV_STATUS` import（如不再使用）。

### TsharkPathCard.jsx

底部文案精简为：
```jsx
<Text type="secondary">pcap 导入依赖 tshark，全本地处理。</Text>
```
移除「未配置时按 PATH 查找；缺失则 pcap 导入禁用」和「不上传任何 pcap/规则/样本」冗余描述。
