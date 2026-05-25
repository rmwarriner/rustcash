//! Reporting engine.
//!
//! Define the [`Report`] trait and ship standard reports.
//! Third-party reports are loaded as WASM modules by the `plugin` crate.

pub mod error;
pub mod standard;

pub use error::ReportError;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rustcash_core::ids::{BookId, CommodityId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A typed report parameter value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    Text(String),
    Date(NaiveDate),
    Amount(Decimal),
    Bool(bool),
}

/// Metadata describing a report — used in the report registry and API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub version: String,
    pub parameters: Vec<ReportParamDef>,
}

/// Definition of one parameter accepted by a report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportParamDef {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<ParamValue>,
}

/// Date range passed to every report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// Output formats a report can produce.
#[derive(Debug, Clone)]
pub enum ReportOutput {
    Html(String),
    Csv(String),
    Json(serde_json::Value),
    // Pdf(Vec<u8>)  — added when a PDF crate is chosen
}

/// The trait every report must implement.
pub trait Report: Send + Sync {
    fn metadata(&self) -> &ReportMetadata;

    fn render(
        &self,
        date_range: DateRange,
        book_id: BookId,
        reporting_commodity: CommodityId,
        params: &HashMap<String, ParamValue>,
    ) -> Result<ReportOutput, ReportError>;
}
