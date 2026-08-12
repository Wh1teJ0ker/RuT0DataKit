```yaml
implemented_changes:
  - crates/core/src/pcap/reader.rs
      - 把 read() 中的 stdout 解析逻辑提取为私有纯函数 parse_tshark_output(stdout: &str) -> Vec<HttpRequest>，read() 改为调用该函数
      - 行为保持不变：空行跳过、tab 分隔 8 字段、缺失字段空串补齐、http.file_data 十六进制经 hex_to_bytes 解码
      - 新增 5 个 parse_tshark_output 单元测试：basic / skips_empty_lines / hex_body_decoded / missing_fields_padded / empty_input
      - 强化 #[ignore] 集成测试 read_fixture_pcap：修正 fixture 路径（tests/base/base.pcap），补强断言 frame_no/method/host/uri 非空
  - crates/core/src/pcap/detect.rs
      - 新增测试 candidate_paths_windows_paths_are_absolute：验证含 "Program Files" 的候选路径以 C:\ 开头且以 tshark.exe 结尾
verification_run:
  - cargo fmt --check（针对改动文件 crates/core/src/pcap/{reader,detect}.rs 已通过；仓库其它无关文件存在既有 fmt 差异，非本任务范围）
  - cargo clippy --workspace --all-targets -- -D warnings
  - cargo test --workspace
verification_results:
  - cargo fmt --check：改动文件无 diff，clean
  - cargo clippy --workspace --all-targets -- -D warnings：Finished 无 warning，通过
  - cargo test --workspace：全部通过
      - core lib：49 passed; 0 failed; 2 ignored（含 6 个新 tshark 测试：parse_tshark_output_basic / skips_empty_lines / hex_body_decoded / missing_fields_padded / empty_input / candidate_paths_windows_paths_are_absolute 全绿；read_fixture_pcap + detect_tshark_local 仍 #[ignore]）
      - src-tauri：72 passed; 0 failed
      - 全部 doc-tests：0/0
docs_updated:
  - none（不改生产行为，仅补测试覆盖，无需更新文档）
commit_summary:
  - none（按交接要求未 commit、未 push；改动留在工作区）
reported_status:
  - verified_complete
scope_deviation:
  - "parse_tshark_output_missing_fields_padded 测试输入调整：HANDOFF 实现指引示例用 '1\\t\\tGET'（2 个 tab → 3 字段），但该输入下 method 在索引 2 而非 3，与生产逻辑字段索引（method=idx3）不符，会断言失败。改为 '1\\t\\t\\tGET'（method 落在 idx 3）以匹配真实字段映射。行为无任何改变，仅修正测试输入使其符合 tshark 8 字段布局。"
  - "read_fixture_pcap 集成测试 fixture 路径修正：HANDOFF 指引示例写 tests/base/base.pcap，原代码误写 tests/pcap/base.pcap（实际不存在）。已统一为 tests/base/base.pcap（仓库实际存在的 fixture），属本任务 in_scope 内的测试修正。"
  - 未触碰 detect.rs / reader.rs 的生产代码逻辑（仅提取函数 + 注释微调），未触碰非 pcap 模块
```

## 执行说明

### 验证命令结果

1. `cargo fmt --check`：本次改动的 `crates/core/src/pcap/reader.rs` 与 `crates/core/src/pcap/detect.rs` 无 diff，fmt clean。仓库其它文件（如 `crates/core/src/datasource/log.rs` 等）存在既有 fmt 差异，属其它任务的工作区残留，不在 T41 in_scope 内，未触碰。
2. `cargo clippy --workspace --all-targets -- -D warnings`：通过，无 warning。
3. `cargo test --workspace`：全绿。core lib 49 passed / 0 failed / 2 ignored；src-tauri 72 passed；doc-tests 0/0。包含 6 个新 tshark 测试全部通过。

### 改动文件清单

- `/Users/joker/code/RuT0DataKit/crates/core/src/pcap/reader.rs`（+128 / -30：提取纯函数 + 5 单元测试 + 集成测试强化）
- `/Users/joker/code/RuT0DataKit/crates/core/src/pcap/detect.rs`（+18：新增 Windows 路径绝对性测试）

未 commit、未 push，按交接要求留在工作区。
