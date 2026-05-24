//! Opt-in CRDT-based multi-device sync.
//!
//! Allows two or more devices to work offline and merge their changes without
//! conflicts. No cloud vendor lock-in — bring your own relay.

pub mod error;

pub use error::SyncError;
