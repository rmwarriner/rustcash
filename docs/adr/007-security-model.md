# ADR 007 — Security model: authentication, authorization, and credential storage

**Status**: Accepted

## Context

RustCash is currently single-user and localhost-only. The API server binds to `127.0.0.1`
by default and auth is explicitly optional for that configuration (see ADR 003). However,
the architecture is API-first and must eventually support multi-user scenarios: self-hosted
web deployments, team accounting against PostgreSQL, and desktop-to-desktop sync.

Security decisions embedded in the data model are expensive to retrofit once financial data
exists. This ADR reserves the right types and schema shapes now, even though the
implementation is deferred.

## Decisions

### 1. Identity type reserved in `core`

`UserId(Uuid)` is defined in `core/src/ids.rs` alongside the other typed ID newtypes.
`Book` carries `owner_id: Option<UserId>` — `None` for local-only installations, `Some`
when auth is active. The schema mirrors this as a nullable `owner_id` column on `books`.

This costs nothing today and prevents a breaking schema migration and a core API change
later.

### 2. Auth is opt-in; localhost installs stay zero-friction

When the server binds to `127.0.0.1` and no user record exists, auth middleware is
disabled. The config gate is `auth.required = false` (the default). Users who just want a
desktop accounting tool never see a login screen.

Auth becomes mandatory when:
- `auth.required = true` is set in config, or
- the server binds to a non-loopback address (middleware enforces this automatically).

### 3. Opaque Bearer tokens for programmatic access — not JWT

API clients (CLI, scripts, third-party integrations) authenticate with opaque Bearer tokens.
Opaque tokens are revoked instantly by soft-deleting the `api_tokens` row or setting
`expires_at`. JWTs cannot be revoked without a server-side blocklist, which negates their
statelessness advantage for a self-hosted app that already has a database.

Token values are stored as **argon2id hashes** in `api_tokens.token_hash`. The raw token
is returned once at creation time and never stored. This means a compromised database
does not expose valid tokens.

### 4. Session cookies for the GUI

The Tauri desktop GUI and any browser-based UI authenticate via short-lived session tokens
stored in the `api_tokens` table with a non-null `expires_at`. Session tokens are
distinguished from API tokens only by their TTL and by a `description` convention
(`"session:..."` vs `"api:..."`).

A future `sessions` table can replace this if session metadata (device, IP, user-agent)
becomes useful, without changing the auth middleware contract.

### 5. Password hashing: argon2id

`argon2id` (via the `argon2` crate) with the current OWASP-recommended parameters:

- Variant: Argon2id
- Memory: 64 MiB
- Iterations: 3
- Parallelism: 4

`password_hash` on the `users` row is `NULL` for installs where no password auth is used
(SSO-only, or local no-auth mode). This is not a valid login credential — the auth
middleware must treat NULL as "cannot authenticate via password".

### 6. Book-level tenancy

Authorization is scoped to the `Book`. A user owns a book; future work can add a
`book_members` join table for shared access (read-only vs. read-write roles).

All existing queries are already scoped by `book_id` — the FK topology is auth-ready
without any query changes. The storage layer will enforce `owner_id` or membership checks
as a repository-level filter, not in application code.

### 7. Import credential encryption (deferred, architecture decided)

Import credentials (bank login, OFX server password, API keys for price feeds) are
encrypted at rest using a per-user symmetric key. The key is derived from the user's
password using Argon2id with a separate salt stored alongside the ciphertext (not in
`password_hash`). AES-256-GCM is the cipher.

The `storage` crate owns encryption and decryption. The `import` crate receives plaintext
credentials and has no knowledge of how they are stored. The `core` crate has no knowledge
of encryption at all.

Implementation is deferred until the import crate reaches its first real downloader.

### 8. Plugin security boundary

WASM plugins run in a wasmtime sandbox (see ADR 004). Plugins inherit the authorization
context of the calling user through the API — they cannot escalate privileges. No
additional auth layer is needed for plugins.

### 9. Transport security: reverse-proxy responsibility

The API server does not terminate TLS. For deployments exposed beyond localhost, TLS is
the responsibility of a reverse proxy (nginx, Caddy). The API server emits a log warning
at startup if it binds to a non-loopback interface without `X-Forwarded-Proto: https`
being present in incoming requests.

Local loopback deployments do not require TLS.

## What is explicitly NOT decided here

- Multi-tenancy isolation at the PostgreSQL row-level security layer (deferred to when
  PostgreSQL support is fleshed out).
- OAuth / OIDC / SSO integration (deferred; nothing prevents adding it later).
- Two-factor authentication (deferred).
- Audit logging schema (deferred; soft-deletes and `modified_at` are the current paper trail).

## Consequences

- `UserId` and `owner_id` are in place; all future auth code can build on them without
  schema migrations.
- Single-user local use remains zero-friction — no code path forces a login for localhost.
- The multi-user deployment path is documented and non-breaking.
- Import credential encryption architecture is decided so the `import` crate API stays
  stable when the implementation arrives.
- No new runtime dependencies are added today; `argon2` and `aes-gcm` are added when the
  first auth endpoint is implemented.
