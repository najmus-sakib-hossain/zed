//! Cost tracking for LLM and media provider usage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a monetary cost in USD microcents (1/1,000,000 of a dollar).
/// This allows tracking sub-cent costs precisely without floating point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MicroCost(pub u64);

impl MicroCost {
    pub const ZERO: Self = Self(0);

    pub fn from_dollars(dollars: f64) -> Self {
        Self((dollars * 1_000_000.0) as u64)
    }

    pub fn as_dollars(&self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }

    pub fn add(&self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl std::ops::Add for MicroCost {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl std::ops::AddAssign for MicroCost {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

/// Token-based pricing for LLM providers (per million tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPricing {
    /// Cost per million input tokens in microcents.
    pub input_per_million: MicroCost,
    /// Cost per million output tokens in microcents.
    pub output_per_million: MicroCost,
    /// Cost per million cached input tokens (if supported).
    pub cached_input_per_million: Option<MicroCost>,
}

/// Per-unit pricing for media providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaPricing {
    /// Cost per generation request in microcents.
    pub per_request: MicroCost,
    /// Cost per second of generated media (video/audio).
    pub per_second: Option<MicroCost>,
    /// Cost per character (TTS).
    pub per_character: Option<MicroCost>,
}

/// Tracks cumulative costs per provider.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CostTracker {
    /// Provider ID → total cost accumulated.
    pub by_provider: HashMap<String, MicroCost>,
    /// Total across all providers.
    pub total: MicroCost,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, provider_id: &str, cost: MicroCost) {
        *self.by_provider.entry(provider_id.to_string()).or_insert(MicroCost::ZERO) += cost;
        self.total += cost;
    }

    pub fn provider_total(&self, provider_id: &str) -> MicroCost {
        self.by_provider.get(provider_id).copied().unwrap_or(MicroCost::ZERO)
    }

    pub fn reset(&mut self) {
        self.by_provider.clear();
        self.total = MicroCost::ZERO;
    }
}

/// Budget configuration for cost control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum daily spend in microcents.
    pub daily_limit: Option<MicroCost>,
    /// Maximum monthly spend in microcents.
    pub monthly_limit: Option<MicroCost>,
    /// Per-provider limits.
    pub provider_limits: HashMap<String, MicroCost>,
    /// Alert threshold (0.0-1.0) — alert at this fraction of limit.
    pub alert_threshold: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            daily_limit: None,
            monthly_limit: None,
            provider_limits: HashMap::new(),
            alert_threshold: 0.8,
        }
    }
}
