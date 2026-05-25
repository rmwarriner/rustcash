use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::{AccountId, BookId, CommodityId, LotId, SplitId, TransactionId};

/// Reconciliation state of a split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReconcileState {
    #[default]
    Unreconciled, // 'n'
    Cleared,    // 'c'  — shown on bank statement, not yet reconciled
    Reconciled, // 'y'  — reconciliation confirmed
}

/// One leg of a double-entry transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub id: SplitId,
    pub account_id: AccountId,
    /// Amount in the account's own commodity.
    pub amount: Decimal,
    /// Value in the transaction's reporting commodity (for multi-currency).
    pub value: Decimal,
    pub commodity_id: CommodityId,
    pub reconcile_state: ReconcileState,
    pub reconcile_date: Option<NaiveDate>,
    pub memo: Option<String>,
    /// First-class per-split tags. Complements transaction-level tags.
    /// Use when legs of the same transaction need different labels
    /// (e.g. one split is "business", another is "personal").
    pub tags: Vec<String>,
    /// Investment action label: "Buy", "Sell", "Div", "IntInc", etc.
    pub action: Option<String>,
    /// Cost-basis lot — used for investment gain/loss tracking.
    pub lot_id: Option<LotId>,
    pub created_at: DateTime<Utc>,
}

impl Split {
    pub fn is_reconciled(&self) -> bool {
        self.reconcile_state == ReconcileState::Reconciled
    }

    pub fn is_cleared(&self) -> bool {
        self.reconcile_state == ReconcileState::Cleared
    }
}

/// GAAP-style transaction lifecycle.
///
/// Once `Posted`, a transaction is immutable. Corrections are made by voiding
/// (which produces a reversing entry) and entering a new correcting transaction.
/// Transactions are **never deleted** — voiding is the only retirement path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    #[default]
    Draft, // staging; editable; excluded from balance calculations
    Posted, // immutable; included in all balance calculations
    Void,   // excluded from balances; reversing entry links back here
}

impl TransactionStatus {
    pub fn is_posted(&self) -> bool {
        matches!(self, Self::Posted)
    }
    pub fn is_draft(&self) -> bool {
        matches!(self, Self::Draft)
    }
    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }
}

/// A balanced double-entry transaction.
///
/// Invariant: `splits.iter().map(|s| s.amount).sum() == Decimal::ZERO`.
/// This is enforced at construction time via [`Transaction::new`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub book_id: BookId,
    pub date: NaiveDate,
    pub description: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub splits: Vec<Split>,
    pub status: TransactionStatus,
    /// Set when `status == Void`. Points to the reversing transaction.
    pub voiding_transaction_id: Option<TransactionId>,
    pub entered_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl Transaction {
    /// Create a new transaction, validating that splits sum to zero.
    pub fn new(
        id: TransactionId,
        book_id: BookId,
        date: NaiveDate,
        description: impl Into<String>,
        splits: Vec<Split>,
    ) -> Result<Self, CoreError> {
        if splits.len() < 2 {
            return Err(CoreError::TooFewSplits);
        }
        let sum: Decimal = splits.iter().map(|s| s.amount).sum();
        if !sum.is_zero() {
            return Err(CoreError::UnbalancedTransaction { sum });
        }
        let now = Utc::now();
        Ok(Self {
            id,
            book_id,
            date,
            description: description.into(),
            notes: None,
            tags: Vec::new(),
            splits,
            status: TransactionStatus::Draft,
            voiding_transaction_id: None,
            entered_at: now,
            modified_at: now,
        })
    }

    /// Returns `true` if every split is reconciled.
    pub fn is_reconciled(&self) -> bool {
        self.splits.iter().all(|s| s.is_reconciled())
    }

    /// Splits that belong to `account_id`.
    pub fn splits_for(&self, account_id: AccountId) -> impl Iterator<Item = &Split> {
        self.splits
            .iter()
            .filter(move |s| s.account_id == account_id)
    }

    /// Net effect of this transaction on `account_id` (sum of matching split amounts).
    pub fn net_for(&self, account_id: AccountId) -> Decimal {
        self.splits_for(account_id).map(|s| s.amount).sum()
    }
}
