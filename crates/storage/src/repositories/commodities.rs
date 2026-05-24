use rustcash_core::ids::{BookId, CommodityId};
use rustcash_core::commodity::Commodity;
use crate::{SqlitePool, StorageError};

pub struct CommodityRepository {
    pool: SqlitePool,
}

impl CommodityRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, _id: CommodityId) -> Result<Option<Commodity>, StorageError> {
        todo!("implement commodity lookup")
    }

    pub async fn find_by_book(&self, _book_id: BookId) -> Result<Vec<Commodity>, StorageError> {
        todo!("implement commodity list")
    }
}
