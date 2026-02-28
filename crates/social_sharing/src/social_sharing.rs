//! social_sharing — Share AI outputs to external platforms.
//!
//! Provides a unified API for sharing text, images, and media to
//! social platforms (Twitter/X, Mastodon, Bluesky) and communication
//! channels (Telegram, Discord, Slack, Email).

pub mod platforms;
pub mod share_service;

pub use platforms::{SharePlatform, ShareTarget};
pub use share_service::{ShareRequest, ShareResult, SocialShareService};
