use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("failed to load plugin '{name}': {reason}")]
    Load { name: String, reason: String },

    #[error("WASM engine error: {0}")]
    Wasm(#[from] anyhow::Error),

    #[error("plugin '{id}' not found")]
    NotFound { id: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("manifest parse error: {0}")]
    Manifest(String),
}
