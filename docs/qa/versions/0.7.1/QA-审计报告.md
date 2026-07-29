# v0.7.1 QA 审计报告

> 版本：0.7.1
> 范围：SQL 解析路径（`parse_sqls` + `detect_sql_blind_features`）自动识别 URL-encoded 输入并解码
> 审计日期：2026-07-25
> 审计人：主会话（automated）
> 结论：**qa_passed**

## 1. 功能验收

### 1.1 新增 helper `looks_like_url_encoded`（`crates/core/src/log/mod.rs`）

| 维度 | 检查项 | 结果 |
|------|--------|------|
| 函数签名 | `pub fn looks_like_url_encoded(input: &str) -> bool` | ✅ |
| 正则 | `(?i)%[0-9a-f]{2}`，`OnceLock` 单例化 | ✅ |
| 零新依赖 | 复用既有 `regex::Regex`（文件顶部已 `use`） | ✅ |
| 单测 1 | `looks_like_url_encoded_pct20` → `true` | ✅ |
| 单测 2 | `looks_like_url_encoded_pct3e_pct23` → `true`（用户报告样本） | ✅ |
| 单测 3 | `looks_like_url_encoded_plain_sql` → `false`（`SELECT * FROM users` / `1' or 1=1#`） | ✅ |
| 单测 4 | `looks_like_url_encoded_bare_pct_no_hex` → `false`（`%` 后非 hex） | ✅ |
| doc 注释 | 含已知边界说明（`%ab` 字面百分号会被识别） | ✅ |

### 1.2 core 调用点 `parse_sqls`（`crates/core/src/tools/sql_parse.rs`）

| 维度 | 检查项 | 结果 |
|------|--------|------|
| 顶部 import | `use crate::log::{looks_like_url_encoded, url_decode_twice};` 新增 | ✅ |
| 条件解码 | `decoded_sql = if looks_like_url_encoded { url_decode_twice } else { clone }` | ✅ |
| `extract_blind_probe` 入参 | 由 `input.sql` 改为 `decoded_sql` | ✅ |
| `parse_payload` 入参 | 由 `input.sql` 改为 `decoded_sql` | ✅ |
| `SqlParseInput.sql` doc | 更新为「v0.7.1 起 caller 可直传 raw」 | ✅ |
| 模块 `//!` | 新增「v0.7.1：自动 URL 解码」小节 | ✅ |
| 单测 1 | `parse_sqls_decodes_url_encoded_ascii_binary`：用户样本 → 1 AsciiBinary 探针，read_target=`database()`，threshold=79 | ✅ |
| 单测 2 | `parse_sqls_decodes_url_encoded_equality`：`%27p%27` → Equality 探针 equality_char=`Some('p')` | ✅ |
| 单测 3 | `parse_sqls_passes_plain_sql_unchanged`：已解码纯文本 → 行为与 v0.7.0 一致（回归保护） | ✅ |
| 单测 4 | `parse_sqls_url_encoded_time_payload`：`1'%20or%20sleep(5)%23` → parsed_payloads 含 time | ✅ |

### 1.3 Tauri 调用点 `detect_sql_blind_features`（`src-tauri/src/commands/log.rs`）

| 维度 | 检查项 | 结果 |
|------|--------|------|
| 顶部 import | `use ruT0_data_kit_core::log::{looks_like_url_encoded, url_decode_twice};` 新增 | ✅ |
| 局部 import | `looks_like_blind_probe` 保留 | ✅ |
| 条件解码 | `probe_target = if looks_like_url_encoded { url_decode_twice } else { clone }` | ✅ |
| samples 收集 | 改为收集 `probe_target`（解码后形态） | ✅ |
| 函数 doc | 补 v0.7.1 自动 URL 解码说明 | ✅ |
| 单测 1 | `detect_sql_blind_features_decodes_url_encoded_cell`：URL-encoded cell → `detected=true`，samples[0] 为解码后形态 | ✅ |
| 单测 2 | `detect_sql_blind_features_plain_cell_unchanged`：已解码 cell → `detected=true`，samples[0] 与原文一致 | ✅ |
| 单测 3 | `detect_sql_blind_features_empty_rows`：空 rows → `detected=false` | ✅ |

### 1.4 版本 bump（T25-2）

| 文件 | 0.7.0 → 0.7.1 |
|------|---------------|
| `Cargo.toml:8` | ✅ |
| `src-tauri/Cargo.toml:3` | ✅ |
| `src-tauri/tauri.conf.json:4` | ✅ |
| `frontend/package.json:4` | ✅ |
| `Cargo.lock` | ✅（core crate `0.7.1`） |
| `src-tauri/Cargo.lock` | ✅（tauri crate `0.7.1`） |

### 1.5 docs（T25-2）

| 文档 | 检查项 | 结果 |
|------|--------|------|
| `docs/04-版本标准.md` | 新增 v0.7.1 里程碑行 | ✅ |
| `docs/versions/0.7.1/更新日志.md` | 新建，含 2 任务明细 + 安全约束 + 向后兼容 | ✅ |
| `docs/qa/versions/0.7.1/QA-审计报告.md` | 新建（本文件） | ✅ |
| `docs/02-技术设计文档.md` | 新增 §2.18 v0.7.1 | ✅ |
| `docs/00-需求文档.md` | §界面 7 Tools 补 v0.7.1 自动 URL 解码 | ✅ |
| `docs/03-开发任务清单.md` | 新增 v0.7.1 段（T25-1 / T25-2） | ✅ |
| `README.md` | header callout + 版本表 + v0.7.1 shipped 小节 | ✅ |
| `README_EN.md` | 镜像英文条目 | ✅ |

