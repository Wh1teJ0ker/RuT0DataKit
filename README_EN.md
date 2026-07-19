# RuT0DataKit

A fast data-masking / parsing toolkit for data-security CTF scenarios. The core is
implemented in Rust; the desktop UI is built on Tauri v2. All processing happens
locally — no samples or rules are ever uploaded. This repository targets the
"sensitive-data quick sanitization / parsing" needs in data-security contests and
red-team workflows, and does not depend on a Python runtime.

> **Current status: v0.1.0 UI refactor + rule-engine refactor patch + dual-state separation / boundary BUG fix (T0-21~T0-26 + follow-up fix patch) is complete.** v0.1.0 ships four-function navigation (Data Masking / Data Validation / Data Export / Rule Management) + abstract-operator rule engine (MaskOp/ValidateOp + preset library + preview API) + rule management single-list + Drawer + dynamic preview + four-segment vertical views for masking/validation + single-Table integrated export + column-selective masking + validation GUI + export column-selection/order/CSV+XLSX + cross-view state persistence + rules/mapping dual-state separation (MaskView/ValidateView temporary mappings do not pollute the global rule library) + `MaskOp::Template` boundary BUG fix (when `keep_prefix + keep_suffix == value length`, correctly outputs only the mask segment, no longer mis-concatenates the original value). See `docs/04-版本标准.md`
> for the version status convention.

## Features

| Version | Capability | Status |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX tabular data masking + validation + export + rule management (four-function GUI) | Released v0.1.0 |
| v0.1.1 (planned) | Log-file parsing (regex extraction of mac / username / id_card / phone, etc.) | Planned |
| v0.1.2 (planned) | Sensitive-data extraction from pcap captures (depends on system `tshark`) | Planned |

What v0.1.0 ships:
- Core pipeline: `detect_type` → `SourceReader` → `mask_pipeline` / `mask_pipeline_selected` (row selection, backward compatible) / `mask_pipeline_columns` (column selection) → `validate_pipeline` (validation) → `write_masked_csv` / `export_records_csv` / `export_records_xlsx`
- 7 built-in maskers + 1 generic `custom` masker + 4 rich-rule maskers (`regex_replace` / `regex_extract` / `delete` / `replace`)
- 7 built-in validators (reused by the v0.1.1 log scanner)
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

## Installation

### Build dependencies

- **Rust toolchain** (stable, 1.75+ recommended): install via rustup.
- **Tauri v2 CLI**: `cargo install tauri-cli --version "^2"` or use `cargo tauri` (for `cargo tauri dev` / `cargo tauri build`).
- **tshark**: required **only for v0.1.2** (pcap parsing). **Not needed** for v0.1.0
  or v0.1.1. macOS: `brew install wireshark`; Debian/Ubuntu: `apt install tshark`.

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
| `mac` | MAC address format (used by v0.1.1 log scanner) |
| `username` | username format (used by v0.1.1 log scanner) |
| `name` | name format |

> As of v0.1.0, every built-in validator converges onto the `ValidateOp` abstract operator (3 generic variants + 7 preset aliases); see `list_validate_op_types()` (src-tauri command) or `docs/02-技术设计文档.md` §2.4 for the alias list.

The v0.1.0 GUI exposes validation via the "Data Validation" view: pick a field + validator → run validation → invalid cells are highlighted → jump to "Data Export" to export the valid/invalid subset. Validators are also reused by the v0.1.1 log-scanning pipeline.

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
| v0.1.1 | Log-file parsing (regex scan for sensitive fields) | Planned |
| v0.1.2 | Sensitive-data extraction from pcap (depends on tshark) | Planned |

See `docs/04-版本标准.md` for the version acceptance criteria.

## Security & Privacy

- All data is processed locally; no remote APIs are called. Samples and rules are
  never uploaded.
- No Python runtime dependency; the core and extensions are native Rust.
- Output paths are chosen explicitly by the user in the GUI; the tool never
  sends anything out on its own.
- pcap parsing (v0.1.2) relies on the system `tshark`, but still runs locally.

## License

TBD (license to be confirmed).
