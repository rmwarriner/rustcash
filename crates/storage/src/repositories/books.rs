use rustcash_core::ids::BookId;
use crate::{SqlitePool, StorageError};

pub struct BookRepository {
    pool: SqlitePool,
}

impl BookRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, _id: BookId) -> Result<Option<rustcash_core::book::Book>, StorageError> {
        todo!("implement book lookup")
    }
}
