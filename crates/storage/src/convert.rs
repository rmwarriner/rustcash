//! Shared helpers for mapping SQLite TEXT columns to typed Rust values.
//! SQLite stores UUIDs, datetimes, and dates as strings; these functions
//! centralise the parsing and produce consistent StorageError messages on failure.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::StorageError;

pub fn uuid_from_str(s: &str, field: &'static str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(s)
        .map_err(|e| StorageError::Constraint(format!("invalid UUID in {field}: {e}")))
}

pub fn datetime_from_str(s: &str, field: &'static str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| StorageError::Constraint(format!("invalid datetime in {field}: {e}")))
}

pub fn datetime_opt_from_str(
    s: Option<&str>,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, StorageError> {
    s.map(|v| datetime_from_str(v, field)).transpose()
}

pub fn date_from_str(s: &str, field: &'static str) -> Result<NaiveDate, StorageError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| StorageError::Constraint(format!("invalid date in {field}: {e}")))
}

pub fn date_opt_from_str(
    s: Option<&str>,
    field: &'static str,
) -> Result<Option<NaiveDate>, StorageError> {
    s.map(|v| date_from_str(v, field)).transpose()
}
