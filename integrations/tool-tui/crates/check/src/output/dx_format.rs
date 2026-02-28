//! DX Serializer Output Module
//!
//! Provides DX Serializer format output for diagnostics, scores, and test results.
//! Supports three representations: binary (fast), LLM (token-efficient), human (readable).

use crate::diagnostics::{Diagnostic as InternalDiagnostic, DiagnosticSeverity};
use crate::scoring_impl::{Category, ProjectScore, Severity as ScoreSeverity, Violation};
use serde::{Deserialize, Serialize};

/// DX Serializer output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxOutputFormat {
    /// Binary format - fastest serialization/deserialization
    Binary,
    /// LLM format - 50-70% fewer tokens than JSON
    Llm,
    /// Human format - beautiful, readable output
    Human,
}

impl std::fmt::Display for DxOutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DxOutputFormat::Binary => write!(f, "dx-binary"),
            DxOutputFormat::Llm => write!(f, "dx-llm"),
            DxOutputFormat::Human => write!(f, "dx-human"),
        }
    }
}

impl std::str::FromStr for DxOutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dx-binary" | "binary" | "bin" => Ok(DxOutputFormat::Binary),
            "dx-llm" | "llm" => Ok(DxOutputFormat::Llm),
            "dx-human" | "human" | "readable" => Ok(DxOutputFormat::Human),
            _ => Err(format!("Unknown DX format: {s}")),
        }
    }
}

/// Unified diagnostic for DX Serializer output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxDiagnostic {
    /// File path (relative)
    pub file: String,
    /// Start line (1-indexed)
    pub line: u32,
    /// Start column (1-indexed)
    pub column: u32,
    /// End line (optional)
    pub end_line: Option<u32>,
    /// End column (optional)
    pub end_column: Option<u32>,
    /// Severity: 0=hint, 1=info, 2=warning, 3=error
    pub severity: u8,
    /// Rule identifier (tool/rule-name)
    pub rule: String,
    /// Human-readable message
    pub message: String,
    /// Suggested fix (optional)
    pub fix: Option<String>,
    /// Scoring category: 0=formatting, 1=linting, 2=security, 3=patterns, 4=structure
    pub category: u8,
}

impl From<&InternalDiagnostic> for DxDiagnostic {
    fn from(d: &InternalDiagnostic) -> Self {
        Self {
            file: d.file.to_string_lossy().to_string(),
            line: d.span.start,
            column: 1,
            end_line: Some(d.span.end),
            end_column: None,
            severity: match d.severity {
                DiagnosticSeverity::Hint => 0,
                DiagnosticSeverity::Info => 1,
                DiagnosticSeverity::Warning => 2,
                DiagnosticSeverity::Error => 3,
            },
            rule: d.rule_id.clone(),
            message: d.message.clone(),
            fix: d.suggestion.clone(),
            category: 1, // Default to linting
        }
    }
}

impl From<&Violation> for DxDiagnostic {
    fn from(v: &Violation) -> Self {
        Self {
            file: v.file.to_string_lossy().to_string(),
            line: v.line,
            column: v.column,
            end_line: None,
            end_column: None,
            severity: match v.severity {
                ScoreSeverity::Low => 0,
                ScoreSeverity::Medium => 2,
                ScoreSeverity::High => 3,
                ScoreSeverity::Critical => 3,
            },
            rule: v.rule_id.clone(),
            message: v.message.clone(),
            fix: None,
            category: match v.category {
                Category::Formatting => 0,
                Category::Linting => 1,
                Category::Security => 2,
                Category::DesignPatterns => 3,
                Category::StructureAndDocs => 4,
            },
        }
    }
}

/// Score breakdown by category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxScoreBreakdown {
    /// Formatting score (0-100)
    pub formatting: u16,
    /// Linting score (0-100)
    pub linting: u16,
    /// Security score (0-100)
    pub security: u16,
    /// Design patterns score (0-100)
    pub patterns: u16,
    /// Structure and docs score (0-100)
    pub structure: u16,
}

