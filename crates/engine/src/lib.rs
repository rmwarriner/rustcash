//! Accounting logic layer.
//!
//! Stateless async functions that operate on repository types from `storage`.
//! No UI, no HTTP, no file I/O.

pub mod account;
pub mod balance;
pub mod error;
pub mod transaction;

pub use error::EngineError;
