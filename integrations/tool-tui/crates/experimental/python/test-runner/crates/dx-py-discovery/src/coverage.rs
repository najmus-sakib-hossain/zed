//! pytest-cov integration for coverage collection
//!
//! This module provides:
//! - Coverage configuration parsing
//! - Coverage data collection interface
//! - Report format support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Coverage report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageReportFormat {
    /// Terminal output
    Term,
    /// Terminal with missing lines
    TermMissing,
    /// HTML report
    Html,
    /// XML report (Cobertura format)
    Xml,
    /// JSON report
    Json,
    /// LCOV format
    Lcov,
}

impl CoverageReportFormat {
    /// Get the coverage.py report type string
    pub fn coverage_py_type(&self) -> &'static str {
        match self {
            CoverageReportFormat::Term => "term",
            CoverageReportFormat::TermMissing => "term-missing",
            CoverageReportFormat::Html => "html",
            CoverageReportFormat::Xml => "xml",
            CoverageReportFormat::Json => "json",
            CoverageReportFormat::Lcov => "lcov",
        }
    }

    /// Parse from string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "term" => Some(CoverageReportFormat::Term),
            "term-missing" | "term_missing" => Some(CoverageReportFormat::TermMissing),
            "html" => Some(CoverageReportFormat::Html),
            "xml" => Some(CoverageReportFormat::Xml),
            "json" => Some(CoverageReportFormat::Json),
            "lcov" => Some(CoverageReportFormat::Lcov),
            _ => None,
        }
    }
}

/// Coverage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageConfig {
    /// Source paths to measure coverage for
    pub source: Vec<PathBuf>,
    /// Paths to omit from coverage
    pub omit: Vec<String>,
    /// Paths to include in coverage
    pub include: Vec<String>,
    /// Report formats to generate
    pub report_formats: Vec<CoverageReportFormat>,
    /// Output directory for reports
    pub report_dir: PathBuf,
    /// Minimum coverage percentage required
    pub fail_under: Option<f64>,
    /// Whether to show missing lines in terminal output
    pub show_missing: bool,
    /// Whether to skip covered files in report
    pub skip_covered: bool,
    /// Whether to skip empty files
    pub skip_empty: bool,
    /// Branch coverage enabled
    pub branch: bool,
    /// Context to add to coverage data
    pub context: Option<String>,
}

impl Default for CoverageConfig {
    fn default() -> Self {
        Self {
            source: Vec::new(),
            omit: vec![
                "*/__pycache__/*".to_string(),
                "*/test_*".to_string(),
                "*_test.py".to_string(),
                "*/tests/*".to_string(),
            ],
            include: Vec::new(),
            report_formats: vec![CoverageReportFormat::Term],
            report_dir: PathBuf::from("htmlcov"),
            fail_under: None,
            show_missing: false,
            skip_covered: false,
            skip_empty: true,
            branch: false,
            context: None,
        }
    }
}

impl CoverageConfig {
    /// Create config from command line arguments
    pub fn from_args(args: &CoverageArgs) -> Self {
        let mut config = Self::default();

        if !args.cov.is_empty() {
            config.source = args.cov.iter().map(PathBuf::from).collect();
        }

        if !args.cov_report.is_empty() {
            config.report_formats = args
                .cov_report
                .iter()
                .filter_map(|s| CoverageReportFormat::from_str(s))
                .collect();
        }

        if let Some(fail_under) = args.cov_fail_under {
            config.fail_under = Some(fail_under);
        }

        config.branch = args.cov_branch;

        config
    }

