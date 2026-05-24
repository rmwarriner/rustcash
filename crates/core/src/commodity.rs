use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::ids::{BookId, CommodityId, PriceId};

/// A currency, stock, fund, or other tradable unit of value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commodity {
    pub id:        CommodityId,
    pub book_id:   BookId,
    /// Grouping namespace: `"CURRENCY"`, `"NYSE"`, `"FUND"`, `"CRYPTO"`, etc.
    pub namespace: String,
    /// Ticker or ISO code: `"USD"`, `"AAPL"`, `"BTC"`, etc.
    pub mnemonic:  String,
    pub name:      String,
    /// Denominator of the smallest representable unit.
    /// 100 = cents (USD), 1000 = mils, 1 = whole units.
    pub fraction:  u32,
    pub notes:     Option<String>,
}

impl Commodity {
    pub fn is_currency(&self) -> bool {
        self.namespace == "CURRENCY"
    }
}

/// The source of a price quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    User,
    AlphaVantage,
    YahooFinance,
    Import,
    Transaction,
}

/// A point-in-time price for a commodity expressed in another commodity (usually a currency).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Price {
    pub id:           PriceId,
    pub book_id:      BookId,
    pub commodity_id: CommodityId,
    pub currency_id:  CommodityId,
    pub date:         NaiveDate,
    pub value:        Decimal,
    pub source:       PriceSource,
}
