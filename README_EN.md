# RuT0DataKit

[中文](./README.md)

A local-first data workbench for data-security CTF. Import, view, and export with templates — all locally, without exfiltrating data.

## Current Status

- Version: v1.2.0
- Lifecycle: `release_complete` (T1–T115 all `verified_complete`; Release QA audit passed; tag v1.2.0 pushed)
- v1.2.0 scope: full data workbench — four-region layout + Sheet/Tab workbench + antd Table + SQLite full persistence + Tauri updater (signed) + AI placeholder + settings entry + CSV/JSON/TXT template export + masking (generic template masking + validate-then-mask) + validation (unified validation page, multi-rule + dual Tab + email validation + multi-format birthday) + extraction (5 extraction rules + runtime phone-prefix override) + search/replace + column operations (column transform + Base64 codec + MD5/SHA1/SHA256 hash + case normalization) + undo/redo + external SQLite .db file parsing + rules management (18 built-in rules, sorted by name) + frontend adaptability (breakpoint-aware + collapsible/pinnable panels + responsive layout).
- Historical v0.8.0 code and docs are archived on the `release/v0.8.0` branch and are not reused.

## Quick Start

> Prerequisites: Node 20+, pnpm, Rust (pinned via `rust-toolchain.toml`), and system dependencies per [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
> The commands below have been verified to pass in this repo (see `docs/qa/versions/1.2.0/QA-审计报告.md`).

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

All commands above have been verified passing (`cargo build` zero warnings; `cargo test` 325 passed / 3 ignored; `pnpm build` 3102 modules transformed successfully).

## Project Structure

Three-layer architecture with a one-way dependency (`frontend → src-tauri → crates/core`):

```
RuT0DataKit/
├── Cargo.toml              # workspace root (members: crates/core, src-tauri)
├── crates/core/            # pure-logic engine (datasource/ per-format submodules csv/xlsx/json/txt/sql/pcap + model + pcap/)
│   └── src/processor/rules/  # rules split into template + extract_params submodules (v1.2.0)
├── src-tauri/              # Tauri command layer + SQLite persistence + updater + AI placeholder
│   ├── src/commands/       # data / settings / update / search / columns
│   │   └── processor/      # split into rules_ops / mask_ops / extract_ops / undo_ops (v1.2.0)
│   └── src/db/             # DbManager(Mutex<Connection>) + 6 tables / 5 indexes schema + migration
│       └── (cells/sheets/rules/search/operations submodules)  # v1.2.0 split
├── frontend/               # React shell (antd Layout four-region + Sheet/Tab + antd Table + export + settings page)
│   └── src/
│       ├── state/          # constants/factory/reducer/AppContext (React Context, no prop drilling)
│       ├── hooks/          # useBreakpoint / useRules / useSheetOps (v1.2.0 shared abstractions)
│       └── components/shared/  # ColumnSelect / PhonePrefixSelect / TemplateEditor (v1.2.0 shared primitives)
└── docs/                   # permanent product docs
```

## Capability Boundary

| Delivered | Deferred to v1.3+ |
|---|---|
| Framework shell: four-region layout + Sheet/Tab workbench + antd Table + CSV/XLSX import + SQLite full persistence + updater + AI placeholder + settings page + operation log | tshark + PCAP data source |
| Masking: generic template masking (TemplateParams) + 3 name rules + rules table persistence + RulesPanel + validate-then-mask (`mask_column` optional validation params) | Real AI (v1.4+) |
| Validation: unified validation page (Form.List multi-rule + one button + dual-Tab output) + generic validation rules (Generic variant) + address structured validation + multi-format birthday validation + 7-field row-level validation + custom symbol whitelist + phone prefix config + email validation | |
| Extraction: 5 extraction rules + runtime phone-prefix override + unified prefix input UX | |
| Search / column ops: `search_cells` / `replace_all` + `parse_column_as_json` / `replace_in_column` + `transform_column` (case normalization) | |
| Undo / redo: undo/redo + mask/replace snapshots | |
| Base64 / hash: Base64 column codec + MD5/SHA1/SHA256 column-wise hash transform (case-selectable hex, undoable) + .log import | |
| DB file parsing: external SQLite `.db` / `.sqlite` / `.sqlite3` file parsing (DbReader) | |
| Rules management: 18 built-in rules (sorted by name Unicode codepoint ascending) | |
| Export: CSV / JSON / TXT template (`{字段名}_{值}` → `username_zhangsan`) | |
| Frontend adaptability: breakpoint-aware (`Grid.useBreakpoint`) + collapsible/pinnable SidePanel/AiPanel + responsive layout + auto-collapse | |

## Essential Links

- Docs entry: [`docs/`](./docs/)
- Version standard: [`docs/04-版本标准.md`](./docs/04-版本标准.md)
- v1.2.0 scope: [`docs/versions/1.2.0/规划需求.md`](./docs/versions/1.2.0/规划需求.md)
- v1.2.0 changelog: [`docs/versions/1.2.0/更新日志.md`](./docs/versions/1.2.0/更新日志.md)
- v1.2.0 release notes: [`docs/versions/1.2.0/release.md`](./docs/versions/1.2.0/release.md)
- v1.2.0 QA report: [`docs/qa/versions/1.2.0/QA-审计报告.md`](./docs/qa/versions/1.2.0/QA-审计报告.md)
- License: see the License file in the repo root (to be added)
- Security feedback: via repo Issues or private contact with maintainers; do not disclose sensitive details in public Issues
