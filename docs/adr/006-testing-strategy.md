# ADR 006 — Testing strategy: TDD, property tests, real SQLite

**Status**: Accepted

## Context

Financial software has a very low tolerance for correctness bugs. A miscomputed balance or
a truncated decimal is not a UX problem — it's wrong money. The testing strategy must be
strong enough to catch regressions in accounting logic before they reach users.

## Decision

**TDD is mandatory.** The cycle is: red → green → refactor. No PR merges without tests.

**Property-based tests** (via `proptest`) are required for all algebraic invariants in `core`
and `engine`. Example invariants:
- A `Transaction` is valid if and only if its splits sum to exactly zero
- Account balance at T is the sum of split amounts for that account with date ≤ T
- Converting an amount to another commodity and back stays within rounding tolerance
- Budget allocations never exceed their envelope total

**Real SQLite for storage tests** (via `sqlx::test`). Each test gets an isolated in-memory
database with all migrations applied automatically:

```rust
#[sqlx::test(migrations = "../../storage/migrations")]
async fn test_find_account(pool: SqlitePool) { ... }
```

Mocking the database is forbidden. Mocks hide SQL bugs, migration drift, and constraint
violations that only manifest against a real engine.

**Integration tests for CLI.** The CLI binary is tested by spawning it as a subprocess
against a temp database. This is the only honest way to test exit codes, stdout/stderr,
and argument parsing.

**Coverage floor** rises with the project:
- Phase 1: reported, not gated (mostly stubs)
- Phase 2: 60%
- Phase 3+: 80% overall, 90% for `core`

## Consequences

- Every accounting invariant has an executable specification via `proptest`
- Storage bugs surface in test (not in production) because tests hit real SQL
- Coverage is tracked from day one so the floor can be raised incrementally
- CI runs `cargo llvm-cov` on every code change
