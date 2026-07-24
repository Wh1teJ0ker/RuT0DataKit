# v0.6.1 Release QA 审计报告

> 小功能版本发布前全局 QA 审计。依据 `orchestrator-workflow` 技能「发布前 QA 门禁」要求，覆盖需求覆盖、端到端流程、构建与测试、代码质量、文档一致性五维度。
>
> 本版本范围：4 视图统一加醒目总行数统计 + RulesView 规则列表默认全部收起。纯前端 UI 调整，未触及后端。

## §0 审计结论

`qa_passed` — 2 项功能任务（T16-1/T16-2）+ 版本同步（T16-3）全部落地，4 视图总行数统计齐备，RulesView 默认收起，npm build 绿，cargo test 全绿（491 passed），4 构建目标 0 error，5 处 manifest 版本号同步 0.6.1，docs 一致，无阻塞项。

## §1 任务对齐

| # | 任务 | 优先级 | 落地证据 | 状态 |
|---|------|--------|----------|------|
| 1 | T16-1 4 视图统一加醒目总行数 | P0 | MaskView/ValidateView/PreprocessView/ExportView 均加 `<Tag color="blue" fontWeight=600>` 总行数统计，reviewer pass | ✅ |
| 2 | T16-2 RulesView 默认全部收起 | P0 | `useState([])` + useEffect `setExpandedKeys([])`，reviewer pass | ✅ |
| 3 | T16-3 版本号 + docs 同步 | P1 | 5 manifest 0.6.1 + 更新日志回填 + QA 报告 | ✅ |

## §2 代码审计

### §2.1 总行数显示（T16-1）

| 视图 | 原始数据行数 | 处理后行数 | 降级 | 视觉权重 |
|------|-------------|-----------|------|----------|
| MaskView | `rowCount`（Tag blue） | `maskedSummary.total`「脱敏后 N 行」（未脱敏「-」） | null→「-」 | Tag fontWeight=600 ✅ |
| ValidateView | `rowCount` | `validateResult.summary.total` + 合法/非法 | 未校验不渲染合法/非法 | Tag fontWeight=600 ✅ |
| PreprocessView | `records.rowCount`「总行数 N 行」 | — | — | Tag fontWeight=600 ✅ |
| ExportView | 按 exportSource 取（raw/records→rowCount, masked→maskedSummary.total, validate→validateResult.summary.total） | — | null→「-」 | Tag fontWeight=600 ✅ |

- 不新增 state action，全部复用现有字段。
- 保留既有「共 N 行 / 显示前 N 行」12px 灰字预览截断提示。
- ExportView `sourceTotalRows` 枚举与 state.js exportSource 取值对齐（raw 作 records 别名归一）。

### §2.2 RulesView 默认收起（T16-2）

| 检查项 | 结果 |
|--------|------|
| `useState([])` 默认收起 | `RulesView.jsx:293` ✅ |
| useEffect `setExpandedKeys([])` | `RulesView.jsx:296` ✅ |
| maskKeys/filteredMaskKeys/toggleAll/allExpanded 保留 | ✅ |
| 「全部展开/收起」按钮保留，初始 allExpanded=false | ✅ |
| 组件头注释更新（默认收起） | ✅ |

### §2.3 scope 合规

- T16-1 仅改 4 个 in_scope 文件（MaskView/ValidateView/PreprocessView/ExportView）。
- T16-2 仅改 RulesView.jsx。
- 未触及 state.js / crates/core / src-tauri / 后端命令 / 加密模块。

## §3 测试审计

```
$ cargo test --workspace
test result: ok. 393 passed; 0 failed; 3 ignored   (lib unittests)
test result: ok. 10 passed; 0 failed; 0 ignored     (integration log_test)
test result: ok. 34 passed; 0 failed; 2 ignored     (integration log_scan_test)
test result: ok. 12 passed; 0 failed; 0 ignored     (integration blind_aggregator_test)
test result: ok. 11 passed; 0 failed; 0 ignored     (integration logsign_test)
test result: ok. 31 passed; 0 failed; 0 ignored     (integration e2e)
```

合计 **491 passed, 0 failed, 5 ignored**，全绿。与 v0.6.0 基线一致（T16 纯前端，不动后端）。

## §4 构建审计

| 构建目标 | 命令 | 结果 |
|----------|------|------|
| core lib | `cargo build -p ruT0-data-kit-core` | 0 error，2 warning（历史遗留 crate 名） |
| src-tauri binary | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error |
| frontend | `npm --prefix frontend run build` | vite build 3008 modules，0 error |

## §5 文档一致性

| 检查项 | 结果 |
|--------|------|
| 4 处 manifest 版本号（+ core workspace 继承） | 全部 0.6.1 ✅ |
| `docs/04-版本标准.md` 里程碑行 | v0.6.1 状态 `release_complete` ✅ |
| `docs/versions/0.6.1/更新日志.md` | 回填完毕，状态 `release_complete` ✅ |
| QA 报告 | 本文件 ✅ |
| 安全约束 §6 | 「不外发数据：全本地处理」保留，v0.6.1 不变 ✅ |

## §6 安全审计

- 纯前端 UI 调整，未触及后端 / 加密 / IO / 网络逻辑。
- `grep -rn 'reqwest\|http::\|net::' crates/core/src/tools/ src-tauri/src/commands/encrypt.rs` 0 命中（沿用 v0.6.0 基线）。

## §7 阻塞项

无。

## §8 结论

`qa_passed` — v0.6.1 全部任务落地，测试 / 构建 / 文档 / 安全五维度通过，可标记 `release_complete`。
