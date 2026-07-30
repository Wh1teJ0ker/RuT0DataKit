# RuT0DataKit

[English](./README_EN.md)

面向数据安全 CTF 的本地优先数据工作台。在本地完成数据导入、脱敏、校验、提取、规则匹配与导出，数据不外发。

## 当前状态

- 版本：v1.0.0（框架阶段）
- 生命周期：开发中（任务 T1~T8 已验证完成，待端到端 + Release QA）
- v1.0.0 范围：纯框架 shell——四区布局 + Sheet/Tab 工作台 + antd Table + SQLite 全量持久 + Tauri updater 签名 + AI 占位 + 设置入口。业务能力（脱敏 / 校验 / 提取 / 规则 / 搜索 / Tools / PCAP / 真实 AI）推迟至 v1.1+。
- 历史 v0.8.0 代码与文档归档于 `release/v0.8.0` 分支，不复用。

## Quick Start

> 前置：Node 22+、pnpm、Rust（通过 `rust-toolchain.toml` 固定版本）、系统依赖见 [Tauri 前置要求](https://v2.tauri.app/start/prerequisites/)。
> 以下命令已在本仓库实测通过（详见 `docs/qa/versions/1.0.0/QA-审计报告.md` §6）。

```sh
# 1. 获取源码
git clone <repo-url> && cd RuT0DataKit

# 2. 安装前端依赖
pnpm --prefix frontend install

# 3. 本地开发启动（首次会编译 SQLite bundled，耗时较长）
cargo tauri dev
```

## 基本验证

```sh
# Rust 编译检查
cargo check --workspace

# 单元测试（持久层等）
cargo test --workspace

# 前端构建
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
```

以上命令均已验证通过（`cargo check` Finished；`cargo test` 9 db + 3 datasource 全 PASS；`pnpm build` 3069 modules 转换成功）。

## 项目结构

三层架构，单向依赖（`frontend → src-tauri → crates/core`）：

```
RuT0DataKit/
├── Cargo.toml              # workspace 根（members: crates/core, src-tauri）
├── crates/core/            # 纯逻辑引擎（v1.0.0 骨架：datasource CSV/XLSX 解析 + model + processor）
├── src-tauri/              # Tauri 命令层 + SQLite 持久 + updater + AI 占位
│   ├── src/commands.rs     # import_file / get_sheet_data / check_update / install_update / ai_suggest / invoke_ai_op
│   └── src/db/             # DbManager(Mutex<Connection>) + 5 表 3 索引 schema + 迁移
├── frontend/               # React shell（antd Layout 四区 + Sheet/Tab + antd Table + 设置页）
└── docs/                   # 永久产品文档
```

## v1.0.0 能力边界

| 已交付 | 推迟至 v1.1+ |
|---|---|
| 四区布局 shell（上方工具栏 + 左侧能力面板 + 中央 Workbench + 右侧 AI 面板） | 脱敏 |
| Sheet/Tab 工作台（新建 / 切换 / 关闭 / 重命名） | 校验 |
| antd Table（行复选 + 区间选 / 列显隐 / 列拖拽 / 50 行分页） | 提取 |
| CSV / XLSX 导入流（detect_format → DB cells → Table） | 规则管理 |
| SQLite 全量持久（5 表 + 3 索引 + 迁移幂等） | 搜索 |
| Tauri updater 自动检查更新（Ed25519 签名，无网静默降级） | Tools |
| AI IPC 契约占位（`ai_suggest` / `invoke_ai_op` 返回 v1.1+ 错误） | PCAP |
| 设置页（updater 检查 / tshark 路径占位 / DB 路径 / 关于） | 真实 AI（v1.4+） |
| 操作日志（`import` 记录） | 状态高亮 `invalid/masked/hit` |

## 必要链接

- 文档入口：[`docs/`](./docs/)
- 版本规划：[`docs/04-版本标准.md`](./docs/04-版本标准.md)
- v1.0.0 规划需求：[`docs/versions/1.0.0/规划需求.md`](./docs/versions/1.0.0/规划需求.md)
- v1.0.0 更新日志：[`docs/versions/1.0.0/更新日志.md`](./docs/versions/1.0.0/更新日志.md)
- v1.0.0 QA 审计报告：[`docs/qa/versions/1.0.0/QA-审计报告.md`](./docs/qa/versions/1.0.0/QA-审计报告.md)
- 公开发布产物：当前无（v1.0.0 尚未发布）
- License：见仓库根目录 License 文件（待添加）
- 安全问题反馈：通过仓库 Issue 或私下联系维护者，勿在公开 Issue 披露敏感细节
