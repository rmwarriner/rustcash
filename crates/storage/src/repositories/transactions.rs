use chrono::{DateTime, Utc};
use rustcash_core::{
    ids::{AccountId, BookId, CommodityId, LotId, SplitId, TransactionId},
    transaction::{ReconcileState, Split, Transaction, TransactionStatus},
};
use sqlx::FromRow;

use crate::{
    convert::{
        date_from_str, date_opt_from_str, datetime_from_str, decimal_from_str, enum_from_str,
        enum_to_str, tags_from_json, tags_to_json, uuid_from_str,
    },
    SqlitePool, StorageError,
};

// ── row types ─────────────────────────────────────────────────────────────────

/// A single row from a JOIN of transactions + splits.
/// Both `find_by_id` and `find_by_account` use this shape with different WHERE clauses.
#[derive(FromRow)]
struct TxnSplitRow {
    // transaction columns
    txn_id:                  String,
    book_id:                 String,
    txn_date:                String,
    description:             String,
    notes:                   Option<String>,
    txn_tags:                String,
    status:                  String,
    voiding_transaction_id:  Option<String>,
    entered_at:              String,
    modified_at:             String,
    // split columns
    split_id:                String,
    account_id:              String,
    amount:                  String,
    value:                   String,
    commodity_id:            String,
    reconcile_state:         String,
    reconcile_date:          Option<String>,
    memo:                    Option<String>,
    split_tags:              String,
    action:                  Option<String>,
    lot_id:                  Option<String>,
    split_created_at:        String,
}

// ── row → domain conversion ───────────────────────────────────────────────────

fn row_to_transaction_stub(row: &TxnSplitRow) -> Result<Transaction, StorageError> {
    Ok(Transaction {
        id:     TransactionId::from(uuid_from_str(&row.txn_id, "transactions.id")?),
        book_id: BookId::from(uuid_from_str(&row.book_id, "transactions.book_id")?),
        date:   date_from_str(&row.txn_date, "transactions.date")?,
        description: row.description.clone(),
        notes:  row.notes.clone(),
        tags:   tags_from_json(&row.txn_tags, "transactions.tags")?,
        splits: Vec::new(),
        status: enum_from_str(&row.status, "transactions.status")?,
        voiding_transaction_id: row
            .voiding_transaction_id
            .as_deref()
            .map(|s| uuid_from_str(s, "transactions.voiding_transaction_id").map(TransactionId::from))
            .transpose()?,
        entered_at:  datetime_from_str(&row.entered_at, "transactions.entered_at")?,
        modified_at: datetime_from_str(&row.modified_at, "transactions.modified_at")?,
    })
}

fn row_to_split(row: &TxnSplitRow) -> Result<Split, StorageError> {
    Ok(Split {
        id:              SplitId::from(uuid_from_str(&row.split_id, "splits.id")?),
        account_id:      AccountId::from(uuid_from_str(&row.account_id, "splits.account_id")?),
        amount:          decimal_from_str(&row.amount, "splits.amount")?,
        value:           decimal_from_str(&row.value, "splits.value")?,
        commodity_id:    CommodityId::from(uuid_from_str(&row.commodity_id, "splits.commodity_id")?),
        reconcile_state: enum_from_str(&row.reconcile_state, "splits.reconcile_state")?,
        reconcile_date:  date_opt_from_str(row.reconcile_date.as_deref(), "splits.reconcile_date")?,
        memo:            row.memo.clone(),
        tags:            tags_from_json(&row.split_tags, "splits.tags")?,
        action:          row.action.clone(),
        lot_id:          row
            .lot_id
            .as_deref()
            .map(|s| uuid_from_str(s, "splits.lot_id").map(LotId::from))
            .transpose()?,
        created_at:      datetime_from_str(&row.split_created_at, "splits.created_at")?,
    })
}

/// Reconstruct a Vec<Transaction> from rows ordered by (date, txn_id, split_id).
/// Consecutive rows with the same txn_id are grouped into a single Transaction.
fn rows_to_transactions(rows: Vec<TxnSplitRow>) -> Result<Vec<Transaction>, StorageError> {
    let mut result: Vec<Transaction> = Vec::new();
    for row in rows {
        let split = row_to_split(&row)?;
        if result.last().is_some_and(|t: &Transaction| t.id.to_string() == row.txn_id) {
            result.last_mut().unwrap().splits.push(split);
        } else {
            let mut txn = row_to_transaction_stub(&row)?;
            txn.splits.push(split);
            result.push(txn);
        }
    }
    Ok(result)
}

// ── SELECT column list (aliased to avoid name collisions) ─────────────────────

