//! Token Usage Metrics
//!
//! Tracks and analyzes token usage patterns.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Token usage metrics collector
#[derive(Debug, Default)]
pub struct TokenMetrics {
    /// Total requests
    pub total_requests: u64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Total tokens saved
    pub total_saved: u64,
    /// Usage by model
    pub by_model: HashMap<String, ModelUsage>,
    /// Usage by operation
    pub by_operation: HashMap<String, OperationUsage>,
    /// Historical data points
    pub history: Vec<MetricDataPoint>,
}

/// Usage per model
#[derive(Debug, Clone, Default)]
pub struct ModelUsage {
    /// Model name
    pub model: String,
    /// Total tokens
    pub tokens: u64,
    /// Request count
    pub requests: u64,
    /// Average tokens per request
    pub avg_tokens: f32,
    /// Estimated cost (USD)
    pub estimated_cost: f32,
}

/// Usage per operation type
#[derive(Debug, Clone, Default)]
pub struct OperationUsage {
    /// Operation name
    pub operation: String,
    /// Total tokens
    pub tokens: u64,
    /// Tokens saved
    pub saved: u64,
    /// Request count
    pub requests: u64,
    /// Compression ratio
    pub compression_ratio: f32,
}

/// Historical data point
#[derive(Debug, Clone)]
pub struct MetricDataPoint {
    /// Timestamp
    pub timestamp: SystemTime,
    /// Tokens used
    pub tokens: u64,
    /// Tokens saved
    pub saved: u64,
    /// Model used
    pub model: String,
    /// Operation type
    pub operation: String,
}

