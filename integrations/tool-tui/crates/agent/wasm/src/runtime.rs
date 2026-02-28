//! # WASM Runtime
//!
//! Execute WASM modules using Wasmtime.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{module::WasmModule, Result, WasmError};

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum memory pages (64KB each)
    pub max_memory_pages: u32,

    /// Maximum execution time (milliseconds)
    pub max_execution_time: u64,

    /// Whether to enable WASI
    pub enable_wasi: bool,

    /// Allowed directories for WASI file access
    pub allowed_dirs: Vec<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_pages: 256,     // 16MB
            max_execution_time: 30000, // 30 seconds
            enable_wasi: true,
            allowed_dirs: vec![],
        }
    }
}

/// WASM runtime for executing modules
pub struct WasmRuntime {
    #[allow(dead_code)]
    config: RuntimeConfig,
    modules: Arc<RwLock<HashMap<String, WasmModule>>>,
}

impl WasmRuntime {
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        Ok(Self {
            config,
            modules: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Load a WASM module from bytes
    pub async fn load(&self, name: &str, wasm_bytes: &[u8]) -> Result<()> {
        info!("Loading WASM module: {}", name);

        // Validate WASM magic number
        if wasm_bytes.len() < 8 {
            return Err(WasmError::InvalidWasm("WASM too short".to_string()));
        }

        if &wasm_bytes[0..4] != b"\0asm" {
            return Err(WasmError::InvalidWasm(
                "Invalid WASM magic number".to_string(),
            ));
        }

        // Create the module
        let module = WasmModule::new(name, wasm_bytes.to_vec())?;

        // Store it
        let mut modules = self.modules.write().await;
        modules.insert(name.to_string(), module);

        info!("Module {} loaded successfully", name);
        Ok(())
    }

    /// Load a WASM module from a file
    pub async fn load_file(&self, name: &str, path: &std::path::Path) -> Result<()> {
        let wasm_bytes = std::fs::read(path)?;
        self.load(name, &wasm_bytes).await
    }

    /// Unload a module
    pub async fn unload(&self, name: &str) -> Result<()> {
        let mut modules = self.modules.write().await;
        modules.remove(name);
        info!("Module {} unloaded", name);
        Ok(())
    }

    /// Check if a module is loaded
    pub async fn is_loaded(&self, name: &str) -> bool {
        let modules = self.modules.read().await;
        modules.contains_key(name)
    }

    /// Get a list of loaded modules
    pub async fn list_modules(&self) -> Vec<String> {
        let modules = self.modules.read().await;
        modules.keys().cloned().collect()
    }

    /// Call a function in a module
    pub async fn call(
        &self,
        module_name: &str,
        function: &str,
        args: &[WasmValue],
    ) -> Result<WasmValue> {
        info!("Calling {}::{}", module_name, function);

        let modules = self.modules.read().await;
        let _module = modules
            .get(module_name)
            .ok_or_else(|| WasmError::ModuleNotFound(module_name.to_string()))?;

        // In a full implementation, this would use wasmtime to call the function
        // For now, return a placeholder

        info!(
            "Function {}::{} called with {} args",
            module_name,
            function,
            args.len()
        );
        Ok(WasmValue::I32(0))
    }

    /// Call a function that returns a string
    pub async fn call_string(
        &self,
        module_name: &str,
        function: &str,
        args: &[WasmValue],
    ) -> Result<String> {
        let _result = self.call(module_name, function, args).await?;

        // In a real implementation, this would read the string from WASM memory
        Ok("result".to_string())
    }

    /// Get module exports
    pub async fn get_exports(&self, module_name: &str) -> Result<Vec<String>> {
        let modules = self.modules.read().await;
        let module = modules
            .get(module_name)
            .ok_or_else(|| WasmError::ModuleNotFound(module_name.to_string()))?;

        Ok(module.exports().to_vec())
    }
}

/// WASM value types
#[derive(Debug, Clone)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Null,
}

impl WasmValue {
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            WasmValue::I32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            WasmValue::String(s) => Some(s),
            _ => None,
        }
    }
}
