//! Runtime execution environment

pub mod async_runtime;
pub mod builtins;
pub mod builtins_instance;
pub mod child_process;
pub mod console;
pub mod context;
pub mod crypto;
pub mod datetime;
pub mod dotenv;
pub mod events;
pub mod http;
pub mod memory;
pub mod nodejs;
pub mod regexp;
pub mod streams;
pub mod unhandled_rejection;
pub mod url;
pub mod util;

use crate::compiler::CompiledModule;
use crate::error::DxResult;
use crate::value::Value;

/// Runtime configuration
#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    /// Arena size per worker
    pub arena_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            arena_size: 256 * 1024 * 1024, // 256MB
        }
    }
}

/// The runtime execution environment
pub struct Runtime {
    #[allow(dead_code)]
    config: RuntimeConfig,
    arena: memory::Arena,
}

impl Runtime {
    /// Create a new runtime
    pub fn new(config: RuntimeConfig) -> DxResult<Self> {
        let arena = memory::Arena::new(config.arena_size)?;

        Ok(Self { config, arena })
    }

    /// Execute a compiled module
    pub fn execute(&mut self, module: &CompiledModule) -> DxResult<Value> {
        // Reset arena for this execution
        self.arena.reset();

        // Execute the module using its built-in execute method
        module.execute()
    }

    /// Get arena usage
    #[allow(dead_code)]
    pub fn memory_usage(&self) -> usize {
        self.arena.usage()
    }
}
