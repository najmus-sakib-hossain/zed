//! dx_daemon — Background agent daemon.
//!
//! Runs as a persistent background service (systemd/launchd/Windows Service)
//! providing: scheduled tasks, channel routing (Telegram/Discord/Slack/Email),
//! memory engine (HNSW + BM25), VPS deployment, agent supervision,
//! agent identity management, and remote agent monitoring.

pub mod agent_identity;
pub mod channel;
pub mod cron;
pub mod memory;
pub mod remote_monitor;
pub mod service;
pub mod supervisor;
pub mod vps;

pub use agent_identity::AgentIdentity;
pub use channel::{ChannelMessage, ChannelRouter, ChannelType};
pub use cron::{CronJob, CronScheduler};
pub use memory::{MemoryEngine, MemoryEntry};
pub use remote_monitor::{RemoteAgent, RemoteHealth, RemoteMonitor, SecureChannel};
pub use service::{DaemonService, DaemonState};
pub use supervisor::{ProcessState, SupervisedProcess, Supervisor};
pub use vps::VpsDeployer;
