//! Code Coverage Implementation for DX Test Runner
//!
//! Provides code instrumentation and coverage tracking:
//! - Statement-level coverage
//! - Branch coverage (if/else, ternary, logical operators)
//! - Function coverage
//! - Coverage report generation (HTML, JSON, LCOV)

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Coverage data for a single source file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileCoverage {
    /// File path
    pub path: PathBuf,
    /// Statement coverage: line -> (hit_count, is_executable)
    pub statements: BTreeMap<u32, StatementCoverage>,
    /// Branch coverage: branch_id -> BranchCoverage
    pub branches: BTreeMap<u32, BranchCoverage>,
    /// Function coverage: function_name -> FunctionCoverage
    pub functions: BTreeMap<String, FunctionCoverage>,
}

/// Coverage data for a single statement
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatementCoverage {
    /// Line number
    pub line: u32,
    /// Column number (start)
    pub column: u32,
    /// Number of times this statement was executed
    pub hit_count: u32,
    /// Whether this is an executable statement
    pub is_executable: bool,
}

/// Coverage data for a branch point
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchCoverage {
    /// Branch ID
    pub id: u32,
    /// Line number where branch occurs
    pub line: u32,
    /// Type of branch (if, ternary, logical, switch)
    pub branch_type: BranchType,
    /// Number of times the "true" branch was taken
    pub true_count: u32,
    /// Number of times the "false" branch was taken
    pub false_count: u32,
}

/// Types of branches we track
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BranchType {
    /// if/else statement
    #[default]
    IfElse,
    /// Ternary operator (? :)
    Ternary,
    /// Logical AND (&&)
    LogicalAnd,
    /// Logical OR (||)
    LogicalOr,
    /// Nullish coalescing (??)
    NullishCoalescing,
    /// Switch case
    SwitchCase,
}

/// Coverage data for a function
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionCoverage {
    /// Function name
    pub name: String,
    /// Start line
    pub start_line: u32,
    /// End line
    pub end_line: u32,
    /// Number of times this function was called
    pub hit_count: u32,
}

impl FileCoverage {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            statements: BTreeMap::new(),
            branches: BTreeMap::new(),
            functions: BTreeMap::new(),
        }
    }

    /// Record a statement hit
    pub fn hit_statement(&mut self, line: u32) {
        if let Some(stmt) = self.statements.get_mut(&line) {
            stmt.hit_count += 1;
        }
    }

    /// Record a branch hit
    pub fn hit_branch(&mut self, branch_id: u32, taken: bool) {
        if let Some(branch) = self.branches.get_mut(&branch_id) {
            if taken {
                branch.true_count += 1;
            } else {
                branch.false_count += 1;
            }
        }
    }

    /// Record a function hit
    pub fn hit_function(&mut self, name: &str) {
        if let Some(func) = self.functions.get_mut(name) {
            func.hit_count += 1;
        }
    }

    /// Calculate statement coverage percentage
    pub fn statement_coverage_percent(&self) -> f64 {
        let executable: Vec<_> = self.statements.values().filter(|s| s.is_executable).collect();

        if executable.is_empty() {
            return 100.0;
        }

        let covered = executable.iter().filter(|s| s.hit_count > 0).count();
        (covered as f64 / executable.len() as f64) * 100.0
    }

    /// Calculate branch coverage percentage
    pub fn branch_coverage_percent(&self) -> f64 {
        if self.branches.is_empty() {
            return 100.0;
        }

        // A branch is fully covered if both true and false paths were taken
        let fully_covered =
            self.branches.values().filter(|b| b.true_count > 0 && b.false_count > 0).count();

        (fully_covered as f64 / self.branches.len() as f64) * 100.0
    }

    /// Calculate function coverage percentage
    pub fn function_coverage_percent(&self) -> f64 {
        if self.functions.is_empty() {
            return 100.0;
        }

        let covered = self.functions.values().filter(|f| f.hit_count > 0).count();

        (covered as f64 / self.functions.len() as f64) * 100.0
    }

    /// Get uncovered lines
    pub fn uncovered_lines(&self) -> Vec<u32> {
        self.statements
            .iter()
            .filter(|(_, s)| s.is_executable && s.hit_count == 0)
            .map(|(line, _)| *line)
            .collect()
    }

    /// Get partially covered branches
    pub fn partial_branches(&self) -> Vec<&BranchCoverage> {
        self.branches
            .values()
            .filter(|b| (b.true_count > 0) != (b.false_count > 0))
            .collect()
    }
}

