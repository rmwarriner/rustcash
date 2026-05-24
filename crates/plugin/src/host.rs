//! WASM host engine — initialises the wasmtime runtime and loads plugins.

use wasmtime::Engine;

/// Shared wasmtime engine. Create once and reuse across plugin loads.
pub struct PluginHost {
    engine: Engine,
}

impl PluginHost {
    pub fn new() -> anyhow::Result<Self> {
        let engine = Engine::default();
        Ok(Self { engine })
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new().expect("failed to initialise WASM engine")
    }
}
