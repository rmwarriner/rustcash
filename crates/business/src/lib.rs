//! Opt-in business features: invoicing, accounts receivable/payable, customers, vendors, payroll.
//!
//! This crate is only compiled when explicitly depended upon — personal finance
//! users pay zero compile-time or binary-size cost for these features.

pub mod customer;
pub mod error;
pub mod invoice;
pub mod vendor;

pub use error::BusinessError;
