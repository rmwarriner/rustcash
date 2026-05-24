use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("storage error: {0}")]
    Storage(#[from] rustcash_storage::StorageError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("merge conflict: {0}")]
    Conflict(String),

    #[error("network error: {0}")]
    Network(String),
}
