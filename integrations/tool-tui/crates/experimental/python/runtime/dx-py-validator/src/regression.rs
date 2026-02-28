//! Regression detection for compatibility tracking

use crate::{CompatibilitySnapshot, FrameworkTestResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of change detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Pass rate improved
    Improvement,
    /// Pass rate decreased
    Regression,
    /// Framework was added
    Added,
    /// Framework was removed
    Removed,
    /// No significant change
    NoChange,
}

/// A single change between snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Framework name
    pub framework: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Previous pass rate (if applicable)
    pub previous_rate: Option<f64>,
    /// Current pass rate (if applicable)
    pub current_rate: Option<f64>,
    /// Change in pass rate (positive = improvement)
    pub delta: f64,
    /// Human-readable description
    pub description: String,
}

impl Change {
    /// Create a new change
    fn new(
        framework: impl Into<String>,
        change_type: ChangeType,
        previous_rate: Option<f64>,
        current_rate: Option<f64>,
    ) -> Self {
        let framework = framework.into();
        let delta = match (previous_rate, current_rate) {
            (Some(prev), Some(curr)) => curr - prev,
            _ => 0.0,
        };

        let description = match change_type {
            ChangeType::Improvement => format!(
                "{}: improved from {:.1}% to {:.1}% (+{:.1}%)",
                framework,
                previous_rate.unwrap_or(0.0) * 100.0,
                current_rate.unwrap_or(0.0) * 100.0,
                delta * 100.0
            ),
            ChangeType::Regression => format!(
                "{}: regressed from {:.1}% to {:.1}% ({:.1}%)",
                framework,
                previous_rate.unwrap_or(0.0) * 100.0,
                current_rate.unwrap_or(0.0) * 100.0,
                delta * 100.0
            ),
            ChangeType::Added => format!(
                "{}: newly added with {:.1}% pass rate",
                framework,
                current_rate.unwrap_or(0.0) * 100.0
            ),
            ChangeType::Removed => format!(
                "{}: removed (was {:.1}% pass rate)",
                framework,
                previous_rate.unwrap_or(0.0) * 100.0
            ),
            ChangeType::NoChange => {
                format!("{}: unchanged at {:.1}%", framework, current_rate.unwrap_or(0.0) * 100.0)
            }
        };

        Self {
            framework,
            change_type,
            previous_rate,
            current_rate,
            delta,
            description,
        }
    }
}

/// Report of regressions and improvements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    /// Previous snapshot version
    pub previous_version: String,
    /// Current snapshot version
    pub current_version: String,
    /// All changes detected
    pub changes: Vec<Change>,
    /// Overall pass rate change
    pub overall_delta: f64,
    /// Number of regressions
    pub regression_count: usize,
    /// Number of improvements
    pub improvement_count: usize,
}

impl RegressionReport {
    /// Check if there are any regressions
    pub fn has_regressions(&self) -> bool {
        self.regression_count > 0
    }

    /// Check if there are any improvements
    pub fn has_improvements(&self) -> bool {
        self.improvement_count > 0
    }

