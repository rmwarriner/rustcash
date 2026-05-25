use chrono::{DateTime, Utc};
use rustcash_core::{
    account::{Account, AccountType},
    ids::{AccountId, BookId, CommodityId},
};
use sqlx::FromRow;

use crate::{
    SqlitePool, StorageError,
    convert::{
        datetime_from_str, datetime_opt_from_str, enum_from_str, enum_to_str, uuid_from_str,
    },
};

#[derive(FromRow)]
struct AccountRow {
    id: String,
    book_id: String,
    parent_id: Option<String>,
    name: String,
    full_name: String,
    account_type: String,
    commodity_id: String,
    description: Option<String>,
    placeholder: i64,
    hidden: i64,
    sort_order: i64,
    created_at: String,
    modified_at: String,
    deleted_at: Option<String>,
}

impl AccountRow {
    fn into_account(self) -> Result<Account, StorageError> {
        Ok(Account {
            id: AccountId::from(uuid_from_str(&self.id, "accounts.id")?),
            book_id: BookId::from(uuid_from_str(&self.book_id, "accounts.book_id")?),
            parent_id: self
                .parent_id
                .as_deref()
                .map(|s| uuid_from_str(s, "accounts.parent_id").map(AccountId::from))
                .transpose()?,
            name: self.name,
            full_name: self.full_name,
            account_type: enum_from_str::<AccountType>(
                &self.account_type,
                "accounts.account_type",
            )?,
            commodity_id: CommodityId::from(uuid_from_str(
                &self.commodity_id,
                "accounts.commodity_id",
            )?),
            description: self.description,
            placeholder: self.placeholder != 0,
            hidden: self.hidden != 0,
            sort_order: self.sort_order as i32,
            created_at: datetime_from_str(&self.created_at, "accounts.created_at")?,
            modified_at: datetime_from_str(&self.modified_at, "accounts.modified_at")?,
            deleted_at: datetime_opt_from_str(self.deleted_at.as_deref(), "accounts.deleted_at")?,
        })
    }
}

const SELECT_COLS: &str = "id, book_id, parent_id, name, full_name, account_type, commodity_id, \
     description, placeholder, hidden, sort_order, created_at, modified_at, deleted_at";

pub struct AccountRepository {
    pool: SqlitePool,
}

impl AccountRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, account: &Account) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO accounts \
             (id, book_id, parent_id, name, full_name, account_type, commodity_id, \
              description, placeholder, hidden, sort_order, created_at, modified_at, deleted_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(account.id.to_string())
        .bind(account.book_id.to_string())
        .bind(account.parent_id.map(|id| id.to_string()))
        .bind(&account.name)
        .bind(&account.full_name)
        .bind(enum_to_str(&account.account_type))
        .bind(account.commodity_id.to_string())
        .bind(&account.description)
        .bind(account.placeholder as i64)
        .bind(account.hidden as i64)
        .bind(account.sort_order as i64)
        .bind(account.created_at.to_rfc3339())
        .bind(account.modified_at.to_rfc3339())
        .bind(account.deleted_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: AccountId) -> Result<Option<Account>, StorageError> {
        sqlx::query_as::<_, AccountRow>(&format!("SELECT {SELECT_COLS} FROM accounts WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .map(AccountRow::into_account)
            .transpose()
    }

    /// All non-deleted accounts in a book, ordered by full_name.
    pub async fn find_by_book(&self, book_id: BookId) -> Result<Vec<Account>, StorageError> {
        sqlx::query_as::<_, AccountRow>(&format!(
            "SELECT {SELECT_COLS} FROM accounts \
             WHERE book_id = ? AND deleted_at IS NULL \
             ORDER BY full_name"
        ))
        .bind(book_id.to_string())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(AccountRow::into_account)
        .collect()
    }

    /// Direct children of a parent account (non-deleted only).
    /// Used by the engine for tree traversal and full_name cascade updates.
    pub async fn find_children(&self, parent_id: AccountId) -> Result<Vec<Account>, StorageError> {
        sqlx::query_as::<_, AccountRow>(&format!(
            "SELECT {SELECT_COLS} FROM accounts \
             WHERE parent_id = ? AND deleted_at IS NULL \
             ORDER BY sort_order, name"
        ))
        .bind(parent_id.to_string())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(AccountRow::into_account)
        .collect()
    }

    pub async fn update(&self, account: &Account) -> Result<(), StorageError> {
        let rows = sqlx::query(
            "UPDATE accounts \
             SET parent_id = ?, name = ?, full_name = ?, account_type = ?, \
                 commodity_id = ?, description = ?, placeholder = ?, hidden = ?, \
                 sort_order = ?, modified_at = ?, deleted_at = ? \
             WHERE id = ?",
        )
        .bind(account.parent_id.map(|id| id.to_string()))
        .bind(&account.name)
        .bind(&account.full_name)
        .bind(enum_to_str(&account.account_type))
        .bind(account.commodity_id.to_string())
        .bind(&account.description)
        .bind(account.placeholder as i64)
        .bind(account.hidden as i64)
        .bind(account.sort_order as i64)
        .bind(account.modified_at.to_rfc3339())
        .bind(account.deleted_at.map(|dt| dt.to_rfc3339()))
        .bind(account.id.to_string())
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "Account",
                id: account.id.to_string(),
            });
        }
        Ok(())
    }

    pub async fn soft_delete(
        &self,
        id: AccountId,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let rows = sqlx::query(
            "UPDATE accounts SET deleted_at = ?, modified_at = ? \
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
                entity: "Account",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}
