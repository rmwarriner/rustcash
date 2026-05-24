use axum::Router;
use std::sync::Arc;
use crate::AppState;

pub mod health;

pub fn v1_router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(health::router())
    // Additional route groups added here as implemented:
    // .merge(accounts::router())
    // .merge(transactions::router())
    // .merge(reports::router())
    // .merge(import::router())
    // .merge(plugins::router())
}
