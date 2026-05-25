use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("record not found: {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("constraint violation: {0}")]
    Constraint(String),

    #[error(
        "database corruption detected: {details}\n\n\
        Do not continue using this database.\n\
        1. Back up the database file immediately (even in its current state).\n\
        2. Run `rustcash db check` for a full integrity report.\n\
        3. Restore from a known-good export if available.\n\
        See https://www.sqlite.org/recovery.html for SQLite recovery tools."
    )]
    Corruption { details: String },
}