/// Code instrumenter that inserts coverage tracking into JavaScript code
#[derive(Debug, Clone)]
pub struct CodeInstrumenter {
    /// Next branch ID to assign
    next_branch_id: u32,
    /// Coverage variable name to use in instrumented code
    coverage_var: String,
}

impl CodeInstrumenter {
    pub fn new() -> Self {
        Self {
            next_branch_id: 0,
            coverage_var: "__coverage__".to_string(),
        }
    }

    /// Set custom coverage variable name
    pub fn with_coverage_var(mut self, var: String) -> Self {
        self.coverage_var = var;
        self
    }

    /// Instrument a JavaScript source file
    /// Returns (instrumented_code, coverage_map)
    pub fn instrument(&mut self, source: &str, file_path: &Path) -> (String, FileCoverage) {
        let mut coverage = FileCoverage::new(file_path.to_path_buf());
        let mut output = String::new();

        // Add coverage initialization header
        output.push_str(&self.generate_header(file_path));

        // Process source line by line
        let lines: Vec<&str> = source.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = (line_num + 1) as u32;

            // Detect executable statements
            if self.is_executable_line(line) {
                coverage.statements.insert(
                    line_number,
                    StatementCoverage {
                        line: line_number,
                        column: 0,
                        hit_count: 0,
                        is_executable: true,
                    },
                );

                // Insert statement counter
                let instrumented = self.instrument_statement(line, line_number);
                output.push_str(&instrumented);
            } else {
                output.push_str(line);
            }
            output.push('\n');

            // Detect branches
            if let Some(branch) = self.detect_branch(line, line_number) {
                coverage.branches.insert(branch.id, branch);
            }

            // Detect functions
            if let Some(func) = self.detect_function(line, line_number) {
                coverage.functions.insert(func.name.clone(), func);
            }
        }

