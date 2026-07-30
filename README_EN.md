# RuT0DataKit

A fast data-masking / parsing toolkit for data-security CTF scenarios. The core is
implemented in Rust; the desktop UI is built on Tauri v2. All processing happens
locally — no samples or rules are ever uploaded. This repository targets the
"sensitive-data quick sanitization / parsing" needs in data-security contests and
red-team workflows, and does not depend on a Python runtime.

> **Current status: v0.8.0 released (T28-1 ~ T28-5 verified_complete, release_complete).** v0.8.0 is a wrap-up minor release:
> - **Two v0.7.4 blind-injection BUG fixes pass through the release gate**: (a) `crates/core/src/tools/sql_parse.rs::SqlParseInput` gains `#[serde(rename_all = "camelCase")]` root-cause fix — Tauri v2 `#[command]` only auto-converts camelCase→snake_case for top-level params; nested struct fields go through raw serde. Without `rename_all`, the frontend's camelCase `responseBodySize`/`sourceIp` are **silently dropped** → `response_body_size: None` → `probe.rs None => Vec::new()` → 0 probes → empty schema. v0.7.2's Rust tests constructed struct literals (snake_case) bypassing serde, so they were green while the GUI never worked. A standalone serde test (`/tmp/serde_test_proj`) reproduces: NoRename → None (dropped); WithRename → Some. (b) `src-tauri/src/commands/log.rs::detect_sql_blind_features` removes the `samples.len() >= 50` cap — truncating long logs created gaps → `insufficient_probes`; deduplication + full pass to `parseSqlTool` keeps IPC payload controlled.
> - **Phone custom-prefix feature already delivered in v0.6.8 revision** (commit `666b496`; this release only indexes, no code changes): user's words "phone extraction should support a custom 3-digit prefix, default to a normal 1-prefixed number if none; the '52' default was wrong" — confirmed `phone`/`pinfo_phone` already accept a `prefixes` param for a custom 1-3 digit prefix, defaulting to a 1-prefix normal number when absent; **no "52" leftover** (only in deprecated historical comments). Entry: RulesView phone/pinfo_phone `prefixes` TextArea (placeholder `"138,159,734"`) → `SET_EXTRACT_OVERRIDE`/`SET_VALIDATE_OVERRIDE`.
> - **Version bump 0.7.4 → 0.8.0**: four manifests synced + Cargo.lock ×2. Non-breaking: the serde attribute changes neither field names/types/signatures — it only makes the frontend camelCase deserialize correctly; the 50-cap removal is strictly additive; phone prefix is zero code change. Version-status convention see `docs/04-版本标准.md`.
> - See `docs/versions/0.8.0/更新日志.md` and `docs/qa/versions/0.8.0/QA-审计报告.md`.

## Features

