//! Report generator implementation

use crate::analysis::StatisticalAnalyzer;
use crate::core::ValidationStatus;
use crate::data::{BenchmarkResults, StoredResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Comparison report containing all benchmark comparisons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub suite: String,
    pub comparisons: Vec<BenchmarkComparison>,
    pub methodology: String,
    /// Feature coverage statistics
    pub feature_coverage: FeatureCoverage,
}

/// Single benchmark comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub name: String,
    pub baseline_mean_ms: f64,
    pub subject_mean_ms: f64,
    pub speedup: f64,
    pub speedup_ci: (f64, f64),
    pub is_significant: bool,
    pub p_value: f64,
    pub is_slower: bool,
    /// Validation status of the subject benchmark
    pub validation_status: String,
    /// Whether this comparison is valid (both executed successfully with matching output)
    pub is_valid: bool,
    /// Error message if benchmark failed
    pub error_message: Option<String>,
}

/// Feature coverage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureCoverage {
    pub total_benchmarks: usize,
    pub successful_benchmarks: usize,
    pub validated_benchmarks: usize,
    pub not_supported_benchmarks: usize,
    pub output_mismatch_benchmarks: usize,
    pub execution_failed_benchmarks: usize,
    pub coverage_percentage: f64,
}

/// Generator for benchmark reports in various formats
pub struct ReportGenerator {
    pub output_dir: PathBuf,
}

impl ReportGenerator {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    /// Generate a Markdown report from benchmark results and comparison
    pub fn generate_markdown(
        &self,
        results: &BenchmarkResults,
        comparison: &ComparisonReport,
    ) -> String {
        let mut md = String::new();

        // Header
        md.push_str(&format!("# Benchmark Report: {}\n\n", results.suite));
        md.push_str(&format!(
            "Generated: {}\n\n",
            results.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Feature Coverage Summary
        md.push_str("## Feature Coverage\n\n");
        md.push_str(&format!(
            "**Coverage: {:.1}%** ({} of {} benchmarks successful)\n\n",
            comparison.feature_coverage.coverage_percentage,
            comparison.feature_coverage.successful_benchmarks,
            comparison.feature_coverage.total_benchmarks
        ));
        md.push_str("| Status | Count |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!(
            "| ✅ Validated | {} |\n",
            comparison.feature_coverage.validated_benchmarks
        ));
        md.push_str(&format!(
            "| ❌ Not Supported | {} |\n",
            comparison.feature_coverage.not_supported_benchmarks
        ));
        md.push_str(&format!(
            "| ⚠️ Output Mismatch | {} |\n",
            comparison.feature_coverage.output_mismatch_benchmarks
        ));
        md.push_str(&format!(
            "| ❌ Execution Failed | {} |\n\n",
            comparison.feature_coverage.execution_failed_benchmarks
        ));

        // System Info
        md.push_str("## System Information\n\n");
        md.push_str(&format!(
            "- **OS**: {} {}\n",
            results.system_info.os, results.system_info.os_version
        ));
        md.push_str(&format!(
            "- **CPU**: {} ({} cores)\n",
            results.system_info.cpu_model, results.system_info.cpu_cores
        ));
        md.push_str(&format!("- **Memory**: {:.1} GB\n", results.system_info.memory_gb));
        md.push_str(&format!("- **Python**: {}\n", results.system_info.python_version));
        md.push_str(&format!("- **DX-Py**: {}\n", results.system_info.dxpy_version));
        if let Some(ref uv) = results.system_info.uv_version {
            md.push_str(&format!("- **UV**: {}\n", uv));
        }
        if let Some(ref pytest) = results.system_info.pytest_version {
            md.push_str(&format!("- **pytest**: {}\n", pytest));
        }
        md.push('\n');

        // Configuration
        md.push_str("## Configuration\n\n");
        md.push_str(&format!("- **Warmup Iterations**: {}\n", results.config.warmup_iterations));
        md.push_str(&format!(
            "- **Measurement Iterations**: {}\n",
            results.config.measurement_iterations
        ));
        md.push_str(&format!("- **Timeout**: {} seconds\n\n", results.config.timeout_seconds));

        // Successful Benchmarks Table (only valid comparisons)
        let valid_comparisons: Vec<_> = comparison.comparisons.iter().filter(|c| c.is_valid).collect();
        
        if !valid_comparisons.is_empty() {
            md.push_str("## Performance Results (Validated Benchmarks Only)\n\n");
            md.push_str("| Benchmark | Baseline (ms) | Subject (ms) | Speedup | 95% CI | Significant | Status |\n");
            md.push_str("|-----------|---------------|--------------|---------|--------|-------------|--------|\n");

            for comp in &valid_comparisons {
                let status = if comp.is_slower {
                    "⚠️ Slower"
                } else if comp.speedup > 1.0 {
                    "✅ Faster"
                } else {
                    "➖ Same"
                };

                let sig = if comp.is_significant { "Yes" } else { "No" };

                md.push_str(&format!(
                    "| {} | {:.3} | {:.3} | {:.2}x | ({:.2}x, {:.2}x) | {} | {} |\n",
                    comp.name,
                    comp.baseline_mean_ms,
                    comp.subject_mean_ms,
                    comp.speedup,
                    comp.speedup_ci.0,
                    comp.speedup_ci.1,
                    sig,
                    status
                ));
            }
            md.push('\n');
        }

        // Failed/Unsupported Benchmarks Table
        let failed_comparisons: Vec<_> = comparison.comparisons.iter().filter(|c| !c.is_valid).collect();
        
        if !failed_comparisons.is_empty() {
            md.push_str("## Unsupported/Failed Benchmarks\n\n");
            md.push_str("| Benchmark | Status | Reason |\n");
            md.push_str("|-----------|--------|--------|\n");

            for comp in &failed_comparisons {
                let reason = comp.error_message.as_deref().unwrap_or("Unknown");
                // Truncate reason for table display
                let truncated_reason: String = reason.chars().take(50).collect();
                let truncated_reason = if reason.len() > 50 {
                    format!("{}...", truncated_reason)
                } else {
                    truncated_reason
                };

                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    comp.name,
                    comp.validation_status,
                    truncated_reason
                ));
            }
            md.push('\n');
        }

