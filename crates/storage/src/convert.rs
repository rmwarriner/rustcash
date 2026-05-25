//! Shared helpers for mapping SQLite TEXT columns to typed Rust values.
//! SQLite stores UUIDs, datetimes, and dates as strings; these functions
//! centralise the parsing and produce consistent StorageError messages on failure.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
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

/// Serialize a unit-variant enum to its serde string representation.
/// Relies on the enum having `#[serde(rename_all = "snake_case")]` or similar.
pub fn enum_to_str<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

/// Deserialize a unit-variant enum from its serde string representation.
pub fn enum_from_str<T: for<'de> Deserialize<'de>>(
    s: &str,
    field: &'static str,
) -> Result<T, StorageError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| StorageError::Constraint(format!("invalid {field} value '{s}': {e}")))
}

pub fn decimal_from_str(s: &str, field: &'static str) -> Result<Decimal, StorageError> {
    s.parse::<Decimal>()
        .map_err(|e| StorageError::Constraint(format!("invalid decimal in {field}: {e}")))
}

pub fn tags_to_json(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
}

pub fn tags_from_json(s: &str, field: &'static str) -> Result<Vec<String>, StorageError> {
    serde_json::from_str(s)
        .map_err(|e| StorageError::Constraint(format!("invalid JSON tags in {field}: {e}")))
}
