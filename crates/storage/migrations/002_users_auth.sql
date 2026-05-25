-- Users and authentication tables (see ADR 007).
-- owner_id on books is nullable so that single-user local installs need no user record.

CREATE TABLE IF NOT EXISTS users (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    display_name  TEXT,
    -- argon2id hash of the password; NULL for SSO-only or local-no-auth installs
    password_hash TEXT,
    created_at    TEXT NOT NULL,
    deleted_at    TEXT  -- soft-delete; never hard-delete auth records
);

CREATE TABLE IF NOT EXISTS api_tokens (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id),
    -- argon2id hash of the raw token; raw value shown once at creation and never stored
    token_hash   TEXT NOT NULL UNIQUE,
    description  TEXT,
    created_at   TEXT NOT NULL,
    last_used_at TEXT,
    expires_at   TEXT,  -- NULL = never expires
    deleted_at   TEXT   -- soft-delete for instant revocation
);

-- Wire books to their owning user.
-- NULL = local single-user install with auth disabled.
ALTER TABLE books ADD COLUMN owner_id TEXT REFERENCES users(id);

CREATE INDEX IF NOT EXISTS idx_api_tokens_user ON api_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_books_owner     ON books(owner_id);
