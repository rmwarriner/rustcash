//! File format importers.
//!
//! Each importer parses a source format and returns an [`ImportPreview`] for
//! the user to review before committing to storage.

pub mod csv;
pub mod error;

pub use error::ImportError;

use rustcash_core::{account::Account, transaction::Transaction};

/// A potential duplicate: an incoming transaction that looks like one already in the book.
#[derive(Debug, Clone)]
pub struct DuplicateCandidate {
    pub incoming: Transaction,
    pub existing_id: rustcash_core::ids::TransactionId,
    pub confidence: f32, // 0.0–1.0
}

/// Bayesian account suggestion for one split in the preview.
#[derive(Debug, Clone)]
pub struct AccountSuggestion {
    /// Index into the corresponding `Transaction::splits` vec.
    pub split_index: usize,
    pub account_id: rustcash_core::ids::AccountId,
    /// Posterior probability from the Naive Bayes classifier (0.0–1.0).
    pub confidence: f32,
}

/// Result of a dry-run import — shown to the user before they confirm.
#[derive(Debug)]
pub struct ImportPreview {
    pub transactions: Vec<Transaction>,
    pub new_accounts: Vec<Account>,
    pub duplicates: Vec<DuplicateCandidate>,
    /// Bayesian account pre-fills, produced by `engine::classify`.
    /// UX: auto-fill if confidence ≥ 0.90, prompt if 0.60–0.89, suggest if < 0.60.
    pub account_suggestions: Vec<AccountSuggestion>,
    pub warnings: Vec<String>,
}

/// Trait implemented by every importer.
pub trait Importer: Send + Sync {
    fn name(&self) -> &str;
    fn supported_extensions(&self) -> &[&str];

    fn import(
        &self,
        source: &mut dyn std::io::Read,
        book_id: rustcash_core::ids::BookId,
    ) -> Result<ImportPreview, ImportError>;
}
