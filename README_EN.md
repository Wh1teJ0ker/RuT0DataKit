# RuT0DataKit

[中文](./README.md)

A local-first data workbench for data-security CTF. Import, mask, validate, extract, apply rules, and export — all locally, without exfiltrating data.

## Current Status

- Version: v1.0.0 (framework stage, `planned`, in development, not yet released)
- Lifecycle: in development
- v1.0.0 scope: pure framework shell — four-region layout + Sheet/Tab workbench + antd Table + SQLite full persistence + Tauri updater (signed) + AI placeholder + settings entry. Business capabilities (masking / validation / extraction / rules / search / Tools / PCAP / real AI) are deferred to v1.1+.
- Historical v0.8.0 code and docs are archived on the `release/v0.8.0` branch and are not reused.

## Quick Start

> Prerequisites: Node 22+, pnpm, Rust (pinned via `rust-toolchain.toml`), and system dependencies per [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

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

# Frontend build
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build

# Unit tests (persistence layer, etc.)
cargo test --workspace
```

> The commands above become available as implementation progresses; v1.0.0 is still in the planning stage, so some commands may fail until the project is initialized. Refer to actual task progress.

## Essential Links

- Docs entry: [`docs/`](./docs/)
- Version standard: [`docs/04-版本标准.md`](./docs/04-版本标准.md)
- v1.0.0 scope: [`docs/versions/1.0.0/规划需求.md`](./docs/versions/1.0.0/规划需求.md)
- Public releases: none yet (v1.0.0 not released)
- License: see the License file in the repo root (to be added)
- Security feedback: via repo Issues or private contact with maintainers; do not disclose sensitive details in public Issues
