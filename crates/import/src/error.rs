use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parse error at line {line}: {reason}")]
    Parse { line: usize, reason: String },

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("core error: {0}")]
    Core(#[from] rustcash_core::error::CoreError),
}
