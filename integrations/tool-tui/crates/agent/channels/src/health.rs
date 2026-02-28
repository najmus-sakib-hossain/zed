//! Health monitoring and diagnostics for channel connections.
//!
//! Tracks connection state, error history, reconnect attempts,
//! and message throughput for each registered channel.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Health snapshot for a single channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelHealth {
    /// Channel identifier.
    pub channel_id: String,
    /// Whether the channel is currently connected.
    pub connected: bool,
    /// Timestamp of last successful connection.
    pub last_connected_at: Option<DateTime<Utc>>,
    /// Timestamp of last disconnection.
    pub last_disconnected_at: Option<DateTime<Utc>>,
    /// Total reconnect attempts since last stable connection.
    pub reconnect_attempts: u32,
    /// Last error message (if any).
    pub last_error: Option<String>,
    /// Timestamp of last inbound/outbound message.
    pub last_message_at: Option<DateTime<Utc>>,
    /// Total messages processed since startup.
    pub messages_processed: u64,
    /// Total errors recorded since startup.
    pub error_count: u64,
}

impl ChannelHealth {
    /// Create a fresh health record for a channel.
    pub fn new(channel_id: impl Into<String>) -> Self {
        Self {
            channel_id: channel_id.into(),
            connected: false,
            last_connected_at: None,
            last_disconnected_at: None,
            reconnect_attempts: 0,
            last_error: None,
            last_message_at: None,
            messages_processed: 0,
            error_count: 0,
        }
    }
}

/// Centralized health monitor for all channels.
///
/// Thread-safe via `DashMap`; can be shared across tasks
/// with `Arc<HealthMonitor>`.
#[derive(Clone)]
pub struct HealthMonitor {
    data: Arc<DashMap<String, ChannelHealth>>,
}

impl HealthMonitor {
    /// Create a new, empty health monitor.
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    /// Ensure a health record exists for `channel_id`.
    pub fn register(&self, channel_id: &str) {
        self.data
            .entry(channel_id.to_string())
            .or_insert_with(|| ChannelHealth::new(channel_id));
    }

    /// Record a successful connection event.
    pub fn record_connection(&self, channel_id: &str) {
        let mut entry = self
            .data
            .entry(channel_id.to_string())
            .or_insert_with(|| ChannelHealth::new(channel_id));
        entry.connected = true;
        entry.last_connected_at = Some(Utc::now());
        entry.reconnect_attempts = 0;
        entry.last_error = None;
        info!(channel = channel_id, "Channel connected");
    }

    /// Record a disconnection, optionally with an error.
    pub fn record_disconnection(&self, channel_id: &str, error: Option<String>) {
        let mut entry = self
            .data
            .entry(channel_id.to_string())
            .or_insert_with(|| ChannelHealth::new(channel_id));
        entry.connected = false;
        entry.last_disconnected_at = Some(Utc::now());
        if let Some(ref err) = error {
            entry.last_error = Some(err.clone());
            entry.error_count += 1;
            warn!(
                channel = channel_id,
                error = %err,
                "Channel disconnected with error"
            );
        } else {
            info!(channel = channel_id, "Channel disconnected");
        }
    }

    /// Record a reconnect attempt.
    pub fn record_reconnect_attempt(&self, channel_id: &str) {
        let mut entry = self
            .data
            .entry(channel_id.to_string())
            .or_insert_with(|| ChannelHealth::new(channel_id));
        entry.reconnect_attempts += 1;
    }

    /// Record that a message was processed.
    pub fn record_message(&self, channel_id: &str) {
        let mut entry = self
            .data
            .entry(channel_id.to_string())
            .or_insert_with(|| ChannelHealth::new(channel_id));
        entry.messages_processed += 1;
        entry.last_message_at = Some(Utc::now());
    }

    /// Record an error without disconnecting.
    pub fn record_error(&self, channel_id: &str, error: &str) {
        let mut entry = self
            .data
            .entry(channel_id.to_string())
            .or_insert_with(|| ChannelHealth::new(channel_id));
        entry.last_error = Some(error.to_string());
        entry.error_count += 1;
    }

    /// Get a snapshot of a channel's health.
    pub fn get_health(&self, channel_id: &str) -> Option<ChannelHealth> {
        self.data.get(channel_id).map(|r| r.value().clone())
    }

    /// Check whether the channel is considered healthy
    /// (connected, no recent errors).
    pub fn is_healthy(&self, channel_id: &str) -> bool {
        self.data
            .get(channel_id)
            .map(|h| h.connected && h.last_error.is_none())
            .unwrap_or(false)
    }

    /// List all tracked channel IDs.
    pub fn list_channels(&self) -> Vec<String> {
        self.data.iter().map(|r| r.key().clone()).collect()
    }

    /// Get health snapshots for every tracked channel.
    pub fn all_health(&self) -> Vec<ChannelHealth> {
        self.data.iter().map(|r| r.value().clone()).collect()
    }

    /// Remove tracking data for a channel.
    pub fn remove(&self, channel_id: &str) {
        self.data.remove(channel_id);
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_health_record() {
        let h = ChannelHealth::new("telegram");
        assert_eq!(h.channel_id, "telegram");
        assert!(!h.connected);
        assert_eq!(h.reconnect_attempts, 0);
        assert!(h.last_error.is_none());
    }

    #[test]
    fn test_record_connection() {
        let mon = HealthMonitor::new();
        mon.record_connection("tg");

        let h = mon.get_health("tg").expect("should exist");
        assert!(h.connected);
        assert!(h.last_connected_at.is_some());
        assert!(mon.is_healthy("tg"));
    }

    #[test]
    fn test_record_disconnection_with_error() {
        let mon = HealthMonitor::new();
        mon.record_connection("tg");
        mon.record_disconnection("tg", Some("timeout".into()));

        let h = mon.get_health("tg").expect("should exist");
        assert!(!h.connected);
        assert_eq!(h.last_error.as_deref(), Some("timeout"));
        assert_eq!(h.error_count, 1);
        assert!(!mon.is_healthy("tg"));
    }

    #[test]
    fn test_record_message() {
        let mon = HealthMonitor::new();
        mon.record_connection("tg");
        mon.record_message("tg");
        mon.record_message("tg");

        let h = mon.get_health("tg").expect("should exist");
        assert_eq!(h.messages_processed, 2);
        assert!(h.last_message_at.is_some());
    }

    #[test]
    fn test_reconnect_attempts() {
        let mon = HealthMonitor::new();
        mon.record_reconnect_attempt("tg");
        mon.record_reconnect_attempt("tg");
        mon.record_reconnect_attempt("tg");

        let h = mon.get_health("tg").expect("should exist");
        assert_eq!(h.reconnect_attempts, 3);

        // Connection resets the counter
        mon.record_connection("tg");
        let h = mon.get_health("tg").expect("should exist");
        assert_eq!(h.reconnect_attempts, 0);
    }

    #[test]
    fn test_list_and_all() {
        let mon = HealthMonitor::new();
        mon.register("a");
        mon.register("b");

        assert_eq!(mon.list_channels().len(), 2);
        assert_eq!(mon.all_health().len(), 2);
    }

    #[test]
    fn test_remove() {
        let mon = HealthMonitor::new();
        mon.register("x");
        assert!(mon.get_health("x").is_some());
        mon.remove("x");
        assert!(mon.get_health("x").is_none());
    }

    #[test]
    fn test_is_healthy_unknown_channel() {
        let mon = HealthMonitor::new();
        assert!(!mon.is_healthy("nonexistent"));
    }
}
