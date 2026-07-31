# TASK-T10-REPORT — datasource mod 拆分

```yaml
task_id: T10
implemented_changes:
  - crates/core/src/datasource/mod.rs：从 731 行 god module 缩减为 88 行，仅保留模块文档注释、子模块 mod 声明（csv/json/pcap/sql/txt/util/xlsx）、pub use 重导出（CsvReader/JsonReader/PcapReader/SqlReader/TxtReader/XlsxReader）、Reader trait 定义、detect_format 工厂、以及 detect_format_routes_by_extension 单测。子模块均为私有 mod，通过 pub use 对外暴露 struct。
  - crates/core/src/datasource/csv.rs：新增。CsvReader + read_all/headers 实现；原 csv_reader_headers_and_rows / csv_reader_flexible_columns 两个单测迁移到本模块 #[cfg(test)] 块，断言保持不变。
  - crates/core/src/datasource/xlsx.rs：新增。XlsxReader + read_all/headers 实现；引用 super::util::cell_to_string。
  - crates/core/src/datasource/json.rs：新增。JsonReader + read_all/headers 实现；引用 super::util::value_to_string。
  - crates/core/src/datasource/txt.rs：新增。TxtReader + read_all/headers 实现。
  - crates/core/src/datasource/sql.rs：新增。SqlReader + split_statements + extract_table_name + read_all/headers 实现；现状注释（不处理块注释/BEGIN END）随实现一起迁移，未改逻辑。
  - crates/core/src/datasource/pcap.rs：新增。datasource 侧 PcapReader 适配器（CorePcapReader = crate::pcap::reader::PcapReader），read_requests/read_all/headers 实现。
  - crates/core/src/datasource/util.rs：新增。共享 helper cell_to_string（calamine::Data → String）与 value_to_string（serde_json::Value → String），消除 mod.rs 散落的私有 fn。
  - crates/core/src/lib.rs：未改动（pub mod datasource 仍可见，公共 API 表面不变）。
  - src-tauri/src/commands.rs 在 working tree 中已被先前的 UI/export 重构会话拆分为 src-tauri/src/commands/ 子模块（与 T10 无关，属于 T11 范畴）；本次 T10 commit 的 git index 中因 commands.rs 既已不在 working tree 而被一并记录为 deleted，但该删除属于既有工作树状态，非 T10 主动改动。详见 scope_deviation。
verification_run:
  - cargo check --workspace
  - cargo test --workspace
  - cargo test -p ruT0-data-kit-core --lib datasource
verification_results:
  - cargo check --workspace：PASS。编译通过；仅余两条与本任务无关的既有 warning（src-tauri/src/commands/mod.rs 的 TsharkSettings 未用 import；非新增）。拆分初版曾因 xlsx.rs 漏导 calamine::Reader trait 报 E0599，已补 `use calamine::Reader as CalamineReader` 修复；并清理 pcap.rs 中未用 CoreError import。
  - cargo test --workspace：PASS。workspace 全部测试通过（core lib 13 项中 11 passed / 2 ignored（tshark 本机探测项），app crate 9 passed / 0 failed）。
  - cargo test -p ruT0-data-kit-core --lib datasource：PASS。3 个 datasource 单测全部通过：
      datasource::tests::detect_format_routes_by_extension ... ok
      datasource::csv::tests::csv_reader_headers_and_rows ... ok
      datasource::csv::tests::csv_reader_flexible_columns ... ok
docs_updated: none（handoff 明确为纯结构搬运，无行为/用法变化，docs/ 无需同步）
commit_summary: |
  commit 85d86d7 (main, 2026-07-31)
  refactor(core): T10 split datasource god module into per-format submodules
  9 files changed, 768 insertions(+), 499 deletions(-)
  - crates/core/src/datasource/{csv,json,pcap,sql,txt,util,xlsx}.rs 新增
  - crates/core/src/datasource/mod.rs 由 731 行精简为 88 行
  - 注意：commit 同时记录了 src-tauri/src/commands.rs 的 deletion（284 行），
    这是 working tree 中由先前 UI/export 会话产生的既有状态（commands.rs 已被
    拆为 commands/ 子模块，属于 T11 范畴），并非 T10 主动改动；后续 T11 commit
    (f1744f2) 已在同一 main 分支补齐 commands/ 子模块的 add。最终 main HEAD 树
    中 src-tauri/src/commands.rs 不存在、commands/ 子模块存在，行为一致。
reported_status: verified_complete
scope_deviation: |
  偏离项：T10 commit (85d86d7) 的 git stat 额外包含了
  `src-tauri/src/commands.rs | 284 ----` 的删除记录，超出 handoff in_scope
  （仅 crates/core/src/datasource/ 与 lib.rs）。
  根因：本次实现基于当前 working tree（含先前 UI/export 会话遗留的 commands.rs
  → commands/ 子模块拆分但尚未各自提交的状态）。`git add crates/core/src/datasource/`
  只暂存了 datasource 目录，但由于 commands.rs 在 working tree 中已物理删除且
  其拆分产物 commands/ 仍为 untracked，commit 时 git 将 commands.rs 的 deletion
  一并计入（因为它在 HEAD 树中存在、在 index/working 中缺失）。
  影响评估：该删除与 T10 的 datasource 拆分在代码层面无耦合；同分支紧随其后的
  T11 commit (f1744f2 "refactor(tauri): T11 split commands.rs ...") 已把
  commands/ 子模块正式入库，main HEAD 的最终状态自洽（commands.rs 不存在、
  commands/ 存在、cargo check/test 全绿）。未对 T10 验收语义造成破坏。
  缓解：未对历史做 rewrite（避免破坏已存在的 T11 commit 与 working tree 未提交
  改动）；如实记录此 scope deviation 供 reviewer 判定。
```

## 说明

T10 的目标 —— 把 731 行 `datasource/mod.rs` 按格式拆为独立子模块、纯结构搬运不改行为 —— 已达成。所有 reader 的解析逻辑、错误映射、返回值结构、测试断言均字节级保留。公共 API 表面不变：`ruT0_data_kit_core::datasource::{Reader, CsvReader, XlsxReader, JsonReader, TxtReader, SqlReader, PcapReader, detect_format}` 全部仍可从原路径访问（通过 mod.rs 的 pub use 重导出）。`SqlReader::split_statements` / `extract_table_name` 已迁入 sql.rs，行为不变（仍不处理块注释 / BEGIN END，符合 handoff「已知现状，不在本任务修复范围」）。

`reported_status: verified_complete` 仅为建议状态，最终完成由主会话判定。
```
