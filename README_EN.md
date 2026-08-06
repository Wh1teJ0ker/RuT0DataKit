# RuT0DataKit

[中文](./README.md)

A local-first data workbench for data-security CTF. Import, view, and export with templates — all locally, without exfiltrating data.

## Current Status

- Version: v1.0.0
- Lifecycle: `qa_passed` (T1–T14 all `verified_complete`; Phase 7 static + app startup verified + Phase 8 GUI interaction acceptance all passed; Release QA audit conclusion `qa_passed`, pending finalize/release)
- v1.0.0 scope: pure framework shell — four-region layout + Sheet/Tab workbench + antd Table + SQLite full persistence + Tauri updater (signed) + AI placeholder + settings entry + CSV/JSON/TXT template export. Business capabilities (masking / validation / extraction / rules / search / Tools / PCAP / real AI) are deferred to v1.1+.
- Historical v0.8.0 code and docs are archived on the `release/v0.8.0` branch and are not reused.

## Quick Start

> Prerequisites: Node 20+, pnpm, Rust (pinned via `rust-toolchain.toml`), and system dependencies per [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
> The commands below have been verified to pass in this repo (see `docs/qa/versions/1.0.0/QA-审计报告.md` §6 + §9.2 + §10).

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

All commands above have been verified passing (`cargo check` Finished; `cargo test` 11 passed / 2 ignored; `pnpm build` 3078 modules transformed successfully).

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

## v1.0.0 Capability Boundary

| Delivered | Deferred to v1.1+ |
|---|---|
| Four-region shell (top toolbar + left capability panel + center workbench + right AI panel) | Masking |
| Sheet/Tab workbench (new / switch / close / rename) | Validation |
| antd Table (row checkbox + range select / column visibility / column drag / 50-row paging) | Extraction |
| CSV / XLSX import flow (detect_format → DB cells → Table) | Rules |
| Export: CSV / JSON / TXT template (`{字段名}_{值}` → `username_zhangsan`) | Search |
| SQLite full persistence (5 tables + 3 indexes + idempotent migration) | Tools |
| Tauri updater auto-check (Ed25519 signed, silent offline downgrade) | PCAP |
| AI IPC contract placeholder (`ai_suggest` / `invoke_ai_op` return "capability under development" error) | Real AI (v1.4+) |
| Settings page (updater check / tshark path placeholder / DB path / about) | Status highlight `invalid/masked/hit` |
| Operation log (`import` record) | |

## Essential Links

- Docs entry: [`docs/`](./docs/)
- Version standard: [`docs/04-版本标准.md`](./docs/04-版本标准.md)
- v1.0.0 scope: [`docs/versions/1.0.0/规划需求.md`](./docs/versions/1.0.0/规划需求.md)
- v1.0.0 changelog: [`docs/versions/1.0.0/更新日志.md`](./docs/versions/1.0.0/更新日志.md)
- v1.0.0 QA report: [`docs/qa/versions/1.0.0/QA-审计报告.md`](./docs/qa/versions/1.0.0/QA-审计报告.md)
- Public releases: none yet (v1.0.0 passed Release QA, pending finalize/release)
- License: see the License file in the repo root (to be added)
- Security feedback: via repo Issues or private contact with maintainers; do not disclose sensitive details in public Issues
