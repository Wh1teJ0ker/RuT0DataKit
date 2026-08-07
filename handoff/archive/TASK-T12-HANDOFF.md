---
id: T12
title: Rust 死代码清理（placeholder / 未用 model 结构 / db dead_code 标注）
depends_on: [T10, T11]
version: 1.0.0
type: refactor
status: planned
---

# TASK-T12-HANDOFF — Rust 死代码清理

## 目标（goal）

清理 `crates/core` 与 `src-tauri` 中本轮架构审计确认的 Rust 死代码 / 占位空骨架，使 workspace 在 `cargo check --workspace` + `cargo test --workspace` + `cargo clippy --workspace -- -D warnings`（或当前等效）下零死代码警告且行为不变。**纯删/标注，不改任何业务逻辑、不改公共 API 对外行为。**

## 背景与审计结论（background）

T10/T11 拆分后，以下死代码暴露出来（主会话审计已确认，coder 无需重新发现）：

1. **`crates/core/src/processor/` 整个模块为 v1.1+ 占位空骨架**：5 个文件（`mod.rs` + `extractor.rs` / `masker.rs` / `rules.rs` / `validator.rs`），每个只含一个空 trait + 一句 `use crate::model::Record;`。全仓 grep 确认 `processor::` 在 `crates/` 和 `src-tauri/` 中**零引用**（仅 `lib.rs:14 pub mod processor;` 声明）。这些骨架在 v1.0.0 不会被调用。
2. **`crates/core/src/model.rs` 中 `Sheet` / `Operation` / `Column` 三个 struct 零外部引用**：全仓 grep `model::Sheet` / `::Sheet` / `model::Operation` / `::Operation` / `model::Column` / `::Column` 在 `model.rs` 之外**零命中**。仅 `Record` 被各 datasource 子模块 + processor 占位 trait 使用。`Sheet/Operation/Column` 是 v1.0.0 shell 轮遗留的「字段占位」骨架，实际前端契约由 serde_json Value + commands 层处理，这三个 struct 从未参与序列化路径。
3. **`src-tauri/src/db/mod.rs` 有 4 处 `#[allow(dead_code)]`**（`SessionSummary` :33、`SheetSummary` :46、`SessionDetail` :59、`impl DbManager` :82）。这是 v1.0.0 shell 轮为「v1.1+ IPC 会用」留的注释性标注。本轮审计结论：**v1.1+ 会真正接入这些 API，当前保留结构但应把 `#[allow(dead_code)]` 改为更精确的 `#[allow(dead_code, reason = "v1.1+ IPC 将接入")]` 或等价注释**，使意图可追溯（不删结构，只改标注）。

## 范围（scope）

### in_scope

1. **删除 `crates/core/src/processor/` 整个模块**：
   - 删 5 个文件：`mod.rs`、`extractor.rs`、`masker.rs`、`rules.rs`、`validator.rs`。
   - 删 `crates/core/src/lib.rs:14` 的 `pub mod processor;` 声明。
   - 理由：v1.1+ 若需处理器，应在具备真实实现时新建模块（避免空骨架先占位）。删除零引用空模块不破坏公共 API（无外部消费者）。
2. **删除 `crates/core/src/model.rs` 中 `Sheet` / `Operation` / `Column` 三个 struct**：
   - 保留 `Record` struct（datasource 子模块在用）。
   - 保留文件头注释，但更新为「v1.0.0 仅 `Record` 被各 datasource 使用；Sheet/Operation/Column 推迟到 v1.1+ 真实需要时再定义」。
   - 理由：零外部引用，删除不破坏编译（无消费者）。
3. **`src-tauri/src/db/mod.rs` 的 4 处 `#[allow(dead_code)]` 改标注**：
   - 改为 `#[allow(dead_code, reason = "v1.1+ IPC 将接入（list_sessions/get_session 等）")]`（或当前 rustc 支持的等价写法；若 `reason =` 不被支持，改为上方 `// SAFETY/NOTE: v1.1+ IPC 将接入` 注释 + 保留 `#[allow(dead_code)]`）。
   - **不删 `SessionSummary` / `SheetSummary` / `SessionDetail` / `DbManager` 任何字段或方法**，不改其 serde 派生。
   - 理由：这些结构被单元测试覆盖（`list_sessions` / `get_session` 在 `#[cfg(test)]` 中调用），删了会破坏测试；标注意图即可。

### out_of_scope

- **不改任何 datasource / commands / pcap / error 代码**（T10/T11 已落，本轮只删死代码）。
- **不改 `model.rs` 的 `Record` struct**（有消费者）。
- **不改 `db/mod.rs` 的任何 struct 字段 / 方法 / 测试**（只改标注）。
- **不改 frontend**（T14 负责）。
- **不改 `Cargo.toml` / `tauri.conf.json` / capabilities**（配置不动）。
- **不补新功能**（v1.1+ 的 processor 实现不在本轮）。
- **不改 `docs/`**（T14 同步 docs/02，T12 不碰文档）。

## 验收标准（acceptance_criteria）

1. `cargo check --workspace` 零错误零警告。
2. `cargo test --workspace` 现有 27 个测试（含 db 模块测试）全过，不回归。
3. `cargo clippy --workspace -- -D warnings` 零警告（若环境支持 clippy；若 clippy 不可用则跳过，但 check 必过）。
4. `crates/core/src/processor/` 目录不存在；`crates/core/src/lib.rs` 无 `pub mod processor;`。
5. `crates/core/src/model.rs` 仅含 `Record` struct（+ 文件头注释）。
6. `src-tauri/src/db/mod.rs` 的 4 处 dead_code 标注含可追溯的「v1.1+」意图说明（`reason=` 或注释）。
7. `src-tauri/src/commands/` 与 `crates/core/src/datasource/` 文件字节级不变（T11/T10 成果不回退）。
8. 纯删/标注，无业务逻辑变更。

## 验证命令（verification_commands）

```sh
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings 2>/dev/null || cargo clippy --workspace
```

## 风险与回退（risks）

- **风险**：若某个外部消费者（未被 grep 命中）引用了 `processor::` 或 `Sheet/Operation/Column`，删除会导致编译失败。
- **缓解**：coder 删除前先跑 `cargo check --workspace` 确认基线绿；删除后再跑一次，若失败则定位消费者并停止删除该符号，回报主会话。
- **回退**：`git revert` 本次 commit 即可恢复（纯删/标注，无行为变更，回退安全）。

## 交付物（deliverables）

- 删除 `crates/core/src/processor/` 5 个文件。
- 修改 `crates/core/src/lib.rs`（删 1 行 mod 声明）。
- 修改 `crates/core/src/model.rs`（删 3 个 struct，保留 Record）。
- 修改 `src-tauri/src/db/mod.rs`（4 处标注改写）。
- `handoff/TASK-T12-REPORT.md`（按 orchestrator-workflow 04-coder-spec 模板）。

## scope_deviation

coder 不得越界。若发现审计遗漏的死代码，**先在 REPORT 记录、不删**，由主会话裁定是否追加 scope。任何「顺手修 bug / 改文案 / 优化结构」均视为越界。
