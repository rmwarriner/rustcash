//! Account balance calculations.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rustcash_core::ids::{AccountId, BookId};
use rustcash_core::transaction::ReconcileState;
use rustcash_storage::{SqlitePool, repositories::transactions::TransactionRepository};

use crate::EngineError;

/// Balances for an account as of a given date.
#[derive(Debug, Clone)]
pub struct AccountBalance {
    pub account_id: AccountId,
    pub balance: Decimal,
    pub cleared_balance: Decimal,
    pub reconciled_balance: Decimal,
    pub as_of: NaiveDate,
}

/// Pure calculation: given a slice of `(date, amount, reconcile_state)` tuples for one account,
/// return the balance as of `as_of` (inclusive).
pub fn compute_balance(
    account_id: AccountId,
    splits: &[(NaiveDate, Decimal, ReconcileState)],
    as_of: NaiveDate,
) -> Result<AccountBalance, EngineError> {
    let mut balance = Decimal::ZERO;
    let mut cleared = Decimal::ZERO;
    let mut reconciled = Decimal::ZERO;

    for (date, amount, state) in splits {
        if *date > as_of {
            continue;
        }
        balance += amount;
        match state {
            ReconcileState::Cleared => cleared += amount,
            ReconcileState::Reconciled => {
                cleared += amount;
                reconciled += amount;
            }
            ReconcileState::Unreconciled => {}
        }
    }

    Ok(AccountBalance {
        account_id,
        balance,
        cleared_balance: cleared,
        reconciled_balance: reconciled,
        as_of,
    })
}

// ── service ───────────────────────────────────────────────────────────────────

pub struct BalanceService {
    pool: SqlitePool,
}

impl BalanceService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Balance of `account_id` as of `as_of`, counting only Posted transactions.
    pub async fn account_balance(
        &self,
        account_id: AccountId,
        book_id: BookId,
        as_of: NaiveDate,
    ) -> Result<AccountBalance, EngineError> {
        let transactions = TransactionRepository::new(self.pool.clone())
            .find_by_account(account_id, book_id)
            .await?;

        let splits: Vec<(NaiveDate, Decimal, ReconcileState)> = transactions
            .iter()
            .filter(|t| t.status.is_posted())
            .flat_map(|t| {
                t.splits_for(account_id)
                    .map(|s| (t.date, s.amount, s.reconcile_state))
            })
            .collect();

        compute_balance(account_id, &splits, as_of)
    }
}
