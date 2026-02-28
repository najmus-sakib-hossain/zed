//! Compatibility matrix generation for framework validation results

use crate::{CompatibilitySnapshot, FailureCategory, FrameworkTestResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single entry in the compatibility matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixEntry {
    /// Framework name
    pub framework: String,
    /// Framework version
    pub version: String,
    /// Total tests
    pub total: usize,
    /// Passed tests
    pub passed: usize,
    /// Failed tests
    pub failed: usize,
    /// Skipped tests
    pub skipped: usize,
    /// Error tests
    pub errors: usize,
    /// Pass rate (0.0 to 1.0)
    pub pass_rate: f64,
    /// Whether it meets minimum requirements
    pub meets_minimum: bool,
    /// Failure breakdown by category
    pub failure_breakdown: HashMap<String, usize>,
}

impl From<&FrameworkTestResult> for MatrixEntry {
    fn from(result: &FrameworkTestResult) -> Self {
        let failure_breakdown: HashMap<String, usize> = result
            .failure_categories
            .iter()
            .map(|(cat, failures)| (cat.description().to_string(), failures.len()))
            .collect();

        Self {
            framework: result.framework.name.clone(),
            version: result.framework.version.clone(),
            total: result.total_tests,
            passed: result.passed,
            failed: result.failed,
            skipped: result.skipped,
            errors: result.errors,
            pass_rate: result.pass_rate(),
            meets_minimum: result.meets_minimum(),
            failure_breakdown,
        }
    }
}

/// Result for a single framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkResult {
    /// The matrix entry
    pub entry: MatrixEntry,
    /// Status indicator (✅, ⚠️, ❌)
    pub status: String,
    /// Summary message
    pub summary: String,
}

impl FrameworkResult {
    /// Create from a test result
    pub fn from_result(result: &FrameworkTestResult) -> Self {
        let entry = MatrixEntry::from(result);

        let (status, summary) = if entry.meets_minimum {
            (
                "✅".to_string(),
                format!("{:.1}% pass rate - meets requirements", entry.pass_rate * 100.0),
            )
        } else if entry.pass_rate >= 0.5 {
            (
                "⚠️".to_string(),
                format!("{:.1}% pass rate - partial compatibility", entry.pass_rate * 100.0),
            )
        } else {
            (
                "❌".to_string(),
                format!("{:.1}% pass rate - not compatible", entry.pass_rate * 100.0),
            )
        };

        Self {
            entry,
            status,
            summary,
        }
    }
}

/// Compatibility matrix generator
#[derive(Debug, Clone, Default)]
pub struct CompatibilityMatrix {
    /// All framework results
    results: Vec<FrameworkTestResult>,
    /// DX-Py version
    dx_py_version: String,
}

impl CompatibilityMatrix {
    /// Create a new compatibility matrix
    pub fn new(dx_py_version: impl Into<String>) -> Self {
        Self {
            results: Vec::new(),
            dx_py_version: dx_py_version.into(),
        }
    }

    /// Add a framework result
    pub fn add_result(&mut self, result: FrameworkTestResult) {
        self.results.push(result);
    }

    /// Add multiple results
    pub fn add_results(&mut self, results: impl IntoIterator<Item = FrameworkTestResult>) {
        self.results.extend(results);
    }

    /// Get all framework results
    pub fn get_results(&self) -> &[FrameworkTestResult] {
        &self.results
    }

    /// Calculate overall pass rate
    pub fn overall_pass_rate(&self) -> f64 {
        let total: usize = self.results.iter().map(|r| r.total_tests).sum();
        let passed: usize = self.results.iter().map(|r| r.passed).sum();
        if total == 0 {
            return 0.0;
        }
        passed as f64 / total as f64
    }

    /// Calculate overall compatibility score (0-100)
    pub fn overall_score(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }

        // Weight by number of tests
        let total_tests: usize = self.results.iter().map(|r| r.total_tests).sum();
        if total_tests == 0 {
            return 0.0;
        }

        let weighted_sum: f64 =
            self.results.iter().map(|r| r.pass_rate() * r.total_tests as f64).sum();

