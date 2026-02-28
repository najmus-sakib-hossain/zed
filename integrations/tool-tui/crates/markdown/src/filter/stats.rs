// Statistics tracking for content filtering

use std::collections::HashMap;

/// Statistics about what was filtered
#[derive(Debug, Default)]
pub struct FilterStats {
    pub original_tokens: usize,
    pub final_tokens: usize,
    pub removed_elements: Vec<RemovedElement>,
    pub by_category: HashMap<String, CategoryStats>,
}

#[derive(Debug)]
pub struct RemovedElement {
    pub category: String,
    pub tokens: usize,
    pub preview: String,
}

#[derive(Debug, Default)]
pub struct CategoryStats {
    pub count: usize,
    pub tokens: usize,
}

impl FilterStats {
    pub fn savings_percent(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        ((self.original_tokens - self.final_tokens) as f64 / self.original_tokens as f64) * 100.0
    }

    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!("Original: {} tokens\n", self.original_tokens));
        report.push_str(&format!("Final: {} tokens\n", self.final_tokens));
        report.push_str(&format!(
            "Saved: {} tokens ({:.1}%)\n\n",
            self.original_tokens - self.final_tokens,
            self.savings_percent()
        ));

        report.push_str("Removed by category:\n");
        for (category, stats) in &self.by_category {
            report.push_str(&format!(
                "  {}: {} elements ({} tokens)\n",
                category, stats.count, stats.tokens
            ));
        }

        report
    }
}
