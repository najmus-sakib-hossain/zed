//! dx_daemon — Background agent daemon.
//!
//! Runs as a persistent background service (systemd/launchd/Windows Service)
//! providing: scheduled tasks, channel routing (Telegram/Discord/Slack/Email),
//! memory engine (HNSW + BM25), and VPS deployment.

pub mod channel;
pub mod cron;
pub mod memory;
pub mod service;
pub mod vps;

pub use channel::{ChannelMessage, ChannelRouter, ChannelType};
pub use cron::{CronJob, CronScheduler};
pub use memory::{MemoryEngine, MemoryEntry};
pub use service::{DaemonService, DaemonState};
pub use vps::VpsDeployer;
