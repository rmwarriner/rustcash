use rustcash_core::{commodity::Commodity, ids::{BookId, CommodityId}};
use sqlx::FromRow;

use crate::{
    convert::{datetime_from_str, uuid_from_str},
    SqlitePool, StorageError,
};

#[derive(FromRow)]
struct CommodityRow {
    id:         String,
    book_id:    String,
    namespace:  String,
    mnemonic:   String,
    name:       String,
    fraction:   i64,
    notes:      Option<String>,
    created_at: String,
}

impl CommodityRow {
    fn into_commodity(self) -> Result<Commodity, StorageError> {
        Ok(Commodity {
            id:         CommodityId::from(uuid_from_str(&self.id, "commodities.id")?),
            book_id:    BookId::from(uuid_from_str(&self.book_id, "commodities.book_id")?),
            namespace:  self.namespace,
            mnemonic:   self.mnemonic,
            name:       self.name,
            fraction:   self.fraction as u32,
            notes:      self.notes,
            created_at: datetime_from_str(&self.created_at, "commodities.created_at")?,
        })
    }
}

const SELECT_COLS: &str =
    "id, book_id, namespace, mnemonic, name, fraction, notes, created_at";

pub struct CommodityRepository {
    pool: SqlitePool,
}

impl CommodityRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, commodity: &Commodity) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO commodities \
             (id, book_id, namespace, mnemonic, name, fraction, notes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(commodity.id.to_string())
        .bind(commodity.book_id.to_string())
        .bind(&commodity.namespace)
        .bind(&commodity.mnemonic)
        .bind(&commodity.name)
        .bind(commodity.fraction as i64)
        .bind(&commodity.notes)
        .bind(commodity.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if matches!(&e, sqlx::Error::Database(db)
                if db.kind() == sqlx::error::ErrorKind::UniqueViolation)
            {
                StorageError::Constraint(format!(
                    "commodity ({}, {}) already exists in this book",
                    commodity.namespace, commodity.mnemonic
                ))
            } else {
                StorageError::Database(e)
            }
        })?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: CommodityId) -> Result<Option<Commodity>, StorageError> {
        sqlx::query_as::<_, CommodityRow>(&format!(
            "SELECT {SELECT_COLS} FROM commodities WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .map(CommodityRow::into_commodity)
        .transpose()
    }

    /// All commodities belonging to a book, ordered by namespace then mnemonic.
    pub async fn find_by_book(&self, book_id: BookId) -> Result<Vec<Commodity>, StorageError> {
        sqlx::query_as::<_, CommodityRow>(&format!(
            "SELECT {SELECT_COLS} FROM commodities \
             WHERE book_id = ? ORDER BY namespace, mnemonic"
        ))
        .bind(book_id.to_string())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(CommodityRow::into_commodity)
        .collect()
    }

    /// Look up a commodity by its (namespace, mnemonic) natural key within a book.
    /// Used during import to avoid creating duplicates.
    pub async fn find_by_mnemonic(
        &self,
        book_id: BookId,
        namespace: &str,
        mnemonic: &str,
    ) -> Result<Option<Commodity>, StorageError> {
        sqlx::query_as::<_, CommodityRow>(&format!(
            "SELECT {SELECT_COLS} FROM commodities \
             WHERE book_id = ? AND namespace = ? AND mnemonic = ?"
        ))
        .bind(book_id.to_string())
        .bind(namespace)
        .bind(mnemonic)
        .fetch_optional(&self.pool)
        .await?
        .map(CommodityRow::into_commodity)
        .transpose()
    }

    /// Update mutable fields: name, fraction, notes. namespace/mnemonic are immutable.
    pub async fn update(&self, commodity: &Commodity) -> Result<(), StorageError> {
        let rows = sqlx::query(
            "UPDATE commodities SET name = ?, fraction = ?, notes = ? WHERE id = ?",
        )
        .bind(&commodity.name)
        .bind(commodity.fraction as i64)
        .bind(&commodity.notes)
        .bind(commodity.id.to_string())
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "Commodity",
                id: commodity.id.to_string(),
            });
        }
        Ok(())
    }
}