impl TokenMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a token usage event
    pub fn record(&mut self, event: TokenEvent) {
        self.total_requests += 1;
        self.total_tokens += event.tokens;
        self.total_saved += event.saved;

        // Update model usage
        let model_usage = self.by_model.entry(event.model.clone()).or_default();
        model_usage.model = event.model.clone();
        model_usage.tokens += event.tokens;
        model_usage.requests += 1;
        model_usage.avg_tokens = model_usage.tokens as f32 / model_usage.requests as f32;
        let total_tokens = model_usage.tokens;
        model_usage.estimated_cost = Self::estimate_cost_static(&event.model, total_tokens);

        // Update operation usage
        let op_usage = self.by_operation.entry(event.operation.clone()).or_default();
        op_usage.operation = event.operation.clone();
        op_usage.tokens += event.tokens;
        op_usage.saved += event.saved;
        op_usage.requests += 1;
        if op_usage.tokens + op_usage.saved > 0 {
            op_usage.compression_ratio =
                op_usage.saved as f32 / (op_usage.tokens + op_usage.saved) as f32;
        }

        // Add to history
        self.history.push(MetricDataPoint {
            timestamp: SystemTime::now(),
            tokens: event.tokens,
            saved: event.saved,
            model: event.model,
            operation: event.operation,
        });

        // Keep history bounded (last 10000 events)
        if self.history.len() > 10000 {
            self.history.remove(0);
        }
    }

    /// Estimate cost based on model pricing
    fn estimate_cost(&self, model: &str, tokens: u64) -> f32 {
        Self::estimate_cost_static(model, tokens)
    }

    /// Estimate cost based on model pricing (static version)
    fn estimate_cost_static(model: &str, tokens: u64) -> f32 {
        // Approximate pricing per 1K tokens (input + output averaged)
        let price_per_1k = match model {
            m if m.contains("gpt-4") => 0.06,
            m if m.contains("gpt-3.5") => 0.002,
            m if m.contains("claude-3-opus") => 0.075,
            m if m.contains("claude-3-sonnet") => 0.015,
            m if m.contains("claude-3-haiku") => 0.00125,
            _ => 0.01, // Default estimate
        };

        (tokens as f32 / 1000.0) * price_per_1k
    }

    /// Get overall compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.total_tokens + self.total_saved == 0 {
            return 0.0;
        }
        self.total_saved as f32 / (self.total_tokens + self.total_saved) as f32
    }

    /// Get total estimated cost
    pub fn total_cost(&self) -> f32 {
        self.by_model.values().map(|m| m.estimated_cost).sum()
    }

    /// Get usage summary
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_requests: self.total_requests,
            total_tokens: self.total_tokens,
            total_saved: self.total_saved,
            compression_ratio: self.compression_ratio(),
            estimated_cost: self.total_cost(),
            top_operations: self.top_operations(5),
            top_models: self.top_models(3),
        }
    }

    /// Get top operations by token usage
    fn top_operations(&self, limit: usize) -> Vec<OperationUsage> {
        let mut ops: Vec<_> = self.by_operation.values().cloned().collect();
        ops.sort_by(|a, b| b.tokens.cmp(&a.tokens));
        ops.truncate(limit);
        ops
    }

    /// Get top models by token usage
    fn top_models(&self, limit: usize) -> Vec<ModelUsage> {
        let mut models: Vec<_> = self.by_model.values().cloned().collect();
        models.sort_by(|a, b| b.tokens.cmp(&a.tokens));
        models.truncate(limit);
        models
    }

    /// Get usage for a time period
    pub fn usage_in_period(&self, duration: Duration) -> PeriodUsage {
        let cutoff = SystemTime::now() - duration;

        let mut tokens = 0u64;
        let mut saved = 0u64;
        let mut requests = 0u64;

        for point in &self.history {
            if point.timestamp >= cutoff {
                tokens += point.tokens;
                saved += point.saved;
                requests += 1;
            }
        }

        PeriodUsage {
            duration,
            tokens,
            saved,
            requests,
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Token usage event
#[derive(Debug, Clone)]
pub struct TokenEvent {
    /// Tokens used
    pub tokens: u64,
    /// Tokens saved by optimization
    pub saved: u64,
    /// Model used
    pub model: String,
    /// Operation type
    pub operation: String,
}

/// Metrics summary
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    /// Total requests
    pub total_requests: u64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Total tokens saved
    pub total_saved: u64,
    /// Overall compression ratio
    pub compression_ratio: f32,
    /// Estimated total cost
    pub estimated_cost: f32,
    /// Top operations
    pub top_operations: Vec<OperationUsage>,
    /// Top models
    pub top_models: Vec<ModelUsage>,
}

/// Usage in a time period
#[derive(Debug, Clone)]
pub struct PeriodUsage {
    /// Time period
    pub duration: Duration,
    /// Tokens used
    pub tokens: u64,
    /// Tokens saved
    pub saved: u64,
    /// Request count
    pub requests: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_event() {
        let mut metrics = TokenMetrics::new();

        metrics.record(TokenEvent {
            tokens: 1000,
            saved: 500,
            model: "gpt-4".to_string(),
            operation: "completion".to_string(),
        });

        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.total_tokens, 1000);
        assert_eq!(metrics.total_saved, 500);
    }

    #[test]
    fn test_compression_ratio() {
        let mut metrics = TokenMetrics::new();

        metrics.record(TokenEvent {
            tokens: 500,
            saved: 500,
            model: "gpt-4".to_string(),
            operation: "completion".to_string(),
        });

        assert!((metrics.compression_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cost_estimation() {
        let mut metrics = TokenMetrics::new();

        metrics.record(TokenEvent {
            tokens: 1000,
            saved: 0,
            model: "gpt-4".to_string(),
            operation: "completion".to_string(),
        });

        let cost = metrics.total_cost();
        assert!(cost > 0.0);
    }

    #[test]
    fn test_summary() {
        let mut metrics = TokenMetrics::new();

        for i in 0..10 {
            metrics.record(TokenEvent {
                tokens: 100 * (i + 1),
                saved: 50 * (i + 1),
                model: if i % 2 == 0 { "gpt-4" } else { "gpt-3.5-turbo" }.to_string(),
                operation: format!("op_{}", i % 3),
            });
        }

        let summary = metrics.summary();
        assert_eq!(summary.total_requests, 10);
        assert!(summary.total_tokens > 0);
        assert!(!summary.top_operations.is_empty());
        assert!(!summary.top_models.is_empty());
    }

    #[test]
    fn test_period_usage() {
        let mut metrics = TokenMetrics::new();

        metrics.record(TokenEvent {
            tokens: 1000,
            saved: 500,
            model: "gpt-4".to_string(),
            operation: "completion".to_string(),
        });

        let usage = metrics.usage_in_period(Duration::from_secs(60));
        assert_eq!(usage.requests, 1);
        assert_eq!(usage.tokens, 1000);
    }
}
