//! axum-based HTTP/JSON API — the canonical interface for all RustCash clients.
//!
//! Clients: CLI (`rustcash serve`), TUI (optional remote mode), GUI (Tauri), third-party tools.

pub mod config;
pub mod error;
pub mod routes;
pub mod state;

pub use error::ApiError;
pub use state::AppState;

use axum::Router;
use std::sync::Arc;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/v1", routes::v1_router())
        .with_state(state)
}
