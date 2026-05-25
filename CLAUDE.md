# RustCash — Claude Project Context

## What This Is

A ground-up Rust rewrite of GnuCash's *concepts*, not its code. Not a port — a modern
financial data platform. See `DESIGN.md` for the full architecture.

The GnuCash source (https://github.com/Gnucash/gnucash) is a useful reference, but we
are not porting it line-by-line. We are reimagining what it should be.

## Key Principles (don't violate these)

1. **`core` has zero I/O** — no async, no files, no network, no database
2. **Exact arithmetic only** — `rust_decimal` everywhere; floats are never acceptable for money
3. **API-first** — the HTTP API (`api` crate) is the canonical interface; all UIs are clients
4. **Typed ID newtypes** — `AccountId(Uuid)` not `Uuid`; never mix ID types
5. **Soft deletes** — financial records use `deleted_at`, never hard-deleted
6. **Splits sum to zero** — this invariant is enforced at construction in `core`, not in storage
7. **TDD mandatory** — failing test first, then minimal code, then refactor. No exceptions.
8. **No `unsafe`** — `unsafe_code = "forbid"` at workspace level

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
- `proptest` — property-based testing (required for accounting invariants)
- `serde` + `serde_json` — serialization
- `tokio` — async runtime
- `thiserror` in lib crates, `anyhow` in binary crates
- `tracing` — structured logging
- Tauri — desktop GUI

## Current Development Phase

**Phase 1: Foundation — complete.** Domain model, all storage repositories (52 tests), and core engine services (24 tests) are implemented. ADRs 001–012 are in place.

**Phase 2: CLI & API — next.**
- `import/csv`: CSV importer for real-world data ingestion
- Working account/transaction CRUD via CLI commands
- Complete REST API with OpenAPI spec
- Income Statement and Balance Sheet reports

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

## Development Process

### TDD — mandatory

Write the failing test first. Then minimal code to make it pass. Then refactor.
PRs without tests for new behaviour will be rejected.

**Test types by scenario:**
- Accounting invariants (split sums, balance arithmetic, amount precision) → `proptest`
- Domain type behaviour → unit test in `core`
- Repository CRUD → `sqlx::test` with real in-memory SQLite
- API endpoints → `axum::test_helpers`
- CLI commands → integration test spawning the real binary

**Never mock the database.** Real SQLite (via `sqlx::test`) is fast and catches bugs.

### Running the project locally

```bash
just ci          # full CI suite: lint, test, coverage, audit
just test        # tests only
just lint        # fmt-check + clippy
just coverage    # HTML coverage report in target/llvm-cov/html/
just audit       # cargo audit for CVEs
just serve       # start API server at http://127.0.0.1:8080
```

### CI surface

CI runs on every push and PR via `.github/workflows/ci.yml`:
- **lint**: `cargo fmt --check` + `cargo clippy -- -D warnings` (always)
- **test**: `cargo llvm-cov` (conditional on Rust source changes)
- **secrets-scan**: gitleaks (always)
- **audit**: `cargo audit` (conditional on dep changes)
- **all-checks-passed**: aggregate gate required by branch protection

**`labeler.yml`** auto-applies `area:<package>` and `area:{ci,docs,tooling}` labels to PRs based on changed paths. Path mapping in `.github/labeler.yml` mirrors the `changes` job's filters.

Watch CI with the Monitor tool, not a Bash polling loop.

When editing any `.github/workflows/*.yml`, the security pre-commit hook fires informationally. Don't reference untrusted PR-controlled context (`github.event.issue.title`, `github.event.pull_request.body`, etc.) inside `run:` blocks; bind them through `env:` variables when needed.

### Commit messages

Conventional Commits with scope:
```
feat(core): add Transaction::net_for helper
test(engine): add proptest for balance monotonicity
fix(storage): use correct pool type in test fixtures
chore(deps): bump sqlx to 0.8.3
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci`, `perf`
Scopes: crate name — `core`, `storage`, `engine`, `api`, `cli`, `tui`, `plugin`, `business`, `sync`, `ci`, `deps`

### Branch naming

```
<type>/<short-description>
feat/transaction-register
fix/split-validation
ci/coverage-gate
docs/adr-006
```

### PR and merge

- Squash-merge only after all checks are green
- Never use GitHub auto-merge — merge manually
- Delete branch after merging
- Use the PR template (`.github/PULL_REQUEST_TEMPLATE.md`)
- Smoke test block in the PR body must be fully reproducible

### After opening a PR

Watch CI with the **`Monitor`** tool, not `Bash run_in_background` — Monitor emits one event per check transition; bash watch goes silent until the very end. The poll script in [`feedback_auto_pr_workflow.md` user memory](../../.claude/projects/-Users-robert-Projects-rustcash/memory/feedback_auto_pr_workflow.md) is the canonical shape.

When the Monitor reports `ALL_GREEN`, squash-merge with `gh pr merge <N> --squash --delete-branch`, sync `main` locally (`git checkout main && git pull --ff-only && git branch -D <branch>`), then surface the next slice.

**Do not** enable repo-level auto-merge or pass `--auto` to `gh pr merge`. Past experience is that auto-merge fired before required checks gated the merge — a manual squash-merge from this side after `ALL_GREEN` is the safer pattern.

## What goes where

- **Bugs and ideas**: GitHub Issues on `rmwarriner/tulip-accounting`. Don't invent parallel TODO files in the repo. The repo has issue templates (`.github/ISSUE_TEMPLATE/{bug,feature}.yml`) for human use; when filing programmatically, follow the same body shape (Why / Scope / Out of scope / Acceptance) recent issues use.
- **Vulnerability reports**: private security advisory per [`SECURITY.md`](SECURITY.md), not a regular issue. The engineering threat model lives separately in [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md).
- **Architectural decisions** (new dependency, deviation from established pattern, design tradeoff): an ADR in [`docs/adrs/`](docs/adrs). Look at existing ones for the format.
- **Phase tracking**: update [`docs/PHASE_STATUS.md`](docs/PHASE_STATUS.md) at the end of a slice — it's the source of truth for project state.

## Testing Approach

- Unit tests in each crate (`#[cfg(test)]` modules)
- `proptest` for accounting invariants (split sums, balance consistency)
- Integration tests in `crates/<name>/tests/` using real SQLite (no mocks)
- `sqlx::test` attribute for repository tests — each gets a fresh DB with migrations applied
- GnuCash XML test fixtures for import testing

## Code Style

- `rustfmt.toml` controls formatting — run `just fmt` or let pre-commit do it
- All `clippy::all` warnings are errors — run `just clippy`
- No `unsafe` — workspace-level forbid
- Error messages must be actionable: name what failed, why, and what to do
- `thiserror` in libs, `anyhow` in binaries

## GnuCash Reference

GnuCash source (https://github.com/Gnucash/gnucash) is useful for:
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
- No `unwrap()` or `expect()` outside of tests and `main()`
- No PRs without tests
- No `unsafe` blocks

## When in doubt

Open an issue and discuss before writing code, especially for anything beyond a small bugfix or doc edit. The maintainer is solo and prefers a five-minute conversation to a 200-line PR that has to be unwound.