| Version | Capability | Status |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX tabular data masking + validation + export + rule management (four-function GUI) | Released v0.1.0 |
| v0.2.0 | Log-file parsing + SQLi signature scanning + weak-password / sensitive-field scanning | Released v0.2.0 |
| v0.2.1 | SQLi payload semantic parsing (6 classes) + field restoration (query/path/UA) + `+` decoding | Released v0.2.1 |
| v0.2.2 | Log-scan boolean-blind binary-sequence aggregation/flag recovery + GUI blind-aggregation result card | Released v0.2.2 |
| v0.2.3 | Blind-injection three-axis optimization: true_size mode algorithm + equality/length types + GUI kind Tag/separator highlight/Collapse position-detail | Released v0.2.3 |
| v0.2.4 | Blind-aggregation database-format reconstruction: ReconstructedDatabase cross-correlates 4 read_target classes + GUI antd Table per table with full columns | Released v0.2.4 |
| v0.3.0 | Sensitive-data extraction from pcap (tshark subprocess + HTTP field extraction + double URL decode + auto base64 field decode + sensitive scan + PcapView four-section GUI) | Released v0.3.0 |
| v0.4.0 | 7-view architectural refactor (unified preprocessing for 6 source types + multi-tag rule engine + unified SearchQuery enum + Tools SQL/regex parsing) | Released v0.4.0 |
| v0.4.1 | Five-issue defect fix: data-flow wired + remove FileToolbar from views + ToolsView dropdown + SQL blind-injection feature auto-jump + RegexTool statement → constructed regex | Released v0.4.1 |
| v0.4.2 | Settings module first installment: Sidebar-bottom "设置" entry + tshark multi-platform auto-detection + path configuration persistence + SettingsView UI (+ 3 patch fixes: file-import dialog adds json/sql extensions / sidebar separated from main content scroll / mask+validate dropdowns use user rules instead of operator templates; version unchanged) | Released v0.4.2 |
| v0.4.3 | txt compatibility (PreprocessView supports .txt import) + standalone "Data Extraction" module (ExtractView: file/text input → phone/bankcard/ip extraction → txt/csv/json export, matching PDF spec type_value format) + rule engine de-absolutization (PhoneValidator drops CTF/real prefix whitelist → `^1\d{10}$`; adds IpValidator) | Released v0.4.3 |
| v0.7.0 | Delete Tools/regex-parsing sub-tool (frontend RegexTool/RegexConstructTab + Tauri explain_regex/regex_construct + core regex_explain/regex_construct/regex_template all removed) + PreprocessView header gains a "column-level SQL parsing" button (takes all non-empty rows of that column → `parseSqlTool` → jumps to Tools/Sql reusing the existing UI) | Released v0.7.0 |
| v0.7.1 | SQL-parsing path auto-recognizes URL-encoded input: new `looks_like_url_encoded` (`(?i)%[0-9a-f]{2}`) + `parse_sqls` / `detect_sql_blind_features` decode via `url_decode_twice` only when matched, then feed the probe regex; callers may pass raw input; already-decoded input is unaffected (non-breaking patch) | Released v0.7.1 |
| v0.7.2 | SqlParseTool supports a `body\|sql` prefix to unblock blind reconstruction: frontend `onParse` parses each line with the regex `^(\d+)\|(.*)$` for a `<body_size>\|<sql>` prefix (e.g. `862\|username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1`); lines without a prefix keep `null` for backward compat with non-blind payloads; 2 core tests verify the pipeline (self-consistent sequence reconstructs 'p' + user's 5-payload gap asserted honestly); aggregation strict self-consistency check `max_true+1==min_false` not relaxed; gap=110/111 probes needed to self-consistently reconstruct 'p' (non-breaking patch) | Released v0.7.2 |
| v0.7.3 | PreprocessView adds a "Blind Auto-Extract" button + `detect_sql_blind_features` returns body_size/source_ip: extends the existing Tauri command's return shape from `samples: Vec<String>` to `samples: Vec<{ sql, body_size, source_ip }>` — locates the `size` column (HTTP response body byte count) + `ip` column (source IP) by header name from the same row, deduplicates by the `(sql, body_size, source_ip)` triple; PreprocessView's jump section gains a button (`handleBlindAutoExtract`) that scans the whole table → builds `body\|sql` text → calls `parseSqlTool` → jumps to Tools/Sql; non-log sources lacking size/ip columns get body_size=null and fall through to `parse_payload`; 4 new Tauri unit tests; return-shape change is zero-break (no frontend caller existed after v0.5.x removed the auto-jump entry) (non-breaking patch) | Released v0.7.3 |
| v0.8.0 | Wrap-up minor release: two v0.7.4 blind-injection BUG fixes (`SqlParseInput` gains `#[serde(rename_all = "camelCase")]` root cause — Tauri v2 nested-struct fields go through raw serde; without rename_all the frontend camelCase `responseBodySize`/`sourceIp` are silently dropped → 0 probes → empty schema; `detect_sql_blind_features` removes the 50-cap — long logs created gaps → insufficient_probes) + phone custom-prefix feature already delivered in v0.6.8 revision, indexed only (non-breaking minor) | Released v0.8.0 |

What v0.1.0 ships:
- Core pipeline: `detect_type` → `SourceReader` → `mask_pipeline` / `mask_pipeline_selected` (row selection, backward compatible) / `mask_pipeline_columns` (column selection) → `validate_pipeline` (validation) → `write_masked_csv` / `export_records_csv` / `export_records_xlsx`
- 7 built-in maskers + 1 generic `custom` masker + 4 rich-rule maskers (`regex_replace` / `regex_extract` / `delete` / `replace`)
- 7 built-in validators (reused by the v0.2.0 log scanner)
- **Abstract-operator rule engine**: `MaskOp` (Template/SplitTemplate/RegexReplace/ConstReplace) + `ValidateOp` (Regex/Algorithm/RegexWithGuard) + preset library (6 masker aliases + 7 validator aliases) + `apply_mask_op`/`apply_validate_op` preview APIs; adding a new masker type = one variant + one preset-library row.
- **Rule management single-list + Drawer + dynamic preview**: masker and validator rules merged into a single list, a top-right "Add Rule" Drawer, each rule has a "Preview" button where the user independently enters a sample value (no dependency on any imported file) — fully decoupled from data-file management.
- **Rules/mapping dual-state separation**: `state.rules` is the global rule library (managed by RulesView, independent of files); `state.maskOverrides` / `state.validateOverrides` are session-level temporary mappings (managed by MaskView/ValidateView, cleared on file import). Selecting an operator in MaskView/ValidateView only writes to overrides — it does not pollute the rule library; on "Apply", override + global rules are merged into a temporary RuleSet so export/validation flows are unaffected.
- **Four-segment vertical views for masking/validation**: raw data → header-rule mapping → preview → action panel; apply button triggers preview; export button jumps.
- **Single-Table integrated export**: header checkbox + reorder-up/down + cell preview, CSV+XLSX dual format.
- **Four-function navigation GUI** (sidebar: 4 active views + 2 disabled; Data Masking / Data Validation / Data Export / Rule Management)
  - **Column-selective masking**: per-column header checkbox + "Apply rules to selected columns"
  - **Validation GUI**: ValidateView as an independent view, per-cell validity matrix + "Jump to Data Export"
  - **Export column selection / order / format**: CSV + XLSX (src-tauri `rust_xlsxwriter`)
  - **Rule description + edit**: `MaskRule` / `FieldRule` carry a `description` field; RulesView supports edit (YAML save/load removed; only add/edit/delete retained)
  - **Cross-view state persistence**: top-level `useReducer` in `App.jsx` keeps data in memory; switching views does not reset data
- **`MaskOp::Template` boundary BUG fix**: when `keep_prefix + keep_suffix == value length` (middle-segment length is 0), the old implementation incorrectly output `original value + mask star` (e.g. `李四海*`); after the fix, it correctly goes through the overlap branch and outputs only `mask_min_len` `mask_char`s (e.g. `*`). The legacy `CustomMask::mask` is fixed in sync.
- Frontend built on React + Ant Design 5 + @ant-design/icons, pure-white theme, no emoji, dropdowns use antd Select

What v0.2.0 adds (log-processing slice):
- **CLF log parsing**: `crates/core/src/log/`: `LogReader::read` + `LogEntry` (ip / time / method / path / query / status / user_agent, etc.) + `parse_query` double URL decoding (`url_decode_twice`; decodes `%XX` but not `+`, so patterns are written in decoded form). Fixture `tests/fixtures/samples/log/access.log` has 1860 lines (including 4 SQLi variant samples).
- **SQLi attack signature engine**: `crates/core/src/logsign/`: `SignatureEngine` + `sqli_signatures.yaml` (6 built-in signature classes: `sqli_blind_binary` / `sqli_union` / `sqli_error_based` / `sqli_time_based` / `sqli_tautology` / `sqli_comment`) + `context_anchor` anchors to suppress weak-signal false positives (e.g. a bare `or` / `--` / `#` does not fire on its own).
- **Signature engine load / reload**: `load_builtin_signatures` (embedded YAML) + `load_signatures_from_str` (runtime YAML string) + `SignatureEngine::from_rules` (construct-time merge with per-rule `enabled` toggle).
- **Log-scan pipeline**: `crates/core/src/pipeline/log_scan.rs`: `scan_log` combines SignatureEngine + weak-password grep (`WEAK_KEYWORDS`: password / passwd / admin / 123456 / root / qwerty, etc.) + `DefaultSensitiveScan` (reuses the v0.1.0 `SensitiveScan` trait), producing the unified `Report(kind="log_scan")`; `summary` carries `total_lines` / `sqli_hits` / `weak_password_hits` / `sensitive_hits` / `top_attack_ips` (sorted by hit count, truncated to 5).
- **GUI log entry activated**: src-tauri `scan_log` command; frontend sidebar "Log Scan" flipped from disabled to active; pcap still greyed out (v0.3.0).
- **Observability**: log-scan results reuse the unified `Report` / `Finding` serde structure, so the frontend can serialize and render them.

What v0.2.1 adds (log-processing slice semantic enhancement):
- **`parse_query` `+` decoding + decoded fields**: `parse_query` now follows the form-urlencoded standard (`+`→space then double `%XX` decode); `LogEntry` gains `decoded_path` / `decoded_query` / `decoded_ua` fields, filled once by `parse_line` and consumed directly by the signature engine and GUI (v0.2.0 only decoded `%XX` and not `+`; v0.2.1 switches to the standard form-urlencoded behavior).
- **SQLi payload semantic parsing**: `crates/core/src/logsign/payload_parser.rs`: `parse_payload(decoded_text) -> Option<ParsedPayload>` performs 6-class structured semantic parsing on the decoded payload after a hit (blind_boolean / union / error / time / tautology / comment), extracting read_target / char_position / compared_ascii / comparator / union_columns / sleep_seconds + a human-readable `summary`.
- **`SignatureHit.parsed_payload`**: `scan_log_entry` calls `parse_payload` on the hit text segment when producing a hit; `None` when nothing parses.
- **`Finding.extra` passthrough**: `Finding` gains `extra: Option<serde_yml::Value>` (`skip_serializing_if = "Option::is_none"`, backward compatible); the `scan_log` pipeline serializes the sqli hit's `parsed_payload` into `Finding.extra` so the frontend can render "Parse Result" / "Read Target" columns; weak_password / sensitive / csv_mask scenarios have `extra=None`.
- **6-class signature pattern variants**: `sqli_union` now accepts `union all select`; `sqli_error_based` now accepts `exp(~...)` / `floor(rand(0)*2)` error-based variants.
- **GUI LogView decoded columns + parse-result/read-target columns**: the raw-log table gains `decoded_path` / `decoded_query` / `decoded_ua` columns; the findings table gains "Parse Result" (`ParsedPayload.summary`) and "Read Target" (`read_target`) columns.

What v0.2.2 adds (log-scan blind-binary-sequence aggregation):
- **blind_aggregator module**: `crates/core/src/logsign/blind_aggregator.rs`: `BlindAggregator::collect_from_entries` runs an embedded regex to extract `ascii(substr((<read_target>),N,1))<cmp><thr>` binary probes; groups by `(read_target, source_ip)` and sub-groups by `char_position`; auto-clusters the true/false direction from `LogEntry.size` (HTTP response body byte count) using `true_size = min(body_size)` (fixture-specific assumption: true-condition responses have a smaller body); stops concatenation at the first `beyond_end`, reconstructing the full flag string (e.g. `database()`=`"person"` / `table_name`=`"person_data"` / `column_name` contains `id,username,password`).
- **BlindProbe / PositionDetail / AggregatedResult structures**: `read_target` / `char_position` / `threshold` / `body_size` / `source_ip` / `line_no` / `decoded_char` / `ascii_val` / `true_size` / `probe_count` / `status` (`resolved` / `unresolved_all_true` / `beyond_end` / `insufficient_probes`) / `decoded_string` / `resolved_chars` / `unresolved_chars` / `beyond_end_positions` / `position_details`.
- **Embedded nested regex**: regex crate has no look-around, so `read_target` inner nested parens are consumed by `(?:[^()]|\([^()]*\))*` for a single `(...)` layer (e.g. `database()` / `group_concat(table_name)`); multi-layer nesting is not supported (known simplification).
- **Report.extra.blind_aggregation passthrough**: `scan_log` serializes `Vec<AggregatedResult>` into `Report.extra` as `Value::Mapping({blind_aggregation: [...]})`; with no blind probes, `extra` is still a Mapping with an empty array; backward compatible (v0.2.1 `Finding.extra: Option<Value>` behavior unchanged).
- **GUI LogView blind-aggregation result card**: a new section ③.5 below the findings table reads `Report.extra.blind_aggregation` and renders an antd inner Card + `Typography.Text` copyable restored string (one-click copy flag) + probe count + source IPs + position-detail Tooltip + an `Empty` empty state.

What v0.2.3 adds (blind-injection three-axis optimization):
- **Algorithm (T3-1)**: `true_size` decision changes from v0.2.2's `min(body_size)` (fixture-specific assumption, mis-decides in reverse scenarios) to `mode_per_position_true_size` — group by `char_position`, take min/max of the true/false body_size clusters per mixed position → cross-position mode (tie → smaller) → fall back to `min(body_size)` when no mixed positions; fixes the v0.2.2 R1 regression where the 4th read_target (`group_concat(id,0x7e,username,0x7e,idcard)`, false frequency 737 > true 669) was mis-decided to 875 → all `?`; the new algorithm takes the per-position true-cluster body_size = 862 → mode 862, correctly resolving `1~zhangsan~...~lisi~...`. The embedded regex extends from single-layer to two-layer nesting `((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)` covering `where table_schema=database()`. `AggregatedResult` gains `separator_char: Option<char>` (`#[serde(skip_serializing_if)]`) decoded from the first `0xNN` literal in `read_target` (`0x7e`→`~`; `0xff+` returns `None`).
- **Types (T3-2)**: `BlindProbe` gains `probe_kind: ProbeKind` enum (`AsciiBinary` / `Equality` / `Length`) + `equality_char: Option<char>`; two new regexes — equality (`substr((...),N,1)=('x'|char(NN))` directly resolves a single char) and length (`length((...))(>=?|<=?|=)(\d+)` resolves the string length, taking the minimum `=` threshold or maximum `<` threshold). The grouping key upgrades from `(read_target, source_ip)` to `(read_target, source_ip, ProbeKind)` three-tuple to prevent cross-contamination between ascii_binary and length probes on the same read_target. `AggregatedResult` gains `kind: Option<String>` (serialized as `ascii_binary` / `equality` / `length`, `#[serde(skip_serializing_if)]`). `PositionDetail.status` adds `equality_resolved` / `length_resolved`.
- **Display (T3-3)**: LogView section ③.5 inner Card — `extra` adds a kind Tag (ascii_binary=red / equality=orange / length=blue; frontend infers from `position_details[0].status` when `r.kind` is missing) + a resolved N/M Tag; the restored string upgrades from v0.2.2's `Text copyable` to `Text strong` + a volcano Tag wrapping the separator for highlighted segmentation + a standalone copy button (`navigator.clipboard.writeText`) when `separator_char` is present, otherwise `Text strong copyable`; position details upgrade from v0.2.2's Tooltip + JSON.stringify to an antd `Collapse` (ghost / size=small / collapsed by default) + nested `Table` (columns position/char/ASCII/probe-count/status with 6-color status Tag: resolved=green / unresolved_all_true=orange / beyond_end=default / insufficient_probes=red / equality_resolved=blue / length_resolved=purple).

What v0.2.4 adds (blind-aggregation database-format reconstruction):
- **Backend structs + algorithm (T4-1)**: `blind_aggregator.rs` adds `ReconstructedDatabase { schema, tables, unmatched_results }` / `ReconstructedTable { name, columns, rows, row_data_columns, column_separator, source_probe_count }` / `ReconstructedRow { cells: Vec<Option<String>> }` structs + `BlindAggregator::reconstruct_database(&[AggregatedResult]) -> ReconstructedDatabase` associated function. Three-step algorithm: Step1 classify each `read_target` into 4 pattern classes (`schema`=database() / `table_list`=group_concat(table_name) from information_schema.tables / `column_list`=group_concat(column_name) ... where table_name='X' / `row_data`=group_concat(col1,0xNN,col2,...) from <table>; unmatched → `unmatched_results`); Step2 assemble (schema direct → table_list names matched to column_list by `table_name='T'` predicate for full columns (no predicate → by index; no column_list → fall back to row_data_columns) → row_data matched by `from T` and parsed: `,` split rows + `column_separator` (decoded from `0xNN`) split cols → projected onto full columns, unfetched = None); Step3 unmatched fallback as-is. 8 unit tests cover full reconstruction / unmatched / partial column-list fallback / empty results / no-separator single-column / 3 regex unit tests.
- **Pipeline passthrough (T4-2)**: `scan_log` calls `reconstruct_database` at the end and writes `Report.extra.reconstructed_database` (alongside `blind_aggregation`, backward compatible with v0.2.3). Fixture auto-reconstruction: schema="person" / 1 table person_data / 7 columns (id/username/password/sex/birth/idcard/phone) / ≥2 rows / row[0].id=Some("1") / row[0].username contains zhangsan/lisi / row_data_columns=[id,username,idcard] / column_separator=Some('~'). e2e `log_scan_full` adds five-segment assertions.
- **GUI display (T4-3)**: LogView adds section ③.5a "还原数据库视图" Card before ③.5 — top `Descriptions` (schema `<Text code>` + table count + unmatched count); each `ReconstructedTable` is an inner Card + antd `Table` (full columns as headers, unfetched cells `render: v => v ?? <Text type="secondary">-</Text>` show `-`) + extra (`column_separator` volcano Tag + `row_data_columns` Tooltip "fetched N/M cols" + probe-count Tag); non-empty `unmatched_results` → `Divider` + simple list (read_target + decoded_string copyable); empty state → `Empty` ("无法还原为数据库结构"). Section ③.5 is renamed "原始聚合明细" and kept as fallback (kind Tag / separator highlight / Collapse+Table position details all preserved), data source `blindAggregation` unchanged.
- **Backward compatibility**: the `blind_aggregation` array is preserved; the new `reconstructed_database` key and the new struct fields `schema` (Option) / `column_separator` (Option) are all `#[serde(skip_serializing_if)]`.
- **Known simplifications**: row splitting relies on the default group_concat row separator `,` (fixture idcard is numeric, no `,`); `group_concat SEPARATOR 'X'` clause parsing, three-layer+ nested regex, UNION error-based aggregation / time-blind aggregation are deferred to v0.3.0+.

What v0.3.0 adds (pcap traffic-capture sensitive-data extraction):
- **tshark subprocess + HttpRequest extraction (T3-1)**: `crates/core/src/pcap/reader.rs::PcapReader::read(path)` invokes system `tshark` (`-Y "http.request" -T fields -e frame.number -e ip.src -e ip.dst -e http.request.method -e http.host -e http.request.uri -e http.file_data -e http.user_agent -E separator=\t -E occurrence=f`) → TSV → `HttpRequest { frame_no, src_ip, dst_ip, method, host, uri, body: Option<String>, user_agent }`; `http.file_data` hex string goes through a hand-written `hex_to_bytes` (no `hex` crate, handles empty / odd-length / invalid-char boundaries) → `String::from_utf8_lossy`. Missing tshark: `Command::new("tshark").arg("--version")` probe fails → `CoreError::DependencyMissing("tshark")`, no panic. Fixture `tests/fixtures/samples/pcap/data.pcapng` (9.4MB / 60290 frames / 5000 HTTP POST) yields ≥5000 HttpRequest.
- **Decoding (T3-2)**: `crates/core/src/pcap/decoder.rs` provides 4 public functions: `decode_url_twice` (independent impl to avoid circular dep on the log module); `try_decode_base64_field` (hand-written `b64_decode`, no `base64` crate; requires `length≥4` + `length%4==0` + charset ⊂ `[A-Za-z0-9+/=]` + `printable_ratio≥0.8` triple false-positive guard; UTF-8 only, GBK deferred to v0.3.1+); `extract_decoded_fields` (triple fallback: ① `serde_json::from_str` Object → recursive base64 per field value → ② form-urlencoded `&`/`=` split → ③ plain text as single `_body` field); `reassemble_base64` (rule-based fallback, fixture doesn't need it, interface preserved for CTF chunked scenarios). `crates/core/Cargo.toml` adds `serde_json = "1"` dependency. Fixture first POST body decodes all 7 fields correctly: username=chenyong / name=付里夏旋 / sex=女 / birth=20010905 / idcard=506051200109055743 / phone=74733385248 / address=黑龙江省哈尔滨市通河县三站镇377号159室.
- **Scanning + pipeline + e2e (T3-3)**: `crates/core/src/pcap/scanner.rs::PcapScanner::scan(path, scan, rules)` reuses `DefaultSensitiveScan`, assembles `scan_text = decode_url_twice(uri)` + body decoded non-empty field values per HttpRequest (accumulating `decoded_fragments`) → `scan.scan(&scan_text, rules)` → each Finding gets `location=Some("frame:{frame_no}")` + `context=Some("{method} {host} -> {src_ip}")` + `extra=None` → accumulates `sensitive_hits` + `ip_counts: HashMap`; produces `Report { source, kind: "pcap_scan", summary: { total_requests, sensitive_hits, decoded_fragments, top_src_ips (sorted desc, top 5, each {ip, hits}) }, findings, extra: Value::Null }`. New `pipeline/pcap_scan.rs::scan_pcap` thin wrapper. e2e `pcap_scan_full` (`#[ignore]`, run manually when tshark is present) asserts ≥4000 sensitive hits + kind=="pcap_scan" + contains idcard type.
- **Command + GUI (T3-4)**: New Tauri command `scan_pcap_file(path) -> Result<Value, String>` (single IPC returns `{ entries: Vec<HttpRequest>, report: Report }`, avoids a second call); `main.rs` `generate_handler!` registers it (21 commands total); `pcap_sensitive_ruleset()` independent helper returns idcard/phone/name validators RuleSet (doesn't pollute mask view's `load_default_mask_ruleset` which has empty validators). Frontend `PcapView.jsx` four-section layout (mirrors LogView): ① import `.pcap/.pcapng` (`select_file` → `detect_source_type === "pcap"` → `scanPcapFile`); ② raw HTTP table (pageSize 50, body slice 200 preview); ③ scan button + summary Descriptions + top_src_ips volcano Tag; ④ Findings table (type Tag colored by `TAG_COLOR_BY_TYPE`: idcard=purple / phone=blue / bankcard=magenta / email=cyan / mac=geekblue / username=gold / name=green). state adds `pcapEntries` / `pcapReport` / `pcapLoading`; `Sidebar.jsx` removes pcap `disabled: true`, all 6 items active. tshark missing: `msg.includes("tshark") || msg.includes("DependencyMissing")` → `message.error("流量分析需要系统 tshark，请先安装 Wireshark CLI (brew install wireshark)")`. 0 emoji / 0 native select maintained.
- **Sensitive scan only, no SQLi signatures** (fixture has none); `kind="pcap_scan"` is new and backward compatible with v0.2.4 (existing `csv_mask` / `log_scan` kinds untouched).

What v0.4.0 adds (architectural refactor):
- **Unified preprocessing entry (View 1)**: `read_records(path)` dispatches in the core layer by `detect_type(path)` to `CsvReader` / `XlsxReader` / `SqlReader` (splits on `;` skipping in-string semicolons, producing `sql_text` + `statement_type` columns) / `JsonReader` (union keys of object arrays as headers, single-value arrays collapse to a `value` column, scalars to_string) / `PcapRecordsReader` (returns `DependencyMissing("tshark")` if tshark is missing) / `LogRecordsReader` (same parsing semantics as v0.2.0 producing unified columns). Frontend `PreprocessView` calls `read_records` then writes `state.records`, exposing four jump buttons (Search / Mask / Validate / Export).
- **Multi-tag rule engine (View 2)**: `FieldRule` and `MaskRule` gain `tags: Vec<String>` (default empty Vec, backward compatible); `RuleSet::by_tag(tag)` / `by_tag_mask(tag)` filter by tag; one rule can carry multiple tags (e.g. `[mask, sensitive]` / `[validate, sensitive]`) so the same rule is reused across mask / validate / search / SQL parse views; `RulesView` Drawer supports multi-tag editing.
- **Unified search (View 3)**: `search_records(records, query)` is the single entry, `SearchQuery` enum `Keyword { terms, mode: And|Or }` / `Regex { pattern }` / `ExactField { field, value }`; keyword uses `SearchIndex::build` inverted index + `search_keyword`, regex scans linearly (invalid pattern returns `InvalidInput`), exact_field locates by column name and matches exactly. `SearchView` exposes a 3-way Radio + hit table (`<mark>` highlight, no `dangerouslySetInnerHTML`), and "Jump to Mask / Jump to Export" dedup-sorts hit row indices into `state.filteredRowIndices`.
- **Mask / Validate / Export (Views 4-6)**: four-segment vertical layouts; MaskView/ValidateView consume `state.filteredRowIndices` for "search-hit rows only" filtering; ExportView is a single antd Table + header Checkbox for export columns + reorder up/down + cell preview + source-data Select (masked / validated / raw) + row filter + format Select (CSV / XLSX / JSON).
- **Tools (View 7)**: Sidebar SubMenu hosts two sub-tools — (a) **SQL parsing**: `parse_sqls(inputs) -> SqlParseResult { probes, aggregated, reconstructed, parsed_payloads }`, `extract_blind_probe(sql, response_body_size, source_ip)` extracts `BlindProbe` (AsciiBinary / Equality / Length), `reconstruct_database` correlates 4 read_target classes to reconstruct the DB schema/tables; non-blind payloads fall back to `parse_payload`. (b) **Encrypt/Decrypt**: generic EncryptTool. **v0.7.0: the former "Regex parsing" sub-tool is removed** (see the v0.7.0 changelog for details).
- **PreprocessView column-level SQL-parse jump (v0.7.0 T24-2)**: a `ConsoleSqlOutlined` button is added to the preview-table header next to the ✏ rename button; clicking it triggers `handleColumnSqlParse(columnName)` — collects all non-empty cell values of that column as `SqlParseInput[]` → `parseSqlTool(inputs)` → writes `SET_SQL_PARSE_INPUT` + `SET_SQL_PARSE_RESULT` → jumps to Tools/Sql reusing the existing `SqlParseTool.jsx` UI (mirroring the Sidebar.jsx:60-62 jump pattern); the user can keep editing the textarea in Tools/Sql to rerun. Empty column → warning, no jump; failure → message.error, no clear of existing results. All-local processing, no data exfiltration.
- **Cross-view state persistence**: top-level `useReducer` in `App.jsx` keeps data in memory; `SET_VIEW` only switches views and never resets data; search-hit row indices, mask results, validation results, SQL parse results all persist across views.
- **No data exfiltration**: all-local processing; rules and samples are never uploaded (preserves the v0.1.0 §6 safety constraint).

What v0.4.1 adds (five-issue defect fix):
- **T6-1 Data-flow wired**: `ExportView` migrates from reading `state.filePath` (the SET_FILE channel, never actually written in v0.4.0) to reading `state.records` (the SET_RECORDS channel, aligned with `PreprocessView.handleImport` as the sole writer); `computeExportArgs` falls back to records as the backend inputPath; new e2e `preprocess_to_search_finds_hits` (csv → `read_records` → `search_records(Keyword "张三")` asserts hit count ≥1 and the matched cell contains "张三") verifies the "preprocess → search" link end-to-end, fixing the user-reported "after preprocessing the import, search can't find the data" root cause.
- **T6-2 Remove FileToolbar from every view**: delete `frontend/src/components/FileToolbar.jsx` (-106 lines) + `App.jsx` drops the `NO_TOOLBAR_VIEWS` set and `<FileToolbar/>` render branch; the only import entry now converges on `PreprocessView`'s built-in "Select File" button (the v0.4.0 design intent); removes the redundant import buttons from the top of each view.
- **T6-3 ToolsView dropdown**: The "Tools" entry in `Sidebar.jsx` changes from a plain Menu item to an antd `Menu.SubMenu`, `children=[{key:"tools.sql",label:"SQL 解析"},{key:"tools.encrypt",label:"加密/解密"}]` (v0.7.0: the former `tools.regex` sub-item is removed); clicking the "Tools" title expands/collapses (`openKeys` controlled as `state.sidebarToolsOpen`, collapsed by default); clicking a sub-item dispatches both `SET_VIEW("tools")` and `SET_TOOLS_ACTIVE_TAB(<sql|encrypt>)`, and `selectedKeys` highlights `tools.<toolsActiveTab>` when `activeView === "tools"`; `ToolsView.jsx` itself no longer renders an in-view `Select` dropdown (semantically duplicates the Sidebar SubMenu) and only renders the Card + sub-tool content, with the Card `title` switching based on `toolsActiveTab`; the user's "sidebar Tools should have a dropdown" requirement ultimately lands as the Sidebar SubMenu form.
- **T6-4 SQL blind-injection feature auto-jump**: core `looks_like_blind_probe(sql: &str) -> bool` reuses `ascii_binary_regex` / `equality_regex` / `length_regex` and does not depend on `response_body_size` (unlike `extract_blind_probe`, so it works at preprocessing time when only SQL text is available); Tauri `detect_sql_blind_features(headers, rows) -> {detected, samples}`; GUI `PreprocessView.handleImport` auto-jumps to SqlParseTool and prefills samples on detection; pure local regex matching, no data exfiltration.
- **T6-5 RegexTool statement → constructed regex** (removed in v0.7.0): historical v0.4.1 refactor — dropped the built-in template Tab and added a `ConstructTab` (antd `TextArea` statement → `regexConstruct(statement)` → `pattern` `Paragraph` copyable + `matched_clues` `Tag` list + sample highlight); core gained `crates/core/src/tools/regex_construct.rs::construct_regex(statement) -> Result<ConstructedRegex, CoreError>`. Removed entirely in v0.7.0; see the v0.7.0 changelog.

## v0.7.0 shipped

- **T24-1 Remove the regex-parsing feature**: per explicit user request. Frontend `RegexTool.jsx` / `RegexConstructTab.jsx` deleted; `ToolsView.jsx` / `Sidebar.jsx` / `state.js` / `tauri.js` cleaned up in sync (5 ACTION constants + 5 state fields + 5 reducer cases removed); Tauri `commands/tools.rs` rewritten to expose only `parse_sql_tool`, `main.rs` drops the `explain_regex` / `regex_construct` registrations (handler count 39 → 37); core `tools/mod.rs` rewritten to expose only `encrypt` + `sql_parse`, deleting `regex_explain.rs` / `regex_construct.rs` / `regex_template.rs`. **Explicitly preserved**: `SearchQuery::Regex`, masker-internal regex (`regex_replace` / `regex_extract`), logsign blind regex, `state.searchRegexInput` / `SEARCH_REGEX_*` — these use regex internally but are unrelated to the Tools/regex-parsing sub-tool.
- **T24-2 PreprocessView column-level SQL-parse jump**: a `ConsoleSqlOutlined` button is added to the preview-table header `<Space>` next to the ✏ rename button; `handleColumnSqlParse(columnName)` iterates `records.rows` by `headers.indexOf(columnName)`, collects all non-empty cell values → `parseSqlTool(inputs)` → writes `SET_SQL_PARSE_INPUT` + `SET_SQL_PARSE_RESULT` → `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB("sql")`. Complementary to T6-4's "blind-injection auto-jump": the former is user-triggered for a single column, the latter is detection-time over all cells during import. `SqlParseTool.jsx` is unchanged (it already reads these two state fields).
- **T24-3 Version bump + docs cleanup**: four manifests bumped `0.6.8 → 0.7.0`; docs/00/01/02/03/04 + README + README_EN sync-deleted regex-parsing entries and added v0.7.0 column-level SQL-jump descriptions; `docs/versions/0.7.0/更新日志.md` and `docs/qa/versions/0.7.0/QA-审计报告.md` landed.
- **T24-4 Release QA + finalize**: 5-dimension Release QA (functionality / regression / build / safety / docs) concluded qa_passed; `cargo test -p ruT0-data-kit-core --release` 464 passed + tauri 5 passed + npm build 3007 modules 0 error; commit + push origin main.

## v0.7.1 shipped

- **T25-1 Two call-site URL-decode-on-detection**: user reported URL-encoded SQLi payloads (`username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1`) failed to parse. Root cause: `parse_sqls` / `detect_sql_blind_features` fed raw input directly to plain-text regexes (expecting literal space / `>` / `#`); `%20` / `%3E` / `%23` did not match → 0 probes. Fix: new `crates/core/src/log/mod.rs::looks_like_url_encoded` (regex `(?i)%[0-9a-f]{2}` + `OnceLock` singleton, reusing the existing `regex::Regex` with zero new deps); core `tools/sql_parse.rs::parse_sqls` conditionally decodes `input.sql` before feeding `extract_blind_probe` / `parse_payload`; Tauri `commands/log.rs::detect_sql_blind_features` conditionally decodes cells before `looks_like_blind_probe`, with `samples` now collecting the decoded form (consistent with the `parse_sqls` entry, avoiding downstream double-decode). `SqlParseInput.sql` field semantics relaxed: callers may pass raw. Already-decoded input is unaffected (`looks_like_url_encoded` returns false and takes the original path — backward compatible with v0.5.0 ~ v0.7.0). Added 8 core + 3 Tauri unit tests.
- **T25-2 Version bump + docs + Release QA + finalize**: four manifests bumped `0.7.0 → 0.7.1`; docs/00/02/03/04 + README + README_EN sync-added v0.7.1 descriptions; `docs/versions/0.7.1/更新日志.md` and `docs/qa/versions/0.7.1/QA-审计报告.md` landed; 5-dimension Release QA concluded qa_passed; `cargo test -p ruT0-data-kit-core --release` 472 passed + tauri 8 passed + npm build 3007 modules 0 error; commit + push origin main.

## v0.7.2 shipped

- **T26-1 Frontend SqlParseTool `body|sql` prefix parsing + UI copy**: user reported that URL-encoded payloads still could not fully reconstruct the database after v0.7.1 URL decoding. Root cause: the frontend `SqlParseTool.jsx:58` constructed `responseBodySize: null` for every line, and `extract_blind_probe_with_line` (`crates/core/src/logsign/blind/probe.rs:61-64`) returns `Vec::new()` when `response_body_size: None` → 0 probes → empty aggregation → empty reconstruction. Blind binary reconstruction **requires** body_size to cluster true (`ascii>thr` true, body==true_size) vs false (body!=true_size) probes. Fix: `onParse` parses each line with the regex `^(\d+)\|(.*)$` for a `<body_size>|<sql>` prefix (e.g. `862|username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1`); matched lines get `responseBodySize` as an integer, otherwise `null` (backward compatible with non-blind payloads: time/error/union/tautology/comment still go through the `parse_payload` fallback). File header comment + help text + placeholder updated to document the prefix syntax. No new deps (pure local regex).
- **T26-2 Core unit tests verify the pipeline + honestly document the gap**: added 2 core tests to `crates/core/src/tools/sql_parse.rs` tests module — (1) `parse_sqls_body_size_enables_full_reconstruction`: uses the existing helper `ascii_binary_probes_for_char("database()", 1, b'p' as u32, 862, 875, "1.1.1.1")` to generate a self-consistent 3-true + 2-false sequence (max_true=111, min_false=112, `111+1=112` ✓ self-consistent); asserts 5 probes + `decoded_string.starts_with('p')` + schema=="p"; regression control: same inputs with `response_body_size: None` → 0 probes + empty aggregated, proving body_size is the key. (2) `parse_sqls_user_five_payloads_with_body_size_gap_documented`: the user's 5 payloads (thresholds 79/103/109 true side body=862, 112/115 false side body=875); asserts 5 probes + 1 aggregated record + position-1 `ascii_val=Some(112)` + `decoded_char=None` + `status="insufficient_probes"`. **Gap truth**: `max_true=109, min_false=112, 109+1=110≠112` → strict self-consistency check (`aggregate.rs:466-469`) fails → `insufficient_probes`. This check is a conservative forensic design; the patch **does not relax** it; test comments note: add 110/111 probes to self-consistently reconstruct 'p'.
- **T26-3 Version bump + docs + Release QA + finalize**: four manifests bumped `0.7.1 → 0.7.2` + Cargo.lock ×2 synced; docs/00/02/03/04 + README + README_EN sync-added v0.7.2 descriptions; `docs/versions/0.7.2/更新日志.md` and `docs/qa/versions/0.7.2/QA-审计报告.md` landed; 5-dimension Release QA concluded qa_passed; `cargo test -p ruT0-data-kit-core --release` 376 passed + 3 ignored + tauri 8 passed + npm build 3007 modules 0 error; commit + push origin main.

## v0.7.3 shipped

- **T27-1 Extend detect_sql_blind_features to return body_size + source_ip + dedup + tests**: user reported they could not manually obtain data prefixed with body_size like `862|username=1'%20or%20ascii(substr((database()),1,1))%3E110%23&password=1`. Root cause: the log source's `size` column IS the HTTP response body byte count and the `ip` column IS the source IP (both already present in records), but `detect_sql_blind_features` (`src-tauri/src/commands/log.rs:58-88`) **discarded** body_size and source_ip when scanning, returning only `samples: Vec<String>` (pure SQL text). Fix: extended the return shape to `samples: Vec<{ sql, body_size, source_ip }>` — locates the `size` column + `ip` column by header name from the same row, pairs body_size (empty/`-`→None) + source_ip (empty→None) when `looks_like_blind_probe` matches; deduplicates by the `(sql, body_size, source_ip)` triple to avoid duplicate collection from the query (encoded) and decoded_query (decoded) columns; samples cap still 50. Added 4 Tauri unit tests + updated 3 existing tests to assert the new shape. Return-shape change is zero-break (no frontend caller existed after v0.5.x removed the auto-jump entry).
- **T27-2 Frontend PreprocessView adds "Blind Auto-Extract" button + tauri.js comment update**: `PreprocessView.jsx` jump section gains a "Blind Auto-Extract" button (`handleBlindAutoExtract`) — imports `detectSqlBlindFeatures` + `parseSqlTool`; on click scans the whole table → pairs body_size/ip → builds `body|sql` text → calls `parseSqlTool` → dispatches `SET_SQL_PARSE_INPUT` + `SET_SQL_PARSE_RESULT` + `SET_VIEW: tools` + `SET_TOOLS_ACTIVE_TAB: sql`; `!res.detected` → `message.warning` and no jump; failure → `showError` without clearing. `tauri.js` `detectSqlBlindFeatures` comment updated to the new return shape. Non-log sources lacking `size`/`ip` columns get body_size=null and fall through to `parse_payload`.
- **T27-3 Version bump + docs + Release QA + finalize**: four manifests bumped `0.7.2 → 0.7.3` + Cargo.lock ×2 synced; docs/00/02/03/04 + README + README_EN sync-added v0.7.3 descriptions; `docs/versions/0.7.3/更新日志.md` and `docs/qa/versions/0.7.3/QA-审计报告.md` landed; 5-dimension Release QA concluded qa_passed; `cargo test -p ruT0-data-kit-core --release` 376 passed + 3 ignored + tauri 12 passed + npm build 3007 modules 0 error; commit.

## Installation

### Build dependencies

- **Rust toolchain** (stable, 1.75+ recommended): install via rustup.
- **Tauri v2 CLI**: `cargo install tauri-cli --version "^2"` or use `cargo tauri` (for `cargo tauri dev` / `cargo tauri build`).
- **tshark**: required **only for v0.3.0** (pcap parsing). **Not needed** for v0.1.0
  or v0.2.0. macOS: `brew install wireshark`; Debian/Ubuntu: `apt install tshark`.

### Build from source

```sh
git clone <repo-url> RuT0DataKit
cd RuT0DataKit
cargo build --workspace
```

## Quick Start

### Run the core pipeline (tests)

```sh
cargo build --workspace
# core unit + integration tests
cargo test --workspace
```

### Launch the GUI

> The frontend is built on React + Vite + Ant Design; the first run requires installing npm deps once
> (fetches react / antd / @ant-design/icons / vite).

```sh
cd frontend && npm install && cd ..
cargo tauri dev
```

Build a release bundle (produces `src-tauri/target/release/ruT0-data-kit`, dist embedded):

```sh
cd frontend && npm install && npm run build && cd ..
cargo build --manifest-path src-tauri/Cargo.toml --release
```

The default rule file lives at `rules/default_mask.yaml`; you can reference it directly
or copy and adapt it. Inside the GUI, switch to "custom rules" to load a local YAML.

## Writing Rules

Rule files are YAML with two lists: `validators` and `maskers`. Each rule specifies a
target `field`, a masker / validator name, and optional `params` (only the `custom`
masker and `regex` validator need parameters).

Reference samples:

- `rules/default_mask.yaml`: v0.1.0 built-in masking rules covering every field of
  `sample_mask.csv`.
- `rules/custom_example.yaml`: demonstrates the generic `custom` masker parameters.

### Built-in maskers

| name | rule | notes |
| --- | --- | --- |
| `idcard_mask` | keep first 6 + last 4, middle 8 chars `*` | 18-digit id card; short input passes through |
| `phone_mask` | keep first 3 + last 4, middle 4 chars `*` | 11-digit phone |
| `bankcard_mask` | keep first 6 + last 4, middle `*` | bank card number |
| `email_mask` | keep first/last of local part, middle `*` × (n-2); 1-char local as-is, 2-char as `z*` | domain untouched |
| `name_mask` | 2 chars `X*`; ≥3 chars `X*…*Y` | CJK names |
| `customer_id_mask` | keep first char, rest `*` | |
| `custom` | `keep_prefix` + `keep_suffix` + `mask_char` + `mask_min_len` | generic configurable |
| `regex_replace` | `pattern` regex + `replacement` (supports `$1`/`$2` capture groups) | rich rule; invalid/missing pattern passes value through |
| `regex_extract` | `pattern` regex extracts first matched substring | rich rule; no match passes value through |
| `delete` | returns `""` for any input | rich rule; no params |
| `replace` | returns `with` for any input | rich rule; missing `with` degrades to empty |

> As of v0.1.0, every built-in masker converges onto the `MaskOp` abstract operator (4 generic variants + 6 preset aliases); see `list_mask_op_types()` (src-tauri command) or `docs/02-技术设计文档.md` §2.3 for the alias list.

### Built-in validators

| name | purpose |
| --- | --- |
| `idcard` | id card number format |
| `phone` | phone number format |
| `bankcard` | bank card number format |
| `email` | email address format |
| `mac` | MAC address format (used by v0.2.0 log scanner) |
| `username` | username format (used by v0.2.0 log scanner) |
| `name` | name format |

> As of v0.1.0, every built-in validator converges onto the `ValidateOp` abstract operator (3 generic variants + 7 preset aliases); see `list_validate_op_types()` (src-tauri command) or `docs/02-技术设计文档.md` §2.4 for the alias list.

The v0.1.0 GUI exposes validation via the "Data Validation" view: pick a field + validator → run validation → invalid cells are highlighted → jump to "Data Export" to export the valid/invalid subset. Validators are also reused by the v0.2.0 log-scanning pipeline.

### custom masker example

```yaml
maskers:
  - field: customer_id
    masker: custom
    params:
      keep_prefix: 2
      keep_suffix: 2
      mask_char: "#"
      mask_min_len: 4
```

Result applied to `12345678`: `12####78`.

## Development Guide

### Repository layout

```
RuT0DataKit/
├── crates/
│   └── core/          # ruT0-data-kit-core: pipeline / readers / rules / maskers / validators / report
├── src-tauri/         # Tauri v2 backend (GUI entry)
├── frontend/          # Tauri frontend
├── rules/             # default_mask.yaml / custom_example.yaml
├── tests/             # fixtures (sample files)
└── docs/              # requirements / tech design / task list / version standard
```

Core integration tests live in `crates/core/tests/e2e.rs` and use `CARGO_MANIFEST_DIR`
to walk up two levels and locate `tests/fixtures/samples` and `rules`.

### Common commands

```sh
# build
cargo build --workspace
# full test suite
cargo test --workspace
# E2E integration tests only
cargo test --test e2e
# launch the GUI
cargo tauri dev
```

## Version Roadmap

| Version | Target capability | Current status |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX masking + validation + export + rule management (four-function GUI) + Tauri GUI | Released v0.1.0 |
| v0.2.0 | Log-file parsing + SQLi signature scanning + weak-password / sensitive-field scanning | Released v0.2.0 |
| v0.2.1 | SQLi payload semantic parsing (6 classes) + field restoration (query/path/UA) + `+` decoding | Released v0.2.1 |
| v0.2.2 | Log-scan boolean-blind binary-sequence aggregation/flag recovery + GUI blind-aggregation result card | Released v0.2.2 |
| v0.2.3 | Blind-injection three-axis optimization: true_size mode algorithm + equality/length types + GUI kind Tag/separator highlight/Collapse position-detail | Released v0.2.3 |
| v0.2.4 | Blind-aggregation database-format reconstruction: ReconstructedDatabase cross-correlates 4 read_target classes + GUI antd Table per table with full columns | Released v0.2.4 |
| v0.3.0 | Sensitive-data extraction from pcap (tshark subprocess + HTTP field extraction + double URL decode + auto base64 field decode + sensitive scan + PcapView four-section GUI) | Released v0.3.0 |
| v0.4.0 | 7-view architectural refactor (unified preprocessing for 6 source types + multi-tag rule engine + unified SearchQuery enum + Tools SQL/regex parsing) | Released v0.4.0 |
| v0.4.1 | Five-issue defect fix: data-flow wired + remove FileToolbar from views + ToolsView dropdown + SQL blind-injection feature auto-jump + RegexTool statement → constructed regex | Released v0.4.1 |
| v0.4.2 | Settings module first installment: Sidebar-bottom "设置" entry + tshark multi-platform auto-detection + path configuration persistence + SettingsView UI (+ 3 patch fixes: file-import dialog adds json/sql extensions / sidebar separated from main content scroll / mask+validate dropdowns use user rules instead of operator templates; version unchanged) | Released v0.4.2 |
| v0.4.3 | txt compatibility (PreprocessView supports .txt import) + standalone "Data Extraction" module (ExtractView: file/text input → phone/bankcard/ip extraction → txt/csv/json export, matching PDF spec type_value format) + rule engine de-absolutization (PhoneValidator drops CTF/real prefix whitelist → `^1\d{10}$`; adds IpValidator) | Released v0.4.3 |
| v0.7.0 | Delete Tools/regex-parsing sub-tool (frontend RegexTool/RegexConstructTab + Tauri explain_regex/regex_construct + core regex_explain/regex_construct/regex_template all removed) + PreprocessView header gains a "column-level SQL parsing" button (takes all non-empty rows of that column → `parseSqlTool` → jumps to Tools/Sql reusing the existing UI) | Released v0.7.0 |
| v0.7.1 | SQL-parsing path auto-recognizes URL-encoded input: new `looks_like_url_encoded` (`(?i)%[0-9a-f]{2}`) + `parse_sqls` / `detect_sql_blind_features` decode via `url_decode_twice` only when matched, then feed the probe regex; callers may pass raw input; already-decoded input is unaffected (non-breaking patch) | Released v0.7.1 |
| v0.7.2 | SqlParseTool supports a `body\|sql` prefix to unblock blind reconstruction: frontend `onParse` parses each line with the regex `^(\d+)\|(.*)$` for a `<body_size>\|<sql>` prefix; lines without a prefix keep `null` for backward compat with non-blind payloads; 2 core tests verify the pipeline (self-consistent sequence reconstructs 'p' + user's 5-payload gap asserted honestly); aggregation strict self-consistency check `max_true+1==min_false` not relaxed (non-breaking patch) | Released v0.7.2 |
| v0.7.3 | PreprocessView adds a "Blind Auto-Extract" button + `detect_sql_blind_features` returns body_size/source_ip: extends the existing Tauri command's return shape to `samples: Vec<{ sql, body_size, source_ip }>` — locates the `size` column (HTTP response body byte count) + `ip` column (source IP) by header name from the same row, deduplicates by the `(sql, body_size, source_ip)` triple; PreprocessView's jump section gains a button that scans the whole table → builds `body\|sql` text → calls `parseSqlTool` → jumps to Tools/Sql; non-log sources lacking size/ip columns get body_size=null and fall through to `parse_payload`; 4 new Tauri unit tests; return-shape change is zero-break (no frontend caller existed after v0.5.x removed the auto-jump entry) (non-breaking patch) | Released v0.7.3 |
| v0.8.0 | Wrap-up minor release: two v0.7.4 blind-injection BUG fixes (`SqlParseInput` gains `#[serde(rename_all = "camelCase")]` root cause — Tauri v2 nested-struct fields go through raw serde; without rename_all the frontend camelCase `responseBodySize`/`sourceIp` are silently dropped → 0 probes → empty schema; `detect_sql_blind_features` removes the 50-cap — long logs created gaps → insufficient_probes) + phone custom-prefix feature already delivered in v0.6.8 revision, indexed only (non-breaking minor) | Released v0.8.0 |

See `docs/04-版本标准.md` for the version acceptance criteria.

## Security & Privacy

- All data is processed locally; no remote APIs are called. Samples and rules are
  never uploaded.
- No Python runtime dependency; the core and extensions are native Rust.
- Output paths are chosen explicitly by the user in the GUI; the tool never
  sends anything out on its own.
- pcap parsing (v0.3.0) relies on the system `tshark`, but still runs locally.

## License

TBD (license to be confirmed).
