```yaml
implemented_changes:
  - frontend/src/components/settings/cards/DbPathCard.jsx：删除冗余 Text 说明「这是 v1.0.0 在 macOS 上的预期数据库位置…」；按钮 message.info 文案精简为「在 Finder 中显示{DEV_STATUS}」（去掉引号包裹）。保留 DB_PATH Paragraph + copyable + 按钮。
  - frontend/src/components/settings/cards/AboutCard.jsx：长文案「数据脱敏 / 校验 / 提取工作台（v1.1.0 处理器原型已接入，业务能力 {DEV_STATUS}）」替换为简短「本地数据处理工作台」；删除未再使用的 DEV_STATUS import，仅保留 APP_VERSION。
  - frontend/src/components/settings/cards/TsharkPathCard.jsx：底部说明精简为「pcap 导入依赖 tshark，全本地处理。」，移除「未配置时按 PATH 查找；缺失则 pcap 导入禁用」和「不上传任何 pcap/规则/样本」冗余描述。
verification_run:
  - pnpm --prefix frontend build
  - git diff --stat
verification_results:
  - pnpm --prefix frontend build：通过，vite build 成功，✓ built in 2.50s（仅 chunk size 警告，非新增）。
  - git diff --stat：本任务 3 个文件改动（AboutCard 6/2、DbPathCard 5/1、TsharkPathCard 5/1）。另有一个未追踪的 AiPanel.jsx 改动属 T40 范围，不在本任务内，未触碰。
docs_updated:
  - none（仅 UI 文案精简，无行为/用法变化）
commit_summary:
  - none（按交接要求不 commit、不 push）
reported_status:
  - verified_complete
scope_deviation:
  - none（严格限制在 3 个 settings 卡片文件内；AiPanel.jsx 的既有未提交改动来自其他任务，本任务未触碰）
```