        (output, coverage)
    }

    /// Generate coverage initialization header
    fn generate_header(&self, file_path: &Path) -> String {
        format!(
            r#"// Coverage instrumentation for {}
if (typeof globalThis.{} === 'undefined') {{
    globalThis.{} = {{}};
}}
globalThis.{}['{}'] = {{
    statements: {{}},
    branches: {{}},
    functions: {{}}
}};
const __cov__ = globalThis.{}['{}'];

"#,
            file_path.display(),
            self.coverage_var,
            self.coverage_var,
            self.coverage_var,
            file_path.display(),
            self.coverage_var,
            file_path.display()
        )
    }

    /// Check if a line contains executable code
    fn is_executable_line(&self, line: &str) -> bool {
        let trimmed = line.trim();

        // Skip empty lines, comments, and structural-only lines
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with("*")
            || trimmed == "{"
            || trimmed == "}"
            || trimmed == "};"
            || trimmed == "});"
            || trimmed.starts_with("import ")
            || trimmed.starts_with("export ")
            || trimmed.starts_with("else {")
            || trimmed == "else"
        {
            return false;
        }

        true
    }

    /// Instrument a statement with coverage tracking
    fn instrument_statement(&self, line: &str, line_number: u32) -> String {
        let trimmed = line.trim();
        let indent = &line[..line.len() - trimmed.len()];

        // Insert statement counter before the statement
        format!(
            "{}__cov__.statements[{}] = (__cov__.statements[{}] || 0) + 1; {}",
            indent, line_number, line_number, trimmed
        )
    }

    /// Detect branch points in a line
    fn detect_branch(&mut self, line: &str, line_number: u32) -> Option<BranchCoverage> {
        let trimmed = line.trim();

        let branch_type = if trimmed.starts_with("if ") || trimmed.starts_with("if(") {
            Some(BranchType::IfElse)
        } else if trimmed.contains(" ? ") && trimmed.contains(" : ") {
            Some(BranchType::Ternary)
        } else if trimmed.contains(" && ") {
            Some(BranchType::LogicalAnd)
        } else if trimmed.contains(" || ") {
            Some(BranchType::LogicalOr)
        } else if trimmed.contains(" ?? ") {
            Some(BranchType::NullishCoalescing)
        } else if trimmed.starts_with("case ") || trimmed == "default:" {
            Some(BranchType::SwitchCase)
        } else {
            None
        };

        branch_type.map(|bt| {
            let id = self.next_branch_id;
            self.next_branch_id += 1;
            BranchCoverage {
                id,
                line: line_number,
                branch_type: bt,
                true_count: 0,
                false_count: 0,
            }
        })
    }

    /// Detect function declarations
    fn detect_function(&self, line: &str, line_number: u32) -> Option<FunctionCoverage> {
        let trimmed = line.trim();

        // Match function declarations
        if let Some(after_function) = trimmed.strip_prefix("function ") {
            // Extract function name
            if let Some(paren_pos) = after_function.find('(') {
                let name = after_function[..paren_pos].trim().to_string();
                if !name.is_empty() {
                    return Some(FunctionCoverage {
                        name,
                        start_line: line_number,
                        end_line: 0, // Will be updated when we find the closing brace
                        hit_count: 0,
                    });
                }
            }
        }

        // Match arrow functions assigned to variables
        if (trimmed.starts_with("const ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("var "))
            && trimmed.contains(" = ")
            && (trimmed.contains("=>") || trimmed.contains("function"))
        {
            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
            if parts.len() >= 2 {
                let rest = parts[1];
                if let Some(eq_pos) = rest.find(" = ") {
                    let name = rest[..eq_pos].trim().to_string();
                    return Some(FunctionCoverage {
                        name,
                        start_line: line_number,
                        end_line: 0,
                        hit_count: 0,
                    });
                }
            }
        }

        // Match method definitions
        if trimmed.contains("(")
            && trimmed.contains(")")
            && trimmed.contains("{")
            && !trimmed.starts_with("if")
            && !trimmed.starts_with("while")
            && !trimmed.starts_with("for")
            && !trimmed.starts_with("switch")
        {
            if let Some(paren_pos) = trimmed.find('(') {
                let before_paren = &trimmed[..paren_pos];
                let name = before_paren.split_whitespace().last().unwrap_or("").to_string();
                if !name.is_empty() && name != "function" {
                    return Some(FunctionCoverage {
                        name,
                        start_line: line_number,
                        end_line: 0,
                        hit_count: 0,
                    });
                }
            }
        }

        None
    }
}

impl Default for CodeInstrumenter {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime coverage collector that aggregates coverage data during test execution
#[derive(Debug, Default)]
pub struct CoverageCollector {
    /// Coverage data per file
    files: HashMap<PathBuf, FileCoverage>,
    /// Whether collection is enabled
    enabled: bool,
}

impl CoverageCollector {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            enabled: true,
        }
    }

    /// Enable or disable coverage collection
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if coverage collection is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Register a file's coverage map (from instrumentation)
    pub fn register_file(&mut self, coverage: FileCoverage) {
        self.files.insert(coverage.path.clone(), coverage);
    }

    /// Record a statement hit
    pub fn hit_statement(&mut self, file: &Path, line: u32) {
        if !self.enabled {
            return;
        }
        if let Some(coverage) = self.files.get_mut(file) {
            coverage.hit_statement(line);
        }
    }

    /// Record a branch hit
    pub fn hit_branch(&mut self, file: &Path, branch_id: u32, taken: bool) {
        if !self.enabled {
            return;
        }
        if let Some(coverage) = self.files.get_mut(file) {
            coverage.hit_branch(branch_id, taken);
        }
    }

    /// Record a function hit
    pub fn hit_function(&mut self, file: &Path, name: &str) {
        if !self.enabled {
            return;
        }
        if let Some(coverage) = self.files.get_mut(file) {
            coverage.hit_function(name);
        }
    }

    /// Merge coverage from another collector
    pub fn merge(&mut self, other: CoverageCollector) {
        for (path, other_coverage) in other.files {
            if let Some(coverage) = self.files.get_mut(&path) {
                // Merge statement hits
                for (line, stmt) in other_coverage.statements {
                    if let Some(existing) = coverage.statements.get_mut(&line) {
                        existing.hit_count += stmt.hit_count;
                    } else {
                        coverage.statements.insert(line, stmt);
                    }
                }

                // Merge branch hits
                for (id, branch) in other_coverage.branches {
                    if let Some(existing) = coverage.branches.get_mut(&id) {
                        existing.true_count += branch.true_count;
                        existing.false_count += branch.false_count;
                    } else {
                        coverage.branches.insert(id, branch);
                    }
                }

                // Merge function hits
                for (name, func) in other_coverage.functions {
                    if let Some(existing) = coverage.functions.get_mut(&name) {
                        existing.hit_count += func.hit_count;
                    } else {
                        coverage.functions.insert(name, func);
                    }
                }
            } else {
                self.files.insert(path, other_coverage);
            }
        }
    }

    /// Get all file coverages
    pub fn files(&self) -> &HashMap<PathBuf, FileCoverage> {
        &self.files
    }

    /// Calculate total statement coverage
    pub fn total_statement_coverage(&self) -> f64 {
        let mut total_executable = 0;
        let mut total_covered = 0;

        for coverage in self.files.values() {
            for stmt in coverage.statements.values() {
                if stmt.is_executable {
                    total_executable += 1;
                    if stmt.hit_count > 0 {
                        total_covered += 1;
                    }
                }
            }
        }

        if total_executable == 0 {
            return 100.0;
        }
        (total_covered as f64 / total_executable as f64) * 100.0
    }

    /// Calculate total branch coverage
    pub fn total_branch_coverage(&self) -> f64 {
        let mut total_branches = 0;
        let mut fully_covered = 0;

        for coverage in self.files.values() {
            for branch in coverage.branches.values() {
                total_branches += 1;
                if branch.true_count > 0 && branch.false_count > 0 {
                    fully_covered += 1;
                }
            }
        }

        if total_branches == 0 {
            return 100.0;
        }
        (fully_covered as f64 / total_branches as f64) * 100.0
    }

    /// Calculate total function coverage
    pub fn total_function_coverage(&self) -> f64 {
        let mut total_functions = 0;
        let mut covered = 0;

        for coverage in self.files.values() {
            for func in coverage.functions.values() {
                total_functions += 1;
                if func.hit_count > 0 {
                    covered += 1;
                }
            }
        }

        if total_functions == 0 {
            return 100.0;
        }
        (covered as f64 / total_functions as f64) * 100.0
    }
}

