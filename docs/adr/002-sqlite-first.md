# ADR 002 — SQLite as primary backend, PostgreSQL as optional

**Status**: Accepted

## Context

Personal finance software must be zero-configuration for the common case: one user, one
machine, a file on disk. GnuCash uses a custom XML format for this. We want a real database.

## Decision

SQLite is the default. The `storage` crate abstracts over both via a trait, and the same
`sqlx` query macros work against both dialects. PostgreSQL is the option for multi-user or
team deployments.

## Consequences

- SQLite in WAL mode handles one writer + multiple readers efficiently.
- Users can copy their book by copying a `.db` file — familiar mental model.
- PostgreSQL support shares the same migration files with minor dialect adaptations.
- No support for MySQL — two dialects is already a maintenance burden; three is not worth it.
