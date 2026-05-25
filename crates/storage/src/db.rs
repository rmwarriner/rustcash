//! Database connection helpers.
//! All callers should open SQLite through `open_sqlite` — never construct a pool directly —
//! so that WAL mode, foreign keys, busy_timeout, and the startup integrity check are always applied.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use std::str::FromStr;
use std::time::Duration;

use crate::{SqlitePool, StorageError};

/// Open (or create) a SQLite database with the project-standard pragmas applied:
/// WAL journal mode, NORMAL synchronous, foreign keys ON, 5 s busy timeout.
///
/// The busy timeout allows the CLI and the API server to share the same file safely:
/// if two writers collide, SQLite retries for up to 5 seconds before returning an error.
///
/// Runs `PRAGMA quick_check` before returning. Returns `StorageError::Corruption`
/// if the check reports any problems.
pub async fn open_sqlite(url: &str) -> Result<SqlitePool, StorageError> {
    let opts = SqliteConnectOptions::from_str(url)?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(opts).await?;

    let rows: Vec<String> = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_all(&pool)
        .await?;

    if rows.as_slice() != ["ok"] {
        return Err(StorageError::Corruption {
            details: rows.join("; "),
        });
    }

    Ok(pool)
}

/// Run all pending migrations against an already-opened pool.
///
/// Safe to call multiple times — sqlx tracks applied migrations in `_sqlx_migrations`.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), StorageError> {
    sqlx::migrate!()
        .run(pool)
        .await
        .map_err(sqlx::Error::from)?;
    Ok(())
}
