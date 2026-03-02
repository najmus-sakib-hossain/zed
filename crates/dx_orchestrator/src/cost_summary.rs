//! Cost summary — aggregated cost tracking across all providers used.

use crate::GeneratedOutput;
use dx_core::cost::MicroCost;
use std::collections::HashMap;

/// Summary of costs across all providers used in a generation request.
#[derive(Debug, Clone)]
pub struct CostSummary {
    /// Total cost.
    pub total: MicroCost,
    /// Cost breakdown by provider.
    pub by_provider: HashMap<String, MicroCost>,
    /// Cost breakdown by task type.
    pub by_task_type: HashMap<String, MicroCost>,
    /// Number of tasks executed.
    pub task_count: usize,
    /// Number of tasks that used local (free) providers.
    pub free_task_count: usize,
}

impl CostSummary {
    /// Build a cost summary from generated outputs.
    pub fn from_outputs(outputs: &[GeneratedOutput]) -> Self {
        let mut by_provider: HashMap<String, MicroCost> = HashMap::new();
        let mut by_task_type: HashMap<String, MicroCost> = HashMap::new();
        let mut total = MicroCost::ZERO;
        let mut free_count = 0;

        for output in outputs {
            total = MicroCost(total.0 + output.cost.0);

            *by_provider
                .entry(output.provider_id.clone())
                .or_insert(MicroCost::ZERO) = MicroCost(
                by_provider
                    .get(&output.provider_id)
                    .unwrap_or(&MicroCost::ZERO)
                    .0
                    + output.cost.0,
            );

            *by_task_type
                .entry(output.task_type.clone())
                .or_insert(MicroCost::ZERO) = MicroCost(
                by_task_type
                    .get(&output.task_type)
                    .unwrap_or(&MicroCost::ZERO)
                    .0
                    + output.cost.0,
            );

            if output.cost == MicroCost::ZERO {
                free_count += 1;
            }
        }

        Self {
            total,
            by_provider,
            by_task_type,
            task_count: outputs.len(),
            free_task_count: free_count,
        }
    }

    /// Format a human-readable cost report.
    pub fn format_report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!(
            "Total cost: ${:.4} ({} tasks, {} free)\n",
            self.total.as_usd(),
            self.task_count,
            self.free_task_count
        ));

        if !self.by_provider.is_empty() {
            report.push_str("\nBy provider:\n");
            let mut providers: Vec<_> = self.by_provider.iter().collect();
            providers.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
            for (provider, cost) in providers {
                report.push_str(&format!("  {} — ${:.4}\n", provider, cost.as_usd()));
            }
        }

        if !self.by_task_type.is_empty() {
            report.push_str("\nBy task type:\n");
            let mut types: Vec<_> = self.by_task_type.iter().collect();
            types.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
            for (task_type, cost) in types {
                report.push_str(&format!("  {} — ${:.4}\n", task_type, cost.as_usd()));
            }
        }

        report
    }
}