    /// Generate coverage.py configuration
    pub fn to_coverage_rc(&self) -> String {
        let mut rc = String::from("[run]\n");

        if !self.source.is_empty() {
            rc.push_str(&format!(
                "source = {}\n",
                self.source
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }

        if !self.omit.is_empty() {
            rc.push_str(&format!("omit = {}\n", self.omit.join(",")));
        }

        if self.branch {
            rc.push_str("branch = True\n");
        }

        if let Some(ref context) = self.context {
            rc.push_str(&format!("context = {}\n", context));
        }

        rc.push_str("\n[report]\n");

        if self.show_missing {
            rc.push_str("show_missing = True\n");
        }

        if self.skip_covered {
            rc.push_str("skip_covered = True\n");
        }

        if self.skip_empty {
            rc.push_str("skip_empty = True\n");
        }

        if let Some(fail_under) = self.fail_under {
            rc.push_str(&format!("fail_under = {}\n", fail_under));
        }

        rc
    }
}

/// Command line arguments for coverage
#[derive(Debug, Clone, Default)]
pub struct CoverageArgs {
    /// Paths to measure coverage for (--cov)
    pub cov: Vec<String>,
    /// Report formats (--cov-report)
    pub cov_report: Vec<String>,
    /// Minimum coverage percentage (--cov-fail-under)
    pub cov_fail_under: Option<f64>,
    /// Enable branch coverage (--cov-branch)
    pub cov_branch: bool,
    /// Append to existing coverage data (--cov-append)
    pub cov_append: bool,
    /// Configuration file (--cov-config)
    pub cov_config: Option<String>,
}

impl CoverageArgs {
    /// Parse coverage arguments from command line args
    pub fn parse(args: &[String]) -> Self {
        let mut result = Self::default();
        let mut i = 0;

        while i < args.len() {
            let arg = &args[i];

            if arg == "--cov" && i + 1 < args.len() {
                i += 1;
                result.cov.push(args[i].clone());
            } else if let Some(stripped) = arg.strip_prefix("--cov=") {
                result.cov.push(stripped.to_string());
            } else if arg == "--cov-report" && i + 1 < args.len() {
                i += 1;
                result.cov_report.push(args[i].clone());
            } else if let Some(stripped) = arg.strip_prefix("--cov-report=") {
                result.cov_report.push(stripped.to_string());
            } else if arg == "--cov-fail-under" && i + 1 < args.len() {
                i += 1;
                result.cov_fail_under = args[i].parse().ok();
            } else if let Some(stripped) = arg.strip_prefix("--cov-fail-under=") {
                result.cov_fail_under = stripped.parse().ok();
            } else if arg == "--cov-branch" {
                result.cov_branch = true;
            } else if arg == "--cov-append" {
                result.cov_append = true;
            } else if arg == "--cov-config" && i + 1 < args.len() {
                i += 1;
                result.cov_config = Some(args[i].clone());
            } else if let Some(stripped) = arg.strip_prefix("--cov-config=") {
                result.cov_config = Some(stripped.to_string());
            }

            i += 1;
        }

        result
    }

    /// Check if coverage is enabled
    pub fn is_enabled(&self) -> bool {
        !self.cov.is_empty()
    }
}

/// Coverage data for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    /// File path
    pub path: PathBuf,
    /// Lines that were executed
    pub executed_lines: Vec<u32>,
    /// Lines that were not executed
    pub missing_lines: Vec<u32>,
    /// Branches taken (if branch coverage enabled)
    pub branches_taken: Vec<(u32, u32)>,
    /// Branches not taken
    pub branches_missing: Vec<(u32, u32)>,
    /// Line coverage percentage
    pub line_rate: f64,
    /// Branch coverage percentage
    pub branch_rate: Option<f64>,
}

impl FileCoverage {
    /// Calculate line coverage percentage
    pub fn calculate_line_rate(&self) -> f64 {
        let total = self.executed_lines.len() + self.missing_lines.len();
        if total == 0 {
            100.0
        } else {
            (self.executed_lines.len() as f64 / total as f64) * 100.0
        }
    }

    /// Calculate branch coverage percentage
    pub fn calculate_branch_rate(&self) -> Option<f64> {
        let total = self.branches_taken.len() + self.branches_missing.len();
        if total == 0 {
            None
        } else {
            Some((self.branches_taken.len() as f64 / total as f64) * 100.0)
        }
    }
}

/// Aggregated coverage data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageData {
    /// Coverage data per file
    pub files: HashMap<PathBuf, FileCoverage>,
    /// Total line coverage percentage
    pub total_line_rate: f64,
    /// Total branch coverage percentage
    pub total_branch_rate: Option<f64>,
    /// Total lines covered
    pub lines_covered: usize,
    /// Total lines
    pub lines_total: usize,
    /// Total branches covered
    pub branches_covered: usize,
    /// Total branches
    pub branches_total: usize,
}

