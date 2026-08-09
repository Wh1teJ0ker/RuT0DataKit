# T45 实现报告 — .log 结构化解析优化

> 任务：T45 — .log 结构化解析优化（格式自动识别 + 多列 + raw_line + fallback）
> 状态：`verified_complete`（cargo fmt/clippy/test 全绿 + pnpm build 绿 + access.log 真实 fixture 集成测试本机验证过）
> 审计轮次：R3（2026-08-09，基于实际命令输出复核）

## 一、改动范围

**仅改 `crates/core/src/datasource/log.rs` 单文件**（重写），不触动 mod.rs / data.rs / model.rs / 前端：

| 文件 | 改动类型 | 说明 |
|---|---|---|
| `crates/core/src/datasource/log.rs` | 重写 | LogFormat 枚举 + 4 正则 OnceLock 缓存 + detect_format probe + split_request + headers/read_all 结构化 + 13 单测 |

文档同步（6 个文件）：

| 文件 | 改动 |
|---|---|
| `docs/02-技术设计文档.md` | §4.10 从「按行单列」改为「格式自动识别 + 结构化多列 + raw_line + fallback」+ 列结构表 + 5 格式说明 + 版本覆盖说明行更新 |
| `docs/versions/1.1.2/更新日志.md` | T45 进度行 + E1/E5 验收更新 + 关键设计决策 + 已知边界 |
| `docs/versions/1.1.2/RELEASE-NOTES.md` | .log 导入段落重写 + 已知限制 + 升级段落 |
| `docs/qa/versions/1.1.2/QA-审计报告.md` | R3 增量复核 + §1 维度表 + §2 需求覆盖 T45 行 + §3 E1/E5 + §4 测试数 + §5 代码质量 + §11 问题记录 + §12 结论 + §13 修复证据 |
| `handoff/TASK-BOARD.md` | T45 行 + DAG 依赖 + E1/E5 更新 + Release QA 结论 |
| `handoff/TASK-T45-HANDOFF.md` | 新建（本文件配套） |

## 二、实现细节

### 2.1 LogFormat 枚举

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFormat { ApacheCombined, ApacheCommon, Syslog, AppLog, Fallback }
```

### 2.2 正则缓存（OnceLock）

4 个 `static OnceLock<Regex>`，首次访问 `get_or_init` 编译一次，后续返回 `&'static Regex`。用 `std::sync::OnceLock`（Rust 1.70+ std 库），无需新增 crate。

| 正则 | 捕获组 | 说明 |
|---|---|---|
| `combined_re()` | 9 | host ident user time request status bytes referer ua |
| `common_re()` | 7 | host ident user time request status bytes |
| `syslog_re()` | 5 | timestamp host process pid(opt) message |
| `app_log_re()` | 4 | timestamp level thread(opt) message |

### 2.3 detect_format probe 探测

```rust
fn detect_format(lines: &[&str]) -> LogFormat {
    let probe = probe_lines(lines, 10);  // 前 10 非空行
    // 对每个正则在 probe 行上匹配计数
    // 选命中数最高的格式（需 ≥1 命中），否则 Fallback
}
```

正则要求严格避免误匹配：Apache 必须有 IP + 时间戳 + 引号请求；syslog 必须 `Mon DD HH:MM:SS`；app log 必须 ISO 日期前缀。

### 2.4 split_request 二次拆分

`request` 字段 `"GET /path HTTP/1.1"` 拆为 (method, url, protocol)；`"-"` 或无法拆分返回三空串。

### 2.5 read_all 结构化解析

按 `detect_format` 结果选列结构，每行用对应正则 `captures()` 提取字段：

- 命中 → 填充结构化字段 + `raw_line` 保留原文
- 未命中 → 各字段留空（HashMap 不插入，import_file 写 NULL）+ `raw_line` 保留原文
- 空行 → `raw_line` 空串，其余字段空
- `body_bytes_sent` 为 `"-"` 时 `replace('-', "0")` 归零

### 2.6 列结构