impl From<&ProjectScore> for DxScoreBreakdown {
    fn from(score: &ProjectScore) -> Self {
        Self {
            formatting: score.get_category_score(Category::Formatting),
            linting: score.get_category_score(Category::Linting),
            security: score.get_category_score(Category::Security),
            patterns: score.get_category_score(Category::DesignPatterns),
            structure: score.get_category_score(Category::StructureAndDocs),
        }
    }
}

/// Test case result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxTestResult {
    /// Test name
    pub name: String,
    /// Test status: 0=passed, 1=failed, 2=skipped, 3=error
    pub status: u8,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Test results summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxTestResults {
    /// Framework name
    pub framework: String,
    /// Total test count
    pub total: u32,
    /// Passed count
    pub passed: u32,
    /// Failed count
    pub failed: u32,
    /// Skipped count
    pub skipped: u32,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Individual test results
    pub tests: Vec<DxTestResult>,
}

/// Coverage data for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxFileCoverage {
    /// File path
    pub file: String,
    /// Lines covered
    pub covered: u32,
    /// Total lines
    pub total: u32,
    /// Coverage percentage
    pub percentage: f64,
}

/// Coverage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxCoverage {
    /// Total lines covered
    pub covered_lines: u32,
    /// Total lines
    pub total_lines: u32,
    /// Overall percentage
    pub percentage: f64,
    /// Per-file coverage
    pub files: Vec<DxFileCoverage>,
}

/// Complete check report for DX Serializer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxCheckReport {
    /// Report version
    pub version: u8,
    /// Unix timestamp (seconds)
    pub timestamp: u64,
    /// Total score (0-500)
    pub score: u16,
    /// Score breakdown by category
    pub breakdown: DxScoreBreakdown,
    /// All diagnostics
    pub diagnostics: Vec<DxDiagnostic>,
    /// Test results (if tests were run)
    pub test_results: Option<DxTestResults>,
    /// Coverage data (if coverage was collected)
    pub coverage: Option<DxCoverage>,
    /// Files analyzed count
    pub files_analyzed: u32,
    /// Total issues count
    pub total_issues: u32,
}

impl DxCheckReport {
    /// Create a new report
    #[must_use]
    pub fn new(score: &ProjectScore) -> Self {
        let diagnostics: Vec<DxDiagnostic> = score
            .categories
            .values()
            .flat_map(|cat| cat.violations.iter().map(DxDiagnostic::from))
            .collect();

        Self {
            version: 1,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            score: score.total_score,
            breakdown: DxScoreBreakdown::from(score),
            total_issues: diagnostics.len() as u32,
            diagnostics,
            test_results: None,
            coverage: None,
            files_analyzed: score.files_analyzed as u32,
        }
    }

    /// Add diagnostics from internal format
    pub fn add_diagnostics(&mut self, diagnostics: &[InternalDiagnostic]) {
        for d in diagnostics {
            self.diagnostics.push(DxDiagnostic::from(d));
        }
        self.total_issues = self.diagnostics.len() as u32;
    }

    /// Set test results
    pub fn set_test_results(&mut self, results: DxTestResults) {
        self.test_results = Some(results);
    }

    /// Set coverage data
    pub fn set_coverage(&mut self, coverage: DxCoverage) {
        self.coverage = Some(coverage);
    }

