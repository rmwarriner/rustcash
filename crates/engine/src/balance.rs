//! Account balance calculations.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rustcash_core::ids::AccountId;
use rustcash_core::transaction::ReconcileState;

use crate::EngineError;

/// Balances for an account as of a given date.
#[derive(Debug, Clone)]
pub struct AccountBalance {
    pub account_id:        AccountId,
    pub balance:           Decimal,
    pub cleared_balance:   Decimal,
    pub reconciled_balance: Decimal,
    pub as_of:             NaiveDate,
}

/// Calculate the balance of a single account as of `as_of`.
///
/// `splits` should be all splits for this account up to and including `as_of`.
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
