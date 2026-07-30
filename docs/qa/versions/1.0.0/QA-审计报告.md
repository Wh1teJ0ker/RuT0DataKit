# v1.0.0 QA 审计报告

> 版本：1.0.0
> 审计类型：Release 前全局审计
> 审计依据：[`docs/00-需求文档.md`](../../../00-需求文档.md) §6 验收标准 + [`docs/03-开发任务清单.md`](../../../03-开发任务清单.md) 各任务验收 + [`docs/04-版本标准.md`](../../../04-版本标准.md) §4 发布门禁
> 结论：`planned`（尚未开始审计，任务进入 `done_e2e` 后填充本报告）

## 1. 审计维度与结论

| 维度 | 结论 | 说明 |
|---|---|---|
| 功能完整性 | `pending` | 待 T1-T8 全部 `verified_complete` 后审计 |
| 回归与端到端 | `pending` | 待 `done_e2e` 后审计 |
| 构建与产物 | `pending` | 待四目标矩阵构建 + checksums 验证 |
| 安全 | `pending` | 待 updater 签名验签 + 数据本地性审计 |
| 文档 | `pending` | 待双语 README + 版本文档一致性审计 |

## 2. 功能完整性审计

依据 [`规划需求.md`](../../versions/1.0.0/规划需求.md) §3 的 11 项验收标准：

| # | 验收项 | 状态 | 证据 |
|---|---|---|---|
| 1 | `cargo check --workspace` 通过，`cargo tauri dev` 可启动 | `pending` | |
| 2 | 四区布局可见，占比符合 01 文档 §1。上方工具栏左组 6 项数据操作 + 右组 4 项能力按钮可见 | `pending` | |
| 3 | 右组能力按钮点击后左侧能力面板切换到对应能力面板（占位提示「v1.1+ 释放」）；左组禁用项点击弹 `v1.1+` 提示；左侧面板底部「⚙ 设置」始终可见，点击进入设置页 | `pending` | |
| 4 | 可新建 / 切换 / 关闭 / 重命名 Sheet Tab | `pending` | |
| 5 | antd Table 支持行复选 + 区间选择、列 checkbox 显隐、列拖拽排序 | `pending` | |
| 6 | SQLite 5 表 3 索引存在，重复启动不重复建表，`schema_version` 正确 | `pending` | |
| 7 | 导入 1000 行 CSV / XLSX：Sheet 新建、Table 渲染首页、分页可翻 | `pending` | |
| 8 | 重启应用后通过「打开历史 Session」可重新加载该 Sheet 全量数据 | `pending` | |
| 9 | `operations` 表有对应 `import` 记录 | `pending` | |
| 10 | `check_update` 无新版本返回 `available=false`；无网络静默降级；签名校验失败拒绝安装 | `pending` | |
| 11 | 前端调用 `aiSuggest` 得到 `v1.1+` 错误文案，UI 不崩溃 | `pending` | |

## 3. 回归与端到端审计

| 场景 | 状态 | 证据 |
|---|---|---|
| 端到端主流程：导入 → 编辑列序 → 重命名列 → 关闭 → 重启 → 恢复 | `pending` | |
| 大表导入（5 万行）性能与分页响应 | `pending` | |
| 错误降级：DB 损坏备份重建 | `pending` | |
| updater 无网络静默降级 | `pending` | |
| AI 占位调用不崩溃 | `pending` | |

## 4. 构建与产物审计

| 项 | 状态 | 证据 |
|---|---|---|
| 四目标矩阵构建成功（Linux x64 / macOS arm64 / macOS x64 / Windows x64） | `pending` | |
| 产物命名符合 `RuT0DataKit_<VERSION>_<PLATFORM>_<ARCH>.<EXT>` | `pending` | |
| SHA-256 checksums 文件生成并上传 | `pending` | |
| updater JSON（`latest.json`）随 Release 上传 | `pending` | |
| 版本一致性：Git tag `v1.0.0` == `tauri.conf.json` version == 产物命名 | `pending` | |

## 5. 安全审计

| 项 | 状态 | 证据 |
|---|---|---|
| 全本地处理，用户数据不外发（updater 除外） | `pending` | |
| updater 仅拉版本元数据 + 签名产物，不传用户数据 | `pending` | |
| Ed25519 签名密钥通过 GitHub Secret 注入，不落盘仓库 | `pending` | |
| 签名验签失败拒绝安装 | `pending` | |
| `endpoints` 指向 GitHub Release（HTTPS），不自建服务器 | `pending` | |
| 嵌套 struct 全部带 `#[serde(rename_all = "camelCase")]`（grep 核验） | `pending` | |

## 6. 文档审计

| 项 | 状态 | 证据 |
|---|---|---|
| 双语 README 存在且互链 | `pending` | |
| README Quick Start 在干净环境可复现 | `pending` | |
| `docs/versions/1.0.0/更新日志.md` 进度表与 `handoff/TASK-BOARD.md` 一致 | `pending` | |
| `docs/04-版本标准.md` 里程碑索引状态正确 | `pending` | |
| 文档间无术语 / 范围冲突（无 v1.1+ 能力被误写成 v1.0.0 已交付） | `pending` | |
| README 不含 `{{PLACEHOLDER}}` 残留 | `pending` | |

## 7. 问题记录

| 严重度 | 问题 | 修复任务 | 状态 |
|---|---|---|---|
| — | 暂无（尚未开始审计） | — | — |

严重度口径：`critical`（阻塞发布）/ `major`（需回流修复）/ `minor`（可带病发布但记录）/ `info`（仅记录）。

## 8. 审计结论

`planned` — v1.0.0 尚未进入审计阶段。待 T1-T8 全部 `verified_complete` 且端到端验收通过（`done_e2e`）后，填充本报告 5 维度审计项，结论推进至 `qa_passed` 或 `qa_failed`。

**发布前置门禁**（[`04-版本标准.md`](../../../04-版本标准.md) §4）全部满足前，禁止 finalize。
