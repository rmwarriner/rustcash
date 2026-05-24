use rustcash_core::ids::{AccountId, BookId};
use crate::{SqlitePool, StorageError};

pub struct AccountRepository {
    pool: SqlitePool,
}

impl AccountRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, _id: AccountId) -> Result<Option<rustcash_core::account::Account>, StorageError> {
        todo!("implement account lookup")
    }

    pub async fn find_by_book(&self, _book_id: BookId) -> Result<Vec<rustcash_core::account::Account>, StorageError> {
        todo!("implement account list")
    }
}
