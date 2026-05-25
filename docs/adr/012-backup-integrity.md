# ADR 012 — Backup and data integrity

**Status**: Accepted

## Context

Financial data loss is catastrophic. Unlike most applications, there is no "undo" for a
corrupted database that holds years of transaction history. The integrity and backup
strategy must be decided and enforced before any real data exists.

## Decisions

### 1. SQLite WAL mode and synchronous=NORMAL — always

See ADR 009. `open_sqlite` in the storage crate enforces both on every connection.
WAL mode means the database file is always in a consistent state; a crash during a write
rolls back on next open rather than corrupting the file.

### 2. PRAGMA quick_check — automatic on every database open

`open_sqlite` runs `PRAGMA quick_check` before returning the pool. This pragma:
- Checks the B-tree structure, free-list, and page counts
- Does **not** read every data cell (that is `integrity_check`)
- Returns in milliseconds even for large databases
- Returns `["ok"]` on success; returns a list of error descriptions on failure

A single corrupt page that breaks the B-tree structure will be caught before the
application reads any data from that file.

### 3. PRAGMA integrity_check — explicit command only

A full integrity check reads every data page and validates every record. It can take
several seconds on a large database and is disproportionate to the risk for a routine
open. It is available as:

```
rustcash db check
```

Users should run this after a suspected filesystem problem, before a major export, or
as a periodic (e.g., monthly) health check. It is not run automatically.

### 4. On corruption: hard stop with recovery instructions

If `quick_check` (or a full `integrity_check`) reports problems, the application refuses
to open the database. The error message includes:

1. The specific problems reported by SQLite
2. The absolute path to the database file
3. Numbered recovery steps:
   - Back up the file immediately (even in its current state) before any tool touches it
   - Run `rustcash db check` for a full report
   - Restore from a known-good export if available
   - Use SQLite's official recovery tools (`sqlite3 .recover`) as a last resort
4. A link to the SQLite recovery documentation

The application **never** silently continues on a corrupt database. Degraded reads risk
compounding the corruption and make recovery harder.

### 5. Backup strategy: export-as-backup

The application does not implement automatic backup. The user's backup responsibility is
met through:

- **Export**: `rustcash export --format gnucash-xml > backup.gnucash` produces a
  complete, human-readable export. This is the primary recovery artefact.
- **VACUUM INTO**: `rustcash db vacuum-into <dest>` writes a clean, defragmented copy
  of the database to a new file. Useful as a manual point-in-time backup of the raw DB.
- **OS-level backup**: Users who want automatic backup point Time Machine, rsync, or
  similar tools at the SQLite file. The WAL checkpoint is flushed before the file is
  read, ensuring a consistent snapshot.

The application does not automatically write backup files. Automatic backup could silently
fill the disk, and the scope of "where to write" and "how many to keep" is a system
administration concern.

### 6. PostgreSQL — server operator's responsibility

WAL archiving, streaming replication, and point-in-time recovery are standard PostgreSQL
features that the server operator configures. The application does not duplicate this.
Export (`rustcash export`) remains available for portability and as an application-level
backup independent of the database server's backup configuration.

### 7. No write-time checksums on financial records

Application-level checksums on transaction rows (hashing the amount + date + splits) are
not added today. SQLite's B-tree integrity check is the integrity guarantee. If tamper
detection (as opposed to corruption detection) becomes a requirement — e.g., for audit
compliance — that is a separate ADR and likely involves a Merkle chain on the transaction
log, which has significant write-path implications.

## Consequences

- Corruption is detected before any data is read, not after an inexplicable wrong balance
- The recovery path is documented in the error message itself — users are not left to search
- Users own their backup schedule; the app makes export easy
- No automatic backup complexity, no disk-fill risk
- The storage crate's `open_sqlite` is the single enforcement point for all of the above