/// Coverage summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    /// Total statement coverage percentage
    pub statements: f64,
    /// Total branch coverage percentage
    pub branches: f64,
    /// Total function coverage percentage
    pub functions: f64,
    /// Number of files
    pub file_count: usize,
    /// Per-file summaries
    pub files: Vec<FileCoverageSummary>,
}

/// Per-file coverage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverageSummary {
    /// File path
    pub path: String,
    /// Statement coverage percentage
    pub statements: f64,
    /// Branch coverage percentage
    pub branches: f64,
    /// Function coverage percentage
    pub functions: f64,
    /// Uncovered lines
    pub uncovered_lines: Vec<u32>,
}

impl CoverageCollector {
    /// Generate a coverage summary
    pub fn summary(&self) -> CoverageSummary {
        let files: Vec<FileCoverageSummary> = self
            .files
            .values()
            .map(|f| FileCoverageSummary {
                path: f.path.display().to_string(),
                statements: f.statement_coverage_percent(),
                branches: f.branch_coverage_percent(),
                functions: f.function_coverage_percent(),
                uncovered_lines: f.uncovered_lines(),
            })
            .collect();

        CoverageSummary {
            statements: self.total_statement_coverage(),
            branches: self.total_branch_coverage(),
            functions: self.total_function_coverage(),
            file_count: self.files.len(),
            files,
        }
    }
}

/// Coverage report generator
pub struct CoverageReporter {
    collector: CoverageCollector,
}

impl CoverageReporter {
    pub fn new(collector: CoverageCollector) -> Self {
        Self { collector }
    }

    /// Generate LCOV format report
    pub fn generate_lcov(&self) -> String {
        let mut output = String::new();

        for coverage in self.collector.files.values() {
            // Source file
            output.push_str(&format!("SF:{}\n", coverage.path.display()));

            // Functions
            for (name, func) in &coverage.functions {
                output.push_str(&format!("FN:{},{}\n", func.start_line, name));
                output.push_str(&format!("FNDA:{},{}\n", func.hit_count, name));
            }
            output.push_str(&format!("FNF:{}\n", coverage.functions.len()));
            let fnh = coverage.functions.values().filter(|f| f.hit_count > 0).count();
            output.push_str(&format!("FNH:{}\n", fnh));

            // Branches
            for (id, branch) in &coverage.branches {
                output.push_str(&format!("BRDA:{},{},0,{}\n", branch.line, id, branch.true_count));
                output.push_str(&format!("BRDA:{},{},1,{}\n", branch.line, id, branch.false_count));
            }
            output.push_str(&format!("BRF:{}\n", coverage.branches.len() * 2));
            let brh = coverage
                .branches
                .values()
                .map(|b| {
                    (if b.true_count > 0 { 1 } else { 0 }) + (if b.false_count > 0 { 1 } else { 0 })
                })
                .sum::<usize>();
            output.push_str(&format!("BRH:{}\n", brh));

            // Lines
            for (line, stmt) in &coverage.statements {
                if stmt.is_executable {
                    output.push_str(&format!("DA:{},{}\n", line, stmt.hit_count));
                }
            }
            let lf = coverage.statements.values().filter(|s| s.is_executable).count();
            let lh = coverage
                .statements
                .values()
                .filter(|s| s.is_executable && s.hit_count > 0)
                .count();
            output.push_str(&format!("LF:{}\n", lf));
            output.push_str(&format!("LH:{}\n", lh));

            output.push_str("end_of_record\n");
        }

        output
    }

