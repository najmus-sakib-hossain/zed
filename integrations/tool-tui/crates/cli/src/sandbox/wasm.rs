//! WebAssembly-based sandbox for safe code execution

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use wasmtime::*;

use super::backend::{SandboxBackend, SandboxBackendType, SandboxResult};
use super::config::SandboxConfig;

/// WASM-based sandbox for executing untrusted code
pub struct WasmSandbox {
    engine: Engine,
    store: Option<Store<()>>,
    config: SandboxConfig,
}

impl WasmSandbox {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_multi_memory(true);
        config.wasm_bulk_memory(true);

        let engine = Engine::new(&config)?;

        Ok(Self {
            engine,
            store: None,
            config: SandboxConfig::default(),
        })
    }

    /// Compile and execute WASM module
    async fn execute_wasm(&self, wasm_bytes: &[u8]) -> Result<SandboxResult> {
        let start_time = Instant::now();

        let module =
            Module::new(&self.engine, wasm_bytes).context("Failed to compile WASM module")?;

        let mut store = Store::new(&self.engine, ());

        let instance =
            Instance::new(&mut store, &module, &[]).context("Failed to instantiate WASM module")?;

        // Try to call main/start function
        let stdout = String::new();
        let mut stderr = String::new();
        let exit_code = if let Some(func) = instance.get_func(&mut store, "_start") {
            match func.call(&mut store, &[], &mut []) {
                Ok(_) => 0,
                Err(e) => {
                    stderr = format!("WASM execution error: {}", e);
                    1
                }
            }
        } else {
            stderr = "No _start function found in WASM module".to_string();
            1
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(SandboxResult {
            exit_code,
            stdout,
            stderr,
            duration_ms,
        })
    }
}

#[async_trait]
impl SandboxBackend for WasmSandbox {
    async fn create(&mut self, config: &SandboxConfig) -> Result<()> {
        self.config = config.clone();
        self.store = Some(Store::new(&self.engine, ()));
        Ok(())
    }

    async fn execute(&self, command: &[String]) -> Result<SandboxResult> {
        if command.is_empty() {
            return Err(anyhow::anyhow!("No command provided"));
        }

        // For WASM sandbox, first argument should be path to .wasm file
        let wasm_path = &command[0];
        let wasm_bytes = tokio::fs::read(wasm_path).await.context("Failed to read WASM file")?;

        self.execute_wasm(&wasm_bytes).await
    }

    async fn copy_in(&self, host_path: &Path, _sandbox_path: &Path) -> Result<()> {
        // WASM sandbox doesn't have a filesystem, just validate file exists
        tokio::fs::metadata(host_path).await.context("Host file not found")?;
        Ok(())
    }

    async fn copy_out(&self, _sandbox_path: &Path, _host_path: &Path) -> Result<()> {
        // WASM sandbox doesn't have a filesystem
        Ok(())
    }

    async fn destroy(&mut self) -> Result<()> {
        self.store = None;
        Ok(())
    }

    fn is_available() -> bool {
        true // WASM is always available
    }

    fn backend_type(&self) -> SandboxBackendType {
        SandboxBackendType::Wasm
    }
}

/// Resource limiter for WASM execution
struct ResourceLimiter {
    memory_limit: usize,
}

impl wasmtime::ResourceLimiter for ResourceLimiter {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        Ok(desired <= self.memory_limit)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        Ok(desired <= 1000) // Limit table size
    }
}
