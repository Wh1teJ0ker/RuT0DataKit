# v1.0.0 文档体系规划

## 已确认决策（来自用户）

| 维度 | 决策 |
|---|---|
| 重构范围 | **完全从零重写**（Rust core + 前端 + Tauri 层全部重写，不复用 v0.8.0 代码） |
| v1.0.0 边界 | **纯框架 shell**——四区布局 + Sheet/Tab 工作台 + SQLite 全量持久 + Tauri updater 签名 + AI 占位；左侧 7 项操作点击提示「v1.1+」，功能从 v1.1 起逐项完整重构 |
| 数据库 | **SQLite (rusqlite bundled)**，持久化全量数据 + 操作日志 |
| 自动更新 | **Tauri updater 插件（带签名）**——TAURI_SIGNING_PRIVATE_KEY + updater JSON |
| 布局 | 上方工具栏（导入/导出/格式/撤销/列操作/运行）+ 左侧操作栏（脱敏/校验/提取/规则/搜索/Tools/设置）+ 中央 Workbench 70%（Sheet/Tab 多页 + antd Table 选区/列选/列序/状态高亮）+ 右侧 AI 面板（占位 + 预留 IPC 契约 ai_suggest/invoke_ai_op 空实现） |

## 文档清单（10 份）

### A. 项目入口（2 份，根目录）

#### 1. `README.md`（中文主版）
- 一句话定位：面向数据安全 CTF 的本地优先数据工作台
- 当前状态：v1.0.0 框架阶段（开发中），历史 v0.8.0 归档于 `release/v0.8.0`
- Quick Start：`pnpm install` / `cargo tauri dev`
- 基本验证：`cargo test --workspace` + `pnpm build`
- 必要链接：docs/ 入口、Release 页、License、安全反馈
- 互链 README_EN.md

#### 2. `README_EN.md`（英文版）
- 与中文版事实一致

### B. 核心文档（5 份，`docs/`）

#### 3. `docs/00-需求文档.md`
- §1 产品定位：数据安全 CTF 快速分析工作台，本地优先
- §2 目标用户与场景
- §3 v1.0.0 范围（**MVP = 纯框架**）：
  - 四区布局 shell（上方工具栏 / 左侧操作栏 / 中央 Workbench 70% / 右侧 AI 面板占位）
  - Sheet/Tab 多页工作台 + antd Table（选区 / 列选 / 列序 / 状态高亮）
  - SQLite 全量数据持久 + 操作日志
  - Tauri updater 自动检查更新（带签名）
  - AI 面板占位 + 预留 IPC 契约（ai_suggest / invoke_ai_op 空实现）
  - 设置入口可用
- §4 非目标（明确不做）：7 项功能实现（脱敏/校验/提取/规则/搜索/Tools/日志/PCAP）推迟 v1.1+；CLI 形态；云同步；真实 AI 能力
- §5 安全约束：全本地处理，数据不外发（updater 仅拉版本元数据 + 签名产物，不传用户数据）
- §6 v1.0.0 验收标准

#### 4. `docs/01-页面与交互说明.md`
- §1 主界面结构（四区 ASCII 图示 + 占比）
  - 上方工具栏：导入 / 导出 / 格式 / 撤销 / 列操作 / 运行（高频）
  - 左侧操作栏：脱敏 / 校验 / 提取 / 规则 / 搜索 / Tools / 设置（中频，v1.0.0 点击提示「v1.1+」，仅设置可用）
  - 中央 Workbench（70%）：Sheet/Tab 多页 + antd Table 选区/列选/列序/状态高亮
  - 右侧 AI 面板：占位 + 预留 IPC 契约
- §2 Sheet/Tab 多页交互（新建/切换/关闭/重命名）
- §3 antd Table 交互（选区框选 / 列 checkbox 选 / 列拖拽排序 / 状态高亮规则）
- §4 导入流（选文件 → 识别 → 新建 Sheet → 写 SQLite → 渲染 Table）
- §5 状态：空状态 / 加载 / 错误 / 未保存
- §6 设置页（updater 检查、tshark 路径预留、DB 路径）

#### 5. `docs/02-技术设计文档.md`
- §1 技术栈：Rust + Tauri v2 + React 18 + antd 5 + rusqlite(bundled)
- §2 架构分层：
  - `frontend/`：React shell（四区布局 + Sheet/Tab + antd Table + AI 占位组件）
  - `src-tauri/`：Tauri 命令层（IPC 契约 + updater 插件 + SQLite 管理器）
  - `crates/core/`：从零重写的处理引擎（v1.0.0 仅骨架 + trait 定义，实现推迟）