        (weighted_sum / total_tests as f64) * 100.0
    }

    /// Get failure summary across all frameworks
    pub fn failure_summary(&self) -> HashMap<FailureCategory, usize> {
        let mut summary: HashMap<FailureCategory, usize> = HashMap::new();

        for result in &self.results {
            for (category, failures) in &result.failure_categories {
                *summary.entry(*category).or_insert(0) += failures.len();
            }
        }

        summary
    }

    /// Generate markdown compatibility report
    pub fn generate_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str("# DX-Py Compatibility Matrix\n\n");
        md.push_str(&format!("**DX-Py Version:** {}\n\n", self.dx_py_version));
        md.push_str(&format!("**Overall Score:** {:.1}%\n\n", self.overall_score()));

        // Summary table
        md.push_str("## Framework Compatibility\n\n");
        md.push_str("| Framework | Version | Status | Pass Rate | Passed | Failed | Skipped |\n");
        md.push_str("|-----------|---------|--------|-----------|--------|--------|--------|\n");

        for result in &self.results {
            let fr = FrameworkResult::from_result(result);
            md.push_str(&format!(
                "| {} | {} | {} | {:.1}% | {} | {} | {} |\n",
                fr.entry.framework,
                fr.entry.version,
                fr.status,
                fr.entry.pass_rate * 100.0,
                fr.entry.passed,
                fr.entry.failed + fr.entry.errors,
                fr.entry.skipped
            ));
        }

        md.push('\n');

        // Failure breakdown
        let failure_summary = self.failure_summary();
        if !failure_summary.is_empty() {
            md.push_str("## Failure Breakdown\n\n");
            md.push_str("| Category | Count |\n");
            md.push_str("|----------|-------|\n");

            let mut sorted: Vec<_> = failure_summary.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));

            for (category, count) in sorted {
                md.push_str(&format!("| {} | {} |\n", category.description(), count));
            }

            md.push('\n');
        }

        // Per-framework details
        md.push_str("## Framework Details\n\n");

        for result in &self.results {
            md.push_str(&format!("### {}\n\n", result.framework.name));
            md.push_str(&format!("- **Version:** {}\n", result.framework.version));
            md.push_str(&format!("- **Pass Rate:** {:.1}%\n", result.pass_rate() * 100.0));
            md.push_str(&format!("- **Total Tests:** {}\n", result.total_tests));
            md.push_str(&format!("- **Duration:** {:.2}s\n", result.duration.as_secs_f64()));

            if !result.failure_categories.is_empty() {
                md.push_str("\n**Failure Categories:**\n\n");
                for (category, failures) in &result.failure_categories {
                    md.push_str(&format!(
                        "- {}: {} failures\n",
                        category.description(),
                        failures.len()
                    ));
                }
            }

            md.push('\n');
        }

        // Footer
        md.push_str("---\n");
        md.push_str(&format!(
            "*Generated at {}*\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        md
    }

    /// Generate JSON report for CI integration
    pub fn generate_json(&self) -> serde_json::Value {
        let entries: Vec<MatrixEntry> = self.results.iter().map(MatrixEntry::from).collect();

        let failure_summary: HashMap<String, usize> = self
            .failure_summary()
            .into_iter()
            .map(|(k, v)| (k.description().to_string(), v))
            .collect();

        serde_json::json!({
            "dx_py_version": self.dx_py_version,
            "overall_score": self.overall_score(),
            "overall_pass_rate": self.overall_pass_rate(),
            "frameworks": entries,
            "failure_summary": failure_summary,
            "generated_at": chrono::Utc::now().to_rfc3339()
        })
    }

    /// Create a snapshot of current results
    pub fn create_snapshot(&self) -> CompatibilitySnapshot {
        CompatibilitySnapshot::new(self.dx_py_version.clone(), self.results.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrameworkInfo;
    use std::time::Duration;

    fn create_test_result(name: &str, passed: usize, failed: usize) -> FrameworkTestResult {
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
    fn test_matrix_entry_from_result() {
        let result = create_test_result("Django", 90, 10);
        let entry = MatrixEntry::from(&result);

        assert_eq!(entry.framework, "Django");
        assert_eq!(entry.total, 100);
        assert_eq!(entry.passed, 90);
        assert!((entry.pass_rate - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_overall_score() {
        let mut matrix = CompatibilityMatrix::new("0.1.0");
        matrix.add_result(create_test_result("Django", 90, 10));
        matrix.add_result(create_test_result("Flask", 95, 5));

        // (90 + 95) / (100 + 100) = 0.925 = 92.5%
        assert!((matrix.overall_score() - 92.5).abs() < 0.1);
    }

    #[test]
    fn test_generate_markdown() {
        let mut matrix = CompatibilityMatrix::new("0.1.0");
        matrix.add_result(create_test_result("Django", 90, 10));

        let md = matrix.generate_markdown();

        assert!(md.contains("DX-Py Compatibility Matrix"));
        assert!(md.contains("Django"));
        assert!(md.contains("90.0%"));
    }

    #[test]
    fn test_generate_json() {
        let mut matrix = CompatibilityMatrix::new("0.1.0");
        matrix.add_result(create_test_result("Django", 90, 10));

        let json = matrix.generate_json();

        assert_eq!(json["dx_py_version"], "0.1.0");
        assert!(json["frameworks"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn test_framework_result_status() {
        // Meets minimum (90%+)
        let result = create_test_result("Django", 95, 5);
        let fr = FrameworkResult::from_result(&result);
        assert_eq!(fr.status, "✅");

        // Partial (50-90%)
        let result = create_test_result("Flask", 70, 30);
        let fr = FrameworkResult::from_result(&result);
        assert_eq!(fr.status, "⚠️");

        // Not compatible (<50%)
        let result = create_test_result("NumPy", 30, 70);
        let fr = FrameworkResult::from_result(&result);
        assert_eq!(fr.status, "❌");
    }
}
