use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("export error: {0}")]
    Export(String),
}
