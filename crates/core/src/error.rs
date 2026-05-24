use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("transaction splits do not balance: sum is {sum} (must be zero)")]
    UnbalancedTransaction { sum: Decimal },

    #[error("account tree contains a cycle at account {account_id}")]
    AccountCycle { account_id: String },

    #[error("a transaction must have at least two splits")]
    TooFewSplits,

    #[error("invalid amount: {reason}")]
    InvalidAmount { reason: String },
}
