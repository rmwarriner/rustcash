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
}
