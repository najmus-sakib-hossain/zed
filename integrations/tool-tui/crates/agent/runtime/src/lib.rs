//! DX Agent Runtime - Multi-provider LLM abstraction
//!
//! Provides a unified interface for interacting with multiple LLM providers
//! (OpenAI, Anthropic, Google, Ollama, etc.) with streaming, failover,
//! token counting, and cost tracking.

pub mod config;
pub mod cost;
pub mod models;
pub mod provider;
pub mod providers;
pub mod router;
pub mod streaming;

pub use config::RuntimeConfig;
pub use cost::CostTracker;
pub use models::{ChatMessage, ChatRequest, ChatResponse, ModelInfo, Role};
pub use provider::{LlmProvider, ProviderCapabilities};
pub use router::ModelRouter;
pub use streaming::StreamEvent;
