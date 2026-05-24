use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("storage error: {0}")]
    Storage(#[from] rustcash_storage::StorageError),

    #[error("core error: {0}")]
    Core(#[from] rustcash_core::error::CoreError),

    #[error("account {id} not found")]
    AccountNotFound { id: String },

    #[error("no price found for {commodity} in {currency} on or before {date}")]
    NoPriceAvailable { commodity: String, currency: String, date: String },
}
