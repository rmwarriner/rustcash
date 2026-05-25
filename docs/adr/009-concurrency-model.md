# ADR 009 — Concurrency and conflict model

**Status**: Accepted

## Context

RustCash is API-first: the HTTP server is the canonical interface to all data. The CLI,
TUI, and GUI are clients of that API. This shapes the concurrency model significantly —
the database has one logical writer (the API server), which serializes all mutations.

## Decisions

### 1. The CLI is a thin HTTP client — not a direct database writer

The CLI sends requests to the running API server. It does not open the SQLite file
directly. This means the only process writing to the database at any time is the API
server, which eliminates whole classes of concurrent write conflict.

Consequence: the API server must be running for the CLI to work against a live book.
A future `--offline` mode (read-only, for inspecting exported data) is the only acceptable
exception, and it must be strictly read-only.

### 2. SQLite WAL mode — always on

WAL (Write-Ahead Logging) is configured at pool initialization via `open_sqlite` in the
storage crate. It is never left to the caller or to SQLite's default (DELETE mode).

WAL provides:
- Concurrent readers while a write is in progress (readers never block writers)
- Better write throughput for burst workloads
- Crash safety: uncommitted WAL frames are rolled back on next open

### 3. PRAGMA synchronous = NORMAL

With WAL enabled, `synchronous=NORMAL` is safe and provides a good durability/performance
balance. Data is durable after the WAL write completes; the only risk is a power failure
between WAL flush and WAL checkpoint, which would replay the transaction on next open
rather than lose it.

`synchronous=FULL` is not used — it adds a `fsync` on every WAL frame write, which
degrades performance without meaningful benefit on modern hardware running a personal
finance app.

### 4. PRAGMA foreign_keys = ON — always

Enabled at every connection. SQLite disables foreign keys by default; relying on
application code to enforce referential integrity is fragile.

### 5. No optimistic locking version columns — for now

Lost-update races between concurrent API requests are the only concurrency hazard once
the CLI-is-a-client decision is made. SQLite serializes writes at the WAL level, so two
concurrent mutating API requests cannot both commit conflicting writes — one will retry
automatically (SQLite returns SQLITE_BUSY, sqlx retries).

`version: i64` columns are not added to domain tables today. If concurrent edit conflicts
need to be surfaced to users (e.g., "someone else modified this transaction while you were
editing it"), that is a UX decision that can be made when the API and GUI exist and real
use cases are known.

### 6. PostgreSQL isolation levels

When PostgreSQL is the backend:
- **Write transactions** (INSERT, UPDATE, DELETE): `SERIALIZABLE` isolation. Prevents
  phantom reads and write skew without explicit `SELECT FOR UPDATE` locking.
- **Read-only transactions** (reports, balance queries): `READ COMMITTED`. Avoids snapshot
  overhead for queries that don't need perfect consistency.

### 7. Sync / CRDT — out of scope here

The `sync` crate will use CRDTs for conflict-free merging of changes from multiple devices.
That conflict model is separate from and layered above the database concurrency model
described here. CRDT merge happens in the `sync` crate before writes reach the storage
layer.

## Consequences

- All database opens go through `open_sqlite`, which enforces WAL + foreign keys
- The CLI cannot be used without a running API server (for writes)
- Concurrent read performance is good; write throughput is sufficient for a personal
  finance app (one user, low write frequency)
- No schema changes required for this ADR
