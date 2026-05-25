use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    Report,
    Importer,
    Exporter,
}

/// Parsed contents of a `plugin.toml` manifest file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    pub wasm: String, // relative path to .wasm file
}
