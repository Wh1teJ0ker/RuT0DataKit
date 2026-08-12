# RuT0DataKit

[中文](./README.md)

A local-first data workbench for data-security CTF. Import, view, and export with templates — all locally, without exfiltrating data.

## Current Status

- Version: v1.1.4
- Lifecycle: `qa_passed` (T1–T79 all `verified_complete`; Phase 7 static + app startup verified + Phase 8 GUI interaction acceptance all passed; Release QA R1/R2/R3/R4 incremental audits passed; branch integration complete)
- v1.1.4 scope: full data workbench — four-region layout + Sheet/Tab workbench + antd Table + SQLite full persistence + Tauri updater (signed) + AI placeholder + settings entry + CSV/JSON/TXT template export + masking (generic template masking) + validation (unified validation page, multi-rule + dual Tab) + extraction (5 extraction rules + runtime phone-prefix override) + search/replace + column operations + undo/redo + Base64 column codec + MD5/SHA1/SHA256 hash column transform + external SQLite .db file parsing + rules management (17 built-in rules, sorted by name).
- Historical v0.8.0 code and docs are archived on the `release/v0.8.0` branch and are not reused.

## Quick Start

> Prerequisites: Node 20+, pnpm, Rust (pinned via `rust-toolchain.toml`), and system dependencies per [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
> The commands below have been verified to pass in this repo (see `docs/qa/versions/1.1.4/QA-审计报告.md` §6 + §9.2 + §10).

```sh
# 1. Get the source
git clone <repo-url> && cd RuT0DataKit

# 2. Install frontend dependencies
pnpm --prefix frontend install

# 3. Start local dev (first run compiles SQLite bundled; takes longer)
cargo tauri dev
```

## Basic Verification

```sh
# Rust compile check
cargo check --workspace

# Unit tests (persistence / datasource / pcap detection)
cargo test --workspace

# Frontend build
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
```

All commands above have been verified passing (`cargo check` Finished; `cargo test` 317 passed / 3 ignored; `pnpm build` 3083 modules transformed successfully).

## Project Structure

Three-layer architecture with a one-way dependency (`frontend → src-tauri → crates/core`):

```
RuT0DataKit/
├── Cargo.toml              # workspace root (members: crates/core, src-tauri)
├── crates/core/            # pure-logic engine (datasource/ per-format submodules csv/xlsx/json/txt/sql/pcap + model + pcap/)
├── src-tauri/              # Tauri command layer + SQLite persistence + updater + AI placeholder
│   ├── src/commands/       # data (import_file/get_sheet_data/ai_suggest/invoke_ai_op)
│   │                       # settings (detect_tshark/load_tshark_path/save_tshark_path)
│   │                       # update (check_update/install_update)
│   └── src/db/             # DbManager(Mutex<Connection>) + 5 tables / 3 indexes schema + migration
├── frontend/               # React shell (antd Layout four-region + Sheet/Tab + antd Table + export + settings page)
│   └── src/state/          # constants/factory/reducer/AppContext (React Context, no prop drilling)
└── docs/                   # permanent product docs
```

## v1.1.4 Capability Boundary

| Delivered | Deferred to v1.2+ |
|---|---|
| Framework shell: four-region layout + Sheet/Tab workbench + antd Table + CSV/XLSX import + SQLite full persistence + updater + AI placeholder + settings page + operation log | Full rules engine |
| Masking: generic template masking (TemplateParams) + 3 name rules + rules table persistence + RulesPanel | Status highlight extension (`invalid/masked/hit`) |
| Validation: unified validation page (Form.List multi-rule + one button + dual-Tab output) + generic validation rules (Generic variant) + address structured validation + birthday separator cleanup + 7-field row-level validation + custom symbol whitelist + phone prefix config | tshark + PCAP |
| Extraction: 5 extraction rules + runtime phone-prefix override + unified prefix input UX | Real AI (v1.4+) |
| Search / column ops: `search_cells` / `replace_all` + `parse_column_as_json` / `replace_in_column` | |
| Undo / redo: undo/redo + mask/replace snapshots | |
| Base64 / hash: Base64 column codec + MD5/SHA1/SHA256 column-wise hash transform (undoable) + .log import | |
| DB file parsing: external SQLite `.db` / `.sqlite` / `.sqlite3` file parsing (DbReader) | |
| Rules management: 17 built-in rules (sorted by name Unicode codepoint ascending) | |
| Export: CSV / JSON / TXT template (`{字段名}_{值}` → `username_zhangsan`) | |

## Essential Links

- Docs entry: [`docs/`](./docs/)
- Version standard: [`docs/04-版本标准.md`](./docs/04-版本标准.md)
- v1.1.4 scope: [`docs/versions/1.1.4/规划需求.md`](./docs/versions/1.1.4/规划需求.md)
- v1.1.4 changelog: [`docs/versions/1.1.4/更新日志.md`](./docs/versions/1.1.4/更新日志.md)
- v1.1.4 QA report: [`docs/qa/versions/1.1.4/QA-审计报告.md`](./docs/qa/versions/1.1.4/QA-审计报告.md)
- Public releases: none yet (v1.1.4 passed Release QA, pending finalize/release)
- License: see the License file in the repo root (to be added)
- Security feedback: via repo Issues or private contact with maintainers; do not disclose sensitive details in public Issues
