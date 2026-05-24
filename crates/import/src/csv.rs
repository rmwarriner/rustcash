//! CSV importer stub.
//!
//! The real implementation will support configurable column mapping.
//! For now this establishes the type and trait impl shape.

use crate::{ImportError, ImportPreview, Importer};
use rustcash_core::ids::BookId;

pub struct CsvImporter;

impl Importer for CsvImporter {
    fn name(&self) -> &str {
        "CSV"
    }

    fn supported_extensions(&self) -> &[&str] {
        &["csv"]
    }

    fn import(
        &self,
        _source: &mut dyn std::io::Read,
        _book_id: BookId,
    ) -> Result<ImportPreview, ImportError> {
        todo!("CSV importer — Phase 1")
    }
}
