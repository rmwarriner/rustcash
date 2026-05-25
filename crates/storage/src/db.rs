//! Database connection helpers.
//! All callers should open SQLite through `open_sqlite` — never construct a pool directly —
//! so that WAL mode, foreign keys, and the startup integrity check are always applied.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use std::str::FromStr;

use crate::{SqlitePool, StorageError};

/// Open (or create) a SQLite database with the project-standard pragmas applied:
/// WAL journal mode, NORMAL synchronous, foreign keys ON.
///
/// Runs `PRAGMA quick_check` before returning. Returns `StorageError::Corruption`
/// if the check reports any problems.
pub async fn open_sqlite(url: &str) -> Result<SqlitePool, StorageError> {
    let opts = SqliteConnectOptions::from_str(url)
        .map_err(sqlx::Error::from)?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(opts).await?;

    let rows: Vec<String> = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_all(&pool)
        .await?;

    if rows.as_slice() != ["ok"] {
        return Err(StorageError::Corruption { details: rows.join("; ") });
    }

    Ok(pool)
}
