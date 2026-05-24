# ADR 005 — Business features as an opt-in crate

**Status**: Accepted

## Context

GnuCash compiles invoicing, AR/AP, payroll, and business reports for every user — even those
who just want to track personal finances. This adds UI complexity and cognitive load.

## Decision

Business features live exclusively in `crates/business`. Personal finance users never
depend on this crate. The `core` accounting types (account tree, transactions) are shared,
but invoice, customer, vendor, and payroll types are in `business` only.

## Consequences

- Personal finance binaries are smaller and simpler.
- Business users opt in by depending on `rustcash-business`.
- The API conditionally registers business route groups when `business` is compiled in.
- Account types `Receivable` and `Payable` exist in `core` (needed for the type system)
  but their business workflow lives in `business`.
