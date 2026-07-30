# RuT0DataKit

[English](./README_EN.md)

面向数据安全 CTF 的本地优先数据工作台。在本地完成数据导入、脱敏、校验、提取、规则匹配与导出，数据不外发。

## 当前状态

- 版本：v1.0.0（框架阶段，`planned`，开发中，尚未发布）
- 生命周期：开发中
- v1.0.0 范围：纯框架 shell——四区布局 + Sheet/Tab 工作台 + antd Table + SQLite 全量持久 + Tauri updater 签名 + AI 占位 + 设置入口。业务能力（脱敏 / 校验 / 提取 / 规则 / 搜索 / Tools / PCAP / 真实 AI）推迟至 v1.1+。
- 历史 v0.8.0 代码与文档归档于 `release/v0.8.0` 分支，不复用。

## Quick Start

> 前置：Node 22+、pnpm、Rust（通过 `rust-toolchain.toml` 固定版本）、系统依赖见 [Tauri 前置要求](https://v2.tauri.app/start/prerequisites/)。

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

# 前端构建
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build

# 单元测试（持久层等）
cargo test --workspace
```

> 以上命令随实现进度陆续可用；v1.0.0 尚处规划阶段，部分命令可能因工程未初始化而失败，以实际任务进度为准。

## 必要链接

- 文档入口：[`docs/`](./docs/)
- 版本规划：[`docs/04-版本标准.md`](./docs/04-版本标准.md)
- v1.0.0 规划需求：[`docs/versions/1.0.0/规划需求.md`](./docs/versions/1.0.0/规划需求.md)
- 公开发布产物：当前无（v1.0.0 尚未发布）
- License：见仓库根目录 License 文件（待添加）
- 安全问题反馈：通过仓库 Issue 或私下联系维护者，勿在公开 Issue 披露敏感细节
