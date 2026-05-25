use chrono::{DateTime, NaiveDate, Utc};
use rustcash_core::{
    commodity::{Price, PriceSource},
    ids::{BookId, CommodityId, PriceId},
};
use sqlx::FromRow;

use crate::{
    convert::{
        date_from_str, datetime_from_str, decimal_from_str, enum_from_str, enum_to_str,
        uuid_from_str,
    },
    SqlitePool, StorageError,
};

// ── row type ──────────────────────────────────────────────────────────────────

#[derive(FromRow)]
struct PriceRow {
    id:           String,
    book_id:      String,
    commodity_id: String,
    currency_id:  String,
    date:         String,
    value:        String,
    source:       String,
    created_at:   String,
}

impl PriceRow {
    fn into_price(self) -> Result<Price, StorageError> {
        Ok(Price {
            id:           PriceId::from(uuid_from_str(&self.id, "prices.id")?),
            book_id:      BookId::from(uuid_from_str(&self.book_id, "prices.book_id")?),
            commodity_id: CommodityId::from(uuid_from_str(&self.commodity_id, "prices.commodity_id")?),
            currency_id:  CommodityId::from(uuid_from_str(&self.currency_id, "prices.currency_id")?),
            date:         date_from_str(&self.date, "prices.date")?,
            value:        decimal_from_str(&self.value, "prices.value")?,
            source:       enum_from_str(&self.source, "prices.source")?,
            created_at:   datetime_from_str(&self.created_at, "prices.created_at")?,
        })
    }
}

const SELECT_COLS: &str =
    "id, book_id, commodity_id, currency_id, date, value, source, created_at";

// ── repository ────────────────────────────────────────────────────────────────

pub struct PriceRepository {
    pool: SqlitePool,
}

impl PriceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, price: &Price) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO prices \
             (id, book_id, commodity_id, currency_id, date, value, source, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(price.id.to_string())
        .bind(price.book_id.to_string())
        .bind(price.commodity_id.to_string())
        .bind(price.currency_id.to_string())
        .bind(price.date.to_string())
        .bind(price.value.to_string())
        .bind(enum_to_str(&price.source))
        .bind(price.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: PriceId) -> Result<Option<Price>, StorageError> {
        sqlx::query_as::<_, PriceRow>(&format!(
            "SELECT {SELECT_COLS} FROM prices WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .map(PriceRow::into_price)
        .transpose()
    }

    /// All prices in a book, ordered by date descending then commodity.
    pub async fn find_by_book(&self, book_id: BookId) -> Result<Vec<Price>, StorageError> {
        sqlx::query_as::<_, PriceRow>(&format!(
            "SELECT {SELECT_COLS} FROM prices \
             WHERE book_id = ? \
             ORDER BY date DESC, commodity_id"
        ))
        .bind(book_id.to_string())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(PriceRow::into_price)
        .collect()
    }

    /// Most recent price for `commodity_id` expressed in `currency_id` on or before `as_of`.
    pub async fn latest_before(
        &self,
        commodity_id: CommodityId,
        currency_id: CommodityId,
        as_of: NaiveDate,
    ) -> Result<Option<Price>, StorageError> {
        sqlx::query_as::<_, PriceRow>(&format!(
            "SELECT {SELECT_COLS} FROM prices \
             WHERE commodity_id = ? AND currency_id = ? AND date <= ? \
             ORDER BY date DESC \
             LIMIT 1"
        ))
        .bind(commodity_id.to_string())
        .bind(currency_id.to_string())
        .bind(as_of.to_string())
        .fetch_optional(&self.pool)
        .await?
        .map(PriceRow::into_price)
        .transpose()
    }

    /// Prices for a specific commodity pair, ordered by date ascending.
    pub async fn find_series(
        &self,
        commodity_id: CommodityId,
        currency_id: CommodityId,
    ) -> Result<Vec<Price>, StorageError> {
        sqlx::query_as::<_, PriceRow>(&format!(
            "SELECT {SELECT_COLS} FROM prices \
             WHERE commodity_id = ? AND currency_id = ? \
             ORDER BY date ASC"
        ))
        .bind(commodity_id.to_string())
        .bind(currency_id.to_string())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(PriceRow::into_price)
        .collect()
    }

    pub async fn delete(&self, id: PriceId) -> Result<(), StorageError> {
        let rows = sqlx::query("DELETE FROM prices WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected();

        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "Price",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    pub async fn update(
        &self,
        id: PriceId,
        date: NaiveDate,
        value: rust_decimal::Decimal,
        source: PriceSource,
        modified_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let rows = sqlx::query(
            "UPDATE prices SET date = ?, value = ?, source = ?, created_at = ? WHERE id = ?",
        )
        .bind(date.to_string())
        .bind(value.to_string())
        .bind(enum_to_str(&source))
        .bind(modified_at.to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "Price",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}
