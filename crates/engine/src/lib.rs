//! Accounting logic layer.
//!
//! Stateless async functions that operate on repository types from `storage`.
//! No UI, no HTTP, no file I/O.

pub mod balance;
pub mod error;

pub use error::EngineError;