    /// Get only regressions
    pub fn regressions(&self) -> Vec<&Change> {
        self.changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Regression)
            .collect()
    }

    /// Get only improvements
    pub fn improvements(&self) -> Vec<&Change> {
        self.changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Improvement)
            .collect()
    }

    /// Generate markdown report
    pub fn generate_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Regression Report\n\n");
        md.push_str(&format!("**Previous Version:** {}\n", self.previous_version));
        md.push_str(&format!("**Current Version:** {}\n\n", self.current_version));

        // Overall summary
        let overall_status = if self.overall_delta > 0.0 {
            "📈 Improved"
        } else if self.overall_delta < 0.0 {
            "📉 Regressed"
        } else {
            "➡️ Unchanged"
        };

        md.push_str(&format!(
            "**Overall Status:** {} ({:+.1}%)\n\n",
            overall_status,
            self.overall_delta * 100.0
        ));

        // Regressions
        if self.has_regressions() {
            md.push_str("## ❌ Regressions\n\n");
            for change in self.regressions() {
                md.push_str(&format!("- {}\n", change.description));
            }
            md.push('\n');
        }

        // Improvements
        if self.has_improvements() {
            md.push_str("## ✅ Improvements\n\n");
            for change in self.improvements() {
                md.push_str(&format!("- {}\n", change.description));
            }
            md.push('\n');
        }

        // All changes table
        md.push_str("## All Changes\n\n");
        md.push_str("| Framework | Previous | Current | Delta | Status |\n");
        md.push_str("|-----------|----------|---------|-------|--------|\n");

        for change in &self.changes {
            let status = match change.change_type {
                ChangeType::Improvement => "✅",
                ChangeType::Regression => "❌",
                ChangeType::Added => "🆕",
                ChangeType::Removed => "🗑️",
                ChangeType::NoChange => "➡️",
            };

            md.push_str(&format!(
                "| {} | {:.1}% | {:.1}% | {:+.1}% | {} |\n",
                change.framework,
                change.previous_rate.unwrap_or(0.0) * 100.0,
                change.current_rate.unwrap_or(0.0) * 100.0,
                change.delta * 100.0,
                status
            ));
        }

        md
    }
}

/// Detector for regressions between snapshots
pub struct RegressionDetector {
    /// Threshold for considering a change significant (default 1%)
    threshold: f64,
}

impl RegressionDetector {
    /// Create a new regression detector
    pub fn new() -> Self {
        Self { threshold: 0.01 }
    }

    /// Set the significance threshold
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Compare two snapshots and detect regressions
    pub fn compare(
        &self,
        previous: &CompatibilitySnapshot,
        current: &CompatibilitySnapshot,
    ) -> RegressionReport {
        let mut changes = Vec::new();

        // Build maps for easy lookup
        let prev_map: HashMap<&str, &FrameworkTestResult> =
            previous.results.iter().map(|r| (r.framework.name.as_str(), r)).collect();

        let curr_map: HashMap<&str, &FrameworkTestResult> =
            current.results.iter().map(|r| (r.framework.name.as_str(), r)).collect();

        // Check all frameworks in current snapshot
        for (name, curr_result) in &curr_map {
            let curr_rate = curr_result.pass_rate();

            if let Some(prev_result) = prev_map.get(name) {
                let prev_rate = prev_result.pass_rate();
                let delta = curr_rate - prev_rate;

                let change_type = if delta > self.threshold {
                    ChangeType::Improvement
                } else if delta < -self.threshold {
                    ChangeType::Regression
                } else {
                    ChangeType::NoChange
                };

                changes.push(Change::new(*name, change_type, Some(prev_rate), Some(curr_rate)));
            } else {
                // Framework was added
                changes.push(Change::new(*name, ChangeType::Added, None, Some(curr_rate)));
            }
        }

        // Check for removed frameworks
        for (name, prev_result) in &prev_map {
            if !curr_map.contains_key(name) {
                changes.push(Change::new(
                    *name,
                    ChangeType::Removed,
                    Some(prev_result.pass_rate()),
                    None,
                ));
            }
        }

        // Sort by delta (regressions first)
        changes.sort_by(|a, b| a.delta.partial_cmp(&b.delta).unwrap_or(std::cmp::Ordering::Equal));

        let regression_count =
            changes.iter().filter(|c| c.change_type == ChangeType::Regression).count();

        let improvement_count =
            changes.iter().filter(|c| c.change_type == ChangeType::Improvement).count();

        let overall_delta = current.overall_pass_rate() - previous.overall_pass_rate();

        RegressionReport {
            previous_version: previous.dx_py_version.clone(),
            current_version: current.dx_py_version.clone(),
            changes,
            overall_delta,
            regression_count,
            improvement_count,
        }
    }
}

