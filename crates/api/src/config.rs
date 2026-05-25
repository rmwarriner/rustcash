use serde::{Deserialize, Serialize};

/// Log output format (see ADR 010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Structured JSON — for log aggregators (Loki, Datadog, etc.) in production.
    Json,
    /// Coloured human-readable output — for terminal use during development.
    #[default]
    Ansi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub log_format: LogFormat,
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self { bind: default_bind(), token: None, log_format: LogFormat::default() }
    }
}
