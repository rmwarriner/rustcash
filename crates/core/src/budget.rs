use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::ids::{AccountId, BookId, BudgetId};

/// Frequency of a budget period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetPeriodType {
    Monthly,
    Quarterly,
    Annually,
}

/// One period in a budget (e.g. "January 2025").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetPeriod {
    pub start_date: NaiveDate,
    pub end_date:   NaiveDate,
}

/// An allocated amount for one account in one period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetAllocation {
    pub account_id: AccountId,
    pub period:     usize, // index into Budget::periods
    pub amount:     Decimal,
}

/// A named budget covering a set of periods and account allocations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub id:          BudgetId,
    pub book_id:     BookId,
    pub name:        String,
    pub description: Option<String>,
    pub period_type: BudgetPeriodType,
    pub periods:     Vec<BudgetPeriod>,
    pub allocations: Vec<BudgetAllocation>,
}