    /// Convert to DX Serializer binary format
    #[must_use]
    pub fn to_dx_binary(&self) -> Vec<u8> {
        // Use serde_json as compact binary alternative (can switch to bincode v2 later)
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Convert to DX Serializer LLM format (token-efficient)
    #[must_use]
    pub fn to_dx_llm(&self) -> String {
        // Compact token-efficient format
        self.to_compact_format()
    }

    /// Convert to compact LLM format (Latest DX Serializer v4 spec)
    fn to_compact_format(&self) -> String {
        let mut lines = Vec::new();

        // Root scalars - one per line, no spaces around =
        lines.push(format!("version={}", self.version));
        lines.push(format!("timestamp={}", self.timestamp));
        lines.push(format!("files_analyzed={}", self.files_analyzed));
        lines.push(format!("total_score={}", self.score));
        lines.push(format!("total_issues={}", self.total_issues));

        // Score breakdown as inline object with parentheses
        let s = &self.breakdown;
        lines.push(format!(
            "breakdown(formatting={} linting={} security={} patterns={} structure={})",
            s.formatting, s.linting, s.security, s.patterns, s.structure
        ));

        // Diagnostics as wrapped dataframe table
        if !self.diagnostics.is_empty() {
            lines.push(format!("diagnostics[file line col severity rule message]("));
            for d in &self.diagnostics {
                // Use quotes for multi-word strings
                let message = if d.message.contains(' ') {
                    format!("\"{}\"", d.message.replace('"', "'"))
                } else {
                    d.message.clone()
                };
                lines.push(format!(
                    "{} {} {} {} {} {}",
                    d.file, d.line, d.column, d.severity, d.rule, message
                ));
            }
            lines.push(")".to_string());
        }

        // Test results as inline object
        if let Some(ref tests) = self.test_results {
            lines.push(format!(
                "test_results(framework=\"{}\" total={} passed={} failed={} skipped={} duration_ms={})",
                tests.framework, tests.total, tests.passed, tests.failed, tests.skipped, tests.duration_ms
            ));

            // Individual tests as wrapped dataframe
            if !tests.tests.is_empty() {
                lines.push("tests[name status duration_ms error](".to_string());
                for t in &tests.tests {
                    let name = if t.name.contains(' ') {
                        format!("\"{}\"", t.name.replace('"', "'"))
                    } else {
                        t.name.clone()
                    };
                    let error = t
                        .error
                        .as_ref()
                        .map(|e| {
                            if e.contains(' ') {
                                format!("\"{}\"", e.replace('"', "'"))
                            } else {
                                e.clone()
                            }
                        })
                        .unwrap_or_else(|| "null".to_string());
                    lines.push(format!("{} {} {} {}", name, t.status, t.duration_ms, error));
                }
                lines.push(")".to_string());
            }
        }

        // Coverage as inline object
        if let Some(ref cov) = self.coverage {
            lines.push(format!(
                "coverage(covered_lines={} total_lines={} percentage={:.2})",
                cov.covered_lines, cov.total_lines, cov.percentage
            ));

            // Per-file coverage as wrapped dataframe
            if !cov.files.is_empty() {
                lines.push("coverage_files[file covered total percentage](".to_string());
                for f in &cov.files {
                    lines.push(format!("{} {} {} {:.2}", f.file, f.covered, f.total, f.percentage));
                }
                lines.push(")".to_string());
            }
        }

        lines.join("\n")
    }

    /// Convert to DX Serializer human format (readable)
    #[must_use]
    pub fn to_dx_human(&self) -> String {
        format_human_report(self)
    }

    /// Convert to specified format
    #[must_use]
    pub fn to_format(&self, format: DxOutputFormat) -> Vec<u8> {
        match format {
            DxOutputFormat::Binary => self.to_dx_binary(),
            DxOutputFormat::Llm => self.to_dx_llm().into_bytes(),
            DxOutputFormat::Human => self.to_dx_human().into_bytes(),
        }
    }

    /// Convert to JSON (legacy format)
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Convert to JSON compact
    #[must_use]
    pub fn to_json_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Format report as human-readable text
fn format_human_report(report: &DxCheckReport) -> String {
    let mut output = String::new();

    // Header
    output.push_str("╔══════════════════════════════════════════════════════════════════╗\n");
    output.push_str("║                      DX CHECK REPORT                             ║\n");
    output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");

    // Score summary
    let grade = match report.score {
        450..=500 => "A+",
        400..=449 => "A",
        350..=399 => "B+",
        300..=349 => "B",
        250..=299 => "C+",
        200..=249 => "C",
        150..=199 => "D",
        _ => "F",
    };

    output.push_str(&format!(
        "║  Score: {:>3}/500 ({})  │  Files: {:>4}  │  Issues: {:>4}        ║\n",
        report.score, grade, report.files_analyzed, report.total_issues
    ));
    output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");

    // Category breakdown
    output.push_str("║  CATEGORY BREAKDOWN                                              ║\n");
    output.push_str("╟──────────────────────────────────────────────────────────────────╢\n");

    let categories = [
        ("Formatting", report.breakdown.formatting),
        ("Linting", report.breakdown.linting),
        ("Security", report.breakdown.security),
        ("Design Patterns", report.breakdown.patterns),
        ("Structure & Docs", report.breakdown.structure),
    ];

    for (name, score) in &categories {
        let bar_len = (*score as usize) / 5; // 20 chars max
        let bar = "█".repeat(bar_len);
        let empty = "░".repeat(20 - bar_len);
        output.push_str(&format!("║  {name:<16} {score:>3}/100  [{bar}{empty}]             ║\n"));
    }

    // Diagnostics by severity
    if !report.diagnostics.is_empty() {
        output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");
        output.push_str("║  ISSUES                                                          ║\n");
        output.push_str("╟──────────────────────────────────────────────────────────────────╢\n");

        let errors: Vec<_> = report.diagnostics.iter().filter(|d| d.severity == 3).collect();
        let warnings: Vec<_> = report.diagnostics.iter().filter(|d| d.severity == 2).collect();
        let infos: Vec<_> = report.diagnostics.iter().filter(|d| d.severity <= 1).collect();

        if !errors.is_empty() {
            output.push_str(&format!(
                "║  ❌ Errors: {}                                                   ║\n",
                errors.len()
            ));
            for (i, d) in errors.iter().take(5).enumerate() {
                let msg = if d.message.len() > 50 {
                    format!("{}...", &d.message[..47])
                } else {
                    d.message.clone()
                };
                output.push_str(&format!(
                    "║    {}. {}:{}  {}║\n",
                    i + 1,
                    truncate_path(&d.file, 20),
                    d.line,
                    pad_right(&msg, 40)
                ));
            }
            if errors.len() > 5 {
                output.push_str(&format!(
                    "║    ... and {} more errors                                      ║\n",
                    errors.len() - 5
                ));
            }
        }

        if !warnings.is_empty() {
            output.push_str(&format!(
                "║  ⚠️  Warnings: {}                                                 ║\n",
                warnings.len()
            ));
        }

        if !infos.is_empty() {
            output.push_str(&format!(
                "║  ℹ️  Info: {}                                                     ║\n",
                infos.len()
            ));
        }
    }

    // Test results
    if let Some(tests) = &report.test_results {
        output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");
        output.push_str("║  TEST RESULTS                                                    ║\n");
        output.push_str("╟──────────────────────────────────────────────────────────────────╢\n");
        output.push_str(&format!(
            "║  Framework: {}  │  Total: {}  │  Duration: {}ms        ║\n",
            pad_right(&tests.framework, 10),
            tests.total,
            tests.duration_ms
        ));
        output.push_str(&format!(
            "║  ✅ Passed: {}  │  ❌ Failed: {}  │  ⏭️  Skipped: {}           ║\n",
            tests.passed, tests.failed, tests.skipped
        ));
    }

    // Coverage
    if let Some(cov) = &report.coverage {
        output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");
        output.push_str("║  COVERAGE                                                        ║\n");
        output.push_str("╟──────────────────────────────────────────────────────────────────╢\n");
        output.push_str(&format!(
            "║  Lines: {}/{} ({:.1}%)                                      ║\n",
            cov.covered_lines, cov.total_lines, cov.percentage
        ));
    }

    output.push_str("╚══════════════════════════════════════════════════════════════════╝\n");

    output
}

/// Truncate a path for display
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        format!("{path:<max_len$}")
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}

/// Pad string to the right
fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s[..width].to_string()
    } else {
        format!("{s:<width$}")
    }
}