    /// Generate JSON format report
    pub fn generate_json(&self) -> String {
        let summary = self.collector.summary();
        serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate HTML format report
    pub fn generate_html(&self) -> String {
        let summary = self.collector.summary();

        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Coverage Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }\n");
        html.push_str(".container { max-width: 1200px; margin: 0 auto; }\n");
        html.push_str("h1 { color: #333; }\n");
        html.push_str(".summary { display: flex; gap: 20px; margin-bottom: 30px; }\n");
        html.push_str(".metric { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); flex: 1; }\n");
        html.push_str(".metric-value { font-size: 36px; font-weight: bold; }\n");
        html.push_str(".metric-label { color: #666; margin-top: 5px; }\n");
        html.push_str(".high { color: #22c55e; }\n");
        html.push_str(".medium { color: #eab308; }\n");
        html.push_str(".low { color: #ef4444; }\n");
        html.push_str("table { width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }\n");
        html.push_str("th { background: #333; color: white; padding: 12px; text-align: left; }\n");
        html.push_str("td { padding: 12px; border-bottom: 1px solid #eee; }\n");
        html.push_str("tr:hover { background: #f9f9f9; }\n");
        html.push_str(
            ".bar { height: 8px; background: #eee; border-radius: 4px; overflow: hidden; }\n",
        );
        html.push_str(".bar-fill { height: 100%; transition: width 0.3s; }\n");
        html.push_str("</style>\n</head>\n<body>\n");
        html.push_str("<div class=\"container\">\n");
        html.push_str("<h1>📊 Coverage Report</h1>\n");

        // Summary metrics
        html.push_str("<div class=\"summary\">\n");
        html.push_str(&format_metric("Statements", summary.statements));
        html.push_str(&format_metric("Branches", summary.branches));
        html.push_str(&format_metric("Functions", summary.functions));
        html.push_str("</div>\n");

        // File table
        html.push_str("<table>\n");
        html.push_str("<tr><th>File</th><th>Statements</th><th>Branches</th><th>Functions</th><th>Uncovered Lines</th></tr>\n");

        for file in &summary.files {
            let stmt_class = coverage_class(file.statements);
            let branch_class = coverage_class(file.branches);
            let func_class = coverage_class(file.functions);

            let uncovered = if file.uncovered_lines.is_empty() {
                "—".to_string()
            } else {
                file.uncovered_lines
                    .iter()
                    .take(10)
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
                    + if file.uncovered_lines.len() > 10 {
                        "..."
                    } else {
                        ""
                    }
            };

            html.push_str(&format!(
                "<tr><td>{}</td><td class=\"{}\">{:.1}%</td><td class=\"{}\">{:.1}%</td><td class=\"{}\">{:.1}%</td><td>{}</td></tr>\n",
                file.path, stmt_class, file.statements, branch_class, file.branches, func_class, file.functions, uncovered
            ));
        }

        html.push_str("</table>\n");
        html.push_str("</div>\n</body>\n</html>");

        html
    }
}

fn format_metric(label: &str, value: f64) -> String {
    let class = coverage_class(value);
    format!(
        "<div class=\"metric\"><div class=\"metric-value {}\">{:.1}%</div><div class=\"metric-label\">{}</div></div>\n",
        class, value, label
    )
}

fn coverage_class(value: f64) -> &'static str {
    if value >= 80.0 {
        "high"
    } else if value >= 50.0 {
        "medium"
    } else {
        "low"
    }
}

/// Coverage threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageThresholds {
    /// Minimum statement coverage percentage
    pub statements: f64,
    /// Minimum branch coverage percentage
    pub branches: f64,
    /// Minimum function coverage percentage
    pub functions: f64,
}

impl Default for CoverageThresholds {
    fn default() -> Self {
        Self {
            statements: 0.0,
            branches: 0.0,
            functions: 0.0,
        }
    }
}