impl CoverageData {
    /// Calculate totals from file coverage
    pub fn calculate_totals(&mut self) {
        self.lines_covered = 0;
        self.lines_total = 0;
        self.branches_covered = 0;
        self.branches_total = 0;

        for file in self.files.values() {
            self.lines_covered += file.executed_lines.len();
            self.lines_total += file.executed_lines.len() + file.missing_lines.len();
            self.branches_covered += file.branches_taken.len();
            self.branches_total += file.branches_taken.len() + file.branches_missing.len();
        }

        self.total_line_rate = if self.lines_total == 0 {
            100.0
        } else {
            (self.lines_covered as f64 / self.lines_total as f64) * 100.0
        };

        self.total_branch_rate = if self.branches_total == 0 {
            None
        } else {
            Some((self.branches_covered as f64 / self.branches_total as f64) * 100.0)
        };
    }

    /// Check if coverage meets the fail_under threshold
    pub fn meets_threshold(&self, fail_under: f64) -> bool {
        self.total_line_rate >= fail_under
    }
}

/// Coverage collector for test execution
#[allow(dead_code)]
pub struct CoverageCollector {
    /// Configuration
    config: CoverageConfig,
    /// Collected coverage data
    data: CoverageData,
    /// Whether collection is active
    active: bool,
}

#[allow(dead_code)]
impl CoverageCollector {
    /// Create a new coverage collector
    pub fn new(config: CoverageConfig) -> Self {
        Self {
            config,
            data: CoverageData::default(),
            active: false,
        }
    }

