use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{AccountId, BookId, CommodityId};

/// High-level classification used for balance-sheet / income-statement placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootType {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

impl RootType {
    /// Assets and expenses are debit-normal (positive balance increases with debits).
    pub fn is_debit_normal(self) -> bool {
        matches!(self, Self::Asset | Self::Expense)
    }
}

/// Fine-grained account type. Each variant knows its root classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    // ── Assets ───────────────────────────────────────────────────────────────
    Asset,
    Cash,
    Bank,
    CreditCard,
    Investment,
    MutualFund,
    // ── Liabilities ───────────────────────────────────────────────────────────
    Liability,
    LongTermLiability,
    // ── Equity ────────────────────────────────────────────────────────────────
    Equity,
    OpeningBalance,
    RetainedEarnings,
    // ── Income ────────────────────────────────────────────────────────────────
    Income,
    // ── Expenses ──────────────────────────────────────────────────────────────
    Expense,
    // ── Business (opt-in, available even in core for type-checking) ───────────
    Receivable,
    Payable,
}

impl AccountType {
    pub fn root(self) -> RootType {
        use AccountType::*;
        match self {
            Asset | Cash | Bank | CreditCard | Investment | MutualFund | Receivable => {
                RootType::Asset
            }
            Liability | LongTermLiability | Payable => RootType::Liability,
            Equity | OpeningBalance | RetainedEarnings => RootType::Equity,
            Income => RootType::Income,
            Expense => RootType::Expense,
        }
    }

    pub fn is_debit_normal(self) -> bool {
        self.root().is_debit_normal()
    }
}

/// A node in the account tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub book_id: BookId,
    pub parent_id: Option<AccountId>,
    pub name: String,
    pub full_name: String, // e.g. "Assets:Current Assets:Checking"
    pub account_type: AccountType,
    pub commodity_id: CommodityId,
    pub description: Option<String>,
    /// Placeholder accounts are containers only — no direct transactions.
    pub placeholder: bool,
    pub hidden: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Account {
    pub fn root_type(&self) -> RootType {
        self.account_type.root()
    }

    pub fn is_debit_normal(&self) -> bool {
        self.account_type.is_debit_normal()
    }
}
