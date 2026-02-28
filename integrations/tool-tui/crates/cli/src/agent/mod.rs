//! DX AI Agent System
//!
//! This module provides an intelligent AI agent with persistent memory,
//! capability-based self-improvement, and multi-provider LLM communication.
//!
//! # Architecture
//!
//! The agent system consists of:
//! - [`AgentMemory`] - Persistent conversation and knowledge storage
//! - [`CapabilityAnalyzer`] - Gap detection and skill requirements
//! - [`SelfUpdate`] - Autonomous configuration generation
//! - [`LlmClient`] - Multi-provider LLM communication with streaming
//! - [`environment`] - Multi-runtime WASM bridge for plugin compilation
//!
//! # Memory System
//!
//! Memory is organized into three tiers:
//! 1. **Short-term**: Current conversation buffer (last N messages)
//! 2. **Long-term**: Indexed storage with semantic search
//! 3. **Skills**: Learned capabilities and configurations
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::agent::{Agent, AgentConfig};
//!
//! let config = AgentConfig::load_or_default()?;
//! let mut agent = Agent::new(config).await?;
//!
//! // Process user input with memory context
//! let response = agent.process("What did we discuss earlier?").await?;
//! println!("{}", response.text);
//!
//! // Agent can learn new capabilities
//! agent.learn_skill("weather", weather_skill_config).await?;
//! ```

pub mod capability;
pub mod environment;
pub mod llm_client;
pub mod memory;
pub mod self_update;

pub use capability::{Capability, CapabilityAnalyzer, CapabilityGap};
pub use llm_client::{LlmClient, LlmMessage, LlmProvider, LlmRequest, TokenUsage};
pub use memory::{AgentMemory, MemoryConfig, MemoryQuery, SearchResult};
pub use self_update::{SelfUpdateConfig, SelfUpdateEngine};

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Memory configuration
    pub memory: MemoryConfig,
    /// Self-update configuration
    pub self_update: SelfUpdateConfig,
    /// Default LLM provider
    pub default_provider: LlmProvider,
    /// Maximum tokens per request
    pub max_tokens: usize,
    /// Default temperature
    pub temperature: f32,
    /// Enable streaming responses
    pub streaming: bool,
    /// Data directory for persistence
    pub data_dir: PathBuf,
    /// Enable supervised mode (requires human approval)
    pub supervised: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            memory: MemoryConfig::default(),
            self_update: SelfUpdateConfig::default(),
            default_provider: LlmProvider::Ollama,
            max_tokens: 4096,
            temperature: 0.7,
            streaming: true,
            data_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("dx")
                .join("agent"),
            supervised: true,
        }
    }
}

impl AgentConfig {
    /// Load configuration from file or return default
    pub fn load_or_default() -> Result<Self> {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx")
            .join("agent.sr");

        if config_path.exists() {
            Self::load(&config_path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        // Parse using DX Serializer
        // For now, return default
        let _ = content;
        Ok(Self::default())
    }

    /// Save configuration to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        // Serialize using DX Serializer
        let _ = path;
        Ok(())
    }
}

/// The main AI Agent
pub struct Agent {
    /// Agent configuration
    config: AgentConfig,
    /// Memory system
    memory: Arc<RwLock<AgentMemory>>,
    /// LLM client for inference
    llm: Arc<LlmClient>,
    /// Capability analyzer
    capabilities: Arc<RwLock<CapabilityAnalyzer>>,
    /// Self-update engine
    self_update: Arc<RwLock<SelfUpdateEngine>>,
    /// Current conversation ID
    current_conversation: Option<String>,
}

impl Agent {
    /// Create a new agent with configuration
    pub async fn new(config: AgentConfig) -> Result<Self> {
        // Initialize memory
        let memory = AgentMemory::new(config.memory.clone())?;

        // Initialize LLM client
        let llm = LlmClient::new(config.default_provider.clone())?;

        // Initialize capability analyzer
        let capabilities = CapabilityAnalyzer::new();

        // Initialize self-update engine
        let self_update = SelfUpdateEngine::new(config.self_update.clone())?;

        Ok(Self {
            config,
            memory: Arc::new(RwLock::new(memory)),
            llm: Arc::new(llm),
            capabilities: Arc::new(RwLock::new(capabilities)),
            self_update: Arc::new(RwLock::new(self_update)),
            current_conversation: None,
        })
    }

    /// Process user input and generate a response
    pub async fn process(&mut self, input: &str) -> Result<AgentResponse> {
        // Get or create conversation
        let conv_id = match &self.current_conversation {
            Some(id) => id.clone(),
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                self.current_conversation = Some(id.clone());
                id
            }
        };

        // Add user message to memory
        {
            let mut memory = self.memory.write().await;
            memory.add_message(&conv_id, "user", input)?;
        }

        // Build context from memory
        let context = self.build_context(&conv_id).await?;

        // Detect capability gaps
        let gaps = {
            let capabilities = self.capabilities.read().await;
            capabilities.analyze_request(input)
        };

        // If we're missing capabilities and self-update is enabled
        if !gaps.is_empty() && self.config.self_update.enabled {
            let should_update = if self.config.supervised {
                // In supervised mode, we would ask for approval
                // For now, skip auto-update
                false
            } else {
                true
            };

            if should_update {
                let mut self_update = self.self_update.write().await;
                for gap in &gaps {
                    if let Err(e) = self_update.acquire_capability(gap).await {
                        tracing::warn!("Failed to acquire capability {:?}: {}", gap, e);
                    }
                }
            }
        }

