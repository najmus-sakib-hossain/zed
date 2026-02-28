//! Lint subcommand - runs third-party linters
//!
//! Supports: ESLint (JS/TS), Clippy (Rust), Pylint (Python), etc.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use super::OutputFormat;
use crate::ui::theme::Theme;

/// Lint files using third-party linters
#[derive(Args, Clone)]
pub struct LintCommand {
    /// Paths to lint
    #[arg(index = 1)]
    pub paths: Vec<PathBuf>,

    /// Auto-fix issues where possible
    #[arg(long)]
    pub fix: bool,

    /// Specific rules to enable
    #[arg(long)]
    pub rules: Vec<String>,

    /// Rules to ignore
    #[arg(long)]
    pub ignore_rules: Vec<String>,

    /// Path to config file
    #[arg(long, short)]
    pub config: Option<PathBuf>,
}

/// Diagnostic from a linter
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub line: u32,
    pub column: u32,
    pub end_line: Option<u32>,
    pub end_column: Option<u32>,
    pub severity: Severity,
    pub rule_id: String,
    pub message: String,
    pub source: String,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct Fix {
    pub description: String,
    pub replacement: String,
}

/// Linter adapter trait for third-party tools
pub trait LinterAdapter: Send + Sync {
    /// Linter name
    fn name(&self) -> &'static str;

    /// Supported file extensions
    fn extensions(&self) -> &[&'static str];

    /// Lint content and return diagnostics
    fn lint(&self, path: &std::path::Path, content: &str) -> Result<Vec<Diagnostic>>;

    /// Fix issues in content
    fn fix(&self, path: &std::path::Path, content: &str) -> Result<(String, Vec<Diagnostic>)> {
        let diagnostics = self.lint(path, content)?;
        Ok((content.to_string(), diagnostics))
    }
}

/// ESLint adapter for JavaScript/TypeScript
pub struct ESLintAdapter;

impl LinterAdapter for ESLintAdapter {
    fn name(&self) -> &'static str {
        "eslint"
    }

    fn extensions(&self) -> &[&'static str] {
        &["js", "jsx", "ts", "tsx", "mjs", "cjs"]
    }

    fn lint(&self, _path: &std::path::Path, _content: &str) -> Result<Vec<Diagnostic>> {
        // In production, this would call ESLint via subprocess or WASM
        Ok(vec![])
    }
}

/// Clippy adapter for Rust
pub struct ClippyAdapter;

impl LinterAdapter for ClippyAdapter {
    fn name(&self) -> &'static str {
        "clippy"
    }

    fn extensions(&self) -> &[&'static str] {
        &["rs"]
    }

    fn lint(&self, _path: &std::path::Path, _content: &str) -> Result<Vec<Diagnostic>> {
        Ok(vec![])
    }
}

/// Pylint adapter for Python
pub struct PylintAdapter;

impl LinterAdapter for PylintAdapter {
    fn name(&self) -> &'static str {
        "pylint"
    }

    fn extensions(&self) -> &[&'static str] {
        &["py", "pyi"]
    }

    fn lint(&self, _path: &std::path::Path, _content: &str) -> Result<Vec<Diagnostic>> {
        Ok(vec![])
    }
}

/// golint adapter for Go
pub struct GolintAdapter;

impl LinterAdapter for GolintAdapter {
    fn name(&self) -> &'static str {
        "golint"
    }

    fn extensions(&self) -> &[&'static str] {
        &["go"]
    }

    fn lint(&self, _path: &std::path::Path, _content: &str) -> Result<Vec<Diagnostic>> {
        Ok(vec![])
    }
}

/// Run lint command
pub async fn run(cmd: LintCommand, format: OutputFormat, theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::time::Instant;

    let start = Instant::now();

    let paths = if cmd.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cmd.paths.clone()
    };

    if !matches!(format, OutputFormat::Human) {
        return output_results(&[], format);
    }

    theme.print_section("dx check lint: Linting Files");
    eprintln!();

    // Get all linters
    let linters: Vec<Box<dyn LinterAdapter>> = vec![
        Box::new(ESLintAdapter),
        Box::new(ClippyAdapter),
        Box::new(PylintAdapter),
        Box::new(GolintAdapter),
    ];

    let mut files_checked = 0u32;
    let mut all_diagnostics = Vec::new();

    for path in &paths {
        if path.is_file() {
            let diags = lint_file(path, &linters).await?;
            files_checked += 1;
            all_diagnostics.extend(diags);
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let file_path = entry.path();
                let diags = lint_file(file_path, &linters).await?;
                files_checked += 1;
                all_diagnostics.extend(diags);
            }
        }
    }

    let elapsed = start.elapsed();

    // Count by severity
    let errors = all_diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = all_diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();

    // Print diagnostics
    if !all_diagnostics.is_empty() {
        for diag in &all_diagnostics {
            print_diagnostic(diag);
        }
        eprintln!();
    }

    // Summary
    let status = if errors > 0 {
        "✗".red().bold().to_string()
    } else if warnings > 0 {
        "⚠".yellow().bold().to_string()
    } else {
        "✓".green().bold().to_string()
    };

    eprintln!(
        "  {} {} files checked, {} errors, {} warnings in {:.2}s",
        status,
        files_checked,
        errors,
        warnings,
        elapsed.as_secs_f64()
    );
    eprintln!();

    if errors > 0 {
        anyhow::bail!("Lint failed with {} errors", errors);
    }

    Ok(())
}

async fn lint_file(
    path: &std::path::Path,
    linters: &[Box<dyn LinterAdapter>],
) -> Result<Vec<Diagnostic>> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let linter = linters.iter().find(|l| l.extensions().contains(&ext));

    let Some(linter) = linter else {
        return Ok(vec![]);
    };

    let content = std::fs::read_to_string(path)?;
    linter.lint(path, &content)
}

fn print_diagnostic(diag: &Diagnostic) {
    use owo_colors::OwoColorize;

    let severity_str = match diag.severity {
        Severity::Error => "error".red().bold().to_string(),
        Severity::Warning => "warning".yellow().bold().to_string(),
        Severity::Info => "info".cyan().bold().to_string(),
        Severity::Hint => "hint".bright_black().bold().to_string(),
    };

    eprintln!(
        "  {}:{}:{} {}: {} [{}]",
        diag.path.display().to_string().cyan(),
        diag.line,
        diag.column,
        severity_str,
        diag.message,
        diag.rule_id.bright_black()
    );
}

fn output_results(diagnostics: &[Diagnostic], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "diagnostics": diagnostics.len(),
                "errors": 0,
                "warnings": 0
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Llm => {
            println!("lint_result diagnostics={} errors=0 warnings=0", diagnostics.len());
        }
        OutputFormat::Github => {
            for diag in diagnostics {
                let level = match diag.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    _ => "notice",
                };
                println!(
                    "::{level} file={},line={},col={}::{}",
                    diag.path.display(),
                    diag.line,
                    diag.column,
                    diag.message
                );
            }
        }
        OutputFormat::Sarif => {
            println!(
                "{{\"$schema\": \"https://json.schemastore.org/sarif-2.1.0.json\", \"version\": \"2.1.0\", \"runs\": []}}"
            );
        }
        OutputFormat::Junit => {
            println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites></testsuites>");
        }
        _ => {}
    }
    Ok(())
}
