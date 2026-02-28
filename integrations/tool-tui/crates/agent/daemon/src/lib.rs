//! # DX Agent Daemon
//!
//! Cross-platform service management: Windows Service, systemd, launchd.

pub mod service;

pub use service::{ServiceConfig, ServiceManager, ServiceStatus, ServiceType};