| 格式 | 列数 | 列名 |
|---|---|---|
| Apache Combined | 12 | remote_host, ident, remote_user, time_local, request_method, request_url, request_protocol, status, body_bytes_sent, http_referer, http_user_agent, raw_line |
| Apache Common | 10 | remote_host, ident, remote_user, time_local, request_method, request_url, request_protocol, status, body_bytes_sent, raw_line |
| Syslog | 6 | timestamp, host, process, pid, message, raw_line |
| AppLog | 5 | timestamp, level, thread, message, raw_line |
| Fallback | 1 | line |

## 三、单测（13 个）

| 测试 | 格式 | 断言 |
|---|---|---|
| `log_reader_reads_lines` | Fallback | 3 行单列 line（既有，保留） |
| `log_reader_preserves_empty_lines` | Fallback | 空行保留为空串（既有，保留） |
| `log_reader_empty_file` | Fallback | 空文件仅表头行（既有，保留） |
| `apache_combined_log_parses_to_columns` | Combined | 12 列 + 字段值正确 + raw_line |
| `apache_log_unparseable_request_keeps_raw` | Combined | request `"-"` → method/url/protocol 空 |
| `apache_common_log_parses_to_columns` | Common | 10 列 + 无 referer/user-agent |
| `syslog_parses_to_columns` | Syslog | 6 列 + pid 提取 |
| `syslog_without_pid` | Syslog | 无 `[pid]` → pid 空 |
| `app_log_parses_to_columns` | AppLog | 5 列 + thread 提取 |
| `app_log_without_thread` | AppLog | 无 `[thread]` → thread 空 |
| `mixed_lines_fallback_to_single_column` | Fallback | 混合非格式行 → 单列 line |
| `raw_line_preserves_original_text` | Combined | raw_line == 原始行（两行） |
| `apache_combined_dash_bytes_normalized_to_zero` | Combined | body_bytes_sent `"-"` → `"0"` |
| `apache_log_with_real_fixture` `#[ignore]` | Combined | access.log 真实 fixture 集成 |

## 四、验证结果

```
cargo fmt --check                        — pass（exit 0）
cargo clippy --workspace -- -D warnings  — pass（exit 0，core + src-tauri 全零警告）
cargo test --workspace                   — 136 passed / 3 ignored / 0 failed
  src-tauri lib: 77 passed / 0 ignored
  core lib:      59 passed / 3 ignored（tshark + pcap fixture + access.log fixture）
cargo test -p ruT0-data-kit-core -- --ignored — 3 passed（含 apache_log_with_real_fixture）
pnpm --prefix frontend build             — pass（3079 modules，2.46s）
```

## 五、scope_deviation 审查

- 改动仅限 `crates/core/src/datasource/log.rs` 单文件（重写）
- 不触动 `mod.rs`（detect_format 工厂 `.log` 分发不变）/ `data.rs`（import_file 已按 headers() 驱动列结构）/ `model.rs`（Record 不变）/ 前端（filter 已含 .log，列结构由后端 headers() 驱动）
- 正则用 `regex` crate（已是 core 依赖，v1.1.0 processor 模块引入）+ `std::sync::OnceLock`（std 库），无新增 crate
- 无新增 SQL，无新增 IPC，无新增依赖

## 六、安全审查

- 无 SQL 改动（LogReader 只读文件 + 返回 Record，不涉及 DB 查询）
- 无凭据改动
- 无网络调用（全本地文件读取）
- 无用户输入传子进程
- 正则 pattern 为编译期常量，不接受用户输入

## 七、已知边界

- `.log` 结构化解析覆盖四种主流格式（Apache Combined/Common、Syslog、通用应用日志），非主流格式回退单列 `line`
- 正则匹配基于行级 probe，不处理跨行多行日志（如 Java stacktrace 多行）—— 推迟 v1.2+
- `read_all` 全量读入内存（与 TxtReader 同模式）—— 大文件内存压力与既有 Reader 一致

## 八、结论

T45 `verified_complete` — LogReader 结构化解析优化已落地，cargo fmt/clippy/test 全绿（136 passed / 3 ignored / 0 failed），pnpm build 绿，access.log 真实 fixture 集成测试本机验证过。改动仅限 log.rs 单文件，无 scope_deviation。文档同步完成（02 设计文档 + 更新日志 + RELEASE-NOTES + QA 报告 + TASK-BOARD + handoff 三件套）。
