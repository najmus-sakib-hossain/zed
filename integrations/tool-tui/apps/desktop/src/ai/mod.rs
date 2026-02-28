// AI module - production integration inspired by Zed's AI system.
//
// Provider API implementations are based on the structures from
// Zed's provider crates (anthropic, open_ai, google_ai, etc.)
// in `crates/ai/`.
pub mod chat;
pub mod client;
pub mod credentials;
pub mod provider;
pub mod registry;
pub mod settings;
pub mod zed_model_catalog;

pub use chat::{ChatMessage, ChatRole};
pub use client::AiClient;
pub use provider::{AiProvider, AiProviderKind};
pub use registry::AiRegistry;
pub use settings::AiSettings;