/// Streaming output writer for real-time diagnostics
pub struct DxStreamWriter {
    format: DxOutputFormat,
    written: usize,
}

impl DxStreamWriter {
    /// Create a new stream writer
    #[must_use]
    pub fn new(format: DxOutputFormat) -> Self {
        Self { format, written: 0 }
    }

    /// Write a single diagnostic
    pub fn write_diagnostic(&mut self, diag: &DxDiagnostic) -> String {
        self.written += 1;
        match self.format {
            DxOutputFormat::Binary => {
                // Newline-delimited binary (base64 encoded for text output)
                let bytes = serde_json::to_vec(diag).unwrap_or_default();
                format!("{}\n", base64_encode(&bytes))
            }
            DxOutputFormat::Llm => {
                // Latest DX Serializer LLM format - use quotes for multi-word strings
                let message = if diag.message.contains(' ') {
                    format!("\"{}\"", diag.message.replace('"', "'"))
                } else {
                    diag.message.clone()
                };
                format!(
                    "{} {} {} {} {} {}\n",
                    diag.file, diag.line, diag.column, diag.severity, diag.rule, message
                )
            }
            DxOutputFormat::Human => {
                // Human-readable single line
                let severity_icon = match diag.severity {
                    3 => "❌",
                    2 => "⚠️",
                    1 => "ℹ️",
                    _ => "💡",
                };
                format!(
                    "{} {}:{}  [{}] {}\n",
                    severity_icon, diag.file, diag.line, diag.rule, diag.message
                )
            }
        }
    }

