//! # DX Integrations
//!
//! Pre-built integrations for connecting to any app:
//! - Messaging: WhatsApp, Telegram, Discord, Slack, X, Messenger
//! - Dev Tools: GitHub, GitLab, Linear, Jira
//! - Productivity: Notion, Todoist, Google Calendar
//! - Media: Spotify, YouTube
//! - Browser: Chrome, Firefox (via CDP)
//!
//! Each integration is designed to work with the DX Agent and uses

// Allow some pedantic lints for this early-stage integration code
#![allow(dead_code)]
//! DX Serializer format for token-efficient communication.

pub mod browser;
pub mod devtools;
pub mod media;
pub mod messaging;
pub mod productivity;

// Re-exports
pub use browser::BrowserIntegration;
pub use devtools::{GitHubIntegration, GitLabIntegration};
pub use media::SpotifyIntegration;
pub use messaging::{
    DiscordIntegration, SlackIntegration, TelegramIntegration, WhatsAppIntegration,
};
pub use productivity::{NotionIntegration, TodoistIntegration};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Integration-specific errors
#[derive(Error, Debug)]
pub enum IntegrationError {
    #[error("Not authenticated for {0}")]
    NotAuthenticated(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Rate limited: retry after {0} seconds")]
    RateLimited(u64),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Network error: {0}")]
    NetworkError(String),
}

pub type Result<T> = std::result::Result<T, IntegrationError>;

/// Common message format for all messaging integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub platform: String,
}

impl Message {
    /// Convert to DX LLM format
    pub fn to_dx_llm(&self) -> String {
        format!(
            "msg:1[id={} sender={} content={} ts={} platform={}]",
            self.id,
            self.sender.replace(' ', "_"),
            self.content.replace(' ', "_").replace('\n', "\\n"),
            self.timestamp,
            self.platform
        )
    }
}

/// Common trait for all integrations
#[async_trait]
pub trait Integration: Send + Sync {
    /// Get the integration name
    fn name(&self) -> &str;

    /// Get the integration type
    fn integration_type(&self) -> &str;

    /// Check if authenticated
    fn is_authenticated(&self) -> bool;

    /// Authenticate with the service
    async fn authenticate(&mut self, token: &str) -> Result<()>;

    /// Disconnect from the service
    async fn disconnect(&mut self) -> Result<()>;

    /// Get capabilities as DX format
    fn capabilities_dx(&self) -> String;
}

/// Trait for messaging integrations
#[async_trait]
pub trait MessagingIntegration: Integration {
    /// Send a message
    async fn send_message(&self, recipient: &str, content: &str) -> Result<Message>;

    /// Poll for new messages
    async fn poll_messages(&self) -> Result<Vec<Message>>;

    /// Mark a message as read
    async fn mark_read(&self, message_id: &str) -> Result<()>;
}

/// Trait for dev tool integrations
#[async_trait]
pub trait DevToolIntegration: Integration {
    /// Create a pull request
    async fn create_pr(&self, repo: &str, title: &str, body: &str, branch: &str) -> Result<String>;

    /// Create an issue
    async fn create_issue(&self, repo: &str, title: &str, body: &str) -> Result<String>;

    /// List repositories
    async fn list_repos(&self) -> Result<Vec<String>>;
}

/// Trait for productivity integrations
#[async_trait]
pub trait ProductivityIntegration: Integration {
    /// Create a page/document
    async fn create_page(&self, title: &str, content: &str) -> Result<String>;

    /// Update a page
    async fn update_page(&self, page_id: &str, content: &str) -> Result<()>;

    /// Query/search
    async fn query(&self, query: &str) -> Result<Vec<String>>;
}

/// Trait for media integrations
#[async_trait]
pub trait MediaIntegration: Integration {
    /// Play/resume
    async fn play(&self) -> Result<()>;

    /// Pause
    async fn pause(&self) -> Result<()>;

    /// Skip to next
    async fn next(&self) -> Result<()>;

    /// Search
    async fn search(&self, query: &str) -> Result<Vec<String>>;
}

/// Trait for browser integrations
#[async_trait]
pub trait BrowserControlIntegration: Integration {
    /// Navigate to a URL
    async fn navigate(&self, url: &str) -> Result<()>;

    /// Click an element
    async fn click(&self, selector: &str) -> Result<()>;

    /// Type text
    async fn type_text(&self, selector: &str, text: &str) -> Result<()>;

    /// Take a screenshot
    async fn screenshot(&self) -> Result<Vec<u8>>;

    /// Get page content
    async fn get_content(&self) -> Result<String>;
}
