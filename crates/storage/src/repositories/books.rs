use chrono::{DateTime, Utc};
use rustcash_core::{
    book::Book,
    ids::{BookId, CommodityId, UserId},
};
use sqlx::FromRow;

use crate::{
    SqlitePool, StorageError,
    convert::{date_opt_from_str, datetime_from_str, datetime_opt_from_str, uuid_from_str},
};

#[derive(FromRow)]
struct BookRow {
    id: String,
    name: String,
    description: Option<String>,
    default_commodity_id: String,
    period_close_date: Option<String>,
    owner_id: Option<String>,
    created_at: String,
    modified_at: String,
    deleted_at: Option<String>,
}

impl BookRow {
    fn into_book(self) -> Result<Book, StorageError> {
        Ok(Book {
            id: BookId::from(uuid_from_str(&self.id, "books.id")?),
            name: self.name,
            description: self.description,
            default_commodity_id: CommodityId::from(uuid_from_str(
                &self.default_commodity_id,
                "books.default_commodity_id",
            )?),
            period_close_date: date_opt_from_str(
                self.period_close_date.as_deref(),
                "books.period_close_date",
            )?,
            owner_id: self
                .owner_id
                .as_deref()
                .map(|s| uuid_from_str(s, "books.owner_id").map(UserId::from))
                .transpose()?,
            created_at: datetime_from_str(&self.created_at, "books.created_at")?,
            modified_at: datetime_from_str(&self.modified_at, "books.modified_at")?,
            deleted_at: datetime_opt_from_str(self.deleted_at.as_deref(), "books.deleted_at")?,
        })
    }
}

const SELECT_COLS: &str = "id, name, description, default_commodity_id, period_close_date, owner_id, \
     created_at, modified_at, deleted_at";

pub struct BookRepository {
    pool: SqlitePool,
}

impl BookRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, book: &Book) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO books \
             (id, name, description, default_commodity_id, period_close_date, \
              owner_id, created_at, modified_at, deleted_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(book.id.to_string())
        .bind(&book.name)
        .bind(&book.description)
        .bind(book.default_commodity_id.to_string())
        .bind(book.period_close_date.map(|d| d.to_string()))
        .bind(book.owner_id.map(|id| id.to_string()))
        .bind(book.created_at.to_rfc3339())
        .bind(book.modified_at.to_rfc3339())
        .bind(book.deleted_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: BookId) -> Result<Option<Book>, StorageError> {
        sqlx::query_as::<_, BookRow>(&format!("SELECT {SELECT_COLS} FROM books WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .map(BookRow::into_book)
            .transpose()
    }

    /// Returns all non-deleted books.
    pub async fn find_all(&self) -> Result<Vec<Book>, StorageError> {
        sqlx::query_as::<_, BookRow>(&format!(
            "SELECT {SELECT_COLS} FROM books WHERE deleted_at IS NULL"
        ))
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(BookRow::into_book)
        .collect()
    }

    pub async fn update(&self, book: &Book) -> Result<(), StorageError> {
        let rows = sqlx::query(
            "UPDATE books \
             SET name = ?, description = ?, default_commodity_id = ?, \
                 period_close_date = ?, owner_id = ?, modified_at = ?, deleted_at = ? \
             WHERE id = ?",
        )
        .bind(&book.name)
        .bind(&book.description)
        .bind(book.default_commodity_id.to_string())
        .bind(book.period_close_date.map(|d| d.to_string()))
        .bind(book.owner_id.map(|id| id.to_string()))
        .bind(book.modified_at.to_rfc3339())
        .bind(book.deleted_at.map(|dt| dt.to_rfc3339()))
        .bind(book.id.to_string())
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "Book",
                id: book.id.to_string(),
            });
        }
        Ok(())
    }

    pub async fn soft_delete(
        &self,
        id: BookId,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let rows = sqlx::query(
            "UPDATE books SET deleted_at = ?, modified_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(deleted_at.to_rfc3339())
        .bind(deleted_at.to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "Book",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}
