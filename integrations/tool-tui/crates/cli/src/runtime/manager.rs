//! Runtime manager for selecting and managing JavaScript runtimes

use super::{BunRuntime, JsResult, Runtime};
use anyhow::{Result, anyhow};
use serde_json::Value;

/// Runtime priority order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePriority {
    Bun,
    V8,
    Deno,
}

/// JavaScript runtime wrapper
pub enum JsRuntime {
    Bun(BunRuntime),
    // V8 and Deno will be added later
}

impl JsRuntime {
    /// Create runtime with auto-detection (Bun → V8 → Deno)
    pub fn new() -> Result<Self> {
        // Try Bun first (priority 1)
        if BunRuntime::is_available() {
            return Ok(Self::Bun(BunRuntime::new()?));
        }

        Err(anyhow!("No JavaScript runtime available. Please install Bun."))
    }

    /// Create runtime with specific priority
    pub fn with_priority(priority: RuntimePriority) -> Result<Self> {
        match priority {
            RuntimePriority::Bun => Ok(Self::Bun(BunRuntime::new()?)),
            RuntimePriority::V8 => Err(anyhow!("V8 runtime not yet implemented")),
            RuntimePriority::Deno => Err(anyhow!("Deno runtime not yet implemented")),
        }
    }

    /// Get runtime name
    pub fn name(&self) -> &str {
        match self {
            Self::Bun(rt) => rt.name(),
        }
    }

    /// Execute JavaScript code
    pub async fn eval(&mut self, code: &str) -> Result<JsResult> {
        match self {
            Self::Bun(rt) => rt.eval(code).await,
        }
    }

    /// Call JavaScript function
    pub async fn call(&mut self, function: &str, args: Value) -> Result<JsResult> {
        match self {
            Self::Bun(rt) => rt.call(function, args).await,
        }
    }
}

/// Runtime manager for the CLI
pub struct RuntimeManager {
    runtime: JsRuntime,
}

impl RuntimeManager {
    /// Create new runtime manager with auto-detection
    pub fn new() -> Result<Self> {
        let runtime = JsRuntime::new()?;
        tracing::info!("Using {} runtime", runtime.name());
        Ok(Self { runtime })
    }

    /// Create runtime manager with specific priority
    pub fn with_priority(priority: RuntimePriority) -> Result<Self> {
        let runtime = JsRuntime::with_priority(priority)?;
        tracing::info!("Using {} runtime", runtime.name());
        Ok(Self { runtime })
    }

    /// Execute JavaScript code
    pub async fn eval(&mut self, code: &str) -> Result<JsResult> {
        self.runtime.eval(code).await
    }

    /// Call JavaScript function
    pub async fn call(&mut self, function: &str, args: Value) -> Result<JsResult> {
        self.runtime.call(function, args).await
    }

    /// Get runtime name
    pub fn runtime_name(&self) -> &str {
        self.runtime.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_manager_creation() {
        let manager = RuntimeManager::new();
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_runtime_manager_eval() {
        if let Ok(mut manager) = RuntimeManager::new() {
            let result = manager.eval("return 'hello'").await;
            assert!(result.is_ok());
        }
    }
}
