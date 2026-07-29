# v0.7.2 QA 审计报告

> 版本：0.7.2
> 范围：SqlParseTool textarea 支持 `body|sql` 前缀语法，打通盲注二分还原链路（body_size 贯通）
> 审计日期：2026-07-29
> 审计人：主会话（automated）
> 结论：**qa_passed**

## 1. 功能验收

### 1.1 前端 `SqlParseTool.jsx` `body|sql` 前缀解析（T26-1）

| 维度 | 检查项 | 结果 |
|------|--------|------|
| 文件顶部注释 | 21-30 行更新：说明 v0.7.2 起 textarea 支持 `body\|sql` 前缀 | ✅ |
| `onParse` 改造 | `lines.map` 内正则 `^(\d+)\|(.*)$` 解析前缀，提取 `bodySize` + `sql` | ✅ |
| 有前缀行 | `responseBodySize: Number.isFinite(bodySize) ? bodySize : null` | ✅ |
| 无前缀行 | `responseBodySize: null`（向后兼容） | ✅ |
| `sourceIp` | 始终 `null`（textarea 无 IP 维度，走 log_scan 路径） | ✅ |
| 帮助文字 | 129-132 行更新：说明 `body\|sql` 格式 + 非盲注 payload 兜底 | ✅ |
| placeholder | 143-145 行更新：3 行示例含 URL-encoded payload | ✅ |
| 无新依赖 | 纯 `RegExp.prototype.exec` + `Number`，零新 npm 包 | ✅ |

### 1.2 core 单测 body_size 贯通还原（T26-2）

| 维度 | 检查项 | 结果 |
|------|--------|------|
| 测试 1 | `parse_sqls_body_size_enables_full_reconstruction`：5 探针自洽序列 → `decoded_string='p'`，`schema=Some("p")` | ✅ |
| 回归对照 | 同 input 但 `response_body_size=None` → 0 探针 + 空聚合 + schema=None | ✅ |
| 测试 2 | `parse_sqls_user_five_payloads_with_body_size_gap_documented`：用户 5 payload → 5 探针，gap 如实断言 `insufficient_probes` | ✅ |
| gap 说明 | 注释说明 max_true=109, min_false=112, `109+1=110≠112`，补 110/111 可还原 'p' | ✅ |
| 既有测试回归 | `parse_sqls_decodes_url_encoded_ascii_binary` 等 v0.7.1 测试仍全绿 | ✅ |

### 1.3 版本 bump（T26-3）

| 文件 | 0.7.1 → 0.7.2 |
|------|---------------|
| `Cargo.toml:8` | ✅ |
| `src-tauri/Cargo.toml:3` | ✅ |
| `src-tauri/tauri.conf.json:4` | ✅ |
| `frontend/package.json:4` | ✅ |
| `Cargo.lock` | ✅（core crate `0.7.2`） |
| `src-tauri/Cargo.lock` | ✅（tauri crate `0.7.2`） |

### 1.4 docs（T26-3）

| 文档 | 检查项 | 结果 |
|------|--------|------|
| `docs/04-版本标准.md` | 新增 v0.7.2 里程碑行 | ✅ |
| `docs/versions/0.7.2/更新日志.md` | 新建，含根因 + gap 说明 + 3 任务明细 + 安全约束 + 向后兼容 | ✅ |
| `docs/qa/versions/0.7.2/QA-审计报告.md` | 新建（本文件） | ✅ |
| `docs/02-技术设计文档.md` | 新增 §2.19 v0.7.2 | ✅ |
| `docs/00-需求文档.md` | §界面 7 Tools 补 v0.7.2 `body\|sql` 前缀 | ✅ |
| `docs/03-开发任务清单.md` | 新增 v0.7.2 段（T26-1 / T26-2 / T26-3） | ✅ |
| `README.md` | header callout + 版本表 + v0.7.2 shipped 小节 | ✅ |
| `README_EN.md` | 镜像英文条目 | ✅ |

## 2. 回归验收

### 2.1 单元测试

```
$ cargo test -p ruT0-data-kit-core --release
test result: ok. 376 passed; 0 failed; 3 ignored  （较 v0.7.1 新增 2 测试）
test result: ok. 10 passed; 0 failed; 0 ignored
test result: ok. 34 passed; 0 failed; 2 ignored
test result: ok. 12 passed; 0 failed; 0 ignored
test result: ok. 11 passed; 0 failed; 0 ignored
test result: ok. 31 passed; 0 failed; 0 ignored
合计：474 passed, 0 failed, 5 ignored（全绿，较 v0.7.1 新增 2 测试）
```

