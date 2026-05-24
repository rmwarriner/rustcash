use thiserror::Error;

#[derive(Debug, Error)]
pub enum BusinessError {
    #[error("storage error: {0}")]
    Storage(#[from] rustcash_storage::StorageError),

    #[error("engine error: {0}")]
    Engine(#[from] rustcash_engine::EngineError),

    #[error("invoice {id} is already posted")]
    AlreadyPosted { id: String },

    #[error("customer {id} not found")]
    CustomerNotFound { id: String },
}
