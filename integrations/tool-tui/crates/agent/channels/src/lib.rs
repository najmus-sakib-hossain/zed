//! # DX Agent Channels
//!
//! Native Rust messaging channel integrations.
//! Implements Telegram (teloxide), Discord (serenity), Slack, WhatsApp,
//! Matrix (matrix-sdk), and Microsoft Teams (Graph API).
//!
//! ## Infrastructure modules
//!
//! Beyond the per-platform channel implementations, this crate provides
//! cross-cutting infrastructure: health monitoring, security policies,
//! threading, sessions, interactive components, polls, groups, media
//! validation, setup wizards, gateway routing, and unified configuration.

// ── Core ─────────────────────────────────────────────
pub mod allowlist;
pub mod message;
pub mod registry;
pub mod traits;

// ── Platform channels ────────────────────────────────
#[cfg(feature = "telegram")]
pub mod telegram;

#[cfg(feature = "discord")]
pub mod discord;

#[cfg(feature = "slack")]
pub mod slack;

#[cfg(feature = "whatsapp")]
pub mod whatsapp;

pub mod matrix;

#[cfg(feature = "teams")]
pub mod teams;

#[cfg(feature = "google-chat")]
pub mod google_chat;

#[cfg(feature = "signal")]
pub mod signal;

pub mod routing;
pub mod webhook;

// ── Infrastructure modules (NEW) ─────────────────────
pub mod actions;
pub mod config;
pub mod directory;
pub mod gateway;
pub mod groups;
pub mod health;
pub mod interactive;
pub mod media;
pub mod mentions;
pub mod polls;
pub mod security;
pub mod session;
pub mod setup;
pub mod streaming;
pub mod threading;

// ── Re-exports — core ────────────────────────────────
pub use message::{
    ButtonAction, ChannelMessage, DeliveryStatus, IncomingMessage, InlineButton, InlineKeyboard,
    MediaAttachment, MessageContent,
};
pub use registry::ChannelRegistry;
pub use traits::{Channel, ChannelCapabilities, ChannelRegistration};

// ── Re-exports — infrastructure ──────────────────────
pub use actions::{Action, ActionContext, ActionRegistry, ActionResult};
pub use config::{ChannelConfig, ThreadingConfig, load_config, save_config};
pub use directory::{Directory, DirectoryEntry, DirectoryEntryKind};
pub use gateway::{Gateway, GatewayConfig};
pub use groups::{GroupInfo, GroupManager};
pub use health::{ChannelHealth, HealthMonitor};
pub use interactive::{Button, Keyboard};
pub use media::{MediaInfo, MediaLimits, validate_media};
pub use mentions::{detect_mentions, strip_mentions};
pub use polls::{Poll, PollManager};
pub use security::{DmPolicy, SecurityPolicy, check_permission};
pub use session::{SessionData, SessionManager};
pub use setup::{SetupStep, SetupWizard};
pub use streaming::{MessageStream, StreamConfig};
pub use threading::{ReplyMode, ThreadContext, ThreadManager};
