//! CSV exporter stub.

use crate::{ExportError, Exporter};
use rustcash_core::ids::BookId;

pub struct CsvExporter;

impl Exporter for CsvExporter {
    fn name(&self) -> &str {
        "CSV"
    }
    fn supported_extensions(&self) -> &[&str] {
        &["csv"]
    }
    fn export(&self, _dest: &mut dyn std::io::Write, _book_id: BookId) -> Result<(), ExportError> {
        todo!("CSV exporter — Phase 2")
    }
}
