-- Book soft-delete and privacy model support (see ADR 008).
-- Books are soft-deleted, consistent with accounts and users.
-- Hard purge is a separate, explicit operation — never automatic.

ALTER TABLE books ADD COLUMN deleted_at TEXT;

CREATE INDEX IF NOT EXISTS idx_books_deleted ON books(deleted_at)
    WHERE deleted_at IS NOT NULL;
