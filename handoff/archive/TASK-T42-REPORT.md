```yaml
implemented_changes:
  - 新建 crates/core/src/datasource/log.rs：LogReader 按 std::fs::read + String::from_utf8_lossy
    全量读取 .log 文件，按行拆分（content.lines()），每行一条 Record，单列 "line"；
    表头行 fields["line"]="line"；空行保留为空串；实现 Reader trait（read_all + headers）。
    含 3 个单元测试：reads_lines / preserves_empty_lines / empty_file。
  - crates/core/src/datasource/mod.rs：
    1) 新增 `mod log;` 与 `pub use log::LogReader;`（按字母序插入 csv/json 之后、pcap 之前）。
    2) detect_format 新增 `Some("log") => Ok(Box::new(LogReader::new(path)))` 分支（紧跟 txt 之后）。
    3) detect_format 的 doc 注释补充 `.log` → `LogReader`。
    4) detect_format_routes_by_extension 测试新增 `assert!(detect_format("/tmp/foo.log").is_ok());`。
  - frontend/src/components/layout/TopToolbar.jsx：导入对话框 filters extensions 数组末尾追加 "log"。
    使用 Edit 工具精确替换 extensions 数组那一段，未触碰文件其它部分。
verification_run:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
  - pnpm --prefix frontend build
verification_results:
  - cargo fmt --check：通过（首跑发现 log.rs 一行超长被 rustfmt 拆行；运行 cargo fmt 自动修复后再次 fmt --check 干净）。
  - cargo clippy --workspace -- -D warnings：通过，无 warning。
  - cargo test --workspace：T42 自身的 4 个测试全绿：
      datasource::log::tests::log_reader_reads_lines ... ok
      datasource::log::tests::log_reader_preserves_empty_lines ... ok
      datasource::log::tests::log_reader_empty_file ... ok
      datasource::tests::detect_format_routes_by_extension ... ok（含 .log 断言）。
    全量 workspace 测试 48 passed / 1 failed / 2 ignored；唯一失败为
    `pcap::reader::tests::parse_tshark_output_missing_fields_padded`（断言 left="" right="GET"）。
    该失败属于 pcap 模块，与本任务的 log.rs 无关；该 pcap 文件（crates/core/src/pcap/reader.rs
    与 detect.rs）不在 T42 in_scope 内，是另一个并行 coder 的改动（工作树里可见）。
  - pnpm --prefix frontend build：通过，vite build 成功（chunk size 警告为既有项目状态，与本次改动无关）。
docs_updated:
  - none（未发现 docs/ 下有需要同步的「支持格式清单」文档；datasource/mod.rs 的 doc 注释已在本次更新）。
commit_summary:
  - none（按要求不 commit、不 push）。
reported_status:
  - verified_complete
scope_deviation:
  - none。严格按 HANDOFF 实现，仅触碰 in_scope 三个文件，未改 import_file 命令、前端 IPC/reducer、
    其它 Reader 或 pcap 模块。工作树中其它 modified/untracked 文件（pcap/*.rs、settings/*、App.jsx、
    Cargo.lock、src-tauri/Cargo.toml、handoff/TASK-T*.md 等）均来自其它并行任务，非本任务改动。
notes:
  - 工作树存在多处非本任务的并行改动（T40/T41 等其它 coder）。本任务的 diff 仅限以下 3 个文件：
      crates/core/src/datasource/log.rs（新增）
      crates/core/src/datasource/mod.rs（mod + pub use + detect_format 分支 + 注释 + 测试断言）
      frontend/src/components/layout/TopToolbar.jsx（extensions 加 "log"）
  - 唯一失败测试 pcap::reader::tests::parse_tshark_output_missing_fields_padded 属于 pcap 模块，
    由其它任务引入，不在 T42 范围内，故仍建议 reported_status = verified_complete；最终判定由主会话确认。
```
