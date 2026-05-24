use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{BookId, CommodityId};

/// Top-level container for all accounting data.
/// Maps to a single SQLite file or a PostgreSQL schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub id:                  BookId,
    pub name:                String,
    pub description:         Option<String>,
    /// Default reporting commodity (usually a currency like USD).
    pub default_commodity_id: CommodityId,
    pub created_at:          DateTime<Utc>,
    pub modified_at:         DateTime<Utc>,
}