impl Default for RegressionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrameworkInfo;
    use std::time::Duration;

    fn create_result(name: &str, passed: usize, failed: usize) -> FrameworkTestResult {
        FrameworkTestResult {
            framework: FrameworkInfo::new(name, "1.0"),
            total_tests: passed + failed,
            passed,
            failed,
            skipped: 0,
            errors: 0,
            failure_categories: HashMap::new(),
            duration: Duration::from_secs(1),
            timestamp: chrono::Utc::now(),
            raw_output: None,
        }
    }

    #[test]
    fn test_detect_regression() {
        let previous = CompatibilitySnapshot::new("0.1.0", vec![create_result("Django", 90, 10)]);

        let current = CompatibilitySnapshot::new("0.2.0", vec![create_result("Django", 80, 20)]);

        let detector = RegressionDetector::new();
        let report = detector.compare(&previous, &current);

        assert!(report.has_regressions());
        assert_eq!(report.regression_count, 1);
        assert!(report.overall_delta < 0.0);
    }

    #[test]
    fn test_detect_improvement() {
        let previous = CompatibilitySnapshot::new("0.1.0", vec![create_result("Django", 80, 20)]);

        let current = CompatibilitySnapshot::new("0.2.0", vec![create_result("Django", 95, 5)]);

        let detector = RegressionDetector::new();
        let report = detector.compare(&previous, &current);

        assert!(report.has_improvements());
        assert_eq!(report.improvement_count, 1);
        assert!(report.overall_delta > 0.0);
    }

    #[test]
    fn test_detect_added_framework() {
        let previous = CompatibilitySnapshot::new("0.1.0", vec![create_result("Django", 90, 10)]);

        let current = CompatibilitySnapshot::new(
            "0.2.0",
            vec![
                create_result("Django", 90, 10),
                create_result("Flask", 95, 5),
            ],
        );

        let detector = RegressionDetector::new();
        let report = detector.compare(&previous, &current);

        let added: Vec<_> =
            report.changes.iter().filter(|c| c.change_type == ChangeType::Added).collect();

        assert_eq!(added.len(), 1);
        assert_eq!(added[0].framework, "Flask");
    }

    #[test]
    fn test_detect_removed_framework() {
        let previous = CompatibilitySnapshot::new(
            "0.1.0",
            vec![
                create_result("Django", 90, 10),
                create_result("Flask", 95, 5),
            ],
        );

        let current = CompatibilitySnapshot::new("0.2.0", vec![create_result("Django", 90, 10)]);

        let detector = RegressionDetector::new();
        let report = detector.compare(&previous, &current);

        let removed: Vec<_> =
            report.changes.iter().filter(|c| c.change_type == ChangeType::Removed).collect();

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].framework, "Flask");
    }

    #[test]
    fn test_no_change_within_threshold() {
        let previous = CompatibilitySnapshot::new("0.1.0", vec![create_result("Django", 90, 10)]);

        let current = CompatibilitySnapshot::new(
            "0.2.0",
            vec![
                create_result("Django", 91, 9), // Only 1% change
            ],
        );

        let detector = RegressionDetector::new().with_threshold(0.02); // 2% threshold
        let report = detector.compare(&previous, &current);

        assert!(!report.has_regressions());
        assert!(!report.has_improvements());
    }

    #[test]
    fn test_generate_markdown() {
        let previous = CompatibilitySnapshot::new("0.1.0", vec![create_result("Django", 80, 20)]);

        let current = CompatibilitySnapshot::new("0.2.0", vec![create_result("Django", 95, 5)]);

        let detector = RegressionDetector::new();
        let report = detector.compare(&previous, &current);
        let md = report.generate_markdown();

        assert!(md.contains("Regression Report"));
        assert!(md.contains("Django"));
        assert!(md.contains("Improvements"));
    }
}
