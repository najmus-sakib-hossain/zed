//! Health check types for the gateway.

use serde::{Deserialize, Serialize};

/// Health check status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub connections: usize,
    pub sessions: usize,
}