const JOIN_SELECT: &str = "
    t.id          AS txn_id,
    t.book_id,
    t.date        AS txn_date,
    t.description,
    t.notes,
    t.tags        AS txn_tags,
    t.status,
    t.voiding_transaction_id,
    t.entered_at,
    t.modified_at,
    s.id          AS split_id,
    s.account_id,
    s.amount,
    s.value,
    s.commodity_id,
    s.reconcile_state,
    s.reconcile_date,
    s.memo,
    s.tags        AS split_tags,
    s.action,
    s.lot_id,
    s.created_at  AS split_created_at
FROM transactions t
JOIN splits s ON s.transaction_id = t.id";

// ── repository ────────────────────────────────────────────────────────────────

pub struct TransactionRepository {
    pool: SqlitePool,
}

impl TransactionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a transaction and all its splits atomically.
    /// The splits-sum-to-zero invariant is enforced upstream by `Transaction::new`.
    pub async fn insert(&self, txn: &Transaction) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO transactions \
             (id, book_id, date, description, notes, tags, status, \
              voiding_transaction_id, entered_at, modified_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(txn.id.to_string())
        .bind(txn.book_id.to_string())
        .bind(txn.date.to_string())
        .bind(&txn.description)
        .bind(&txn.notes)
        .bind(tags_to_json(&txn.tags))
        .bind(enum_to_str(&txn.status))
        .bind(txn.voiding_transaction_id.map(|id| id.to_string()))
        .bind(txn.entered_at.to_rfc3339())
        .bind(txn.modified_at.to_rfc3339())
        .execute(&mut *tx)
        .await?;

        for split in &txn.splits {
            sqlx::query(
                "INSERT INTO splits \
                 (id, transaction_id, account_id, amount, value, commodity_id, \
                  reconcile_state, reconcile_date, memo, tags, action, lot_id, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(split.id.to_string())
            .bind(txn.id.to_string())
            .bind(split.account_id.to_string())
            .bind(split.amount.to_string())
            .bind(split.value.to_string())
            .bind(split.commodity_id.to_string())
            .bind(enum_to_str(&split.reconcile_state))
            .bind(split.reconcile_date.map(|d| d.to_string()))
            .bind(&split.memo)
            .bind(tags_to_json(&split.tags))
            .bind(&split.action)
            .bind(split.lot_id.map(|id| id.to_string()))
            .bind(split.created_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: TransactionId) -> Result<Option<Transaction>, StorageError> {
        let rows = sqlx::query_as::<_, TxnSplitRow>(&format!(
            "SELECT {JOIN_SELECT} WHERE t.id = ? ORDER BY s.id"
        ))
        .bind(id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut txns = rows_to_transactions(rows)?;
        Ok(txns.pop())
    }

    /// All transactions that have at least one split on `account_id`, with all their splits.
    pub async fn find_by_account(
        &self,
        account_id: AccountId,
        book_id: BookId,
    ) -> Result<Vec<Transaction>, StorageError> {
        let rows = sqlx::query_as::<_, TxnSplitRow>(&format!(
            "SELECT {JOIN_SELECT}
             WHERE t.id IN (
                 SELECT DISTINCT transaction_id FROM splits WHERE account_id = ?
             ) AND t.book_id = ?
             ORDER BY t.date, t.id, s.id"
        ))
        .bind(account_id.to_string())
        .bind(book_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows_to_transactions(rows)
    }

    /// Post or void a transaction. Voiding requires `voiding_id` set to the reversing transaction.
    pub async fn update_status(
        &self,
        id: TransactionId,
        status: TransactionStatus,
        voiding_id: Option<TransactionId>,
        modified_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let rows = sqlx::query(
            "UPDATE transactions \
             SET status = ?, voiding_transaction_id = ?, modified_at = ? \
             WHERE id = ?",
        )
        .bind(enum_to_str(&status))
        .bind(voiding_id.map(|id| id.to_string()))
        .bind(modified_at.to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "Transaction",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Update reconciliation state on a single split.
    pub async fn update_split_reconcile(
        &self,
        split_id: SplitId,
        state: ReconcileState,
        date: Option<chrono::NaiveDate>,
    ) -> Result<(), StorageError> {
        let rows = sqlx::query(
            "UPDATE splits SET reconcile_state = ?, reconcile_date = ? WHERE id = ?",
        )
        .bind(enum_to_str(&state))
        .bind(date.map(|d| d.to_string()))
        .bind(split_id.to_string())
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "Split",
                id: split_id.to_string(),
            });
        }
        Ok(())
    }
}
