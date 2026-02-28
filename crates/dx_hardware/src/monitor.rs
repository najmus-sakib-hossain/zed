//! Resource monitor — watches CPU, RAM, GPU utilization in real time.

use dx_core::DeviceTier;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Snapshot of current resource utilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    /// CPU utilization 0.0 – 1.0.
    pub cpu_usage: f64,
    /// RAM used in bytes.
    pub ram_used_bytes: u64,
    /// RAM total in bytes.
    pub ram_total_bytes: u64,
    /// GPU utilization 0.0 – 1.0 (0 if no GPU).
    pub gpu_usage: f64,
    /// VRAM used in bytes.
    pub vram_used_bytes: u64,
    /// Timestamp.
    pub timestamp: std::time::Instant,
}

impl ResourceSnapshot {
    /// RAM usage as fraction.
    pub fn ram_fraction(&self) -> f64 {
        if self.ram_total_bytes == 0 {
            0.0
        } else {
            self.ram_used_bytes as f64 / self.ram_total_bytes as f64
        }
    }

    /// Whether the system is under memory pressure.
    pub fn is_memory_pressure(&self) -> bool {
        self.ram_fraction() > 0.85
    }
}

/// Callback for resource pressure events.
pub type PressureCallback = Arc<dyn Fn(&ResourceSnapshot) + Send + Sync>;

/// Monitors system resources and triggers callbacks on pressure.
pub struct ResourceMonitor {
    snapshots: Vec<ResourceSnapshot>,
    pressure_callbacks: Vec<PressureCallback>,
    max_snapshots: usize,
    current_tier: DeviceTier,
}

impl ResourceMonitor {
    pub fn new(tier: DeviceTier) -> Self {
        Self {
            snapshots: Vec::new(),
            pressure_callbacks: Vec::new(),
            max_snapshots: 60, // Keep last 60 samples
            current_tier: tier,
        }
    }

    /// Register a callback that fires when memory pressure is detected.
    pub fn on_pressure(&mut self, callback: PressureCallback) {
        self.pressure_callbacks.push(callback);
    }

    /// Record a new snapshot and check for pressure.
    pub fn record(&mut self, snapshot: ResourceSnapshot) {
        let is_pressure = snapshot.is_memory_pressure();
        self.snapshots.push(snapshot.clone());

        // Trim old snapshots
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.drain(..self.snapshots.len() - self.max_snapshots);
        }

        if is_pressure {
            for cb in &self.pressure_callbacks {
                cb(&snapshot);
            }
        }
    }

    /// Get the most recent snapshot.
    pub fn latest(&self) -> Option<&ResourceSnapshot> {
        self.snapshots.last()
    }

    /// Average CPU usage over the last N samples.
    pub fn avg_cpu(&self, n: usize) -> f64 {
        let samples: Vec<_> = self.snapshots.iter().rev().take(n).collect();
        if samples.is_empty() {
            return 0.0;
        }
        samples.iter().map(|s| s.cpu_usage).sum::<f64>() / samples.len() as f64
    }

    /// Whether we should suggest downgrading model size.
    pub fn should_downgrade_model(&self) -> bool {
        let avg_cpu = self.avg_cpu(5);
        let latest = self.latest();

        match self.current_tier {
            DeviceTier::UltraLow | DeviceTier::Low => {
                avg_cpu > 0.7
                    || latest.map_or(false, |s| s.ram_fraction() > 0.8)
            }
            DeviceTier::Mid => {
                avg_cpu > 0.85
                    || latest.map_or(false, |s| s.ram_fraction() > 0.85)
            }
            _ => {
                avg_cpu > 0.95
                    || latest.map_or(false, |s| s.ram_fraction() > 0.9)
            }
        }
    }
}