        // Methodology
        md.push_str("## Methodology\n\n");
        md.push_str(&comparison.methodology);
        md.push('\n');

        md
    }

    /// Generate a JSON report from benchmark results
    pub fn generate_json(&self, results: &BenchmarkResults) -> String {
        serde_json::to_string_pretty(results).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate a JSON report with comparison data
    pub fn generate_json_with_comparison(
        &self,
        results: &BenchmarkResults,
        comparison: &ComparisonReport,
    ) -> String {
        #[derive(Serialize)]
        struct FullReport<'a> {
            results: &'a BenchmarkResults,
            comparison: &'a ComparisonReport,
        }

        let report = FullReport {
            results,
            comparison,
        };
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate historical comparison report
    pub fn generate_historical_comparison(
        &self,
        current: &BenchmarkResults,
        previous: &[StoredResult],
    ) -> String {
        let mut md = String::new();

        md.push_str("## Historical Comparison\n\n");

        if previous.is_empty() {
            md.push_str("No previous results available for comparison.\n\n");
            return md;
        }

        md.push_str("### Performance Trends\n\n");
        md.push_str("| Benchmark | Current (ms) | Previous (ms) | Change | Trend |\n");
        md.push_str("|-----------|--------------|---------------|--------|-------|\n");

        let analyzer = StatisticalAnalyzer::new();

        for current_bench in &current.benchmarks {
            // Find matching benchmark in most recent previous result
            if let Some(prev_result) = previous.first() {
                if let Some(prev_bench) =
                    prev_result.results.benchmarks.iter().find(|b| b.name == current_bench.name)
                {
                    let current_stats = analyzer.compute_statistics(&current_bench.timings);
                    let prev_stats = analyzer.compute_statistics(&prev_bench.timings);

                    let current_ms = current_stats.mean * 1000.0;
                    let prev_ms = prev_stats.mean * 1000.0;

                    let change = if prev_ms > 0.0 {
                        ((current_ms - prev_ms) / prev_ms) * 100.0
                    } else {
                        0.0
                    };

                    let trend = if change < -5.0 {
                        "📈 Improved"
                    } else if change > 5.0 {
                        "📉 Regressed"
                    } else {
                        "➖ Stable"
                    };

                    md.push_str(&format!(
                        "| {} | {:.3} | {:.3} | {:+.1}% | {} |\n",
                        current_bench.name, current_ms, prev_ms, change, trend
                    ));
                }
            }
        }

        md.push('\n');

        // Historical runs summary
        md.push_str("### Previous Runs\n\n");
        for (i, prev) in previous.iter().take(5).enumerate() {
            md.push_str(&format!(
                "{}. {} - {} benchmarks\n",
                i + 1,
                prev.timestamp.format("%Y-%m-%d %H:%M:%S"),
                prev.results.benchmarks.len()
            ));
        }
        md.push('\n');

        md
    }

    /// Create a comparison report from two sets of results
    pub fn create_comparison(
        &self,
        baseline_results: &BenchmarkResults,
        subject_results: &BenchmarkResults,
    ) -> ComparisonReport {
        let analyzer = StatisticalAnalyzer::new();
        let mut comparisons = Vec::new();

        // Track feature coverage
        let mut total_benchmarks = 0;
        let mut successful_benchmarks = 0;
        let mut validated_benchmarks = 0;
        let mut not_supported_benchmarks = 0;
        let mut output_mismatch_benchmarks = 0;
        let mut execution_failed_benchmarks = 0;

        for baseline_bench in &baseline_results.benchmarks {
            total_benchmarks += 1;

            if let Some(subject_bench) =
                subject_results.benchmarks.iter().find(|b| b.name == baseline_bench.name)
            {
                // Check validation status
                let validation_status = subject_bench.validation_status;
                let is_valid = subject_bench.is_valid_for_comparison();

                match validation_status {
                    ValidationStatus::Validated => validated_benchmarks += 1,
                    ValidationStatus::NotSupported => not_supported_benchmarks += 1,
                    ValidationStatus::OutputMismatch => output_mismatch_benchmarks += 1,
                    ValidationStatus::ExecutionFailed => execution_failed_benchmarks += 1,
                    ValidationStatus::NoValidation => {}
                }

                if is_valid {
                    successful_benchmarks += 1;
                }

                // Only compute timing comparison for valid benchmarks
                let (baseline_mean_ms, subject_mean_ms, speedup, speedup_ci, is_significant, p_value, is_slower) = 
                    if is_valid && !baseline_bench.timings.is_empty() && !subject_bench.timings.is_empty() {
                        let baseline_values: Vec<f64> =
                            baseline_bench.timings.iter().map(|d| d.as_secs_f64()).collect();
                        let subject_values: Vec<f64> =
                            subject_bench.timings.iter().map(|d| d.as_secs_f64()).collect();

                        let comparison = analyzer.compare(&baseline_values, &subject_values);
                        (
                            comparison.baseline_stats.mean * 1000.0,
                            comparison.subject_stats.mean * 1000.0,
                            comparison.speedup,
                            comparison.speedup_ci,
                            comparison.is_significant,
                            comparison.p_value,
                            comparison.speedup < 1.0,
                        )
                    } else {
                        (0.0, 0.0, 0.0, (0.0, 0.0), false, 1.0, false)
                    };

                let validation_status_str = match validation_status {
                    ValidationStatus::Validated => "✅ Validated".to_string(),
                    ValidationStatus::NotSupported => "❌ Not Supported".to_string(),
                    ValidationStatus::OutputMismatch => "⚠️ Output Mismatch".to_string(),
                    ValidationStatus::ExecutionFailed => "❌ Execution Failed".to_string(),
                    ValidationStatus::NoValidation => "➖ No Validation".to_string(),
                };

                comparisons.push(BenchmarkComparison {
                    name: baseline_bench.name.clone(),
                    baseline_mean_ms,
                    subject_mean_ms,
                    speedup,
                    speedup_ci,
                    is_significant,
                    p_value,
                    is_slower,
                    validation_status: validation_status_str,
                    is_valid,
                    error_message: subject_bench.error_message.clone(),
                });
            } else {
                // Subject benchmark not found - mark as not supported
                not_supported_benchmarks += 1;
                comparisons.push(BenchmarkComparison {
                    name: baseline_bench.name.clone(),
                    baseline_mean_ms: 0.0,
                    subject_mean_ms: 0.0,
                    speedup: 0.0,
                    speedup_ci: (0.0, 0.0),
                    is_significant: false,
                    p_value: 1.0,
                    is_slower: false,
                    validation_status: "❌ Not Supported".to_string(),
                    is_valid: false,
                    error_message: Some("Benchmark not found in subject results".to_string()),
                });
            }
        }

        let coverage_percentage = if total_benchmarks > 0 {
            (successful_benchmarks as f64 / total_benchmarks as f64) * 100.0
        } else {
            0.0
        };

        let feature_coverage = FeatureCoverage {
            total_benchmarks,
            successful_benchmarks,
            validated_benchmarks,
            not_supported_benchmarks,
            output_mismatch_benchmarks,
            execution_failed_benchmarks,
            coverage_percentage,
        };

        ComparisonReport {
            suite: baseline_results.suite.clone(),
            comparisons,
            methodology: self.default_methodology(),
            feature_coverage,
        }
    }

    /// Default methodology text
    fn default_methodology(&self) -> String {
        r#"### Benchmark Methodology

1. **Warmup Phase**: Each benchmark runs warmup iterations to allow JIT compilation and cache warming. Warmup timings are discarded.

2. **Measurement Phase**: After warmup, measurement iterations are executed and timed. All timings are recorded for statistical analysis.

3. **Statistical Analysis**:
   - Mean, median, and standard deviation are computed for all measurements
   - 95% confidence intervals are calculated using t-distribution
   - Welch's t-test is used for significance testing (p < 0.05)
   - Outliers are detected using the IQR method

4. **Speedup Calculation**: Speedup = Baseline Mean / Subject Mean
   - Speedup > 1.0 indicates the subject is faster
   - Speedup < 1.0 indicates the subject is slower

5. **Reproducibility**: All benchmarks use seeded random number generation where applicable. System information and configuration are recorded with each run.
"#.to_string()
    }

    /// Check if a report contains all required elements
    pub fn validate_markdown_report(&self, markdown: &str) -> ReportValidation {
        let mut validation = ReportValidation {
            has_comparison_table: false,
            has_speedup_factors: false,
            has_confidence_intervals: false,
            has_slowdown_indication: false,
            has_methodology: false,
        };

        // Check for comparison table
        validation.has_comparison_table =
            markdown.contains("| Benchmark |") && markdown.contains("| Baseline");

        // Check for speedup factors
        validation.has_speedup_factors = markdown.contains("Speedup");

        // Check for confidence intervals
        validation.has_confidence_intervals =
            markdown.contains("95% CI") || markdown.contains("CI");

        // Check for slowdown indication
        validation.has_slowdown_indication = markdown.contains("Slower") || markdown.contains("⚠️");

        // Check for methodology section
        validation.has_methodology =
            markdown.contains("## Methodology") || markdown.contains("### Methodology");

        validation
    }
}

/// Validation result for report content
#[derive(Debug, Clone)]
pub struct ReportValidation {
    pub has_comparison_table: bool,
    pub has_speedup_factors: bool,
    pub has_confidence_intervals: bool,
    pub has_slowdown_indication: bool,
    pub has_methodology: bool,
}

impl ReportValidation {
    /// Check if all required elements are present
    pub fn is_complete(&self) -> bool {
        self.has_comparison_table
            && self.has_speedup_factors
            && self.has_confidence_intervals
            && self.has_methodology
    }
}
