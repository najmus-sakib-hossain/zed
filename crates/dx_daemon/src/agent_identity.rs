//! Agent identity — AIEOS-compatible JSON persona.
//!
//! Defines how the AI agent presents itself, following the AIEOS
//! and OpenClaw IDENTITY.md conventions.

use serde::{Deserialize, Serialize};

/// The AI agent's identity configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Agent's display name.
    pub name: String,
    /// Short description of the agent.
    pub description: String,
    /// Version string.
    pub version: String,
    /// The system prompt / persona instructions.
    pub system_prompt: String,
    /// Personality traits.
    pub traits: Vec<String>,
    /// Communication style.
    pub style: CommunicationStyle,
    /// Capabilities this agent has.
    pub capabilities: Vec<String>,
    /// Author / organization.
    pub author: String,
    /// Optional avatar URL or path.
    pub avatar: Option<String>,
}

/// Communication style settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationStyle {
    /// Formality level (0.0 = very casual, 1.0 = very formal).
    pub formality: f32,
    /// Verbosity level (0.0 = terse, 1.0 = verbose).
    pub verbosity: f32,
    /// Humor level (0.0 = serious, 1.0 = playful).
    pub humor: f32,
    /// Emoji usage (0.0 = none, 1.0 = heavy).
    pub emoji_usage: f32,
    /// Preferred response length.
    pub preferred_length: ResponseLength,
}

/// Preferred response length.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ResponseLength {
    Concise,
    Moderate,
    Detailed,
    Comprehensive,
}

impl Default for AgentIdentity {
    fn default() -> Self {
        Self {
            name: "DX Agent".to_string(),
            description: "Universal AI assistant powered by DX".to_string(),
            version: "1.0.0".to_string(),
            system_prompt: "You are DX, a helpful AI assistant. Be concise, accurate, and helpful.".to_string(),
            traits: vec![
                "helpful".to_string(),
                "concise".to_string(),
                "technical".to_string(),
            ],
            style: CommunicationStyle::default(),
            capabilities: vec![
                "text-generation".to_string(),
                "code-generation".to_string(),
                "image-generation".to_string(),
                "voice-conversation".to_string(),
                "grammar-checking".to_string(),
            ],
            author: "DX Team".to_string(),
            avatar: None,
        }
    }
}

impl Default for CommunicationStyle {
    fn default() -> Self {
        Self {
            formality: 0.5,
            verbosity: 0.4,
            humor: 0.2,
            emoji_usage: 0.1,
            preferred_length: ResponseLength::Moderate,
        }
    }
}

impl AgentIdentity {
    /// Load identity from a JSON file.
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let identity: Self = serde_json::from_str(&content)?;
        Ok(identity)
    }

    /// Save identity to a JSON file.
    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load from OpenClaw IDENTITY.md format.
    ///
    /// Parses a markdown file with YAML frontmatter containing identity fields.
    pub fn load_from_identity_md(content: &str) -> anyhow::Result<Self> {
        // Simple parser: extract name from first # heading, system prompt from body
        let mut identity = Self::default();

        for line in content.lines() {
            if let Some(name) = line.strip_prefix("# ") {
                identity.name = name.trim().to_string();
            } else if let Some(desc) = line.strip_prefix("## Description") {
                let _ = desc;
                // Next non-empty line is the description
            }
        }

        // Use the full content as system prompt
        identity.system_prompt = content.to_string();
        Ok(identity)
    }

    /// Generate the system prompt with identity context.
    pub fn build_system_prompt(&self) -> String {
        format!(
            "{}\n\nYou are {}. {}",
            self.system_prompt, self.name, self.description
        )
    }
}
