use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("missing required parameter: {0}")]
    MissingParameter(String),

    #[error("invalid parameter '{name}': {reason}")]
    InvalidParameter { name: String, reason: String },

    #[error("engine error: {0}")]
    Engine(#[from] rustcash_engine::EngineError),

    #[error("render error: {0}")]
    Render(String),
}
