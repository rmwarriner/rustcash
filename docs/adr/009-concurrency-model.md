# ADR 009 — Concurrency and conflict model

**Status**: Accepted (revised — see amendment below)

## Context

RustCash is API-first, which means business logic lives in `engine` and is accessible
through the HTTP API. It does **not** mean HTTP is the only permitted transport to that
logic. The CLI on a single-user machine should work without a running server — requiring
one is unnecessary friction for the common case.

## Amendment: CLI accesses storage directly via `engine`

The original decision required the CLI to go through the HTTP API. This has been revised.

**The CLI links `engine` and `storage` directly.** It opens the SQLite file the same way
the API server does — through `open_sqlite`. When only the CLI is running there is one
writer; when both the CLI and the API server are running simultaneously, SQLite's WAL mode
and busy timeout (see §4) handle concurrent writes safely without any application-level
coordination.

The "API-first" principle is preserved in its meaningful form: all business logic,
validation, and accounting invariants live in `engine`. Both the CLI and the API server
are consumers of `engine` — neither embeds logic that the other doesn't have access to.
HTTP is the transport for remote clients (GUI, third-party integrations, multi-user
deployments); it is not a required hop for a local CLI talking to a local file.

| Client | Access path | When appropriate |
|--------|-------------|-----------------|
| CLI | `engine` → `storage` → SQLite file directly | Single-machine use; no server required |
| TUI | `engine` → `storage` → SQLite file directly | Same as CLI |
| GUI (Tauri) | `engine` → `storage` → SQLite file directly | Bundled desktop app |
| HTTP API clients | HTTP → `api` → `engine` → `storage` | Remote access; multi-user; third-party |
| Sync | CRDT merge → `engine` → `storage` | Multi-device (see §7) |

## Decisions

### 1. SQLite WAL mode — always on

WAL (Write-Ahead Logging) is configured at pool initialisation via `open_sqlite` in the
storage crate. It is never left to the caller or to SQLite's default (DELETE mode).

WAL provides:
- Concurrent readers while a write is in progress (readers never block writers)
- Better write throughput for burst workloads
- Crash safety: uncommitted WAL frames are rolled back on next open

### 2. PRAGMA synchronous = NORMAL

With WAL enabled, `synchronous=NORMAL` is safe and provides a good durability/performance
balance. Data is durable after the WAL write completes; the only risk is a power failure
between WAL flush and WAL checkpoint, which would replay the transaction on next open
rather than lose it.

### 3. PRAGMA foreign_keys = ON — always

Enabled at every connection. SQLite disables foreign keys by default; relying on
application code to enforce referential integrity is fragile.

### 4. busy_timeout = 5 seconds

`open_sqlite` sets a 5-second busy timeout. When two writers (e.g., CLI and API server)
attempt to write simultaneously, SQLite retries automatically for up to 5 seconds before
returning `SQLITE_BUSY`. For a personal finance app — low write frequency, short write
transactions — contention is rare and 5 seconds is ample.

If the timeout expires, the error surfaces to the user as a `StorageError::Database`
with SQLite's BUSY code. The message should tell the user to retry; it is not a
corruption condition.

### 5. No optimistic locking version columns

SQLite's WAL serialises concurrent writes; the busy timeout handles the retry. Lost-update
races between truly concurrent writes are extremely unlikely in a personal finance workload.
`version: i64` columns are not added today. If concurrent edit conflicts need to be
surfaced explicitly to users ("someone else modified this while you were editing it"),
that is a UX decision to make when real use cases arise.

### 6. PostgreSQL isolation levels

When PostgreSQL is the backend:
- **Write transactions** (INSERT, UPDATE, DELETE): `SERIALIZABLE` isolation. Prevents
  phantom reads and write skew without explicit `SELECT FOR UPDATE` locking.
- **Read-only transactions** (reports, balance queries): `READ COMMITTED`. Avoids snapshot
  overhead for queries that don't need perfect consistency.

### 7. Sync / CRDT — out of scope here

The `sync` crate uses CRDTs for conflict-free merging of changes across devices. That
conflict model is separate from and layered above the database concurrency model described
here. CRDT merge happens in `sync` before writes reach `storage`.

## Consequences

- All database opens go through `open_sqlite`, which enforces WAL + foreign keys + busy timeout
- The CLI and TUI work without a running API server — no friction for single-user setups
- The API server and CLI can safely share the same SQLite file; SQLite arbitrates writers
- Business logic stays in `engine`; the access path (HTTP vs. direct) is an operational choice
- No schema changes required for this ADR
