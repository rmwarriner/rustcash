use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rustcash_core::ids::{AccountId, BookId, CommodityId, TransactionId};
use crate::customer::CustomerId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InvoiceId(pub Uuid);

impl InvoiceId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for InvoiceId {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Posted,
    Paid,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub description: String,
    pub quantity:    Decimal,
    pub unit_price:  Decimal,
    pub account_id:  AccountId,
}

impl InvoiceLine {
    pub fn total(&self) -> Decimal {
        self.quantity * self.unit_price
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id:           InvoiceId,
    pub book_id:      BookId,
    pub customer_id:  CustomerId,
    pub number:       String,
    pub date:         NaiveDate,
    pub due_date:     Option<NaiveDate>,
    pub lines:        Vec<InvoiceLine>,
    pub commodity_id: CommodityId,
    pub status:       InvoiceStatus,
    /// Set when invoice is posted — links to the AR transaction.
    pub transaction_id: Option<TransactionId>,
    pub notes:        Option<String>,
}

impl Invoice {
    pub fn total(&self) -> Decimal {
        self.lines.iter().map(|l| l.total()).sum()
    }

    pub fn is_posted(&self) -> bool {
        self.status != InvoiceStatus::Draft
    }
}
