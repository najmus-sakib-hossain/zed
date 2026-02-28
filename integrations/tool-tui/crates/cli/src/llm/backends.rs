//! LLM backend implementations

use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Ollama,
    Khroma,
    Google,
    Antigravity,
    ElevenLabs,
}

#[async_trait]
pub trait Backend: Send + Sync {
    /// Initialize the backend
    async fn initialize(&mut self) -> Result<()>;

    /// Generate text from prompt
    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String>;

    /// Stream tokens as they're generated
    async fn generate_stream(
        &self,
        prompt: &str,
        max_tokens: usize,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<()>;

    /// Check if backend is available
    fn is_available(&self) -> bool;

    /// Get backend type
    fn backend_type(&self) -> BackendType;
}

pub mod antigravity;
pub mod elevenlabs;
pub mod google;
pub mod khroma;
pub mod ollama;
