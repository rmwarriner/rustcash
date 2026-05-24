use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default)]
    pub token: Option<String>,
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self { bind: default_bind(), token: None }
    }
}
