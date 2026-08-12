# RuT0DataKit

[English](./README_EN.md)

面向数据安全 CTF 的本地优先数据工作台。在本地完成数据导入、查看、模板化导出，数据不外发。

## 当前状态

- 版本：v1.1.4
- 生命周期：`qa_passed`（T1~T79 全部 `verified_complete`，Phase 7 静态+app 启动实测 + Phase 8 GUI 交互验收全通过；Release QA R1/R2/R3/R4 增量审计通过；分支整合完成）
- v1.1.4 范围：完整数据工作台——四区布局 + Sheet/Tab 工作台 + antd Table + SQLite 全量持久 + Tauri updater 签名 + AI 占位 + 设置入口 + CSV/JSON/TXT 模板化导出 + 脱敏（通用模板脱敏）+ 校验（统一校验页面，多规则 + 双 Tab）+ 提取（5 条提取规则 + 手机号前缀运行时覆盖）+ 搜索/替换 + 列操作 + 撤销/重做 + Base64 列编解码 + MD5/SHA1/SHA256 哈希列变换 + 外部 SQLite .db 文件解析 + 规则管理（17 条内置规则，按 name 排序）。
- 历史 v0.8.0 代码与文档归档于 `release/v0.8.0` 分支，不复用。

## Quick Start

> 前置：Node 20+、pnpm、Rust（通过 `rust-toolchain.toml` 固定版本）、系统依赖见 [Tauri 前置要求](https://v2.tauri.app/start/prerequisites/)。
> 以下命令已在本仓库实测通过（详见 `docs/qa/versions/1.1.4/QA-审计报告.md` §6 + §9.2 + §10）。

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

# 单元测试（持久层 / 数据源 / pcap 探测）
cargo test --workspace

# 前端构建
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
```

以上命令均已验证通过（`cargo check` Finished；`cargo test` 317 passed / 3 ignored；`pnpm build` 3083 modules 转换成功）。

## 项目结构

三层架构，单向依赖（`frontend → src-tauri → crates/core`）：

```
RuT0DataKit/
├── Cargo.toml              # workspace 根（members: crates/core, src-tauri）
├── crates/core/            # 纯逻辑引擎（datasource/ 按格式分子模块 csv/xlsx/json/txt/sql/pcap + model + pcap/）
├── src-tauri/              # Tauri 命令层 + SQLite 持久 + updater + AI 占位
│   ├── src/commands/       # data（import_file/get_sheet_data/ai_suggest/invoke_ai_op）
│   │                       # settings（detect_tshark/load_tshark_path/save_tshark_path）
│   │                       # update（check_update/install_update）
│   └── src/db/             # DbManager(Mutex<Connection>) + 5 表 3 索引 schema + 迁移
├── frontend/               # React shell（antd Layout 四区 + Sheet/Tab + antd Table + 导出 + 设置页）
│   └── src/state/          # constants/factory/reducer/AppContext（React Context，无 prop drilling）
└── docs/                   # 永久产品文档
```

## v1.1.4 能力边界

| 已交付 | 推迟至 v1.2+ |
|---|---|
| 框架 shell：四区布局 + Sheet/Tab 工作台 + antd Table + CSV/XLSX 导入 + SQLite 全量持久 + updater + AI 占位 + 设置页 + 操作日志 | 规则引擎完整 |
| 脱敏：通用模板脱敏（TemplateParams）+ 3 条姓名规则 + rules 表持久化 + RulesPanel | 状态高亮扩展（`invalid/masked/hit`） |
| 校验：统一校验页面（Form.List 多规则 + 一按钮 + 双 Tab 输出）+ 通用校验规则（Generic 变体）+ 地址结构化校验 + 生日分隔符清理 + 行级 7 字段校验 + 特殊符号自定义白名单 + 手机号前缀配置 | tshark + PCAP |
| 提取：5 条提取规则 + 手机号前缀运行时覆盖 + 前缀输入 UX 统一 | 真实 AI（v1.4+） |
| 搜索 / 列操作：`search_cells` / `replace_all` + `parse_column_as_json` / `replace_in_column` | |
| 撤销 / 重做：undo/redo + mask/replace 快照 | |
| Base64 / 哈希：Base64 列编解码 + MD5/SHA1/SHA256 列式哈希变换（可撤销）+ .log 导入 | |
| DB 文件解析：外部 SQLite `.db` / `.sqlite` / `.sqlite3` 文件解析（DbReader） | |
| 规则管理：17 条内置规则（按 name Unicode 码点升序排序） | |
| 导出：CSV / JSON / TXT 模板化（`{字段名}_{值}` → `username_zhangsan`） | |

## 必要链接

- 文档入口：[`docs/`](./docs/)
- 版本规划：[`docs/04-版本标准.md`](./docs/04-版本标准.md)
- v1.1.4 规划需求：[`docs/versions/1.1.4/规划需求.md`](./docs/versions/1.1.4/规划需求.md)
- v1.1.4 更新日志：[`docs/versions/1.1.4/更新日志.md`](./docs/versions/1.1.4/更新日志.md)
- v1.1.4 QA 审计报告：[`docs/qa/versions/1.1.4/QA-审计报告.md`](./docs/qa/versions/1.1.4/QA-审计报告.md)
- 公开发布产物：暂无（v1.1.4 已通过 Release QA，待 finalize 发布）
- License：见仓库根目录 License 文件（待添加）
- 安全问题反馈：通过仓库 Issue 或私下联系维护者，勿在公开 Issue 披露敏感细节
