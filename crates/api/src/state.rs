/// Shared application state injected into every axum handler.
pub struct AppState {
    // Repositories and service handles will be added here as they are implemented.
    // Using a placeholder to keep the type non-empty.
    pub version: &'static str,
}

impl AppState {
    pub fn new() -> Self {
        Self { version: env!("CARGO_PKG_VERSION") }
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}