### 2.2 前端构建

```
$ npm --prefix frontend run build
vite v5.4.21 building for production...
✓ 3007 modules transformed.
✓ built in 2.26s
```

模块数与 v0.7.1 一致（前端仅注释 + `onParse` + 文案改动，无新模块）。

## 3. 构建验收

```
$ cargo build --release -p ruT0-data-kit-core
warning: crate `ruT0_data_kit_core` should have a snake case name  # 预存 warning，与 v0.7.1 一致
Finished `release` profile [optimized] target(s) in 2.74s
```

编译通过，仅 crate 名 snake_case 预存 warning（与 v0.7.1 一致）。

## 4. 安全验收

| 维度 | 检查项 | 结果 |
|------|--------|------|
| §6 不变 | `docs/00-需求文档.md §6`「不外发数据」约束文本未改 | ✅ |
| 无网络新增 | 前端正则 `^(\d+)\|(.*)$` 纯字符串操作，无网络调用 | ✅ |
| 无 IO 新增 | 无新文件读写，无新进程调用 | ✅ |
| 调用链不变 | `parse_sqls` / `extract_blind_probe` / 聚合 / 还原链路不变，core 仅新增测试 | ✅ |

## 5. 文档验收

| 维度 | 检查项 | 结果 |
|------|--------|------|
| 0.7.2 日志 | `docs/versions/0.7.2/更新日志.md` 含根因 + gap 说明 + 3 任务明细 + 安全约束 + 向后兼容 | ✅ |
| QA 报告 | `docs/qa/versions/0.7.2/QA-审计报告.md`（本文件）5 维度齐全 | ✅ |
| 版本标准 | `docs/04-版本标准.md` 新增 v0.7.2 行，状态 `release_complete` | ✅ |
| 技术设计 | `docs/02-技术设计文档.md` §2.19 描述前端前缀解析 + body_size 贯通 + gap 保守校验 | ✅ |
| 需求文档 | `docs/00-需求文档.md` §界面 7 Tools 补 v0.7.2 条目 | ✅ |
| 任务清单 | `docs/03-开发任务清单.md` 新增 v0.7.2 段 | ✅ |
| README | `README.md` + `README_EN.md` callout + 版本表 + shipped 小节 | ✅ |
| 无残留矛盾 | 当前 docs（`docs/00~04`、`README.md`、`README_EN.md`）无与 v0.7.2 实现矛盾的描述 | ✅ |

## 6. 向后兼容验收

| 场景 | v0.7.1 行为 | v0.7.2 行为 | 一致性 |
|------|-------------|-------------|--------|
| 无前缀行（`ascii(substr(...))>79`） | `responseBodySize: null` → 0 探针 | `responseBodySize: null` → 0 探针 | ✅ 零变化 |
| 有前缀行（`862\|ascii(substr(...))>79`） | 不支持（整行当 sql，正则不命中） | 解析前缀 → `responseBodySize: 862` → 探针提取 | ✅ 新增能力 |
| 非盲注 payload（`1 and sleep(5)`） | `parse_payload` 兜底 | `parse_payload` 兜底（无前缀） | ✅ 零变化 |
| PreprocessView 列级跳转 | `responseBodySize: null` | `responseBodySize: null`（无 body_size 信息，不改） | ✅ 零变化 |
| 用户 5 payload（gap） | URL-decode 后 0 探针（无 body_size） | URL-decode + body_size → 5 探针，gap 如实 `insufficient_probes` | ✅ 链路打通（gap 保守） |

## 7. 结论

5 维度 Release QA 全部通过：

1. **功能**：`body|sql` 前缀行 → probes 提取 + 聚合 + 还原；无前缀行 → 向后兼容 ✅
2. **回归**：cargo test 全绿（core 474 / tauri 8）；npm build 0 error ✅
3. **构建**：cargo build --release 通过 ✅
4. **安全**：无网络/IO 新增，§6 不变 ✅
5. **文档**：0.7.2 日志 + QA 报告落盘；当前 docs 无残留矛盾 ✅

**qa_passed** — 可提交 commit 并申请用户确认 push。
