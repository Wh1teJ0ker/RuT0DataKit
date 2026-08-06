# v1.0.0 Release Notes

> Git tag：`v1.0.0`（待打）
> Release QA：`qa_passed`（详见 [`docs/qa/versions/1.0.0/QA-审计报告.md`](../../qa/versions/1.0.0/QA-审计报告.md)）
> 状态：待 finalize 发布（draft → finalize 流程）

## 这是什么

RuT0DataKit v1.0.0 是从零重写后的首个里程碑，交付纯框架 shell：一个面向数据安全 CTF 的本地优先数据工作台。数据全程在本地处理，不外发。

## 更新了什么

### 新增

- **四区布局 shell**：上方工具栏（左组 6 项数据操作 + 右组 4 项能力按钮）+ 左侧动态能力面板 + 中央 Workbench + 右侧 AI 面板占位
- **Sheet/Tab 多页工作台**：新建 / 切换 / 关闭 / 双击重命名
- **antd Table**：行复选 + Shift 区间选 / 列显隐 / @dnd-kit 列拖拽 / 50 行分页
- **CSV / XLSX 导入流**：选文件 → detect_format 识别 → 新建 Sheet → 批量写 DB（每 5000 行一批）→ 渲染 Table
- **导出**：CSV / JSON / TXT 模板化（`{字段名}_{值}` → `username_zhangsan`，连接符 `_`/`-`/`:` 等自由填写）
- **SQLite 全量持久**：5 表 + 3 索引 + 幂等迁移
- **Tauri updater 自动检查更新**：Ed25519 签名，无网络静默降级
- **AI IPC 契约占位**：`ai_suggest` / `invoke_ai_op`（返回「开发中」错误，UI 不崩溃）
- **设置页**：updater 检查 / tshark 路径占位 / DB 路径展示 / 关于
- **操作日志**：`import` 记录

### 修复

- ExportModal TXT 格式切换空白崩溃（`{_}` 未声明 JSX 变量 → ReferenceError）→ 已修复并进一步改造为模板化语法
- TXT 导出只导当前页 → 改造为内部拉全表，全量导出

### 变更

- TXT 导出从固定分隔符重写为模板化语法 `{字段名}_{值}`
- XLSX 导出格式从 ExportModal 下拉项移除（保留 XLSX 导入能力）

### 明确不做（推迟至 v1.1+）

脱敏 / 校验 / 提取 / 规则管理 / 搜索 / Tools / PCAP / 真实 AI / 状态高亮 `invalid/masked/hit`

## 验证

- `cargo check --workspace`：Finished
- `cargo test --workspace`：11 passed / 2 ignored
- `pnpm --prefix frontend build`：3078 modules transformed
- 版本一致性：1.0.0（Cargo.toml / src-tauri/Cargo.toml / tauri.conf.json / frontend/package.json）
- Phase 8 GUI 交互验收：A1 四区布局 / A2 能力按钮切换+设置页 / A3 Sheet Tab CRUD / A4 导出 TXT / A5 Table 行复选+列显隐+拖拽 全 pass

## 下载

> 待 finalize 后补充各平台产物链接与 SHA-256 checksums。

---

完整更新日志：[`docs/versions/1.0.0/更新日志.md`](更新日志.md)
QA 审计报告：[`docs/qa/versions/1.0.0/QA-审计报告.md`](../../qa/versions/1.0.0/QA-审计报告.md)
