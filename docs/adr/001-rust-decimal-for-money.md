# ADR 001 — Use `rust_decimal` for all monetary amounts

**Status**: Accepted

## Context

Floating-point arithmetic is fundamentally unsuitable for financial calculations.
`0.1 + 0.2 != 0.3` in IEEE 754. Accumulated rounding errors in balance aggregations
are a correctness bug, not a performance trade-off.

## Decision

All monetary amounts throughout RustCash use `rust_decimal::Decimal`. Floats (`f32`, `f64`)
are never used for money. This applies in the domain model, storage (amounts stored as
decimal strings in SQLite, NUMERIC in PostgreSQL), and the API (amounts as strings in JSON).

## Consequences

- Correct arithmetic at the cost of slightly more verbose code.
- `sqlx` has first-class `rust_decimal` support via the `rust_decimal` feature flag.
- JSON amounts are transmitted as strings to avoid JavaScript number precision loss.
- Clippy lint `clippy::float_arithmetic` should be enabled in `core` to catch violations.
