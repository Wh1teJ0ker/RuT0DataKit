# RuT0DataKit

A fast data-masking / parsing toolkit for data-security CTF scenarios. The core is
implemented in Rust; the desktop UI is built on Tauri v2. All processing happens
locally — no samples or rules are ever uploaded. This repository targets the
"sensitive-data quick sanitization / parsing" needs in data-security contests and
red-team workflows, and does not depend on a Python runtime.

> **Current status: v0.2.4 log-scan blind-injection aggregation database-format reconstruction (T4-1 ~ T4-4 all verified_complete, release_complete).** v0.2.4 upgrades the v0.2.3 blind-aggregation display from "per-read_target cards" to "database-format reconstruction":
> - **Backend structs + algorithm (T4-1)**: `blind_aggregator` adds `ReconstructedDatabase` / `ReconstructedTable` / `ReconstructedRow` structs + `BlindAggregator::reconstruct_database` associated function. Three-step algorithm: Step1 classify each `AggregatedResult`'s `read_target` into 4 pattern classes (`schema`=database() / `table_list`=group_concat(table_name) from information_schema.tables / `column_list`=group_concat(column_name) ... where table_name='X' / `row_data`=group_concat(col1,0xNN,col2,...) from <table>; unmatched → `unmatched_results`); Step2 assemble schema/tables/columns/rows (schema direct → table_list names matched to column_list by `table_name='T'` predicate for full columns → row_data matched by `from T` and parsed: `,` split rows + `column_separator` (decoded from `0xNN`) split cols → projected onto full columns, unfetched = None); Step3 unmatched fallback.
> - **Pipeline passthrough (T4-2)**: `scan_log` calls `reconstruct_database` at the end and writes `Report.extra.reconstructed_database` (alongside `blind_aggregation`, backward compatible with v0.2.3). Fixture auto-reconstruction: schema="person" / 1 table person_data / 7 columns / ≥2 rows / row[0].id=Some("1").
> - **GUI display (T4-3)**: LogView adds section ③.5a "还原数据库视图" Card before ③.5 — antd `Table` per table (full columns as headers, unfetched cells render `-`) + unmatched fallback list + Empty state; section ③.5 is renamed "原始聚合明细" and kept as a fallback with the existing per-read_target cards.
> The `blind_aggregation` array is preserved unchanged; the new `reconstructed_database` key and the new struct fields `schema` (Option) / `column_separator` (Option) are all `#[serde(skip_serializing_if)]`, backward compatible with v0.2.3. See `docs/04-版本标准.md` for the version status convention.

## Features

| Version | Capability | Status |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX tabular data masking + validation + export + rule management (four-function GUI) | Released v0.1.0 |
| v0.2.0 | Log-file parsing + SQLi signature scanning + weak-password / sensitive-field scanning | Released v0.2.0 |
| v0.2.1 | SQLi payload semantic parsing (6 classes) + field restoration (query/path/UA) + `+` decoding | Released v0.2.1 |
| v0.2.2 | Log-scan boolean-blind binary-sequence aggregation/flag recovery + GUI blind-aggregation result card | Released v0.2.2 |
| v0.2.3 | Blind-injection three-axis optimization: true_size mode algorithm + equality/length types + GUI kind Tag/separator highlight/Collapse position-detail | Released v0.2.3 |
| v0.2.4 | Blind-aggregation database-format reconstruction: ReconstructedDatabase cross-correlates 4 read_target classes + GUI antd Table per table with full columns | Released v0.2.4 |
| v0.3.0 (planned) | Sensitive-data extraction from pcap captures (depends on system `tshark`) | Planned |

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
| v0.3.0 | Sensitive-data extraction from pcap (depends on tshark) | Planned |

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