- §3 SQLite 表结构：
  - `sessions`（id, name, source_path, source_type, row_count, created_at, updated_at）
  - `sheets`（id, session_id, name, position, created_at）
  - `cells`（sheet_id, row_idx, col_idx, value）—— 全量 cell 持久
  - `operations`（id, sheet_id, kind, params_json, result_snapshot_json, created_at）—— 操作日志
  - `app_settings`（key, value）
- §4 Tauri IPC 契约（v1.0.0 骨架）：
  - 数据：`import_file` / `get_sheet_data` / `list_sessions` / `open_session`
  - 选区/列：`set_selection` / `reorder_columns` / `rename_column`
  - updater：`check_update` / `install_update`（委托 Tauri updater 插件）
  - AI 占位：`ai_suggest` / `invoke_ai_op`（空实现，返回 `unimplemented`）
  - 设置：`get_setting` / `set_setting`
- §5 Tauri updater 签名配置（TAURI_SIGNING_PRIVATE_KEY secret + updater JSON 上传）
- §6 状态结构（前端 reducer：sheets[] / activeSheetId / selection / columnOrder / AI 面板状态）
- §7 错误处理与边界

#### 6. `docs/03-开发任务清单.md`
- v1.0.0 任务 DAG（串行/并行依赖）：
  - T1 工作区脚手架（Cargo workspace + frontend + src-tauri 初始化 + 版本号 1.0.0）
  - T2 四区布局 shell（上方工具栏 + 左侧操作栏 + 中央 Workbench + 右侧 AI 占位，antd Layout）
  - T3 Sheet/Tab 多页 + antd Table（选区/列选/列序/状态高亮）
  - T4 SQLite 持久层（rusqlite bundled + 表结构 + 迁移 + 管理器 trait）
  - T5 导入流（选文件 → 识别 → 新建 Sheet → 写 DB → 渲染 Table，CSV/XLSX 优先）
  - T6 Tauri updater 插件（签名密钥 + updater JSON + 检查/安装命令）
  - T7 AI 占位 IPC 契约（ai_suggest / invoke_ai_op 空实现）
  - T8 设置页 + 7 项操作占位提示
  - T9 双语 README + docs + QA
- 每任务含：目标 / 前置依赖 / 实现要点 / 交付物 / 验收标准 / 验证方式

#### 7. `docs/04-版本标准.md`
- 版本号语义（v1.x 系列 = 框架与能力渐进重构）
- 里程碑索引表：
  - v1.0.0 | 纯框架 shell（四区布局 + Sheet/Tab + SQLite 全量 + updater 签名 + AI 占位）| planned
  - v1.1.0+ | 7 项能力逐项重构（脱敏/校验/提取/规则/搜索/Tools/日志/PCAP）| planned
- 版本状态口径 + 发布门禁

### C. v1.0.0 版本子文档（2 份）

#### 8. `docs/versions/1.0.0/规划需求.md`
- v1.0.0 范围、目标、任务清单（与 03 对齐）

#### 9. `docs/versions/1.0.0/更新日志.md`
- v1.0.0 进度表（与 handoff/TASK-BOARD.md 对齐，初始全 planned）

### D. v1.0.0 QA 规划（1 份）

#### 10. `docs/qa/versions/1.0.0/QA-审计报告.md`
- 5 维度审计框架（功能 / 回归 / 构建 / 安全 / 文档），初始结论 `planned`

## 写作原则

- 先边界后细节，先 MVP 后扩展
- v1.0.0 所有文档围绕「纯框架」边界组织，不提前写功能实现
- 7 项操作在 01/02 里写明「v1.1+ 重构」，避免范围蔓延
- SQLite 表结构、IPC 契约、updater 配置写到可指导编码的粒度
- 安全约束明确：updater 只拉版本元数据 + 签名产物，不传用户数据

## 执行方式

- 全部文档由主会话直接落盘（文档规划阶段，不分派 coder）
- 落盘后 `git add docs/ README.md README_EN.md` + commit（不自动 push）
- 落盘后进入 orchestrator Phase 1：基于 03 任务清单拆 TASK-BOARD + HANDOFF，分派 coder 实现

## 不做（本阶段）

- 不写任何代码（仅文档）
- 不创建 Cargo.toml / frontend / src-tauri（留给 T1 任务）
- 不生成 updater 签名密钥（留给 T6 任务，需用户参与）
- 不 push（commit 后等用户确认）
