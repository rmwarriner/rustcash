# ADR 008 — Privacy model: data ownership, minimization, and erasure

**Status**: Accepted

## Context

RustCash handles personal financial data — one of the most sensitive categories of personal
information. The privacy model must be decided before user data accumulates, because
retrofitting privacy protections into a populated schema is expensive and the architectural
choices (local-first, API-first, plugin-extensible, future cloud sync) each create specific
privacy risks that need to be explicitly addressed.

## Decisions

### 1. Local-first; no data leaves the device by default

The application makes no outbound network requests without explicit user action. This is
enforced architecturally: `core` and `engine` have zero I/O (CLAUDE.md principle #1), and
all network-capable features (price feeds, bank connections, sync) require explicit opt-in
configuration. A default install with an empty config does not phone home.

### 2. No telemetry without explicit opt-in

The application collects no usage data, crash reports, or analytics by default. If
telemetry is added in the future it must be:
- Opt-in (not opt-out), with UI disclosure before the first data point is sent
- Limited to non-PII operational data (e.g., aggregate error counts)
- Never transaction data, amounts, payee names, memo text, or account names

### 3. User data ownership and portability

Users can export all their data at any time in open formats: GnuCash XML, CSV, OFX. The
`export` crate is a first-class citizen, not an afterthought. No data is held in a
proprietary format. Import and export are not gated behind subscriptions or account tiers.

### 4. PII lives in free-text fields

Transaction descriptions, memo fields, payee names, and tags contain PII. These are
standard `TEXT` columns today. The primary mitigation is encryption at rest: SQLCipher for
SQLite files, PostgreSQL tablespace encryption for server deployments. Future work may add
per-field redaction for an "anonymize old records" workflow, but this is not designed here.

Consequence: **do not add structured PII columns** (government ID, SSN, phone number) to
the schema without a separate privacy review. If business features (invoicing, payroll)
need these, they go in the `business` crate with explicit documentation.

### 5. Right to erasure vs. financial record retention

GDPR Article 17 grants the right to erasure. Most accounting jurisdictions require
financial records to be retained for 7+ years. These are in tension. The resolution:

**User account deletion** — soft-delete the `users` row; hash or redact the email so the
UNIQUE constraint doesn't block re-registration; nullify `owner_id` on their books.
Transaction records are retained. The user's identity is gone; the financial audit trail
remains.

**Book deletion** — soft-delete via `books.deleted_at`. The book and its data remain in
the database until the retention period expires or a manual purge is requested.

**Manual purge** — a future `purge` command hard-deletes a soft-deleted book and all its
financial data. This is explicit, irreversible, requires confirmation, and is the user's
responsibility to time correctly relative to their local legal retention obligations. The
application will warn but will not enforce a retention period floor, because retention
requirements vary by jurisdiction.

### 6. Soft-delete on `books` — structural decision

`Book::deleted_at: Option<DateTime<Utc>>` is added to the domain type and schema (migration
003), consistent with the pattern already used for accounts and users. Repositories filter
out soft-deleted books by default; a `include_deleted: bool` option is available for admin
operations.

### 7. Multi-user books: the book is the privacy boundary

When a book is shared (future `book_members` table), all members see all transactions in
that book. There are no per-transaction or per-account visibility controls within a shared
book. Users who need segregated visibility should use separate books.

This is a deliberate simplification. Per-row ACLs inside a book would make balance
calculations and report generation significantly more complex, and personal financial
software rarely needs them.

### 8. Third-party service data flows

Three categories, each with different privacy treatment:

| Service | Data sent | Condition |
|---------|-----------|-----------|
| Price feeds | Commodity symbol only (e.g., `AAPL`, `EUR`) | Opt-in per commodity |
| Bank / OFX connections | Credentials + account numbers | Explicit per-institution opt-in; credentials encrypted at rest (ADR 007) |
| Cloud sync (future) | All book data, encrypted end-to-end | Opt-in; sync server sees only opaque ciphertext |

No data is sent to any third party that is not in one of these categories without a new
ADR and explicit user consent.

### 9. Plugin data access

WASM plugins run in a wasmtime sandbox (ADR 004) and can only access data explicitly
passed to them by the host. Plugins cannot initiate outbound network connections unless a
future capability gate explicitly permits it. Plugin authors must declare their data access
requirements in a manifest file; users see this declaration before installation. The host
never passes unrelated books' data to a plugin.

### 10. Audit logging (deferred)

When auth is active, access to financial data should be logged. The schema and
implementation are deferred to when auth middleware is implemented (ADR 007). Minimum
fields when implemented: `user_id`, `action`, `resource_type`, `resource_id`, `ip_address`,
`timestamp`. Audit log rows are never soft-deleted — they are append-only.

## What is explicitly NOT decided here

- Encryption key management (KMS, OS keychain integration) — deferred
- Regulatory compliance certifications (SOC 2, ISO 27001) — out of scope for self-hosted
- Cookie consent / GDPR banner (only relevant if a hosted web version is built)
- Per-field encryption of memo/payee columns — deferred; SQLCipher is the current answer

## Consequences

- No data ever leaves the device without a deliberate, documented code path
- The erasure vs. retention tension has a principled resolution documented before any user
  data exists
- Plugin and third-party data flows have clear rules before any integrations are built
- `books.deleted_at` is in place; book archival and closure are implementable without
  schema changes
- Structured PII (SSN, government ID) is explicitly out of scope for the core schema
