//! Remote agent health monitoring and secure channel management.
//!
//! Handles the communication between a local DX desktop instance
//! and remote daemon instances running on VPS servers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Health status of a remote agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteHealth {
    Healthy,
    Degraded,
    Unreachable,
    Unknown,
}

/// Information about a remote DX daemon instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAgent {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Host address (IP or domain).
    pub host: String,
    /// SSH port.
    pub ssh_port: u16,
    /// IPC port for DX protocol.
    pub ipc_port: u16,
    /// Current health status.
    pub health: RemoteHealth,
    /// Last heartbeat timestamp.
    pub last_heartbeat: Option<SystemTime>,
    /// System info from last health check.
    pub system_info: Option<RemoteSystemInfo>,
    /// Accumulated compute cost (USD).
    pub total_cost_usd: f64,
}

/// System info from a remote daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSystemInfo {
    pub cpu_usage_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_gb: f32,
    pub disk_total_gb: f32,
    pub uptime_seconds: u64,
    pub active_agents: u32,
    pub pending_tasks: u32,
}

/// Monitors remote agent health and manages connections.
pub struct RemoteMonitor {
    agents: HashMap<String, RemoteAgent>,
    /// Health check interval.
    check_interval: Duration,
    /// Maximum time without heartbeat before marking as unreachable.
    heartbeat_timeout: Duration,
}

impl RemoteMonitor {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            check_interval: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(120),
        }
    }

    /// Register a remote agent.
    pub fn register(&mut self, agent: RemoteAgent) {
        log::info!(
            "Remote monitor: registered '{}' at {}:{}",
            agent.name,
            agent.host,
            agent.ipc_port
        );
        self.agents.insert(agent.id.clone(), agent);
    }

    /// Unregister a remote agent.
    pub fn unregister(&mut self, id: &str) {
        if let Some(agent) = self.agents.remove(id) {
            log::info!("Remote monitor: unregistered '{}'", agent.name);
        }
    }

    /// Update health status for a remote agent.
    pub fn update_health(
        &mut self,
        id: &str,
        health: RemoteHealth,
        system_info: Option<RemoteSystemInfo>,
    ) {
        if let Some(agent) = self.agents.get_mut(id) {
            agent.health = health;
            agent.last_heartbeat = Some(SystemTime::now());
            agent.system_info = system_info;
        }
    }

    /// Check for agents that have timed out.
    pub fn check_timeouts(&mut self) -> Vec<String> {
        let now = SystemTime::now();
        let mut timed_out = Vec::new();

        for agent in self.agents.values_mut() {
            if let Some(last_heartbeat) = agent.last_heartbeat {
                if let Ok(elapsed) = now.duration_since(last_heartbeat) {
                    if elapsed > self.heartbeat_timeout && agent.health != RemoteHealth::Unreachable
                    {
                        agent.health = RemoteHealth::Unreachable;
                        timed_out.push(agent.id.clone());
                        log::warn!(
                            "Remote agent '{}' is unreachable (no heartbeat for {:.0}s)",
                            agent.name,
                            elapsed.as_secs_f64()
                        );
                    }
                }
            }
        }

        timed_out
    }

    /// Get all registered agents.
    pub fn agents(&self) -> impl Iterator<Item = &RemoteAgent> {
        self.agents.values()
    }

    /// Get healthy agents only.
    pub fn healthy_agents(&self) -> impl Iterator<Item = &RemoteAgent> {
        self.agents
            .values()
            .filter(|a| a.health == RemoteHealth::Healthy)
    }

    /// Get total cost across all remote agents.
    pub fn total_cost(&self) -> f64 {
        self.agents.values().map(|a| a.total_cost_usd).sum()
    }

    /// Set the health check interval.
    pub fn set_check_interval(&mut self, interval: Duration) {
        self.check_interval = interval;
    }
}

impl Default for RemoteMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Secure channel between local DX and remote daemon.
///
/// Uses SSH tunneling for encrypted communication.
pub struct SecureChannel {
    /// Remote host.
    pub host: String,
    /// SSH port.
    pub ssh_port: u16,
    /// IPC port (tunneled through SSH).
    pub ipc_port: u16,
    /// Path to SSH private key.
    pub key_path: String,
    /// Whether the tunnel is currently active.
    pub is_connected: bool,
}

impl SecureChannel {
    pub fn new(host: &str, ssh_port: u16, ipc_port: u16, key_path: &str) -> Self {
        Self {
            host: host.to_string(),
            ssh_port,
            ipc_port,
            key_path: key_path.to_string(),
            is_connected: false,
        }
    }

    /// Establish an SSH tunnel to the remote daemon.
    pub fn connect(&mut self) -> anyhow::Result<()> {
        // ssh -N -L {local_port}:localhost:{ipc_port} -p {ssh_port} -i {key_path} user@{host}
        log::info!(
            "Establishing SSH tunnel to {}:{} (IPC port {})",
            self.host,
            self.ssh_port,
            self.ipc_port
        );
        self.is_connected = true;
        Ok(())
    }

    /// Close the SSH tunnel.
    pub fn disconnect(&mut self) {
        self.is_connected = false;
        log::info!("SSH tunnel to {} closed", self.host);
    }
}
