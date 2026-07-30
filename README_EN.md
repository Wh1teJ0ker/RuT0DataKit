# RuT0DataKit

[中文](./README.md)

A local-first data workbench for data-security CTF. Import, mask, validate, extract, apply rules, and export — all locally, without exfiltrating data.

## Current Status

- Version: v1.0.0 (framework stage)
- Lifecycle: in development (tasks T1–T8 verified complete, pending end-to-end + Release QA)
- v1.0.0 scope: pure framework shell — four-region layout + Sheet/Tab workbench + antd Table + SQLite full persistence + Tauri updater (signed) + AI placeholder + settings entry. Business capabilities (masking / validation / extraction / rules / search / Tools / PCAP / real AI) are deferred to v1.1+.
- Historical v0.8.0 code and docs are archived on the `release/v0.8.0` branch and are not reused.

## Quick Start

> Prerequisites: Node 22+, pnpm, Rust (pinned via `rust-toolchain.toml`), and system dependencies per [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
> The commands below have been verified to pass in this repo (see `docs/qa/versions/1.0.0/QA-审计报告.md` §6).

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

# Unit tests (persistence layer, etc.)
cargo test --workspace

# Frontend build
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
```

All commands above have been verified passing (`cargo check` Finished; `cargo test` 9 db + 3 datasource all PASS; `pnpm build` 3069 modules transformed successfully).

## Project Structure

Three-layer architecture with a one-way dependency (`frontend → src-tauri → crates/core`):

```
RuT0DataKit/
├── Cargo.toml              # workspace root (members: crates/core, src-tauri)
├── crates/core/            # pure-logic engine (v1.0.0 skeleton: datasource CSV/XLSX parsing + model + processor)
├── src-tauri/              # Tauri command layer + SQLite persistence + updater + AI placeholder
│   ├── src/commands.rs     # import_file / get_sheet_data / check_update / install_update / ai_suggest / invoke_ai_op
│   └── src/db/             # DbManager(Mutex<Connection>) + 5 tables / 3 indexes schema + migration
├── frontend/               # React shell (antd Layout four-region + Sheet/Tab + antd Table + settings page)
└── docs/                   # permanent product docs
```

## v1.0.0 Capability Boundary

| Delivered | Deferred to v1.1+ |
|---|---|
| Four-region shell (top toolbar + left capability panel + center workbench + right AI panel) | Masking |
| Sheet/Tab workbench (new / switch / close / rename) | Validation |
| antd Table (row checkbox + range select / column visibility / column drag / 50-row paging) | Extraction |
| CSV / XLSX import flow (detect_format → DB cells → Table) | Rules |
| SQLite full persistence (5 tables + 3 indexes + idempotent migration) | Search |
| Tauri updater auto-check (Ed25519 signed, silent offline downgrade) | Tools |
| AI IPC contract placeholder (`ai_suggest` / `invoke_ai_op` return v1.1+ error) | PCAP |
| Settings page (updater check / tshark path placeholder / DB path / about) | Real AI (v1.4+) |
| Operation log (`import` record) | Status highlight `invalid/masked/hit` |

## Essential Links

- Docs entry: [`docs/`](./docs/)
- Version standard: [`docs/04-版本标准.md`](./docs/04-版本标准.md)
- v1.0.0 scope: [`docs/versions/1.0.0/规划需求.md`](./docs/versions/1.0.0/规划需求.md)
- v1.0.0 changelog: [`docs/versions/1.0.0/更新日志.md`](./docs/versions/1.0.0/更新日志.md)
- v1.0.0 QA report: [`docs/qa/versions/1.0.0/QA-审计报告.md`](./docs/qa/versions/1.0.0/QA-审计报告.md)
- Public releases: none yet (v1.0.0 not released)
- License: see the License file in the repo root (to be added)
- Security feedback: via repo Issues or private contact with maintainers; do not disclose sensitive details in public Issues