    /// Start coverage collection
    pub fn start(&mut self) -> String {
        self.active = true;

        // Generate Python code to start coverage
        let mut code = String::from("import coverage\n");
        code.push_str("_cov = coverage.Coverage(\n");

        if !self.config.source.is_empty() {
            code.push_str(&format!(
                "    source=[{}],\n",
                self.config
                    .source
                    .iter()
                    .map(|p| format!("'{}'", p.to_string_lossy()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if !self.config.omit.is_empty() {
            code.push_str(&format!(
                "    omit=[{}],\n",
                self.config
                    .omit
                    .iter()
                    .map(|p| format!("'{}'", p))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if self.config.branch {
            code.push_str("    branch=True,\n");
        }

        code.push_str(")\n");
        code.push_str("_cov.start()\n");

        code
    }

    /// Stop coverage collection and get data
    pub fn stop(&mut self) -> String {
        self.active = false;

        // Generate Python code to stop coverage and get data
        let code = r#"
_cov.stop()
_cov.save()
import json
_cov_data = _cov.get_data()
_result = {
    'files': {},
    'totals': {
        'lines_covered': 0,
        'lines_total': 0,
        'branches_covered': 0,
        'branches_total': 0,
    }
}
for filename in _cov_data.measured_files():
    lines = _cov_data.lines(filename) or []
    missing = _cov_data.missing_lines(filename) or []
    _result['files'][filename] = {
        'executed_lines': list(lines),
        'missing_lines': list(missing),
    }
    _result['totals']['lines_covered'] += len(lines)
    _result['totals']['lines_total'] += len(lines) + len(missing)
print(json.dumps(_result))
"#;
        code.to_string()
    }

    /// Parse coverage data from JSON output
    pub fn parse_coverage_json(&mut self, json_str: &str) -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse coverage JSON: {}", e))?;

        if let Some(files) = value.get("files").and_then(|f| f.as_object()) {
            for (path, file_data) in files {
                let executed_lines: Vec<u32> = file_data
                    .get("executed_lines")
                    .and_then(|l| l.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect())
                    .unwrap_or_default();

                let missing_lines: Vec<u32> = file_data
                    .get("missing_lines")
                    .and_then(|l| l.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect())
                    .unwrap_or_default();

                let mut file_cov = FileCoverage {
                    path: PathBuf::from(path),
                    executed_lines,
                    missing_lines,
                    branches_taken: Vec::new(),
                    branches_missing: Vec::new(),
                    line_rate: 0.0,
                    branch_rate: None,
                };

                file_cov.line_rate = file_cov.calculate_line_rate();
                file_cov.branch_rate = file_cov.calculate_branch_rate();

                self.data.files.insert(PathBuf::from(path), file_cov);
            }
        }

        self.data.calculate_totals();
        Ok(())
    }

    /// Get the collected coverage data
    pub fn get_data(&self) -> &CoverageData {
        &self.data
    }

    /// Generate terminal report
    pub fn generate_term_report(&self) -> String {
        let mut report = String::new();

        report.push_str("Name                                      Stmts   Miss  Cover\n");
        report.push_str("-------------------------------------------------------------\n");

        let mut files: Vec<_> = self.data.files.iter().collect();
        files.sort_by_key(|(path, _)| path.to_string_lossy().to_string());

        for (path, file_cov) in files {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            let stmts = file_cov.executed_lines.len() + file_cov.missing_lines.len();
            let miss = file_cov.missing_lines.len();
            let cover = file_cov.line_rate;

            if self.config.skip_covered && miss == 0 {
                continue;
            }

            if self.config.skip_empty && stmts == 0 {
                continue;
            }

            report.push_str(&format!(
                "{:<40} {:>6} {:>6}  {:>5.0}%\n",
                truncate_name(&name, 40),
                stmts,
                miss,
                cover
            ));
        }

        report.push_str("-------------------------------------------------------------\n");
        report.push_str(&format!(
            "{:<40} {:>6} {:>6}  {:>5.1}%\n",
            "TOTAL",
            self.data.lines_total,
            self.data.lines_total - self.data.lines_covered,
            self.data.total_line_rate
        ));

        report
    }

    /// Generate terminal report with missing lines
    pub fn generate_term_missing_report(&self) -> String {
        let mut report = String::new();

        report
            .push_str("Name                                      Stmts   Miss  Cover   Missing\n");
        report
            .push_str("-----------------------------------------------------------------------\n");

        let mut files: Vec<_> = self.data.files.iter().collect();
        files.sort_by_key(|(path, _)| path.to_string_lossy().to_string());

        for (path, file_cov) in files {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            let stmts = file_cov.executed_lines.len() + file_cov.missing_lines.len();
            let miss = file_cov.missing_lines.len();
            let cover = file_cov.line_rate;

            if self.config.skip_covered && miss == 0 {
                continue;
            }

            if self.config.skip_empty && stmts == 0 {
                continue;
            }

            let missing = format_line_ranges(&file_cov.missing_lines);

            report.push_str(&format!(
                "{:<40} {:>6} {:>6}  {:>5.0}%   {}\n",
                truncate_name(&name, 40),
                stmts,
                miss,
                cover,
                missing
            ));
        }

        report
            .push_str("-----------------------------------------------------------------------\n");
        report.push_str(&format!(
            "{:<40} {:>6} {:>6}  {:>5.1}%\n",
            "TOTAL",
            self.data.lines_total,
            self.data.lines_total - self.data.lines_covered,
            self.data.total_line_rate
        ));

        report
    }

    /// Generate JSON report
    pub fn generate_json_report(&self) -> String {
        serde_json::to_string_pretty(&self.data).unwrap_or_default()
    }

    /// Generate XML report (Cobertura format)
    pub fn generate_xml_report(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" ?>
<coverage version="1.0" timestamp="" lines-valid="" lines-covered="" line-rate="" branches-valid="" branches-covered="" branch-rate="" complexity="">
    <packages>
        <package name="" line-rate="" branch-rate="" complexity="">
            <classes>
"#,
        );

        for (path, file_cov) in &self.data.files {
            let filename = path.to_string_lossy();
            xml.push_str(&format!(
                r#"                <class name="{}" filename="{}" line-rate="{:.4}" branch-rate="0">
                    <lines>
"#,
                path.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default(),
                filename,
                file_cov.line_rate / 100.0
            ));

            for line in &file_cov.executed_lines {
                xml.push_str(&format!(
                    r#"                        <line number="{}" hits="1"/>
"#,
                    line
                ));
            }

            for line in &file_cov.missing_lines {
                xml.push_str(&format!(
                    r#"                        <line number="{}" hits="0"/>
"#,
                    line
                ));
            }

            xml.push_str(
                r#"                    </lines>
                </class>
"#,
            );
        }

        xml.push_str(
            r#"            </classes>
        </package>
    </packages>
</coverage>
"#,
        );

        xml
    }

    /// Generate report in the specified format
    pub fn generate_report(&self, format: CoverageReportFormat) -> String {
        match format {
            CoverageReportFormat::Term => self.generate_term_report(),
            CoverageReportFormat::TermMissing => self.generate_term_missing_report(),
            CoverageReportFormat::Json => self.generate_json_report(),
            CoverageReportFormat::Xml => self.generate_xml_report(),
            CoverageReportFormat::Html => self.generate_html_report(),
            CoverageReportFormat::Lcov => self.generate_lcov_report(),
        }
    }

    /// Generate HTML report (simplified)
    pub fn generate_html_report(&self) -> String {
        let mut html = String::from(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Coverage Report</title>
    <style>
        body { font-family: sans-serif; margin: 20px; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #4CAF50; color: white; }
        tr:nth-child(even) { background-color: #f2f2f2; }
        .covered { color: green; }
        .missing { color: red; }
    </style>
</head>
<body>
    <h1>Coverage Report</h1>
    <table>
        <tr>
            <th>File</th>
            <th>Statements</th>
            <th>Missing</th>
            <th>Coverage</th>
        </tr>
"#,
        );

        for (path, file_cov) in &self.data.files {
            let stmts = file_cov.executed_lines.len() + file_cov.missing_lines.len();
            let miss = file_cov.missing_lines.len();

            html.push_str(&format!(
                r#"        <tr>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>{:.1}%</td>
        </tr>
"#,
                path.to_string_lossy(),
                stmts,
                miss,
                file_cov.line_rate
            ));
        }

        html.push_str(&format!(
            r#"        <tr style="font-weight: bold;">
            <td>TOTAL</td>
            <td>{}</td>
            <td>{}</td>
            <td>{:.1}%</td>
        </tr>
    </table>
</body>
</html>
"#,
            self.data.lines_total,
            self.data.lines_total - self.data.lines_covered,
            self.data.total_line_rate
        ));

        html
    }

    /// Generate LCOV report
    pub fn generate_lcov_report(&self) -> String {
        let mut lcov = String::new();

        for (path, file_cov) in &self.data.files {
            lcov.push_str(&format!("SF:{}\n", path.to_string_lossy()));

            for line in &file_cov.executed_lines {
                lcov.push_str(&format!("DA:{},1\n", line));
            }

            for line in &file_cov.missing_lines {
                lcov.push_str(&format!("DA:{},0\n", line));
            }

            let total = file_cov.executed_lines.len() + file_cov.missing_lines.len();
            lcov.push_str(&format!("LF:{}\n", total));
            lcov.push_str(&format!("LH:{}\n", file_cov.executed_lines.len()));
            lcov.push_str("end_of_record\n");
        }

        lcov
    }

    /// Check if coverage meets the fail_under threshold
    pub fn check_threshold(&self) -> Result<(), String> {
        if let Some(fail_under) = self.config.fail_under {
            if !self.data.meets_threshold(fail_under) {
                return Err(format!(
                    "Coverage {:.1}% is below the required {:.1}%",
                    self.data.total_line_rate, fail_under
                ));
            }
        }
        Ok(())
    }

    /// Check if collection is active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Truncate a name to fit in a column
#[allow(dead_code)]
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("...{}", &name[name.len() - max_len + 3..])
    }
}

/// Format line numbers as ranges (e.g., "1-5, 10, 15-20")
#[allow(dead_code)]
fn format_line_ranges(lines: &[u32]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let mut sorted: Vec<u32> = lines.to_vec();
    sorted.sort();

    let mut ranges = Vec::new();
    let mut start = sorted[0];
    let mut end = sorted[0];

    for &line in sorted.iter().skip(1) {
        if line == end + 1 {
            end = line;
        } else {
            if start == end {
                ranges.push(format!("{}", start));
            } else {
                ranges.push(format!("{}-{}", start, end));
            }
            start = line;
            end = line;
        }
    }

    // Add the last range
    if start == end {
        ranges.push(format!("{}", start));
    } else {
        ranges.push(format!("{}-{}", start, end));
    }

    ranges.join(", ")
}
