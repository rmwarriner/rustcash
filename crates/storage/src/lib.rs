//! Persistence layer. Provides repository types backed by SQLite or PostgreSQL.
//!
//! All SQL runs through `sqlx`; migrations live in `migrations/`.

pub mod error;
pub mod repositories;

pub use error::StorageError;

/// Shared type alias so callers don't repeat the pool type everywhere.
pub type SqlitePool = sqlx::SqlitePool;
pub type PgPool     = sqlx::PgPool;
