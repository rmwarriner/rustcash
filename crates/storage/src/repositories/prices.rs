use chrono::NaiveDate;
use rustcash_core::ids::{BookId, CommodityId};
use rustcash_core::commodity::Price;
use crate::{SqlitePool, StorageError};

pub struct PriceRepository {
    pool: SqlitePool,
}

impl PriceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn latest_before(
        &self,
        _commodity_id: CommodityId,
        _currency_id: CommodityId,
        _as_of: NaiveDate,
    ) -> Result<Option<Price>, StorageError> {
        todo!("implement price lookup")
    }

    pub async fn find_by_book(&self, _book_id: BookId) -> Result<Vec<Price>, StorageError> {
        todo!("implement price list")
    }
}