impl CoverageThresholds {
    /// Check if coverage meets thresholds
    pub fn check(&self, summary: &CoverageSummary) -> ThresholdResult {
        let mut failures = Vec::new();

        if summary.statements < self.statements {
            failures.push(ThresholdFailure {
                metric: "statements".to_string(),
                threshold: self.statements,
                actual: summary.statements,
            });
        }

        if summary.branches < self.branches {
            failures.push(ThresholdFailure {
                metric: "branches".to_string(),
                threshold: self.branches,
                actual: summary.branches,
            });
        }

        if summary.functions < self.functions {
            failures.push(ThresholdFailure {
                metric: "functions".to_string(),
                threshold: self.functions,
                actual: summary.functions,
            });
        }

        ThresholdResult { failures }
    }
}

/// Result of threshold check
#[derive(Debug, Clone)]
pub struct ThresholdResult {
    pub failures: Vec<ThresholdFailure>,
}

impl ThresholdResult {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// A single threshold failure
#[derive(Debug, Clone)]
pub struct ThresholdFailure {
    pub metric: String,
    pub threshold: f64,
    pub actual: f64,
}

impl std::fmt::Display for ThresholdFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} coverage ({:.1}%) is below threshold ({:.1}%)",
            self.metric, self.actual, self.threshold
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_coverage_basic() {
        let mut coverage = FileCoverage::new(PathBuf::from("test.js"));

        // Add some statements
        coverage.statements.insert(
            1,
            StatementCoverage {
                line: 1,
                column: 0,
                hit_count: 0,
                is_executable: true,
            },
        );
        coverage.statements.insert(
            2,
            StatementCoverage {
                line: 2,
                column: 0,
                hit_count: 0,
                is_executable: true,
            },
        );

        // Initially 0% coverage
        assert_eq!(coverage.statement_coverage_percent(), 0.0);

        // Hit one statement
        coverage.hit_statement(1);
        assert_eq!(coverage.statement_coverage_percent(), 50.0);

        // Hit both
        coverage.hit_statement(2);
        assert_eq!(coverage.statement_coverage_percent(), 100.0);
    }

    #[test]
    fn test_branch_coverage() {
        let mut coverage = FileCoverage::new(PathBuf::from("test.js"));

        coverage.branches.insert(
            0,
            BranchCoverage {
                id: 0,
                line: 5,
                branch_type: BranchType::IfElse,
                true_count: 0,
                false_count: 0,
            },
        );

        // Initially 0% (neither branch taken)
        assert_eq!(coverage.branch_coverage_percent(), 0.0);

        // Take true branch only - still 0% (need both)
        coverage.hit_branch(0, true);
        assert_eq!(coverage.branch_coverage_percent(), 0.0);

        // Take false branch too - now 100%
        coverage.hit_branch(0, false);
        assert_eq!(coverage.branch_coverage_percent(), 100.0);
    }

    #[test]
    fn test_function_coverage() {
        let mut coverage = FileCoverage::new(PathBuf::from("test.js"));

        coverage.functions.insert(
            "foo".to_string(),
            FunctionCoverage {
                name: "foo".to_string(),
                start_line: 1,
                end_line: 5,
                hit_count: 0,
            },
        );
        coverage.functions.insert(
            "bar".to_string(),
            FunctionCoverage {
                name: "bar".to_string(),
                start_line: 7,
                end_line: 10,
                hit_count: 0,
            },
        );

        assert_eq!(coverage.function_coverage_percent(), 0.0);

        coverage.hit_function("foo");
        assert_eq!(coverage.function_coverage_percent(), 50.0);

        coverage.hit_function("bar");
        assert_eq!(coverage.function_coverage_percent(), 100.0);
    }

    #[test]
    fn test_code_instrumenter() {
        let mut instrumenter = CodeInstrumenter::new();

        let source = r#"function add(a, b) {
    return a + b;
}

