//! WASM plugin host.
//!
//! Loads `.wasm` files via `wasmtime` and exposes them as [`Report`] or [`Importer`]
//! implementations. Plugins run in a sandboxed environment with no host filesystem
//! or network access unless explicitly granted.

pub mod error;
pub mod host;
pub mod manifest;

pub use error::PluginError;
pub use manifest::PluginManifest;
