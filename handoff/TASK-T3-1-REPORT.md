# TASK-T3-1 Report

## implemented_changes

- `crates/core/src/logsign/blind_aggregator.rs`
  - **双簇自动判定**：新增 `mode_body_size_from_mixed_positions(probes)`，
    替换 line 215 原 `group_true_size = min(body_size)`。算法只在「混合位置」
    （同 char_position 内 body_size 不全同，即能直接区分真假簇的位置）上
    统计 body_size 频次取众数；并列取较小者；无混合位置时退化到 `min(body_size)`
    （v0.2.2 行为，不破坏 fixture）。避免 beyond_end / unresolved_all_true
    单簇位置投票压倒 true 簇（HANDOFF risks 指出的退化场景）。
  - **多层嵌套正则**：`blind_probe_regex()` 的 read_target 捕获组从
    `((?:[^()]|\([^()]*\))*)` 改为 `((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)`，
    吃最多 2 层嵌套括号（覆盖 `where table_schema=database()`）。
  - **separator_char 字段**：`AggregatedResult` 末尾新增
    `#[serde(skip_serializing_if = "Option::is_none")] pub separator_char: Option<char>`。
    新增 `parse_separator_char(read_target)` + `hex_literal_regex()`，从 read_target
    解析首个 `0x([0-9a-fA-F]{2})` 字面量得到 ASCII 字符（0x7e→'~'、0x41→'A'）；
    字面量值 > 0x7f（非 ASCII）返回 None。aggregate() 末尾填入。
  - **模块级文档**：删去「true_size=min 是 fixture-specific」段落与 fixture
    thr/body 对照表，改为描述「双簇自动判定（混合位置众数）」算法 + 分隔符
    高亮 + 2 层嵌套正则限制。
  - **新增单测**（lib 内 `mod tests`）：
    - `true_size_auto_detects_larger_body`：合成 true=900/false=850（与 fixture
      相反方向），验证 pos 1 true_size==900、decoded_string 首字符 'p'。
    - `regex_captures_double_nested_parens`：read_target 含
      `where table_schema=database()`，验证完整捕获。
    - `separator_char_extracted_from_hex_literal`：0x7e→Some('~')、
      0x41→Some('A')、无字面量→None。
    - `mode_body_size_picks_smaller_on_tie`：混合位置并列取较小者、
      无混合位置退化 min。
    - `parse_separator_char_rejects_non_ascii`：0xff→None。
- 未改 `crates/core/src/logsign/mod.rs`（re-export 集合不变，AggregatedResult
  字段只增不删，旧 re-export 仍有效）。
- 未新建 `tests/blind_aggregator_test.rs`（已存在，T3-1 新测全部放在 lib
  内 `mod tests`，与 v0.2.2 既有风格一致；既有集成测试保持不动并通过）。

## verification_run

- `cargo build --workspace`
- `cargo test --workspace`

## verification_results

- `cargo build --workspace`：通过（仅 `ruT0_data_kit_core` 应为 snake_case
  的预存 warning，与 T3-1 无关）。
- `cargo test --workspace`：全绿。
  - lib：204 passed; 0 failed
  - 单测 1：6 passed; 0 failed
  - 单测 2：29 passed; 0 failed
  - 单测 3：12 passed; 0 failed
  - 单测 4：11 passed; 0 failed
  - tests/logsign_test.rs：31 passed; 0 failed
  - doc-tests：0 passed; 0 failed
  - 合计 293 passed; 0 failed（v0.2.2 既有 + v0.2.3 新增 5 个全绿）。
- 首轮 `cargo test` 曾失败 1 个（`true_size_auto_detects_larger_body`），
  原因：初版众数法在「全位置 body」上统计，被 pos 8 全 false body=850
  投票压倒众数 → group_true_size 错判为 850 → pos 1 自洽校验失败标
  `insufficient_probes`。修复为「只在混合位置上统计众数」（退化到 min），
  pos 8 单簇不再投票，pos 1 众数正确为 900。二轮全绿。
- fixture 回归（e2e::log_scan_full）：database()="person" /
  group_concat(table_name)="person_data" / group_concat(column_name) 含
  "id,username,password" 全部通过，4 个 read_target 仍正确聚合。

## docs_updated

- `crates/core/src/logsign/blind_aggregator.rs` 模块级文档注释（永久产品文档
  的一部分，与代码同文件）：删去 fixture-specific 段落，改写为双簇自动
  判定 + 分隔符高亮 + 2 层嵌套正则限制。
- `docs/` 下永久产品文档（02-技术设计文档.md 等）按 HANDOFF 约定留给
  T3-4 主会话统一同步，本任务不动。

## reported_status

verified_complete（建议状态；最终由主会话判定）

## scope_deviation

none。严格限定在 `crates/core/src/logsign/blind_aggregator.rs` 内；未触碰
payload_parser.rs / sqli_signatures.yaml / frontend / pipeline/log_scan.rs
调用契约 / access.log fixture。AggregatedResult 字段只增不删，新字段用
`#[serde(skip_serializing_if = "Option::is_none")]`，向后兼容。
