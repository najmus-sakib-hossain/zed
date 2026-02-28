use serde::{Deserialize, Serialize};

/// Role in a chat conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
        }
    }

    #[allow(dead_code)]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
        }
    }
}

/// An active conversation with an AI model.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Conversation {
    pub messages: Vec<ChatMessage>,
    pub provider_id: String,
    pub model_id: String,
}

impl Conversation {
    pub fn new(provider_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            messages: Vec::new(),
            provider_id: provider_id.into(),
            model_id: model_id.into(),
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}
