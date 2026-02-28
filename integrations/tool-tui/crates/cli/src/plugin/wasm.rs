//! WASM Plugin Executor
//!
//! Executes WebAssembly plugins with sandboxed capabilities using wasmtime.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use wasmtime::*;

use super::PluginType;
use super::traits::{Capability, DxPlugin, PluginContext, PluginMetadata, PluginResult};

/// WASM plugin executor using wasmtime
pub struct WasmExecutor {
    engine: Engine,
    config: WasmConfig,
}

/// WASM execution configuration
#[derive(Debug, Clone)]
pub struct WasmConfig {
    /// Maximum memory pages (64KB each)
    pub max_memory_pages: u32,
    /// Maximum table elements
    pub max_table_elements: u32,
    /// Enable WASI preview
    pub enable_wasi: bool,
    /// Fuel limit (for metering)
    pub fuel_limit: Option<u64>,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_memory_pages: 4096, // 256 MB
            max_table_elements: 10000,
            enable_wasi: true,
            fuel_limit: Some(1_000_000_000), // 1 billion instructions
        }
    }
}

impl WasmExecutor {
    /// Create a new WASM executor
    pub fn new() -> Result<Self> {
        let mut config = Config::new();

        // Enable useful WASM features
        config.wasm_bulk_memory(true);
        config.wasm_multi_memory(true);
        config.wasm_simd(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;

        Ok(Self {
            engine,
            config: WasmConfig::default(),
        })
    }

    /// Create executor with custom config
    pub fn with_config(wasm_config: WasmConfig) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_bulk_memory(true);
        config.wasm_multi_memory(true);
        config.wasm_simd(true);

        if wasm_config.fuel_limit.is_some() {
            config.consume_fuel(true);
        }

        let engine = Engine::new(&config)?;

        Ok(Self {
            engine,
            config: wasm_config,
        })
    }

    /// Load a WASM plugin from file
    pub async fn load(&self, path: &Path) -> Result<WasmPlugin> {
        let wasm_bytes = tokio::fs::read(path).await.context("Failed to read WASM file")?;

        let module =
            Module::new(&self.engine, &wasm_bytes).context("Failed to compile WASM module")?;

        // Extract metadata from module name or separate file
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        let metadata = PluginMetadata {
            name,
            version: "1.0.0".to_string(),
            description: String::new(),
            author: String::new(),
            capabilities: vec![Capability::Network], // Default minimal capabilities
            plugin_type: PluginType::Wasm,
            path: path.to_path_buf(),
            signature: None,
        };

        Ok(WasmPlugin {
            module: Arc::new(module),
            engine: self.engine.clone(),
            metadata,
            config: self.config.clone(),
        })
    }

    /// Load a WASM plugin from bytes
    pub fn load_bytes(&self, name: &str, wasm_bytes: &[u8]) -> Result<WasmPlugin> {
        let module =
            Module::new(&self.engine, wasm_bytes).context("Failed to compile WASM module")?;

        let metadata = PluginMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: String::new(),
            capabilities: vec![],
            plugin_type: PluginType::Wasm,
            path: PathBuf::new(),
            signature: None,
        };

        Ok(WasmPlugin {
            module: Arc::new(module),
            engine: self.engine.clone(),
            metadata,
            config: self.config.clone(),
        })
    }
}

/// A loaded WASM plugin
pub struct WasmPlugin {
    module: Arc<Module>,
    engine: Engine,
    metadata: PluginMetadata,
    config: WasmConfig,
}

impl WasmPlugin {
    /// Get the module for inspection
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Get exported functions
    pub fn exports(&self) -> Vec<String> {
        self.module
            .exports()
            .filter_map(|e| {
                if matches!(e.ty(), ExternType::Func(_)) {
                    Some(e.name().to_string())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[async_trait]
impl DxPlugin for WasmPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&mut self) -> Result<()> {
        // WASM plugins don't need initialization
        Ok(())
    }

    async fn execute(&self, _ctx: &PluginContext) -> Result<PluginResult> {
        let start_time = Instant::now();

        // Create a new store for this execution
        let mut store = Store::new(&self.engine, ());

        // Set fuel limit if configured
        if let Some(fuel) = self.config.fuel_limit {
            store.set_fuel(fuel)?;
        }

        // Create instance
        let instance = Instance::new(&mut store, &self.module, &[])
            .context("Failed to instantiate WASM module")?;

        // Try to find and call the main function
        let stdout = String::new();
        let mut stderr = String::new();
        let exit_code;

        if let Some(func) = instance.get_func(&mut store, "_start") {
            // WASI-style entry point
            match func.call(&mut store, &[], &mut []) {
                Ok(_) => {
                    exit_code = 0;
                }
                Err(e) => {
                    stderr = format!("WASM execution error: {}", e);
                    exit_code = 1;
                }
            }
        } else if let Some(func) = instance.get_func(&mut store, "main") {
            // Traditional main function
            let mut results = [Val::I32(0)];
            match func.call(&mut store, &[], &mut results) {
                Ok(_) => {
                    exit_code = match results[0] {
                        Val::I32(code) => code,
                        _ => 0,
                    };
                }
                Err(e) => {
                    stderr = format!("WASM execution error: {}", e);
                    exit_code = 1;
                }
            }
        } else if let Some(func) = instance.get_func(&mut store, "run") {
            // DX plugin entry point
            match func.call(&mut store, &[], &mut []) {
                Ok(_) => {
                    exit_code = 0;
                }
                Err(e) => {
                    stderr = format!("WASM execution error: {}", e);
                    exit_code = 1;
                }
            }
        } else {
            stderr = "No entry point found (_start, main, or run)".to_string();
            exit_code = 1;
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Get fuel consumed as a proxy for work done
        let fuel_consumed = if self.config.fuel_limit.is_some() {
            self.config
                .fuel_limit
                .unwrap_or(0)
                .saturating_sub(store.get_fuel().unwrap_or(0))
        } else {
            0
        };

        Ok(PluginResult {
            exit_code,
            stdout,
            stderr,
            duration_ms,
            memory_used: fuel_consumed as usize, // Approximate
            return_value: None,
        })
    }

    async fn shutdown(&mut self) -> Result<()> {
        // WASM plugins don't need explicit shutdown
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_config_default() {
        let config = WasmConfig::default();
        assert_eq!(config.max_memory_pages, 4096);
        assert!(config.enable_wasi);
    }

    #[tokio::test]
    async fn test_wasm_executor_creation() {
        let executor = WasmExecutor::new();
        assert!(executor.is_ok());
    }
}
