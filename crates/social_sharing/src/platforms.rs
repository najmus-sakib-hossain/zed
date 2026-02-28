//! Platform definitions for social sharing.

use serde::{Deserialize, Serialize};

/// Supported sharing platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SharePlatform {
    // Social media
    Twitter,
    Mastodon,
    Bluesky,
    LinkedIn,
    Reddit,

    // Communication
    Telegram,
    Discord,
    Slack,
    Email,

    // File sharing
    Gist,
    Pastebin,

    // Clipboard
    Clipboard,
}

impl SharePlatform {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Twitter => "Twitter/X",
            Self::Mastodon => "Mastodon",
            Self::Bluesky => "Bluesky",
            Self::LinkedIn => "LinkedIn",
            Self::Reddit => "Reddit",
            Self::Telegram => "Telegram",
            Self::Discord => "Discord",
            Self::Slack => "Slack",
            Self::Email => "Email",
            Self::Gist => "GitHub Gist",
            Self::Pastebin => "Pastebin",
            Self::Clipboard => "Clipboard",
        }
    }

    /// Whether this platform requires authentication.
    pub fn requires_auth(&self) -> bool {
        !matches!(self, Self::Clipboard)
    }

    /// Maximum text length for the platform.
    pub fn max_text_length(&self) -> Option<usize> {
        match self {
            Self::Twitter => Some(280),
            Self::Mastodon => Some(500),
            Self::Bluesky => Some(300),
            Self::Discord => Some(2000),
            Self::Slack => Some(40000),
            _ => None,
        }
    }

    /// Whether the platform supports media attachments.
    pub fn supports_media(&self) -> bool {
        matches!(
            self,
            Self::Twitter
                | Self::Mastodon
                | Self::Bluesky
                | Self::Discord
                | Self::Slack
                | Self::Telegram
                | Self::Email
        )
    }
}

/// A configured sharing target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareTarget {
    pub platform: SharePlatform,
    pub name: String,
    /// API token or auth credential.
    pub token: Option<String>,
    /// Target channel/user/group ID.
    pub target_id: Option<String>,
    /// Instance URL (for federated platforms like Mastodon).
    pub instance_url: Option<String>,
    pub enabled: bool,
}

impl ShareTarget {
    pub fn clipboard() -> Self {
        Self {
            platform: SharePlatform::Clipboard,
            name: "Clipboard".into(),
            token: None,
            target_id: None,
            instance_url: None,
            enabled: true,
        }
    }
}
