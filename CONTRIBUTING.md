# Contributing to RustCash

## First reads

Before contributing code, read in this order:

1. [`DESIGN.md`](DESIGN.md) — architecture, crate dependency rules, tech stack
2. This file — process and conventions
3. [`docs/adr/`](docs/adr/) — decisions that have already been made and why

---

## Development setup

### Prerequisites

```bash
# Rust stable toolchain
rustup update stable
rustup component add llvm-tools-preview  # for coverage

# Cargo tools
cargo install cargo-llvm-cov             # coverage
cargo install cargo-audit                # dependency CVE scanning
cargo install just                       # task runner (or: brew install just)

# Pre-commit hooks
pip install pre-commit
pre-commit install
```

### Verify everything works

```bash
just ci          # runs: fmt-check, clippy, test, coverage, audit
```

---

## Test-driven development (mandatory)

**TDD is not optional.** PRs without tests covering the new behaviour are rejected.

The cycle:

1. **Red** — write a failing test that describes the intended behaviour
2. **Green** — write the minimum code to make it pass
3. **Refactor** — clean up; tests must still pass

### What kind of test to write

| Scenario | Test type |
|---|---|
| Accounting invariants (splits sum to zero, balance arithmetic, amount precision) | Property test (`proptest`) |
| Domain type behaviour (`Account`, `Transaction`, `Split`) | Unit test in `core` |
| Repository CRUD | Integration test against real in-memory SQLite via `sqlx::test` |
| API endpoint request/response | Integration test with `axum::test_helpers` |
| CLI command | Integration test spawning the real binary against a temp DB |

### Property-based tests

Algebraic invariants in `core` and `engine` must have property tests. Examples of invariants that require `proptest`:

- A `Transaction` with any number of splits whose amounts sum to zero is always valid
- Account balance at T is always equal to the sum of split amounts for that account up to T
- Converting an amount to a different commodity and back is within rounding tolerance
- Budget allocation totals never exceed the budget's total envelope

Add `proptest` to `[dev-dependencies]` in the relevant crate's `Cargo.toml`:

```toml
[dev-dependencies]
proptest = { workspace = true }
```

### Storage test isolation

Use `sqlx::test` for repository tests. Each test gets a fresh in-memory SQLite database with all migrations applied automatically:

```rust
#[sqlx::test(migrations = "../../storage/migrations")]
async fn test_insert_account(pool: SqlitePool) {
    // pool is a fresh, isolated database — no shared state between tests
}
```

**Never mock the database.** Real SQLite is fast and catches SQL bugs that mocks hide.

### Test organisation

- Unit tests live in `#[cfg(test)]` modules inside the same file as the code they test
- Integration tests live in `crates/<name>/tests/`
- Shared fixtures and helpers live in `crates/<name>/tests/helpers.rs` or `tests/common/`
- Test names use `snake_case` and describe the scenario: `splits_summing_to_nonzero_are_rejected`

---

## Coverage

Coverage is tracked with `cargo-llvm-cov`. The floor rises as the project matures:

| Phase | Minimum floor |
|---|---|
| Phase 1 (current) | reported, not gated |
| Phase 2 | 60% |
| Phase 3+ | 80% (`core` crate: 90%) |

Run `just coverage` to generate an HTML report. CI reports coverage on every run.

---

## Code style

### Formatting

`cargo fmt` (configured in `rustfmt.toml`). This is not negotiable — CI fails if formatting is off. Run `just fmt` before committing, or let the pre-commit hook do it.

### Clippy

All `clippy::all` warnings are errors in CI. Run `just clippy` locally. If you need to suppress a lint, you must add a comment explaining why:

```rust
#[allow(clippy::too_many_arguments)] // this function is a constructor; splitting it would obscure the structure
```

### No `unsafe`

`unsafe_code = "forbid"` is set at the workspace level. There are no exceptions.

### No floats for money

`f32` and `f64` are never used for monetary amounts. Use `rust_decimal::Decimal`. This is a correctness requirement, not a style preference. See [ADR 001](docs/adr/001-rust-decimal-for-money.md).

### Error types

- Library crates (`core`, `storage`, `engine`, etc.): use `thiserror` with typed error enums
- Binary crates (`cli`, `tui`, `api` main): use `anyhow` for propagation
- Every error variant must be actionable: name what failed, why, and what the caller can do

### Imports

Group imports in this order (enforced by `rustfmt`):
1. `std`
2. External crates
3. Internal crates (`rustcash_*`)
4. Current crate (`crate::`)

---

## Commit messages

Format: **Conventional Commits** with scope.

```
<type>(<scope>): <subject in imperative mood>

<body — explain WHY, not WHAT; the diff shows what>
```

**Types:** `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci`, `perf`

**Scope:** crate name or cross-cutting area — `core`, `storage`, `engine`, `api`, `cli`, `tui`, `gui`, `plugin`, `business`, `sync`, `ci`, `deps`

**Subject:** imperative, lowercase, no period — "add split balance invariant test", not "Added test" or "Adds test"

**Examples:**

```
feat(core): add Transaction::net_for helper

test(engine): add proptest for balance monotonicity

fix(storage): use NullPool in module-level test fixtures

chore(deps): bump sqlx to 0.8.3
```

---

## Branch naming

```
<type>/<short-description>
```

Match the commit type: `feat/transaction-register`, `fix/split-validation`, `ci/coverage-gate`, `docs/adr-006`

Include the GitHub issue number in the PR body (`closes #N`), not the branch name.

---

## Pull request process

### Before opening a PR

1. `just ci` passes locally — all of: fmt-check, clippy, tests, coverage, audit
2. Every changed behaviour has a test
3. Property tests exist for any new accounting invariant

### PR description

Use the template (`.github/PULL_REQUEST_TEMPLATE.md`). The **Summary** and **Test plan** sections are required.

The **smoke test** block in the test plan must be fully reproducible — exact commands, expected output, cleanup. A reviewer must be able to paste and reproduce it without questions.

### Review and merge

- Squash-merge only (linear history required)
- Merge manually after ALL checks are green — do not use GitHub auto-merge
- Delete the branch after merging

---

## Architecture decisions

Significant design decisions go in `docs/adr/` as Architecture Decision Records. Use the existing ADRs as a template. Open a discussion issue before writing an ADR for anything that could go multiple ways — get alignment first.

---

## What gets rejected

- Code without tests for the new behaviour
- Floats used for money (`f32`, `f64` for amounts)
- `unsafe` blocks
- Hard-deletes of financial records
- Business logic in `storage` repositories
- I/O imports (`sqlx`, `tokio`, file I/O) in the `core` crate
- UI framework imports (`axum`, `ratatui`, `clap`) in `engine` or below
- `unwrap()` or `expect()` outside of tests and `main()`
- Commits without conventional commit format