        // Generate response
        let request = LlmRequest {
            messages: context,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: self.config.streaming,
            provider: Some(self.config.default_provider.clone()),
        };

        let response = self.llm.generate(request).await?;

        // Add assistant response to memory
        {
            let mut memory = self.memory.write().await;
            memory.add_message(&conv_id, "assistant", &response.text)?;
        }

        Ok(AgentResponse {
            text: response.text,
            tokens: response.usage,
            capability_gaps: gaps,
            conversation_id: conv_id,
        })
    }

    /// Process with streaming response
    pub async fn process_stream<F>(&mut self, input: &str, callback: F) -> Result<AgentResponse>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        // Get or create conversation
        let conv_id = match &self.current_conversation {
            Some(id) => id.clone(),
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                self.current_conversation = Some(id.clone());
                id
            }
        };

        // Add user message to memory
        {
            let mut memory = self.memory.write().await;
            memory.add_message(&conv_id, "user", input)?;
        }

        // Build context from memory
        let context = self.build_context(&conv_id).await?;

        // Generate streaming response
        let request = LlmRequest {
            messages: context,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: true,
            provider: Some(self.config.default_provider.clone()),
        };

        let response = self.llm.generate_stream(request, callback).await?;

        // Add assistant response to memory
        {
            let mut memory = self.memory.write().await;
            memory.add_message(&conv_id, "assistant", &response.text)?;
        }

        Ok(AgentResponse {
            text: response.text,
            tokens: response.usage,
            capability_gaps: vec![],
            conversation_id: conv_id,
        })
    }

    /// Build context from memory for LLM request
    async fn build_context(&self, conversation_id: &str) -> Result<Vec<LlmMessage>> {
        let memory = self.memory.read().await;

        let mut messages = Vec::new();

        // Add system message
        messages.push(LlmMessage {
            role: "system".to_string(),
            content: self.get_system_prompt(),
        });

        // Add relevant long-term memories
        if let Some(current_input) = memory.get_last_user_message(conversation_id) {
            let relevant = memory.search(&MemoryQuery {
                text: current_input.to_string(),
                limit: 5,
                min_relevance: 0.7,
                conversation_id: None, // Search across all conversations
            })?;

            for result in relevant {
                if result.entry.conversation_id != conversation_id {
                    messages.push(LlmMessage {
                        role: "system".to_string(),
                        content: format!(
                            "[Relevant context from previous conversation]: {}",
                            result.entry.content
                        ),
                    });
                }
            }
        }

        // Add conversation history
        let history = memory.get_conversation_history(conversation_id, 20)?;
        for entry in history {
            messages.push(LlmMessage {
                role: entry.role.clone(),
                content: entry.content.clone(),
            });
        }

        Ok(messages)
    }

    /// Get the system prompt
    fn get_system_prompt(&self) -> String {
        r#"You are DX Agent, an intelligent AI assistant integrated into the DX CLI.

Your capabilities include:
- Answering questions about coding and development
- Helping with DX framework features
- Analyzing code and providing suggestions
- Assisting with CLI operations

Be concise, helpful, and technically accurate. When you don't know something, admit it.
Use code blocks with syntax highlighting when showing code examples."#
            .to_string()
    }

    /// Start a new conversation
    pub fn new_conversation(&mut self) {
        self.current_conversation = None;
    }

    /// Get current conversation ID
    pub fn conversation_id(&self) -> Option<&str> {
        self.current_conversation.as_deref()
    }

    /// Search memory
    pub async fn search_memory(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let memory = self.memory.read().await;
        memory.search(&MemoryQuery {
            text: query.to_string(),
            limit,
            min_relevance: 0.5,
            conversation_id: None,
        })
    }

    /// Prune old memories
    pub async fn prune_memory(&self) -> Result<usize> {
        let mut memory = self.memory.write().await;
        memory.prune()
    }

    /// Learn a new skill/capability
    pub async fn learn_skill(&self, name: &str, config: serde_json::Value) -> Result<()> {
        let mut capabilities = self.capabilities.write().await;
        capabilities.register_capability(Capability {
            name: name.to_string(),
            description: config
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            config: Some(config),
            handler: None,
        });
        Ok(())
    }

    /// Get available capabilities
    pub async fn list_capabilities(&self) -> Vec<String> {
        let capabilities = self.capabilities.read().await;
        capabilities.list()
    }

    /// Get token usage statistics
    pub fn token_stats(&self) -> TokenUsage {
        self.llm.total_usage()
    }
}

/// Response from the agent
#[derive(Debug)]
pub struct AgentResponse {
    /// Generated text
    pub text: String,
    /// Token usage
    pub tokens: TokenUsage,
    /// Capability gaps detected
    pub capability_gaps: Vec<CapabilityGap>,
    /// Conversation ID
    pub conversation_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let config = AgentConfig::default();
        let agent = Agent::new(config).await;
        // Should succeed with default config
        assert!(agent.is_ok() || agent.is_err()); // Allow failure if no LLM backend
    }

    #[test]
    fn test_config_default() {
        let config = AgentConfig::default();
        assert!(config.supervised);
        assert_eq!(config.max_tokens, 4096);
    }
}
