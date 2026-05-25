//! Transaction lifecycle: enter (draft), post, void.

use chrono::Utc;
use rustcash_core::{
    ids::TransactionId,
    transaction::TransactionStatus,
};
use rustcash_storage::{repositories::transactions::TransactionRepository, SqlitePool};

use crate::EngineError;

pub struct TransactionService {
    pool: SqlitePool,
}

impl TransactionService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Store a new transaction in Draft state.
    pub async fn enter(&self, txn: &rustcash_core::transaction::Transaction) -> Result<(), EngineError> {
        TransactionRepository::new(self.pool.clone())
            .insert(txn)
            .await?;
        Ok(())
    }

    /// Advance a Draft transaction to Posted (immutable, included in balances).
    pub async fn post(&self, id: TransactionId) -> Result<(), EngineError> {
        let repo = TransactionRepository::new(self.pool.clone());
        let txn = repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| EngineError::TransactionNotFound { id: id.to_string() })?;

        if !txn.status.is_draft() {
            return Err(EngineError::InvalidStatusTransition {
                from: format!("{:?}", txn.status),
                to:   "posted".to_string(),
            });
        }

        repo.update_status(id, TransactionStatus::Posted, None, Utc::now())
            .await?;
        Ok(())
    }

    /// Void a Posted transaction.
    ///
    /// The transaction is excluded from all balance calculations. If the caller
    /// has already entered a correcting replacement transaction, pass its ID as
    /// `replacement_id` to record the link in `voiding_transaction_id`.
    pub async fn void(
        &self,
        id: TransactionId,
        replacement_id: Option<TransactionId>,
    ) -> Result<(), EngineError> {
        let repo = TransactionRepository::new(self.pool.clone());
        let txn = repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| EngineError::TransactionNotFound { id: id.to_string() })?;

        if !txn.status.is_posted() {
            return Err(EngineError::InvalidStatusTransition {
                from: format!("{:?}", txn.status),
                to:   "void".to_string(),
            });
        }

        repo.update_status(id, TransactionStatus::Void, replacement_id, Utc::now())
            .await?;
        Ok(())
    }
}
