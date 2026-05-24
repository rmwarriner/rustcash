# RustCash — Claude Project Context

## What This Is

A ground-up Rust rewrite of GnuCash's *concepts*, not its code. Not a port — a modern
financial data platform. See `DESIGN.md` for the full architecture.

The GnuCash source is available at `/users/robert/projects/gnucash` for reference, but we
are not porting it line-by-line. We are reimagining what it should be.

## Key Principles (don't violate these)

1. **`core` has zero I/O** — no async, no files, no network, no database
2. **Exact arithmetic only** — `rust_decimal` everywhere; floats are never acceptable for money
3. **API-first** — the HTTP API (`api` crate) is the canonical interface; all UIs are clients
4. **Typed ID newtypes** — `AccountId(Uuid)` not `Uuid`; never mix ID types
5. **Soft deletes** — financial records use `deleted_at`, never hard-deleted
6. **Splits sum to zero** — this invariant is enforced at construction in `core`, not in storage

## Workspace Structure

```
crates/
  core/      ← domain types only (Account, Transaction, Split, Commodity, Price)
  storage/   ← sqlx SQLite + PostgreSQL repositories
  engine/    ← accounting logic (balances, reconciliation, budgeting)
  reports/   ← Report trait + standard report library
  import/    ← file format importers (CSV, OFX, GnuCash XML)
  export/    ← file format exporters
  api/       ← axum HTTP/JSON API server
  plugin/    ← wasmtime WASM plugin host
  cli/       ← clap CLI
  tui/       ← ratatui TUI
  gui/       ← Tauri desktop app
  business/  ← opt-in: invoicing, AR/AP, payroll
  sync/      ← opt-in: CRDT multi-device sync
```

## Tech Stack

- **Rust 2024 edition**
- `rust_decimal` — all monetary amounts
- `chrono` — dates and times
- `uuid` v4 — all IDs
- `sqlx` — database (SQLite primary, PostgreSQL optional)
- `axum` — HTTP API
- `clap` — CLI
- `ratatui` — TUI
- `wasmtime` — WASM plugin host
- `serde` + `serde_json` — serialization
- `tokio` — async runtime
- `thiserror` in lib crates, `anyhow` in binary crates
- `tracing` — structured logging
- `utoipa` — OpenAPI generation
- Tauri — desktop GUI

## Current Development Phase

**Phase 1: Foundation**
- Workspace scaffold → `core` → `storage` → `engine` → `import/csv` + `import/gnucash-xml`

## Dependency Direction

```
core ← storage ← engine ← reports ← api
                         ← import
                         ← export
                                   ← cli
                                   ← tui
                                   ← gui (via api)
```
Nothing flows upward. `gui` does not import `storage`.

## Testing Approach

- Unit tests in each crate (`#[cfg(test)]` modules)
- `proptest` for accounting invariants (split sums, balance consistency)
- Integration tests in `tests/` directories using real SQLite (no mocks for storage)
- GnuCash XML test fixtures for import testing

## GnuCash Reference

GnuCash source at `/users/robert/projects/gnucash` is useful for:
- Understanding the account type taxonomy
- OFX/QIF import logic reference
- Report calculation reference (to verify our numbers match)
- GnuCash XML file format specification

Do NOT copy C code patterns. Refer to logic only.

## What Not To Do

- No floats for money — ever
- No hard deletes of financial records
- No monolithic `use everything::*` re-exports from `core`
- No business logic in `storage` — repositories are CRUD only
- No UI framework imports in `engine` or below
- No mocking the database in integration tests (use real SQLite in-memory)
