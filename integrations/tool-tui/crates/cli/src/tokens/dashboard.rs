//! Token Usage Dashboard
//!
//! Real-time token usage visualization and monitoring.

use super::metrics::{MetricsSummary, TokenMetrics};
use std::io::{self, Write};
use std::time::Duration;

/// Token usage dashboard
pub struct TokenDashboard {
    /// Metrics reference
    metrics: std::sync::Arc<tokio::sync::RwLock<TokenMetrics>>,
    /// Refresh interval
    refresh_interval: Duration,
}

impl TokenDashboard {
    /// Create a new dashboard
    pub fn new(
        metrics: std::sync::Arc<tokio::sync::RwLock<TokenMetrics>>,
        refresh_interval: Duration,
    ) -> Self {
        Self {
            metrics,
            refresh_interval,
        }
    }

    /// Render dashboard to string
    pub async fn render(&self) -> String {
        let metrics = self.metrics.read().await;
        let summary = metrics.summary();

        Self::format_dashboard(&summary)
    }

    /// Format dashboard as string
    fn format_dashboard(summary: &MetricsSummary) -> String {
        let mut output = String::new();

        // Header
        output.push_str("╔══════════════════════════════════════════════════════════════════╗\n");
        output.push_str("║                    DX Token Usage Dashboard                       ║\n");
        output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");

        // Overall stats
        output.push_str(&format!(
            "║  Total Requests: {:>10}  │  Total Tokens: {:>12}        ║\n",
            format_number(summary.total_requests),
            format_number(summary.total_tokens)
        ));

        output.push_str(&format!(
            "║  Tokens Saved:   {:>10}  │  Compression: {:>11}%        ║\n",
            format_number(summary.total_saved),
            format!("{:.1}", summary.compression_ratio * 100.0)
        ));

        output.push_str(&format!(
            "║  Estimated Cost: ${:>9.4}                                        ║\n",
            summary.estimated_cost
        ));

        // Savings bar
        output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");
        output.push_str("║  Token Savings:                                                   ║\n");
        output.push_str(&format!("║  {}║\n", Self::render_bar(summary.compression_ratio, 60)));

        // Top operations
        output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");
        output.push_str("║  Top Operations:                                                  ║\n");

        for op in &summary.top_operations {
            output.push_str(&format!(
                "║    {:<20} {:>10} tokens ({:.0}% saved)           ║\n",
                truncate(&op.operation, 20),
                format_number(op.tokens),
                op.compression_ratio * 100.0
            ));
        }

        // Top models
        output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");
        output.push_str("║  Model Usage:                                                     ║\n");

        for model in &summary.top_models {
            output.push_str(&format!(
                "║    {:<20} {:>10} tokens  ${:.4}               ║\n",
                truncate(&model.model, 20),
                format_number(model.tokens),
                model.estimated_cost
            ));
        }

        // Footer
        output.push_str("╚══════════════════════════════════════════════════════════════════╝\n");

        output
    }

    /// Render a progress bar
    fn render_bar(ratio: f32, width: usize) -> String {
        let filled = (ratio * width as f32).round() as usize;
        let empty = width.saturating_sub(filled);

        format!("[{}{}] {:.1}%", "█".repeat(filled.min(width)), "░".repeat(empty), ratio * 100.0)
    }

    /// Print dashboard to stdout
    pub async fn print(&self) {
        let dashboard = self.render().await;
        print!("\x1B[2J\x1B[1;1H"); // Clear screen
        println!("{}", dashboard);
        io::stdout().flush().ok();
    }

    /// Run dashboard in live mode
    pub async fn run_live(&self) -> io::Result<()> {
        loop {
            self.print().await;
            tokio::time::sleep(self.refresh_interval).await;
        }
    }

    /// Export metrics as JSON
    pub async fn export_json(&self) -> serde_json::Value {
        let metrics = self.metrics.read().await;
        let summary = metrics.summary();

        serde_json::json!({
            "total_requests": summary.total_requests,
            "total_tokens": summary.total_tokens,
            "total_saved": summary.total_saved,
            "compression_ratio": summary.compression_ratio,
            "estimated_cost": summary.estimated_cost,
            "top_operations": summary.top_operations.iter().map(|op| {
                serde_json::json!({
                    "operation": op.operation,
                    "tokens": op.tokens,
                    "saved": op.saved,
                    "requests": op.requests,
                    "compression_ratio": op.compression_ratio
                })
            }).collect::<Vec<_>>(),
            "top_models": summary.top_models.iter().map(|m| {
                serde_json::json!({
                    "model": m.model,
                    "tokens": m.tokens,
                    "requests": m.requests,
                    "avg_tokens": m.avg_tokens,
                    "estimated_cost": m.estimated_cost
                })
            }).collect::<Vec<_>>()
        })
    }

    /// Export metrics as CSV
    pub async fn export_csv(&self) -> String {
        let metrics = self.metrics.read().await;
        let summary = metrics.summary();

        let mut csv = String::new();
        csv.push_str("metric,value\n");
        csv.push_str(&format!("total_requests,{}\n", summary.total_requests));
        csv.push_str(&format!("total_tokens,{}\n", summary.total_tokens));
        csv.push_str(&format!("total_saved,{}\n", summary.total_saved));
        csv.push_str(&format!("compression_ratio,{:.4}\n", summary.compression_ratio));
        csv.push_str(&format!("estimated_cost,{:.4}\n", summary.estimated_cost));

        csv
    }
}

/// Format a number with thousands separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let len = s.len();

    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    result
}

/// Truncate string to max length
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:<width$}", s, width = max)
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short     ");
        assert_eq!(truncate("very long string", 10), "very lo...");
    }

    #[test]
    fn test_render_bar() {
        let bar = TokenDashboard::render_bar(0.5, 10);
        assert!(bar.contains("█████"));
        assert!(bar.contains("░░░░░"));
        assert!(bar.contains("50.0%"));
    }

    #[tokio::test]
    async fn test_dashboard_render() {
        let metrics = Arc::new(RwLock::new(TokenMetrics::new()));
        let dashboard = TokenDashboard::new(metrics, Duration::from_secs(1));

        let output = dashboard.render().await;
        assert!(output.contains("Token Usage Dashboard"));
        assert!(output.contains("Total Requests"));
    }

    #[tokio::test]
    async fn test_export_json() {
        let metrics = Arc::new(RwLock::new(TokenMetrics::new()));
        let dashboard = TokenDashboard::new(metrics, Duration::from_secs(1));

        let json = dashboard.export_json().await;
        assert!(json.get("total_requests").is_some());
        assert!(json.get("compression_ratio").is_some());
    }

    #[tokio::test]
    async fn test_export_csv() {
        let metrics = Arc::new(RwLock::new(TokenMetrics::new()));
        let dashboard = TokenDashboard::new(metrics, Duration::from_secs(1));

        let csv = dashboard.export_csv().await;
        assert!(csv.contains("metric,value"));
        assert!(csv.contains("total_requests"));
    }
}