const result = add(1, 2);
console.log(result);
"#;

        let (instrumented, coverage) = instrumenter.instrument(source, Path::new("test.js"));

        // Should have detected the function
        assert!(coverage.functions.contains_key("add"));

        // Should have executable statements
        assert!(!coverage.statements.is_empty());

        // Instrumented code should contain coverage tracking
        assert!(instrumented.contains("__cov__"));
    }

    #[test]
    fn test_coverage_collector_merge() {
        let mut collector1 = CoverageCollector::new();
        let mut collector2 = CoverageCollector::new();

        let mut cov1 = FileCoverage::new(PathBuf::from("test.js"));
        cov1.statements.insert(
            1,
            StatementCoverage {
                line: 1,
                column: 0,
                hit_count: 5,
                is_executable: true,
            },
        );
        collector1.register_file(cov1);

        let mut cov2 = FileCoverage::new(PathBuf::from("test.js"));
        cov2.statements.insert(
            1,
            StatementCoverage {
                line: 1,
                column: 0,
                hit_count: 3,
                is_executable: true,
            },
        );
        collector2.register_file(cov2);

        collector1.merge(collector2);

        // Hit counts should be merged
        let merged = collector1.files.get(&PathBuf::from("test.js")).unwrap();
        assert_eq!(merged.statements.get(&1).unwrap().hit_count, 8);
    }

    #[test]
    fn test_coverage_thresholds() {
        let thresholds = CoverageThresholds {
            statements: 80.0,
            branches: 70.0,
            functions: 90.0,
        };

        let summary = CoverageSummary {
            statements: 85.0,
            branches: 65.0, // Below threshold
            functions: 95.0,
            file_count: 1,
            files: vec![],
        };

        let result = thresholds.check(&summary);
        assert!(!result.passed());
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].metric, "branches");
    }

    #[test]
    fn test_lcov_generation() {
        let mut collector = CoverageCollector::new();

        let mut coverage = FileCoverage::new(PathBuf::from("test.js"));
        coverage.statements.insert(
            1,
            StatementCoverage {
                line: 1,
                column: 0,
                hit_count: 1,
                is_executable: true,
            },
        );
        coverage.functions.insert(
            "test".to_string(),
            FunctionCoverage {
                name: "test".to_string(),
                start_line: 1,
                end_line: 5,
                hit_count: 1,
            },
        );
        collector.register_file(coverage);

        let reporter = CoverageReporter::new(collector);
        let lcov = reporter.generate_lcov();

        assert!(lcov.contains("SF:test.js"));
        assert!(lcov.contains("FN:1,test"));
        assert!(lcov.contains("DA:1,1"));
        assert!(lcov.contains("end_of_record"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary statement coverage
    fn arb_statement_coverage() -> impl Strategy<Value = StatementCoverage> {
        (1u32..1000, 0u32..100, 0u32..1000, any::<bool>()).prop_map(
            |(line, column, hit_count, is_executable)| StatementCoverage {
                line,
                column,
                hit_count,
                is_executable,
            },
        )
    }

    /// Generate arbitrary branch coverage
    fn arb_branch_coverage() -> impl Strategy<Value = BranchCoverage> {
        (0u32..100, 1u32..1000, 0u32..1000, 0u32..1000).prop_map(
            |(id, line, true_count, false_count)| BranchCoverage {
                id,
                line,
                branch_type: BranchType::IfElse,
                true_count,
                false_count,
            },
        )
    }

    /// Generate arbitrary function coverage
    fn arb_function_coverage() -> impl Strategy<Value = FunctionCoverage> {
        ("[a-zA-Z_][a-zA-Z0-9_]{0,20}", 1u32..500, 0u32..1000).prop_map(
            |(name, start_line, hit_count)| FunctionCoverage {
                name,
                start_line,
                end_line: start_line + 10,
                hit_count,
            },
        )
    }

    proptest! {
        /// Property 10: Coverage Accuracy - Statement coverage percentage is accurate
        /// Feature: dx-js-production-complete, Property 10: Coverage Accuracy
        /// Validates: Requirements 12.2, 12.3, 12.4
        #[test]
        fn prop_statement_coverage_accuracy(
            statements in prop::collection::vec(arb_statement_coverage(), 1..50)
        ) {
            let mut coverage = FileCoverage::new(PathBuf::from("test.js"));

            for (i, stmt) in statements.iter().enumerate() {
                coverage.statements.insert(i as u32, stmt.clone());
            }

            let executable_count = statements.iter().filter(|s| s.is_executable).count();
            let covered_count = statements.iter().filter(|s| s.is_executable && s.hit_count > 0).count();

            let expected = if executable_count == 0 {
                100.0
            } else {
                (covered_count as f64 / executable_count as f64) * 100.0
            };

            let actual = coverage.statement_coverage_percent();

            prop_assert!((actual - expected).abs() < 0.001,
                "Expected {:.3}%, got {:.3}%", expected, actual);
        }

        /// Property: Branch coverage percentage is accurate
        /// Both branches must be taken for full coverage
        #[test]
        fn prop_branch_coverage_accuracy(
            branches in prop::collection::vec(arb_branch_coverage(), 1..20)
        ) {
            let mut coverage = FileCoverage::new(PathBuf::from("test.js"));

            // Use unique IDs to avoid collisions
            for (i, mut branch) in branches.into_iter().enumerate() {
                branch.id = i as u32;
                coverage.branches.insert(branch.id, branch);
            }

            let fully_covered = coverage.branches.values()
                .filter(|b| b.true_count > 0 && b.false_count > 0)
                .count();

            let expected = if coverage.branches.is_empty() {
                100.0
            } else {
                (fully_covered as f64 / coverage.branches.len() as f64) * 100.0
            };

            let actual = coverage.branch_coverage_percent();

            prop_assert!((actual - expected).abs() < 0.001,
                "Expected {:.3}%, got {:.3}%", expected, actual);
        }

        /// Property: Function coverage percentage is accurate
        #[test]
        fn prop_function_coverage_accuracy(
            functions in prop::collection::vec(arb_function_coverage(), 1..20)
        ) {
            let mut coverage = FileCoverage::new(PathBuf::from("test.js"));

            // Use unique keys to avoid collisions (functions may have same generated name)
            for (i, func) in functions.iter().enumerate() {
                let unique_name = format!("{}_{}", func.name, i);
                let mut func_with_unique_name = func.clone();
                func_with_unique_name.name = unique_name.clone();
                coverage.functions.insert(unique_name, func_with_unique_name);
            }

            let covered = functions.iter().filter(|f| f.hit_count > 0).count();

            let expected = if functions.is_empty() {
                100.0
            } else {
                (covered as f64 / functions.len() as f64) * 100.0
            };

            let actual = coverage.function_coverage_percent();

            prop_assert!((actual - expected).abs() < 0.001,
                "Expected {:.3}%, got {:.3}%", expected, actual);
        }

        /// Property: Coverage merge is additive
        /// Merging two collectors should sum hit counts
        #[test]
        fn prop_coverage_merge_additive(
            hits1 in 0u32..1000,
            hits2 in 0u32..1000
        ) {
            let mut collector1 = CoverageCollector::new();
            let mut collector2 = CoverageCollector::new();

            let mut cov1 = FileCoverage::new(PathBuf::from("test.js"));
            cov1.statements.insert(1, StatementCoverage {
                line: 1,
                column: 0,
                hit_count: hits1,
                is_executable: true,
            });
            collector1.register_file(cov1);

            let mut cov2 = FileCoverage::new(PathBuf::from("test.js"));
            cov2.statements.insert(1, StatementCoverage {
                line: 1,
                column: 0,
                hit_count: hits2,
                is_executable: true,
            });
            collector2.register_file(cov2);

            collector1.merge(collector2);

            let merged = collector1.files.get(&PathBuf::from("test.js")).unwrap();
            let merged_hits = merged.statements.get(&1).unwrap().hit_count;

            prop_assert_eq!(merged_hits, hits1 + hits2);
        }

        /// Property: Threshold check is correct
        /// Coverage below threshold should fail, above should pass
        #[test]
        fn prop_threshold_check_correct(
            threshold in 0.0f64..100.0,
            actual in 0.0f64..100.0
        ) {
            let thresholds = CoverageThresholds {
                statements: threshold,
                branches: 0.0,
                functions: 0.0,
            };

            let summary = CoverageSummary {
                statements: actual,
                branches: 100.0,
                functions: 100.0,
                file_count: 1,
                files: vec![],
            };

            let result = thresholds.check(&summary);

            if actual >= threshold {
                prop_assert!(result.passed(),
                    "Should pass: actual {:.1}% >= threshold {:.1}%", actual, threshold);
            } else {
                prop_assert!(!result.passed(),
                    "Should fail: actual {:.1}% < threshold {:.1}%", actual, threshold);
            }
        }

        /// Property: Uncovered lines are accurate
        /// Lines with hit_count == 0 should be in uncovered list
        #[test]
        fn prop_uncovered_lines_accurate(
            statements in prop::collection::vec(arb_statement_coverage(), 1..50)
        ) {
            let mut coverage = FileCoverage::new(PathBuf::from("test.js"));

            for (i, stmt) in statements.iter().enumerate() {
                coverage.statements.insert(i as u32, stmt.clone());
            }

            let uncovered = coverage.uncovered_lines();

            // All uncovered lines should have hit_count == 0 and be executable
            for line in &uncovered {
                let stmt = coverage.statements.get(line).unwrap();
                prop_assert!(stmt.is_executable && stmt.hit_count == 0,
                    "Line {} should be uncovered", line);
            }

            // All executable lines with hit_count == 0 should be in uncovered
            for (line, stmt) in &coverage.statements {
                if stmt.is_executable && stmt.hit_count == 0 {
                    prop_assert!(uncovered.contains(line),
                        "Line {} should be in uncovered list", line);
                }
            }
        }
    }
}
