```yaml
task_id: T45
goal: |
  优化 v1.1.2 的 .log 解析：从「按行单列」升级为「格式自动识别 + 结构化多列解析 + raw_line 保留」。
  支持四种格式：Apache Combined/Common、Syslog（RFC 3164）、通用应用日志；未识别格式回退单列 line。
in_scope:
  - crates/core/src/datasource/log.rs（重写 LogReader：LogFormat 枚举 + 4 正则 OnceLock 缓存 + detect_format + split_request + headers/read_all 结构化）
out_of_scope:
  - 不改 crates/core/src/datasource/mod.rs（detect_format 工厂 .log 分发不变）
  - 不改 src-tauri/src/commands/data.rs（import_file 已按 headers() 驱动列结构）
  - 不改 crates/core/src/model.rs（Record 模型不变）
  - 不改前端（filter 已含 .log，import_file 按后端 headers 渲染列）
  - 不改其他 Reader
acceptance_criteria:
  - Apache Combined 行解析出 12 列（11 结构化 + raw_line），request 拆 method/url/protocol，body_bytes_sent 为 "-" 时归零
  - Apache Common 行解析出 10 列（9 结构化 + raw_line，无 referer/user-agent）
  - Syslog 行解析出 6 列（timestamp/host/process/pid/message/raw_line），pid 可选
  - 通用应用日志行解析出 5 列（timestamp/level/thread/message/raw_line），thread 可选
  - 未识别格式回退单列 line（既有 3 测试保留通过）
  - 解析失败的行各结构化字段留空，raw_line 保留原文
  - 空行保留为空串（不丢弃）
  - cargo fmt --check + cargo clippy --workspace -- -D warnings + cargo test --workspace 全绿
  - pnpm build 通过
  - access.log 真实 fixture 集成测试（#[ignore]）本机验证过
verification_commands:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
  - cargo test -p ruT0-data-kit-core -- --ignored  # 含 access.log fixture
  - pnpm --prefix frontend build
files_changed:
  - crates/core/src/datasource/log.rs（重写）
  - docs/02-技术设计文档.md（§4.10 LogReader 段落更新）
  - docs/versions/1.1.2/更新日志.md（T45 进度 + E1/E5 + 关键设计 + 已知边界）
  - docs/versions/1.1.2/RELEASE-NOTES.md（.log 导入段落 + 已知限制 + 升级）
  - docs/qa/versions/1.1.2/QA-审计报告.md（R3 增量复核 + T45 需求覆盖 + 测试数 + 问题记录）
  - handoff/TASK-BOARD.md（T45 行 + E1/E5 更新）
  - handoff/TASK-T45-HANDOFF.md（本文件）
  - handoff/TASK-T45-REPORT.md（实现报告）
risks:
  - 正则误匹配：detect_format 用 probe 前 10 非空行 + 命中数最高者，避免单行误判；正则要求严格（Apache 必须有 IP+时间戳+引号请求；syslog 必须 Mon DD HH:MM:SS；app log 必须 ISO 日期前缀）
  - 大 .log 文件内存：read_all 全量读入（与 TxtReader 同模式），import_file 有 5000 行批量写入
  - 非 UTF-8 文件：用 std::fs::read + String::from_utf8_lossy（与 v1.1.2 T42 初版一致）
depends_on: [T42]
status: verified_complete
```

## 实现说明

### LogFormat 枚举 + 正则缓存

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFormat { ApacheCombined, ApacheCommon, Syslog, AppLog, Fallback }

static APACHE_COMBINED_RE: OnceLock<Regex> = OnceLock::new();
// ... 4 个 OnceLock<Regex>
fn combined_re() -> &'static Regex { APACHE_COMBINED_RE.get_or_init(|| ...) }
```

正则首次访问编译一次，后续直接返回 `&'static Regex`。用 `std::sync::OnceLock`（Rust 1.70+ std 库），无需新增 crate。

### detect_format probe 探测

```rust
fn detect_format(lines: &[&str]) -> LogFormat {
    let probe = probe_lines(lines, 10);  // 前 10 非空行
    // 对每个正则在 probe 行上匹配计数
    // 选命中数最高的格式（需 ≥1 命中），否则 Fallback
}
```

### split_request 二次拆分

`request` 字段 `"GET /path HTTP/1.1"` 拆为 (method, url, protocol)；`"-"` 或无法拆分返回三空串。

### read_all 结构化解析

按 `detect_format` 结果选列结构，每行用对应正则 `captures()` 提取字段；命中 → 填充结构化字段 + raw_line；未命中 → 各字段留空 + raw_line 保留原文；空行 → raw_line 空串。

### 单测（13 个）

- 3 既有 fallback 测试（reads_lines / preserves_empty_lines / empty_file）保留不变
- 10 新增结构化测试：apache_combined/common + unparseable_request + dash_bytes + syslog + syslog_without_pid + app_log + app_log_without_thread + mixed_fallback + raw_line_preserves
- 1 access.log 真实 fixture 集成测试（`#[ignore]`，CI 无 fixture 时跳过，本机验证过）

## 验证结果

```
cargo fmt --check          — pass（exit 0）
cargo clippy --workspace -- -D warnings — pass（exit 0，core + src-tauri 全零警告）
cargo test --workspace     — 136 passed / 3 ignored / 0 failed
  src-tauri lib: 77 passed / 0 ignored
  core lib:      59 passed / 3 ignored（tshark + pcap fixture + access.log fixture）
cargo test -p ruT0-data-kit-core -- --ignored — 3 passed（含 apache_log_with_real_fixture）
pnpm --prefix frontend build — pass（3079 modules，2.46s）
```

## 文档同步

- `docs/02-技术设计文档.md` §4.10 从「按行单列」改为「格式自动识别 + 结构化多列 + raw_line + fallback」，列结构表 + 5 格式说明
- `docs/versions/1.1.2/更新日志.md` T45 进度行 + E1/E5 验收更新 + 关键设计决策 + 已知边界
- `docs/versions/1.1.2/RELEASE-NOTES.md` .log 导入段落重写 + 已知限制 + 升级
- `docs/qa/versions/1.1.2/QA-审计报告.md` R3 增量复核 + T45 需求覆盖 + 测试数 136/3 + 问题记录更新
- `handoff/TASK-BOARD.md` T45 行 + DAG 依赖 + E1/E5 更新