## 2. 回归验收

### 2.1 单元测试

```
$ cargo test -p ruT0-data-kit-core --release
test result: ok. 374 passed; 0 failed; 3 ignored
test result: ok. 10 passed; 0 failed; 0 ignored
test result: ok. 34 passed; 0 failed; 2 ignored
test result: ok. 12 passed; 0 failed; 0 ignored
test result: ok. 11 passed; 0 failed; 0 ignored
test result: ok. 31 passed; 0 failed; 0 ignored
合计：472 passed, 0 failed, 5 ignored（全绿，较 v0.7.0 新增 8 测试）
```

```
$ cargo test --manifest-path src-tauri/Cargo.toml
test result: ok. 8 passed; 0 failed; 0 ignored（较 v0.7.0 新增 3 测试）
```

### 2.2 前端构建（回归保护，前端无改动）

```
$ npm --prefix frontend run build
vite v5.4.21 building for production...
✓ 3007 modules transformed.
✓ built in 2.30s
```

模块数与 v0.7.0 一致，前端零改动。

## 3. 构建验收

```
$ cargo build --release -p ruT0-data-kit-core
warning: crate `ruT0_data_kit_core` should have a snake case name  # 预存 warning，与 v0.7.0 一致
Finished `release` profile [optimized] target(s) in 3.43s
```

编译通过，仅 crate 名 snake_case 预存 warning（与 v0.7.0 一致）。

## 4. 安全验收

| 维度 | 检查项 | 结果 |
|------|--------|------|
| §6 不变 | `docs/00-需求文档.md §6`「不外发数据」约束文本未改 | ✅ |
| 无网络新增 | `looks_like_url_encoded` 纯正则匹配，`url_decode_twice` 纯字节操作，无网络调用 | ✅ |
| 无 IO 新增 | 无新文件读写，无新进程调用 | ✅ |
| 调用链不变 | `parse_sqls` / `detect_sql_blind_features` 仍是本地 Tauri 命令，前端→Tauri→core 链路不变 | ✅ |

## 5. 文档验收

| 维度 | 检查项 | 结果 |
|------|--------|------|
| 0.7.1 日志 | `docs/versions/0.7.1/更新日志.md` 含根因 + 双调用点明细 + 安全约束 + 向后兼容 | ✅ |
| QA 报告 | `docs/qa/versions/0.7.1/QA-审计报告.md`（本文件）5 维度齐全 | ✅ |
| 版本标准 | `docs/04-版本标准.md` 新增 v0.7.1 行，状态 `release_complete` | ✅ |
| 技术设计 | `docs/02-技术设计文档.md` §2.18 描述 helper + 双调用点 + 边界 | ✅ |
| 需求文档 | `docs/00-需求文档.md` §界面 7 Tools 补 v0.7.1 条目 | ✅ |
| 任务清单 | `docs/03-开发任务清单.md` 新增 v0.7.1 段 | ✅ |
| README | `README.md` + `README_EN.md` callout + 版本表 + shipped 小节 | ✅ |
| 无残留矛盾 | 当前 docs（`docs/00~04`、`README.md`、`README_EN.md`）无与 v0.7.1 实现矛盾的描述 | ✅ |

## 6. 向后兼容验收

| 场景 | v0.7.0 行为 | v0.7.1 行为 | 一致性 |
|------|-------------|-------------|--------|
| 已解码纯文本 SQL（`1' or ascii(...)>79#`） | 喂探针正则 → 命中 | `looks_like_url_encoded` 返回 `false` → 喂探针正则 → 命中 | ✅ 零变化 |
| URL-encoded SQL（`1'%20or%20ascii(...)%3E79%23`） | 喂探针正则 → 0 探针（BUG） | 解码后喂探针正则 → 命中（修复） | ✅ 修复 |
| `LIKE '%ab%'` 字面百分号 | 喂探针正则（不命中盲注） | 识别为编码 → 解码（0xAB → replacement char）→ 喂探针正则（仍不命中盲注） | ✅ 不影响盲注结果 |
| `detect_sql_blind_features` samples 形态 | 收集 raw cell | 收集解码后形态 | ⚠️ 微调（下游 `SqlParseTool` 二次过 `looks_like_url_encoded` 返回 `false`，无双重解码问题） |

## 7. 结论

5 维度 Release QA 全部通过：

1. **功能**：URL-encoded SQLi payload 可解析（用户报告样本修复）；plain SQL 行为零变化 ✅
2. **回归**：cargo test 全绿（core 472 / tauri 8）；npm build 0 error ✅
3. **构建**：cargo build --release 通过 ✅
4. **安全**：无网络/IO 新增，§6 不变 ✅
5. **文档**：0.7.1 日志 + QA 报告落盘；当前 docs 无残留矛盾 ✅

**qa_passed** — 可提交 commit 并申请用户确认 push。
