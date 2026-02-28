//! JavaScript runtime integration for Dx CLI
//!
//! Priority order: Bun → V8 → Deno
//! Bun is the default runtime for npm packages and social media integrations

pub mod bun;
pub mod manager;

pub use bun::BunRuntime;
pub use manager::{JsRuntime, RuntimeManager, RuntimePriority};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Result from JavaScript execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// JavaScript runtime trait
#[async_trait::async_trait]
pub trait Runtime: Send + Sync {
    /// Execute JavaScript code
    async fn eval(&mut self, code: &str) -> Result<JsResult>;

    /// Call a JavaScript function
    async fn call(&mut self, function: &str, args: serde_json::Value) -> Result<JsResult>;

    /// Check if runtime is available
    fn is_available() -> bool;

    /// Get runtime name
    fn name(&self) -> &str;
}
