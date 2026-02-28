//! Share service — unified sharing API.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::platforms::{SharePlatform, ShareTarget};

/// A request to share content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareRequest {
    /// Text content to share.
    pub text: String,
    /// Optional title/subject.
    pub title: Option<String>,
    /// Attached media (URLs or base64 data).
    pub media_urls: Vec<String>,
    /// Target platform.
    pub platform: SharePlatform,
    /// Target ID override (optional).
    pub target_id: Option<String>,
}

/// Result of a share operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareResult {
    pub platform: SharePlatform,
    pub success: bool,
    /// URL of the shared content (if applicable).
    pub url: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Service that manages sharing across all platforms.
pub struct SocialShareService {
    targets: HashMap<SharePlatform, ShareTarget>,
}

impl SocialShareService {
    pub fn new() -> Self {
        let mut service = Self {
            targets: HashMap::new(),
        };
        // Clipboard is always available
        service.register_target(ShareTarget::clipboard());
        service
    }

    /// Register a sharing target.
    pub fn register_target(&mut self, target: ShareTarget) {
        self.targets.insert(target.platform, target);
    }

    /// Remove a target.
    pub fn remove_target(&mut self, platform: SharePlatform) {
        self.targets.remove(&platform);
    }

    /// List all configured targets.
    pub fn targets(&self) -> impl Iterator<Item = &ShareTarget> {
        self.targets.values()
    }

    /// Available targets (configured and enabled).
    pub fn available_targets(&self) -> Vec<&ShareTarget> {
        self.targets.values().filter(|t| t.enabled).collect()
    }

    /// Share content to a platform.
    pub async fn share(&self, request: &ShareRequest) -> ShareResult {
        let target = match self.targets.get(&request.platform) {
            Some(t) if t.enabled => t,
            Some(_) => {
                return ShareResult {
                    platform: request.platform,
                    success: false,
                    url: None,
                    error: Some("Platform is disabled".into()),
                };
            }
            None => {
                return ShareResult {
                    platform: request.platform,
                    success: false,
                    url: None,
                    error: Some("Platform not configured".into()),
                };
            }
        };

        // Truncate text if platform has a limit
        let text = if let Some(max_len) = request.platform.max_text_length() {
            if request.text.len() > max_len {
                format!("{}…", &request.text[..max_len - 1])
            } else {
                request.text.clone()
            }
        } else {
            request.text.clone()
        };

        match request.platform {
            SharePlatform::Clipboard => self.share_clipboard(&text),
            SharePlatform::Twitter => self.share_rest(target, &text, "Twitter").await,
            SharePlatform::Mastodon => self.share_rest(target, &text, "Mastodon").await,
            SharePlatform::Bluesky => self.share_rest(target, &text, "Bluesky").await,
            SharePlatform::Discord => self.share_rest(target, &text, "Discord").await,
            SharePlatform::Slack => self.share_rest(target, &text, "Slack").await,
            SharePlatform::Telegram => self.share_rest(target, &text, "Telegram").await,
            SharePlatform::Email => self.share_email(target, &request.title, &text).await,
            SharePlatform::Gist => self.share_rest(target, &text, "Gist").await,
            _ => ShareResult {
                platform: request.platform,
                success: false,
                url: None,
                error: Some(format!(
                    "{} not yet implemented",
                    request.platform.display_name()
                )),
            },
        }
    }

    fn share_clipboard(&self, text: &str) -> ShareResult {
        // Placeholder — real implementation uses clipboard crate
        log::info!("Copied {} chars to clipboard", text.len());
        ShareResult {
            platform: SharePlatform::Clipboard,
            success: true,
            url: None,
            error: None,
        }
    }

    async fn share_rest(
        &self,
        _target: &ShareTarget,
        text: &str,
        platform_name: &str,
    ) -> ShareResult {
        // Placeholder — real implementation makes HTTP requests
        log::info!("{}: would share {} chars", platform_name, text.len());
        ShareResult {
            platform: _target.platform,
            success: true,
            url: None,
            error: None,
        }
    }

    async fn share_email(
        &self,
        _target: &ShareTarget,
        subject: &Option<String>,
        body: &str,
    ) -> ShareResult {
        log::info!(
            "Email: would send {} chars, subject: {:?}",
            body.len(),
            subject
        );
        ShareResult {
            platform: SharePlatform::Email,
            success: true,
            url: None,
            error: None,
        }
    }
}

impl Default for SocialShareService {
    fn default() -> Self {
        Self::new()
    }
}
