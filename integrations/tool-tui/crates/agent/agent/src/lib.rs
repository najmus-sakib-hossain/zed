//! # DX Agent - AGI-like AI Agent
//!
//! The DX Agent is a revolutionary AI system that can:
//! - Connect to ANY app (WhatsApp, Telegram, Discord, GitHub, Notion, Spotify, etc.)
//! - Create its own integrations dynamically via WASM compilation
//! - Auto-update itself by detecting local changes and creating PRs
//! - Run 24/7 as a daemon with minimal CPU usage
//! - Save 70%+ tokens using DX Serializer LLM format
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                     DX AGENT DAEMON                          │
//! ├──────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │   Skills    │  │ Integrations│  │    WASM Runtime     │  │
//! │  │   System    │  │   Manager   │  │ (Python/JS → WASM)  │  │
//! │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
//! │         │                │                     │             │
//! │         └────────────────┼─────────────────────┘             │
//! │                          │                                   │
//! │  ┌───────────────────────▼────────────────────────────────┐  │
//! │  │              Message Bus (async channels)              │  │
//! │  └───────────────────────┬────────────────────────────────┘  │
//! │                          │                                   │
//! │  ┌───────────────────────▼────────────────────────────────┐  │
//! │  │                DX Serializer (LLM Format)              │  │
//! │  │              52-73% token savings vs JSON              │  │

// Allow some pedantic lints for this early-stage agent code
#![allow(dead_code)]
//! │  └────────────────────────────────────────────────────────┘  │
//! │                                                              │
//! └──────────────────────────────────────────────────────────────┘
//!           │                    │                    │
//!           ▼                    ▼                    ▼
//!    ┌──────────┐         ┌──────────┐         ┌──────────┐
//!    │ WhatsApp │         │ Telegram │         │  GitHub  │
//!    │ Discord  │         │  Notion  │         │ Spotify  │
//!    │ Messenger│         │   Slack  │         │    ...   │
//!    └──────────┘         └──────────┘         └──────────┘
//! ```
//!
//! ## Key Features
//!
//! 1. **Self-Creating Integrations**: The agent can write code in any language
//!    (Python, Node.js, etc.), compile it to WASM, and inject it as a new plugin
//!
//! 2. **Auto-PR System**: When local DX has new features not in the repo,
//!    it automatically creates a PR to share the integration
//!
//! 3. **24/7 Daemon**: Runs continuously with minimal CPU, executing tasks
//!    like checking emails, updating Notion, coding websites - all in parallel
//!
//! 4. **DX Serializer Integration**: All LLM communication uses DX format
//!    to save 70%+ tokens compared to JSON

pub mod auth;
pub mod capabilities;
pub mod daemon;
pub mod integrations;
pub mod llm;
pub mod plugins;
pub mod pr_detector;
pub mod scheduler;
pub mod skills;
pub mod wasm_runtime;

// Re-exports
pub use auth::{AuthProvider, OAuthFlow, TokenStore};
pub use capabilities::{Capability, CapabilityRegistry};
pub use daemon::{AgentDaemon, DaemonConfig};
pub use integrations::{Integration, IntegrationConfig, IntegrationManager};
pub use llm::{LlmClient, LlmMessage, LlmResponse};
pub use plugins::{Plugin, PluginLoader, PluginManifest};
pub use pr_detector::{LocalDiff, PrDetector};
pub use scheduler::{CronSchedule, Task, TaskScheduler};
pub use skills::{Skill, SkillDefinition, SkillRegistry};
pub use wasm_runtime::{WasmModule, WasmPlugin, WasmRuntime};

use thiserror::Error;

/// Agent-specific errors
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Integration not found: {0}")]
    IntegrationNotFound(String),

    #[error("Authentication failed for {provider}: {message}")]
    AuthFailed { provider: String, message: String },

    #[error("WASM compilation failed: {0}")]
    WasmCompilationFailed(String),

    #[error("Skill execution failed: {0}")]
    SkillExecutionFailed(String),

    #[error("Plugin load failed: {0}")]
    PluginLoadFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AgentError>;
