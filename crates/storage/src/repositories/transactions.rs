use rustcash_core::ids::{AccountId, BookId, TransactionId};
use rustcash_core::transaction::Transaction;
use crate::{SqlitePool, StorageError};

pub struct TransactionRepository {
    pool: SqlitePool,
}

impl TransactionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, _id: TransactionId) -> Result<Option<Transaction>, StorageError> {
        todo!("implement transaction lookup")
    }

    pub async fn find_by_account(
        &self,
        _account_id: AccountId,
        _book_id: BookId,
    ) -> Result<Vec<Transaction>, StorageError> {
        todo!("implement transaction list by account")
    }

    pub async fn insert(&self, _txn: &Transaction) -> Result<(), StorageError> {
        todo!("implement transaction insert")
    }

    pub async fn soft_delete(&self, _id: TransactionId) -> Result<(), StorageError> {
        todo!("implement transaction soft delete")
    }
}
