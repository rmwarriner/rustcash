use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{BookId, CommodityId, UserId};

/// Top-level container for all accounting data.
/// Maps to a single SQLite file or a PostgreSQL schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub id:                   BookId,
    pub name:                 String,
    pub description:          Option<String>,
    /// Default reporting commodity (usually a currency like USD).
    pub default_commodity_id: CommodityId,
    /// None for local single-user installs; Some when auth is required (see ADR 007).
    pub owner_id:             Option<UserId>,
    pub created_at:           DateTime<Utc>,
    pub modified_at:          DateTime<Utc>,
}
