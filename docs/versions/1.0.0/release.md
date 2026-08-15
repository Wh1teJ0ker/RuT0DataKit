# RuT0DataKit v1.0.0

## Update

- 四区布局 shell：上方工具栏（左组 6 项数据操作 + 右组 4 项能力按钮）+ 左侧动态能力面板 + 中央 Workbench 70% + 右侧 AI 面板占位
- Sheet/Tab 多页工作台 + antd Table：新建/切换/关闭/双击重命名 Tab，行复选 + Shift 区间选择、列显隐 Dropdown、@dnd-kit 列拖拽、状态高亮骨架
- SQLite 全量数据持久：5 表（sheets/columns/cells/operations/session_meta）+ 3 索引 + 迁移框架
- CSV / XLSX 导入流：选文件 → 格式识别 → 新建 Sheet → 分批写 DB（5000/批）→ 渲染 Table
- Tauri updater 自动检查更新：Ed25519 签名验证，构建自动生成 .sig 签名文件 + latest.json 清单
- AI IPC 契约占位：`ai_suggest` / `invoke_ai_op` 空实现（v1.1+ 错误降级），AiPanel 手动触发按钮
- 设置页：updater 检查更新 + tshark 路径占位（v1.3+）+ DB 路径展示 + 关于
- 操作日志：`import` / `column_rename` / `column_reorder` 三类操作记录
- TXT 模板化导出：模板语法 `{字段名}_{值}`，连接符自由填写，每行每列各渲染一行

## Fix

- 修复 ExportModal TXT 格式空白崩溃：`{_}` 被当作未声明 JSX 变量（ReferenceError → 无 ErrorBoundary 兜底 → Modal 子树卸载空白），修正为字符串字面量 `{"{_}"}`
- 修复 TXT 导出只导当前页：改造前依赖 `sheet.rows`（仅当前页数据），大表导出丢失非当前页行；改造后内部 `fetchAllRowsForExport` 拉全表，确保全量导出

## 下载

| 平台 | 文件 |
|---|---|
| Windows x64 | RuT0DataKit_1.0.0_x64-setup.exe / RuT0DataKit_1.0.0_x64_en-US.msi |
| macOS arm64 | RuT0DataKit_1.0.0_aarch64.dmg / RuT0DataKit_aarch64.app.tar.gz |
| Linux x64 | RuT0DataKit_1.0.0_amd64.deb / RuT0DataKit-1.0.0-1.x86_64.rpm / RuT0DataKit_1.0.0_amd64.AppImage |

每个安装包均附带 .sig 签名文件，供 updater 验签。

## 升级

v1.0.0 是从零重写后的首个里程碑，历史 v0.8.0 归档于 `release/v0.8.0` 分支。从 v0.8.0 无法直接升级（全新基线），需全新安装。

完整技术文档见 docs/versions/1.0.0/。
