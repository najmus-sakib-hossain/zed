//! Dynamic model swapping — runtime resource management.
//!
//! Monitors system resources in real-time and makes decisions about:
//! - Swapping to smaller quantized models under memory pressure
//! - Power-aware model selection (battery vs plugged in)
//! - Idle detection for background model upgrades
//! - Multi-feature sharing of GPU resources

use dx_core::DeviceTier;
use serde::{Deserialize, Serialize};

/// Power source state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    /// On AC power — full performance.
    PluggedIn,
    /// On battery with sufficient charge (>30%).
    Battery,
    /// Low battery (<30%) — aggressive power saving.
    LowBattery,
    /// Unknown power state.
    Unknown,
}

/// System resource snapshot at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    /// CPU utilization (0.0 to 1.0).
    pub cpu_usage: f64,
    /// RAM usage in bytes.
    pub ram_used: u64,
    /// Total RAM in bytes.
    pub ram_total: u64,
    /// GPU VRAM usage in bytes (if available).
    pub vram_used: Option<u64>,
    /// Total GPU VRAM in bytes (if available).
    pub vram_total: Option<u64>,
    /// GPU temperature in Celsius (if available).
    pub gpu_temp: Option<f64>,
    /// Power state.
    pub power_state: PowerState,
    /// System idle time in seconds.
    pub idle_seconds: u64,
}

impl ResourceSnapshot {
    /// RAM pressure as a fraction (0.0 to 1.0).
    pub fn ram_pressure(&self) -> f64 {
        if self.ram_total == 0 {
            return 1.0;
        }
        self.ram_used as f64 / self.ram_total as f64
    }

    /// VRAM pressure as a fraction (0.0 to 1.0).
    pub fn vram_pressure(&self) -> Option<f64> {
        match (self.vram_used, self.vram_total) {
            (Some(used), Some(total)) if total > 0 => Some(used as f64 / total as f64),
            _ => None,
        }
    }

    /// Whether the system is idle (no user activity for 5+ minutes).
    pub fn is_idle(&self) -> bool {
        self.idle_seconds >= 300
    }

    /// Whether thermal throttling might be occurring.
    pub fn is_thermal_throttled(&self) -> bool {
        self.gpu_temp.map_or(false, |t| t > 90.0)
    }
}

/// Model swap decision — what action the swap engine recommends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapDecision {
    /// Keep current model — no action needed.
    KeepCurrent,
    /// Downgrade to a smaller quantization (e.g., Q5_K_M → Q4_K_M).
    Downgrade { from_quant: String, to_quant: String },
    /// Upgrade to a larger model (system has spare resources).
    Upgrade { from_model: String, to_model: String },
    /// Unload a non-essential model to free memory.
    Unload { model_id: String, reason: String },
    /// Switch from GPU to CPU inference (thermal throttling or low VRAM).
    FallbackToCpu { model_id: String },
    /// Load a larger model during idle time.
    IdleUpgrade { model_id: String },
}

/// Dynamic model swapper — decides when to swap models based on resource state.
pub struct ModelSwapper {
    tier: DeviceTier,
    swap_history: Vec<(std::time::Instant, SwapDecision)>,
    min_swap_interval: std::time::Duration,
}

impl ModelSwapper {
    /// Create a new model swapper for the given tier.
    pub fn new(tier: DeviceTier) -> Self {
        Self {
            tier,
            swap_history: Vec::new(),
            min_swap_interval: std::time::Duration::from_secs(30),
        }
    }

    /// Evaluate current resources and decide if a model swap is needed.
    pub fn evaluate(&mut self, snapshot: &ResourceSnapshot) -> Vec<SwapDecision> {
        // Don't swap too frequently
        if let Some((last_time, _)) = self.swap_history.last() {
            if last_time.elapsed() < self.min_swap_interval {
                return vec![SwapDecision::KeepCurrent];
            }
        }

        let mut decisions = Vec::new();

        // RAM pressure check
        if snapshot.ram_pressure() > 0.90 {
            decisions.push(SwapDecision::Downgrade {
                from_quant: "Q5_K_M".to_string(),
                to_quant: "Q4_K_M".to_string(),
            });
        }

        // VRAM pressure check
        if let Some(vram_pressure) = snapshot.vram_pressure() {
            if vram_pressure > 0.95 {
                decisions.push(SwapDecision::Unload {
                    model_id: "lowest_priority".to_string(),
                    reason: "VRAM pressure >95%".to_string(),
                });
            }
        }

        // Thermal throttling
        if snapshot.is_thermal_throttled() {
            decisions.push(SwapDecision::FallbackToCpu {
                model_id: "current_gpu_model".to_string(),
            });
        }

        // Battery conservation
        if snapshot.power_state == PowerState::LowBattery {
            decisions.push(SwapDecision::Downgrade {
                from_quant: "current".to_string(),
                to_quant: "smallest_available".to_string(),
            });
        }

        // Idle upgrade opportunity
        if snapshot.is_idle() && snapshot.ram_pressure() < 0.60 {
            let _ = self.tier;
            decisions.push(SwapDecision::IdleUpgrade {
                model_id: "next_tier_model".to_string(),
            });
        }

        if decisions.is_empty() {
            decisions.push(SwapDecision::KeepCurrent);
        }

        // Record decisions
        let now = std::time::Instant::now();
        for decision in &decisions {
            self.swap_history.push((now, decision.clone()));
        }

        // Keep history manageable
        if self.swap_history.len() > 100 {
            self.swap_history.drain(..50);
        }

        decisions
    }

    /// Get recent swap history.
    pub fn recent_history(&self) -> &[(std::time::Instant, SwapDecision)] {
        &self.swap_history
    }
}
