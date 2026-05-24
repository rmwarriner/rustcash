//! File format exporters.

pub mod csv;
pub mod error;

pub use error::ExportError;

/// Trait implemented by every exporter.
pub trait Exporter: Send + Sync {
    fn name(&self) -> &str;
    fn supported_extensions(&self) -> &[&str];

    fn export(
        &self,
        dest: &mut dyn std::io::Write,
        book_id: rustcash_core::ids::BookId,
    ) -> Result<(), ExportError>;
}
