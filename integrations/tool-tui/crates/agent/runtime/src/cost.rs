//! Cost tracking for LLM usage

use chrono::{DateTime, Datelike, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::{Usage, known_models};

/// Cost entry for a single request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub timestamp: DateTime<Utc>,
    pub provider: String,
    pub model: String,
    pub usage: Usage,
    pub cost_usd: f64,
    pub session_id: Option<String>,
}

/// Aggregated cost summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cost_usd: f64,
    pub total_requests: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub cost_by_provider: std::collections::HashMap<String, f64>,
    pub cost_by_model: std::collections::HashMap<String, f64>,
}

/// Cost tracker with budget alerts
pub struct CostTracker {
    entries: Arc<RwLock<Vec<CostEntry>>>,
    model_costs: DashMap<String, (f64, f64)>, // model -> (input_cost_per_1k, output_cost_per_1k)
    monthly_budget: Option<f64>,
    daily_budget: Option<f64>,
    alert_threshold: f64,
}

impl CostTracker {
    pub fn new(monthly_budget: Option<f64>, daily_budget: Option<f64>) -> Self {
        let tracker = Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            model_costs: DashMap::new(),
            monthly_budget,
            daily_budget,
            alert_threshold: 0.8,
        };

        // Load known model costs
        for model in known_models() {
            tracker
                .model_costs
                .insert(model.id, (model.input_cost_per_1k, model.output_cost_per_1k));
        }

        tracker
    }

    /// Record usage for a request
    pub fn record(
        &self,
        provider: &str,
        model: &str,
        usage: &Usage,
        session_id: Option<&str>,
    ) -> CostEntry {
        let cost = self.calculate_cost(model, usage);

        let entry = CostEntry {
            timestamp: Utc::now(),
            provider: provider.to_string(),
            model: model.to_string(),
            usage: usage.clone(),
            cost_usd: cost,
            session_id: session_id.map(String::from),
        };

        self.entries.write().push(entry.clone());
        entry
    }

    /// Calculate cost for given model and usage
    pub fn calculate_cost(&self, model: &str, usage: &Usage) -> f64 {
        if let Some(costs) = self.model_costs.get(model) {
            let (input_cost, output_cost) = *costs;
            let input = (usage.prompt_tokens as f64 / 1000.0) * input_cost;
            let output = (usage.completion_tokens as f64 / 1000.0) * output_cost;
            input + output
        } else {
            // Default estimate: $0.01/1k tokens
            usage.total_tokens as f64 / 1000.0 * 0.01
        }
    }

    /// Get cost summary for today
    pub fn today_summary(&self) -> CostSummary {
        let today = Utc::now().date_naive();
        let entries = self.entries.read();

        let mut summary = CostSummary::default();
        for entry in entries.iter() {
            if entry.timestamp.date_naive() == today {
                summary.total_cost_usd += entry.cost_usd;
                summary.total_requests += 1;
                summary.total_prompt_tokens += entry.usage.prompt_tokens as u64;
                summary.total_completion_tokens += entry.usage.completion_tokens as u64;
                *summary.cost_by_provider.entry(entry.provider.clone()).or_insert(0.0) +=
                    entry.cost_usd;
                *summary.cost_by_model.entry(entry.model.clone()).or_insert(0.0) += entry.cost_usd;
            }
        }
        summary
    }

    /// Get cost summary for current month
    pub fn monthly_summary(&self) -> CostSummary {
        let now = Utc::now();
        let entries = self.entries.read();

        let mut summary = CostSummary::default();
        for entry in entries.iter() {
            if entry.timestamp.month() == now.month() && entry.timestamp.year() == now.year() {
                summary.total_cost_usd += entry.cost_usd;
                summary.total_requests += 1;
                summary.total_prompt_tokens += entry.usage.prompt_tokens as u64;
                summary.total_completion_tokens += entry.usage.completion_tokens as u64;
                *summary.cost_by_provider.entry(entry.provider.clone()).or_insert(0.0) +=
                    entry.cost_usd;
                *summary.cost_by_model.entry(entry.model.clone()).or_insert(0.0) += entry.cost_usd;
            }
        }
        summary
    }

    /// Check if budget limit is approaching or exceeded
    pub fn check_budget(&self) -> BudgetStatus {
        let daily = self.today_summary();
        let monthly = self.monthly_summary();

        if let Some(daily_limit) = self.daily_budget {
            if daily.total_cost_usd >= daily_limit {
                return BudgetStatus::DailyExceeded {
                    used: daily.total_cost_usd,
                    limit: daily_limit,
                };
            }
            if daily.total_cost_usd >= daily_limit * self.alert_threshold {
                return BudgetStatus::DailyWarning {
                    used: daily.total_cost_usd,
                    limit: daily_limit,
                };
            }
        }

        if let Some(monthly_limit) = self.monthly_budget {
            if monthly.total_cost_usd >= monthly_limit {
                return BudgetStatus::MonthlyExceeded {
                    used: monthly.total_cost_usd,
                    limit: monthly_limit,
                };
            }
            if monthly.total_cost_usd >= monthly_limit * self.alert_threshold {
                return BudgetStatus::MonthlyWarning {
                    used: monthly.total_cost_usd,
                    limit: monthly_limit,
                };
            }
        }

        BudgetStatus::Ok
    }

    /// Get all entries
    pub fn entries(&self) -> Vec<CostEntry> {
        self.entries.read().clone()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.entries.write().clear();
    }
}

/// Budget status
#[derive(Debug, Clone)]
pub enum BudgetStatus {
    Ok,
    DailyWarning { used: f64, limit: f64 },
    DailyExceeded { used: f64, limit: f64 },
    MonthlyWarning { used: f64, limit: f64 },
    MonthlyExceeded { used: f64, limit: f64 },
}

impl BudgetStatus {
    pub fn is_exceeded(&self) -> bool {
        matches!(self, BudgetStatus::DailyExceeded { .. } | BudgetStatus::MonthlyExceeded { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_cost_tracking() {
        let tracker = CostTracker::new(Some(100.0), Some(10.0));

        let usage = Usage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
        };

        let entry = tracker.record("openai", "gpt-4o", &usage, Some("session-1"));
        assert!(entry.cost_usd > 0.0);
    }

    #[test]
    fn test_budget_check() {
        let tracker = CostTracker::new(Some(100.0), Some(0.001));

        let usage = Usage {
            prompt_tokens: 10000,
            completion_tokens: 5000,
            total_tokens: 15000,
        };

        tracker.record("openai", "gpt-4o", &usage, None);
        let status = tracker.check_budget();
        // With such a small daily budget, should be exceeded
        assert!(status.is_exceeded());
    }

    #[test]
    fn test_monthly_summary() {
        let tracker = CostTracker::new(None, None);

        let usage = Usage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        tracker.record("anthropic", "claude-sonnet-4-20250514", &usage, None);
        tracker.record("openai", "gpt-4o", &usage, None);

        let summary = tracker.monthly_summary();
        assert_eq!(summary.total_requests, 2);
        assert!(summary.total_cost_usd > 0.0);
        assert_eq!(summary.cost_by_provider.len(), 2);
    }
}
