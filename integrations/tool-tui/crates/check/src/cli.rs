//! CLI Module
//!
//! Command-line interface for dx-check.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Dx Check - The binary-first linter
#[derive(Parser, Debug)]
#[command(
    name = "dx-check",
    version,
    about = "The binary-first linter - 10x faster than Biome",
    long_about = "Dx Check is a high-performance code linter and formatter built in Rust.\n\
                  It uses binary protocols and SIMD acceleration to achieve unprecedented speed."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Files or directories to check
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Apply safe fixes automatically
    #[arg(short, long)]
    pub fix: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "pretty")]
    pub format: OutputFormat,

    /// Number of threads (0 = auto)
    #[arg(short, long, default_value = "0")]
    pub threads: usize,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress all output except errors
    #[arg(short, long)]
    pub quiet: bool,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Disable caching
    #[arg(long)]
    pub no_cache: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check files for issues
    Check {
        /// Files or directories to check
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Apply safe fixes automatically
        #[arg(short, long)]
        fix: bool,

        /// Write changes to files (for multi-language support)
        #[arg(short, long)]
        write: bool,

        /// Output format
        #[arg(short = 'o', long = "format", value_enum, default_value = "pretty")]
        format: OutputFormat,
    },

    /// Format files
    Format {
        /// Files or directories to format
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Check if files are formatted (don't modify)
        #[arg(long)]
        check: bool,

        /// Write changes to files
        #[arg(short, long)]
        write: bool,
    },

    /// Lint files for issues (multi-language support)
    Lint {
        /// Files or directories to lint
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "pretty")]
        format: OutputFormat,
    },

    /// Initialize configuration
    Init {
        /// Overwrite existing configuration
        #[arg(short, long)]
        force: bool,
    },

    /// Show project analysis
    Analyze {
        /// Directory to analyze
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Manage rules
    Rule {
        #[command(subcommand)]
        command: RuleCommands,
    },

    /// Show cache information
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Run in watch mode
    Watch {
        /// Directory containing .sr files
        #[arg(long, default_value = "rules")]
        rules_dir: PathBuf,

        /// Output directory for compiled rules
        #[arg(long, default_value = "rules")]
        output_dir: PathBuf,

        /// Debounce delay in milliseconds
        #[arg(long, default_value = "250")]
        debounce: u64,
    },

    /// Start LSP server
    Lsp,

    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },

    /// Generate CI configuration
    Ci {
        /// CI platform
        #[arg(short, long, value_enum)]
        platform: Option<CiPlatformArg>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Score code quality (0-500 point system)
    Score {
        /// Directory to score
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Minimum threshold to pass (0-500)
        #[arg(short, long)]
        threshold: Option<u16>,

        /// Show detailed breakdown by category
        #[arg(short, long)]
        breakdown: bool,

        /// Compare with previous score
        #[arg(long)]
        trend: bool,
    },

    /// Run tests with framework auto-detection
    Test {
        /// Test pattern or path
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Collect code coverage
        #[arg(short, long)]
        coverage: bool,

        /// Test framework to use (auto-detected if not specified)
        #[arg(short, long)]
        framework: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    /// List installed plugins
    List,

    /// Install a plugin
    Install {
        /// Plugin name or path
        name: String,

        /// Plugin version
        #[arg(short, long)]
        version: Option<String>,
    },

    /// Uninstall a plugin
    Uninstall {
        /// Plugin name
        name: String,
    },

    /// Update plugins
    Update {
        /// Specific plugin to update (updates all if not specified)
        name: Option<String>,
    },

    /// Search for plugins
    Search {
        /// Search query
        query: String,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum CiPlatformArg {
    /// GitHub Actions
    Github,
    /// GitLab CI
    Gitlab,
    /// Azure DevOps Pipelines
    Azure,
    /// `CircleCI`
    Circleci,
}

#[derive(Subcommand, Debug)]
pub enum RuleCommands {
    /// List all available rules
    List {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Show only enabled rules
        #[arg(long)]
        enabled: bool,
    },

    /// Show rule details
    Show {
        /// Rule name
        rule: String,
    },

    /// Enable a rule
    Enable {
        /// Rule name
        rule: String,

        /// Severity level
        #[arg(short, long, value_enum, default_value = "warn")]
        severity: SeverityArg,
    },

    /// Disable a rule
    Disable {
        /// Rule name
        rule: String,
    },

    /// Compile rules to binary format
    Compile {
        /// Output directory for compiled rules
        #[arg(short, long, default_value = "rules")]
        output: PathBuf,

        /// Verify the compiled rules after generation
        #[arg(short, long)]
        verify: bool,
    },

    /// Verify compiled rules file
    Verify {
        /// Path to compiled rules file
        #[arg(default_value = "rules/rules.dxm")]
        path: PathBuf,
    },

    /// Generate .sr files from extracted rules
    Generate {
        /// Output directory for .sr files
        #[arg(short, long, default_value = "rules")]
        output: PathBuf,
    },

    /// Compile from .sr files
    CompileFromDxs {
        /// Directory containing .sr files
        #[arg(short, long, default_value = "rules")]
        input: PathBuf,

        /// Output directory for compiled rules
        #[arg(short, long, default_value = "rules")]
        output: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// Show cache statistics
    Stats,

    /// Clear the cache
    Clear,

    /// Show cache directory path
    Path,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Human-readable with colors
    Pretty,
    /// Compact single-line format
    Compact,
    /// JSON output
    Json,
    /// GitHub Actions format
    Github,
    /// `JUnit` XML format
    Junit,
    /// SARIF format
    Sarif,
    /// DX Serializer binary format (base64)
    DxBinary,
    /// DX Serializer LLM format (50-70% token savings)
    DxLlm,
    /// DX Serializer human format (beautiful)
    DxHuman,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum SeverityArg {
    Off,
    Warn,
    Error,
}

impl Cli {
    /// Parse CLI arguments
    #[must_use]
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

/// Terminal output helpers
pub mod output {
    use crate::diagnostics::{Diagnostic, DiagnosticSeverity, LineIndex};
    use colored::Colorize;

    /// Print a diagnostic with context
    pub fn print_diagnostic(diagnostic: &Diagnostic, source: &str) {
        let line_index = LineIndex::new(source);
        let (start_lc, end_lc) = diagnostic.span.to_line_col(&line_index);

        // Header
        let severity_str = match diagnostic.severity {
            DiagnosticSeverity::Error => "error".red().bold(),
            DiagnosticSeverity::Warning => "warning".yellow().bold(),
            DiagnosticSeverity::Info => "info".blue().bold(),
            DiagnosticSeverity::Hint => "hint".cyan().bold(),
        };

        println!(
            "{}{}{} {}",
            severity_str,
            "[".dimmed(),
            diagnostic.rule_id.dimmed(),
            "]".dimmed(),
        );
        println!("  {} {}", "-->".blue(), diagnostic.file.display());
        println!();

        // Source context
        let lines: Vec<&str> = source.lines().collect();
        let line_num = start_lc.line as usize;

        if line_num > 0 && line_num <= lines.len() {
            let line = lines[line_num - 1];
            let line_num_str = format!("{line_num:4}");

            println!("  {} {}", line_num_str.blue(), "|".blue());
            println!("  {} {} {}", line_num_str.blue(), "|".blue(), line);

            // Underline
            let start_col = (start_lc.col - 1) as usize;
            let end_col = if start_lc.line == end_lc.line {
                (end_lc.col - 1) as usize
            } else {
                line.len()
            };
            let underline_len = (end_col - start_col).max(1);

            let underline = format!("{}{}", " ".repeat(start_col), "^".repeat(underline_len));

            let underline_colored = match diagnostic.severity {
                DiagnosticSeverity::Error => underline.red(),
                DiagnosticSeverity::Warning => underline.yellow(),
                _ => underline.cyan(),
            };

            println!("  {} {} {}", "    ".blue(), "|".blue(), underline_colored);
        }

        // Message
        println!("  {} {}", "=".blue(), diagnostic.message);

        // Suggestion
        if let Some(ref suggestion) = diagnostic.suggestion {
            println!("  {} {}: {}", "=".blue(), "help".green(), suggestion);
        }

        println!();
    }

    /// Print summary
    pub fn print_summary(
        files_checked: usize,
        errors: usize,
        warnings: usize,
        duration_ms: u64,
        files_per_second: f64,
    ) {
        println!();

        if errors == 0 && warnings == 0 {
            println!(
                "{} {} files checked in {}ms ({:.0} files/sec)",
                "✓".green().bold(),
                files_checked,
                duration_ms,
                files_per_second
            );
        } else {
            let error_str = if errors > 0 {
                format!("{errors} errors").red().bold().to_string()
            } else {
                String::new()
            };

            let warning_str = if warnings > 0 {
                format!("{warnings} warnings").yellow().bold().to_string()
            } else {
                String::new()
            };

            let separator = if errors > 0 && warnings > 0 { ", " } else { "" };

            println!(
                "{} {} files checked: {}{}{} ({}ms, {:.0} files/sec)",
                "✗".red().bold(),
                files_checked,
                error_str,
                separator,
                warning_str,
                duration_ms,
                files_per_second
            );
        }
    }

    /// Print project profile
    pub fn print_profile(profile: &crate::project::ProjectProfile) {
        println!("{}", "🔍 Project Analysis".bold());
        println!("{}", "─".repeat(40));

        if !profile.frameworks.is_empty() {
            let frameworks: Vec<_> = profile
                .frameworks
                .iter()
                .map(super::super::project::Framework::as_str)
                .collect();
            println!("  {}: {}", "Frameworks".cyan(), frameworks.join(", "));
        }

        println!(
            "  {}: {}",
            "Language".cyan(),
            match profile.language {
                crate::project::Language::JavaScript => "JavaScript",
                crate::project::Language::TypeScript => "TypeScript",
            }
        );

        if let Some(ref test) = profile.test_framework {
            println!("  {}: {:?}", "Test Runner".cyan(), test);
        }

        if let Some(ref mono) = profile.monorepo {
            println!("  {}: {:?} ({} packages)", "Monorepo".cyan(), mono.kind, mono.packages.len());
        }

        println!("  {}: {:?}", "Package Manager".cyan(), profile.package_manager);

        println!();
        println!("{}", "📐 Inferred Style".bold());
        println!("{}", "─".repeat(40));
        println!(
            "  {}: {}",
            "Semicolons".cyan(),
            if profile.style.semicolons {
                "Yes"
            } else {
                "No"
            }
        );
        println!("  {}: {:?}", "Quotes".cyan(), profile.style.quotes);
        println!("  {}: {:?}", "Indentation".cyan(), profile.style.indent);
        println!();
    }
}
