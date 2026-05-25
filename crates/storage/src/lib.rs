//! Persistence layer. Provides repository types backed by SQLite or PostgreSQL.
//!
//! All SQL runs through `sqlx`; migrations live in `migrations/`.

pub mod db;
pub mod error;
pub mod repositories;

pub use db::open_sqlite;
pub use error::StorageError;

/// Shared type alias so callers don't repeat the pool type everywhere.
pub type SqlitePool = sqlx::SqlitePool;
pub type PgPool     = sqlx::PgPool;