    /// Get count of written items
    #[must_use]
    pub fn count(&self) -> usize {
        self.written
    }
}

/// Simple base64 encoding for binary streaming
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);

        let n = (u32::from(buf[0]) << 16) | (u32::from(buf[1]) << 8) | u32::from(buf[2]);

        result.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        result.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[(n & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_dx_output_format_parsing() {
        assert_eq!("dx-binary".parse::<DxOutputFormat>().unwrap(), DxOutputFormat::Binary);
        assert_eq!("llm".parse::<DxOutputFormat>().unwrap(), DxOutputFormat::Llm);
        assert_eq!("human".parse::<DxOutputFormat>().unwrap(), DxOutputFormat::Human);
    }

    #[test]
    fn test_dx_diagnostic_from_internal() {
        let internal = InternalDiagnostic {
            file: PathBuf::from("test.rs"),
            span: crate::diagnostics::Span::new(10, 15),
            severity: DiagnosticSeverity::Warning,
            rule_id: "test/rule".to_string(),
            message: "Test message".to_string(),
            suggestion: Some("Fix it".to_string()),
            related: Vec::new(),
            fix: None,
        };

        let dx = DxDiagnostic::from(&internal);
        assert_eq!(dx.file, "test.rs");
        assert_eq!(dx.line, 10);
        assert_eq!(dx.severity, 2);
        assert_eq!(dx.rule, "test/rule");
    }

    #[test]
    fn test_human_format_output() {
        let report = DxCheckReport {
            version: 1,
            timestamp: 0,
            score: 450,
            breakdown: DxScoreBreakdown {
                formatting: 95,
                linting: 90,
                security: 100,
                patterns: 85,
                structure: 80,
            },
            diagnostics: vec![],
            test_results: None,
            coverage: None,
            files_analyzed: 10,
            total_issues: 0,
        };

        let output = report.to_dx_human();
        assert!(output.contains("450/500"));
        assert!(output.contains("A+"));
    }
}
