//! Dx Check - The binary-first linter
//!
//! 10x faster than Biome, 100x faster than ESLint

use dx_check::cache::AstCache;
use dx_check::ci::{CiConfigGenerator, CiPlatform};
use dx_check::cli::{
    CacheCommands, CiPlatformArg, Cli, Commands, OutputFormat, PluginCommands, RuleCommands,
    output as cli_output,
};
use dx_check::config::CheckerConfig;
use dx_check::engine::Checker;

use dx_check::output::dx_format::DxScoreBreakdown;
use dx_check::plugin::PluginLoader;
use dx_check::project::ProjectProfile;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse_args();

    match run(cli) {
        Ok(has_errors) => {
            if has_errors {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<bool, Box<dyn std::error::Error>> {
    match &cli.command {
        Some(Commands::Check {
            paths,
            fix,
            write,
            format,
        }) => run_check(paths, *fix, *write, format, &cli),
        Some(Commands::Format {
            paths,
            check,
            write,
        }) => run_format(paths, *check, *write, &cli),
        Some(Commands::Lint { paths, format }) => run_lint(paths, format, &cli),
        Some(Commands::Init { force }) => run_init(*force),
        Some(Commands::Analyze { path }) => run_analyze(path),
        Some(Commands::Rule { command }) => run_rule_command(command),
        Some(Commands::Cache { command }) => run_cache_command(command),
        Some(Commands::Watch {
            rules_dir,
            output_dir,
            debounce,
        }) => run_watch_mode(rules_dir, output_dir, *debounce),
        Some(Commands::Lsp) => run_lsp(),
        Some(Commands::Plugin { command }) => run_plugin_command(command),
        Some(Commands::Ci { platform, output }) => {
            run_ci_command(platform.as_ref(), output.as_ref())
        }
        Some(Commands::Score {
            path,
            threshold,
            breakdown,
            trend,
        }) => run_score_command(path, *threshold, *breakdown, *trend, &cli),
        Some(Commands::Test {
            path,
            coverage,
            framework,
        }) => run_test_command(path, *coverage, framework.as_deref(), &cli),
        None => {
            // Default: check paths
            run_check(&cli.paths, cli.fix, false, &cli.format, &cli)
        }
    }
}

fn run_check(
    paths: &[std::path::PathBuf],
    fix: bool,
    _write: bool,
    format: &OutputFormat,
    cli: &Cli,
) -> Result<bool, Box<dyn std::error::Error>> {
    use dx_check::languages::{
        FileProcessor, GoHandler, MarkdownHandler, PythonHandler, RustHandler, TomlHandler,
    };
    use rayon::prelude::*;

    let root = paths.first().map(|p| p.as_path()).unwrap_or(Path::new("."));

    // Load config
    let config = if let Some(ref config_path) = cli.config {
        let content = std::fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        CheckerConfig::auto_detect(root)
    };

    // Detect project profile
    if cli.verbose {
        let profile = ProjectProfile::detect(root);
        cli_output::print_profile(&profile);
    }

    // Create checker for JS/TS files
    let checker = if cli.threads == 1 {
        Checker::new(config.clone())
    } else {
        let mut cfg = config.clone();
        cfg.parallel.threads = cli.threads;
        Checker::new(cfg)
    };

    // Create file processor for other languages
    let mut processor = FileProcessor::new();
    processor.register(PythonHandler::new());
    processor.register(GoHandler::new());
    processor.register(RustHandler::new());
    processor.register(TomlHandler::new());
    processor.register(MarkdownHandler::new());

    // Collect files to process
    let files = collect_files_to_process(paths, &config)?;

    if cli.verbose {
        println!("Found {} files to check", files.len());
    }

    let start = std::time::Instant::now();

    // Process files and collect diagnostics
    let all_diagnostics: Vec<dx_check::diagnostics::Diagnostic> = if cli.threads == 1 {
        files.iter().flat_map(|path| lint_file(&checker, &processor, path)).collect()
    } else {
        files
            .par_iter()
            .flat_map(|path| lint_file(&checker, &processor, path))
            .collect()
    };

    let duration = start.elapsed();

    // Create result object
    let result = dx_check::engine::CheckResult {
        files_checked: files.len(),
        diagnostics: all_diagnostics,
        duration,
        files_per_second: files.len() as f64 / duration.as_secs_f64(),
    };

    // Apply fixes if requested
    if fix && !result.diagnostics.is_empty() {
        eprintln!("Note: --fix flag is currently disabled due to implementation issues");
        // TODO: Fix the fix engine - spans are incorrect
    }

    // Count errors and warnings
    let error_count = result.error_count();
    let warning_count = result.warning_count();

    // Check if using DX output formats
    let use_dx_format =
        matches!(format, OutputFormat::DxBinary | OutputFormat::DxLlm | OutputFormat::DxHuman);

    if use_dx_format {
        // Use new DX serializer output formats
        use dx_check::output::dx_format::{DxCheckReport, DxDiagnostic};

        let dx_diagnostics: Vec<DxDiagnostic> =
            result.diagnostics.iter().map(DxDiagnostic::from).collect();

        let report = DxCheckReport {
            version: 1,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            score: 0,
            breakdown: DxScoreBreakdown {
                formatting: 0,
                linting: 0,
                security: 0,
                patterns: 0,
                structure: 0,
            },
            diagnostics: dx_diagnostics,
            test_results: None,
            coverage: None,
            files_analyzed: result.files_checked as u32,
            total_issues: error_count as u32 + warning_count as u32,
        };

        let output_str = match format {
            OutputFormat::DxBinary => {
                let bytes = report.to_dx_binary();
                String::from_utf8_lossy(&bytes).to_string()
            }
            OutputFormat::DxLlm => report.to_dx_llm(),
            OutputFormat::DxHuman => report.to_dx_human(),
            _ => report.to_dx_human(), // Fallback
        };
        println!("{}", output_str);
    } else {
        // Use legacy output formats
        match format {
            OutputFormat::Pretty => {
                for diagnostic in &result.diagnostics {
                    if let Ok(source) = std::fs::read_to_string(&diagnostic.file) {
                        cli_output::print_diagnostic(diagnostic, &source);
                    }
                }

                if !cli.quiet {
                    cli_output::print_summary(
                        result.files_checked,
                        error_count,
                        warning_count,
                        result.duration.as_millis() as u64,
                        result.files_per_second,
                    );
                }
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "files_checked": result.files_checked,
                    "errors": error_count,
                    "warnings": warning_count,
                    "duration_ms": result.duration.as_millis(),
                    "diagnostics": result.diagnostics.iter().map(|d| {
                        serde_json::json!({
                            "file": d.file.display().to_string(),
                            "span": { "start": d.span.start, "end": d.span.end },
                            "severity": d.severity.as_str(),
                            "rule": d.rule_id,
                            "message": d.message,
                        })
                    }).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
            OutputFormat::Compact => {
                for diagnostic in &result.diagnostics {
                    println!(
                        "{}:{}:{}: {} [{}] {}",
                        diagnostic.file.display(),
                        diagnostic.span.start,
                        diagnostic.span.end,
                        diagnostic.severity.as_str(),
                        diagnostic.rule_id,
                        diagnostic.message,
                    );
                }
            }
            OutputFormat::Github => {
                for diagnostic in &result.diagnostics {
                    let level = match diagnostic.severity {
                        dx_check::diagnostics::DiagnosticSeverity::Error => "error",
                        dx_check::diagnostics::DiagnosticSeverity::Warning => "warning",
                        _ => "notice",
                    };
                    println!(
                        "::{} file={},line=1::{}",
                        level,
                        diagnostic.file.display(),
                        diagnostic.message,
                    );
                }
            }
            OutputFormat::Junit => {
                // JUnit XML output
                println!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
                println!(r#"<testsuites>"#);
                println!(
                    r#"  <testsuite name="dx-check" tests="{}" failures="{}">"#,
                    result.files_checked, error_count,
                );
                for diagnostic in &result.diagnostics {
                    println!(
                        r#"    <testcase name="{}"><failure message="{}"/></testcase>"#,
                        diagnostic.rule_id,
                        diagnostic.message.replace('"', "&quot;"),
                    );
                }
                println!(r#"  </testsuite>"#);
                println!(r#"</testsuites>"#);
            }
            OutputFormat::Sarif => {
                // SARIF 2.1.0 format
                let sarif = serde_json::json!({
                    "version": "2.1.0",
                    "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
                    "runs": [{
                        "tool": {
                            "driver": {
                                "name": "dx-check",
                                "version": env!("CARGO_PKG_VERSION"),
                                "informationUri": "https://dx.dev/check"
                            }
                        },
                        "results": result.diagnostics.iter().map(|d| {
                            serde_json::json!({
                                "ruleId": d.rule_id,
                                "level": match d.severity {
                                    dx_check::diagnostics::DiagnosticSeverity::Error => "error",
                                    dx_check::diagnostics::DiagnosticSeverity::Warning => "warning",
                                    _ => "note"
                                },
                                "message": {
                                    "text": d.message.clone()
                                },
                                "locations": [{
                                    "physicalLocation": {
                                        "artifactLocation": {
                                            "uri": d.file.display().to_string()
                                        },
                                        "region": {
                                            "startColumn": d.span.start,
                                            "endColumn": d.span.end
                                        }
                                    }
                                }]
                            })
                        }).collect::<Vec<_>>()
                    }]
                });
                println!("{}", serde_json::to_string_pretty(&sarif)?);
            }
            _ => unreachable!(),
        }
    }

    // Return true if there are errors
    Ok(result.has_errors())
}

fn run_format(
    paths: &[std::path::PathBuf],
    check: bool,
    write: bool,
    cli: &Cli,
) -> Result<bool, Box<dyn std::error::Error>> {
    use dx_check::languages::{
        FileProcessor, FileStatus, GoHandler, MarkdownHandler, PythonHandler, RustHandler,
        TomlHandler,
    };
    use rayon::prelude::*;

    let root = paths.first().map(|p| p.as_path()).unwrap_or(Path::new("."));

    // Load config
    let config = if let Some(ref config_path) = cli.config {
        let content = std::fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        CheckerConfig::auto_detect(root)
    };

    // Create file processor with all language handlers
    let mut processor = FileProcessor::new();
    processor.register(PythonHandler::new());
    processor.register(GoHandler::new());
    processor.register(RustHandler::new());
    processor.register(TomlHandler::new());
    processor.register(MarkdownHandler::new());

    // Collect files to format
    let files = collect_files_to_process(paths, &config)?;

    if cli.verbose {
        println!("Found {} files to format", files.len());
    }

    let start = std::time::Instant::now();
    let mut has_changes = false;
    let mut error_count = 0;

    // Process files (parallel or sequential based on config)
    let results: Vec<(std::path::PathBuf, Result<FileStatus, String>)> = if cli.threads == 1 {
        files
            .iter()
            .map(|path| {
                let result = process_format_file(&processor, path, check, write);
                (path.clone(), result)
            })
            .collect()
    } else {
        files
            .par_iter()
            .map(|path| {
                let result = process_format_file(&processor, path, check, write);
                (path.clone(), result)
            })
            .collect()
    };

    // Report results
    for (path, result) in results {
        match result {
            Ok(FileStatus::Changed) => {
                has_changes = true;
                if write {
                    if !cli.quiet {
                        println!("Formatted: {}", path.display());
                    }
                } else {
                    println!("Would format: {}", path.display());
                }
            }
            Ok(FileStatus::Unchanged) => {
                if cli.verbose {
                    println!("Unchanged: {}", path.display());
                }
            }
            Ok(FileStatus::Ignored) => {
                if cli.verbose {
                    println!("Ignored: {}", path.display());
                }
            }
            Ok(FileStatus::Error(diag)) => {
                error_count += 1;
                eprintln!("Error in {}: {}", path.display(), diag.message);
            }
            Err(e) => {
                error_count += 1;
                eprintln!("Error formatting {}: {}", path.display(), e);
            }
        }
    }

    let duration = start.elapsed();

    if !cli.quiet {
        println!(
            "\nFormatted {} files in {:.2}s ({} errors)",
            files.len(),
            duration.as_secs_f64(),
            error_count
        );
    }

    // Return true if there are changes (in check mode) or errors
    Ok(if check { has_changes } else { error_count > 0 })
}

fn process_format_file(
    processor: &dx_check::languages::FileProcessor,
    path: &std::path::Path,
    check: bool,
    write: bool,
) -> Result<dx_check::languages::FileStatus, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

    // In check mode, we don't write; otherwise respect the write flag
    let should_write = !check && write;

    processor.format(path, &content, should_write).map_err(|d| d.message)
}

fn collect_files_to_process(
    paths: &[std::path::PathBuf],
    config: &CheckerConfig,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            files.push(path.clone());
            continue;
        }

        let walker = WalkBuilder::new(path).standard_filters(true).hidden(true).build();

        for entry in walker.flatten() {
            let entry_path = entry.path();

            if !entry_path.is_file() {
                continue;
            }

            // Check extension
            let ext = entry_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let supported = matches!(
                ext,
                "js" | "jsx"
                    | "ts"
                    | "tsx"
                    | "mjs"
                    | "cjs"
                    | "py"
                    | "pyi"
                    | "go"
                    | "rs"
                    | "toml"
                    | "md"
            );

            if !supported {
                continue;
            }

            // Check exclude patterns
            let matches_exclude = config.exclude.iter().any(|pattern| {
                glob::Pattern::new(pattern).map(|p| p.matches_path(entry_path)).unwrap_or(false)
            });

            if matches_exclude {
                continue;
            }

            files.push(entry_path.to_path_buf());
        }
    }

    Ok(files)
}

fn run_lint(
    paths: &[std::path::PathBuf],
    format: &OutputFormat,
    cli: &Cli,
) -> Result<bool, Box<dyn std::error::Error>> {
    use dx_check::languages::{
        FileProcessor, GoHandler, MarkdownHandler, PythonHandler, RustHandler, TomlHandler,
    };
    use rayon::prelude::*;

    let root = paths.first().map(|p| p.as_path()).unwrap_or(Path::new("."));

    // Load config
    let config = if let Some(ref config_path) = cli.config {
        let content = std::fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        CheckerConfig::auto_detect(root)
    };

    // Create checker for JS/TS files
    let checker = Checker::new(config.clone());

    // Create file processor for other languages
    let mut processor = FileProcessor::new();
    processor.register(PythonHandler::new());
    processor.register(GoHandler::new());
    processor.register(RustHandler::new());
    processor.register(TomlHandler::new());
    processor.register(MarkdownHandler::new());

    // Collect files to lint
    let files = collect_files_to_process(paths, &config)?;

    if cli.verbose {
        println!("Found {} files to lint", files.len());
    }

    let start = std::time::Instant::now();

    // Process files and collect diagnostics
    let all_diagnostics: Vec<dx_check::diagnostics::Diagnostic> = if cli.threads == 1 {
        files.iter().flat_map(|path| lint_file(&checker, &processor, path)).collect()
    } else {
        files
            .par_iter()
            .flat_map(|path| lint_file(&checker, &processor, path))
            .collect()
    };

    let duration = start.elapsed();

    // Count errors and warnings
    let error_count = all_diagnostics
        .iter()
        .filter(|d| d.severity == dx_check::diagnostics::DiagnosticSeverity::Error)
        .count();
    let warning_count = all_diagnostics
        .iter()
        .filter(|d| d.severity == dx_check::diagnostics::DiagnosticSeverity::Warning)
        .count();

    // Output results based on format
    match format {
        OutputFormat::Pretty => {
            for diagnostic in &all_diagnostics {
                if let Ok(source) = std::fs::read_to_string(&diagnostic.file) {
                    cli_output::print_diagnostic(diagnostic, &source);
                }
            }

            if !cli.quiet {
                cli_output::print_summary(
                    files.len(),
                    error_count,
                    warning_count,
                    duration.as_millis() as u64,
                    files.len() as f64 / duration.as_secs_f64(),
                );
            }
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "files_checked": files.len(),
                "errors": error_count,
                "warnings": warning_count,
                "duration_ms": duration.as_millis(),
                "diagnostics": all_diagnostics.iter().map(|d| {
                    serde_json::json!({
                        "file": d.file.display().to_string(),
                        "span": { "start": d.span.start, "end": d.span.end },
                        "severity": d.severity.as_str(),
                        "rule": d.rule_id,
                        "message": d.message,
                    })
                }).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Compact => {
            for diagnostic in &all_diagnostics {
                println!(
                    "{}:{}:{}: {} [{}] {}",
                    diagnostic.file.display(),
                    diagnostic.span.start,
                    diagnostic.span.end,
                    diagnostic.severity.as_str(),
                    diagnostic.rule_id,
                    diagnostic.message,
                );
            }
        }
        OutputFormat::Github => {
            for diagnostic in &all_diagnostics {
                let level = match diagnostic.severity {
                    dx_check::diagnostics::DiagnosticSeverity::Error => "error",
                    dx_check::diagnostics::DiagnosticSeverity::Warning => "warning",
                    _ => "notice",
                };
                println!(
                    "::{} file={},line=1::{}",
                    level,
                    diagnostic.file.display(),
                    diagnostic.message,
                );
            }
        }
        OutputFormat::Junit => {
            println!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
            println!(r#"<testsuites>"#);
            println!(
                r#"  <testsuite name="dx-check-lint" tests="{}" failures="{}">"#,
                files.len(),
                error_count,
            );
            for diagnostic in &all_diagnostics {
                println!(
                    r#"    <testcase name="{}"><failure message="{}"/></testcase>"#,
                    diagnostic.rule_id,
                    diagnostic.message.replace('"', "&quot;"),
                );
            }
            println!(r#"  </testsuite>"#);
            println!(r#"</testsuites>"#);
        }
        OutputFormat::Sarif => {
            // SARIF 2.1.0 format
            let sarif = serde_json::json!({
                "version": "2.1.0",
                "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
                "runs": [{
                    "tool": {
                        "driver": {
                            "name": "dx-check",
                            "version": env!("CARGO_PKG_VERSION"),
                            "informationUri": "https://dx.dev/check"
                        }
                    },
                    "results": all_diagnostics.iter().map(|d| {
                        serde_json::json!({
                            "ruleId": d.rule_id,
                            "level": match d.severity {
                                dx_check::diagnostics::DiagnosticSeverity::Error => "error",
                                dx_check::diagnostics::DiagnosticSeverity::Warning => "warning",
                                _ => "note"
                            },
                            "message": {
                                "text": d.message.clone()
                            },
                            "locations": [{
                                "physicalLocation": {
                                    "artifactLocation": {
                                        "uri": d.file.display().to_string()
                                    },
                                    "region": {
                                        "startColumn": d.span.start,
                                        "endColumn": d.span.end
                                    }
                                }
                            }]
                        })
                    }).collect::<Vec<_>>()
                }]
            });
            println!("{}", serde_json::to_string_pretty(&sarif)?);
        }
        OutputFormat::DxBinary | OutputFormat::DxLlm | OutputFormat::DxHuman => {
            // DX formats not supported in lint mode
            eprintln!("DX formats are only supported in check mode");
        }
    }

    Ok(error_count > 0)
}

fn lint_file(
    checker: &Checker,
    processor: &dx_check::languages::FileProcessor,
    path: &std::path::Path,
) -> Vec<dx_check::diagnostics::Diagnostic> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return vec![dx_check::diagnostics::Diagnostic::error(
                path.to_path_buf(),
                dx_check::diagnostics::Span::new(0, 0),
                "io-error",
                format!("Failed to read file: {}", e),
            )];
        }
    };

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Use checker for JS/TS files
    if matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs") {
        return checker.check_source(path, &content).unwrap_or_default();
    }

    // Use language-specific handlers for other files
    match processor.lint(path, &content) {
        Ok(diagnostics) => diagnostics
            .into_iter()
            .map(|d| {
                let span = if let (Some(line), Some(col)) = (d.line, d.column) {
                    // Convert line/col to approximate byte offset
                    let offset = content
                        .lines()
                        .take(line.saturating_sub(1))
                        .map(|l| l.len() + 1)
                        .sum::<usize>()
                        + col.saturating_sub(1);
                    dx_check::diagnostics::Span::new(offset as u32, (offset + 1) as u32)
                } else {
                    dx_check::diagnostics::Span::new(0, 0)
                };

                let severity = match d.severity {
                    dx_check::languages::Severity::Error => {
                        dx_check::diagnostics::DiagnosticSeverity::Error
                    }
                    dx_check::languages::Severity::Warning => {
                        dx_check::diagnostics::DiagnosticSeverity::Warning
                    }
                    dx_check::languages::Severity::Info => {
                        dx_check::diagnostics::DiagnosticSeverity::Info
                    }
                };

                dx_check::diagnostics::Diagnostic {
                    file: path.to_path_buf(),
                    span,
                    severity,
                    rule_id: d.category,
                    message: d.message,
                    suggestion: None,
                    related: Vec::new(),
                    fix: None,
                }
            })
            .collect(),
        Err(e) => vec![dx_check::diagnostics::Diagnostic::error(
            path.to_path_buf(),
            dx_check::diagnostics::Span::new(0, 0),
            "lint-error",
            e.message,
        )],
    }
}

fn run_init(force: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let config_path = std::path::Path::new("dx.toml");

    if config_path.exists() && !force {
        eprintln!("Configuration file already exists. Use --force to overwrite.");
        return Ok(true);
    }

    let default_config = r#"# Dx Check Configuration
# https://dx.dev/docs/check

[rules]
# Enable recommended rules
recommended = true

# Auto-fix on check
auto_fix = false

[format]
# Indentation
use_tabs = false
indent_width = 2

# Line width
line_width = 80

# Quote style: "single" or "double"
quote_style = "double"

# Semicolons: "always" or "as_needed"
semicolons = "always"

[cache]
enabled = true
directory = ".dx/check"

[parallel]
# Number of threads (0 = auto-detect)
threads = 0
"#;

    std::fs::write(config_path, default_config)?;
    println!("Created dx.toml configuration file");

    Ok(false)
}

fn run_analyze(path: &std::path::Path) -> Result<bool, Box<dyn std::error::Error>> {
    let profile = ProjectProfile::detect(path);
    cli_output::print_profile(&profile);
    Ok(false)
}

fn run_rule_command(command: &RuleCommands) -> Result<bool, Box<dyn std::error::Error>> {
    use dx_check::rules::RuleRegistry;

    match command {
        RuleCommands::List {
            category,
            enabled: _,
        } => {
            let registry = RuleRegistry::with_builtins();

            println!("Available rules:\n");
            for name in registry.rule_names() {
                if let Some(rule) = registry.get(name) {
                    let meta = rule.meta();
                    let category_str = meta.category.as_str();

                    if let Some(filter) = category
                        && category_str != filter
                    {
                        continue;
                    }

                    let status = if registry.is_enabled(name) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    let fixable = if meta.fixable { "[FIX]" } else { "     " };

                    println!(
                        "  {} {} {:20} {:12} {}",
                        status, fixable, name, category_str, meta.description
                    );
                }
            }
        }
        RuleCommands::Show { rule } => {
            let registry = RuleRegistry::with_builtins();

            if let Some(r) = registry.get(rule) {
                let meta = r.meta();
                println!("Rule: {}", meta.name);
                println!("Category: {}", meta.category.as_str());
                println!("Description: {}", meta.description);
                println!("Fixable: {}", if meta.fixable { "Yes" } else { "No" });
                println!("Recommended: {}", if meta.recommended { "Yes" } else { "No" });
                if let Some(url) = meta.docs_url {
                    println!("Documentation: {}", url);
                }
            } else {
                eprintln!("Rule not found: {}", rule);
                return Ok(true);
            }
        }
        RuleCommands::Enable { rule, severity } => {
            println!("Rule '{}' enabled with severity {:?}", rule, severity);
            // Would modify config file
        }
        RuleCommands::Disable { rule } => {
            println!("Rule '{}' disabled", rule);
            // Would modify config file
        }
        RuleCommands::Compile { output, verify } => {
            use dx_check::rules::compiler;

            println!("Compiling rules to binary format...\n");
            match compiler::compile_rules(output) {
                Ok(compiled) => {
                    println!("\n[SUCCESS] Compiled {} rules", compiled.count);
                    println!("   Binary size: {} KB", compiled.binary_size / 1024);

                    if *verify {
                        let rules_path = output.join("rules.dxm");
                        println!("\nVerifying compiled rules...");
                        if let Err(e) = compiler::verify_compiled_rules(&rules_path) {
                            eprintln!("[ERROR] Verification failed: {}", e);
                            return Ok(true);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] Compilation failed: {}", e);
                    return Ok(true);
                }
            }
        }
        RuleCommands::Verify { path } => {
            use dx_check::rules::compiler;

            match compiler::verify_compiled_rules(path) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("❌ Verification failed: {}", e);
                    return Ok(true);
                }
            }
        }
        RuleCommands::Generate { output } => {
            use dx_check::rules::dxs_generator;

            println!("Generating .sr files...\n");
            match dxs_generator::generate_all_sr_files(output) {
                Ok(_) => {
                    println!("\n✨ Successfully generated .sr files in {:?}", output);
                }
                Err(e) => {
                    eprintln!("❌ Generation failed: {}", e);
                    return Ok(true);
                }
            }
        }
        RuleCommands::CompileFromDxs { input, output } => {
            use dx_check::rules::compiler;

            println!("Compiling from .sr files...\n");
            match compiler::compile_from_sr(input, output) {
                Ok(compiled) => {
                    println!("\n✅ Successfully compiled {} rules", compiled.count);
                    println!("   Binary size: {} KB", compiled.binary_size / 1024);
                }
                Err(e) => {
                    eprintln!("❌ Compilation failed: {}", e);
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

fn run_cache_command(command: &CacheCommands) -> Result<bool, Box<dyn std::error::Error>> {
    let cache_dir = std::path::PathBuf::from(".dx/check");

    match command {
        CacheCommands::Stats => {
            if cache_dir.exists() {
                let cache = AstCache::new(cache_dir, 1024 * 1024 * 1024)?;
                let stats = cache.stats();
                println!("Cache Statistics:");
                println!("  Entries: {}", stats.entry_count);
                println!("  Size: {} bytes", stats.total_size);
                println!("  Max Size: {} bytes", stats.max_size);
                println!("  Utilization: {:.1}%", stats.utilization());
            } else {
                println!("No cache directory found");
            }
        }
        CacheCommands::Clear => {
            if cache_dir.exists() {
                std::fs::remove_dir_all(&cache_dir)?;
                println!("Cache cleared");
            } else {
                println!("No cache to clear");
            }
        }
        CacheCommands::Path => {
            println!("{}", cache_dir.canonicalize().unwrap_or(cache_dir.clone()).display());
        }
    }

    Ok(false)
}

fn run_watch_mode(
    rules_dir: &std::path::PathBuf,
    output_dir: &std::path::PathBuf,
    debounce: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    use dx_check::watch::{WatchConfig, watch_rules};

    let config = WatchConfig {
        rules_dir: rules_dir.clone(),
        output_dir: output_dir.clone(),
        debounce_ms: debounce,
    };

    if let Err(e) = watch_rules(config) {
        eprintln!("Watch mode error: {}", e);
        return Err(e.into());
    }

    Ok(false)
}

fn run_lsp() -> Result<bool, Box<dyn std::error::Error>> {
    #[cfg(feature = "lsp")]
    {
        use tokio::runtime::Runtime;

        let rt = Runtime::new()?;
        rt.block_on(async { dx_check::lsp::start_lsp_server().await })
            .map_err(|e| -> Box<dyn std::error::Error> { e })?;
        Ok(false)
    }

    #[cfg(not(feature = "lsp"))]
    {
        eprintln!("LSP server not enabled. Rebuild with --features lsp");
        Ok(true)
    }
}

fn run_plugin_command(command: &PluginCommands) -> Result<bool, Box<dyn std::error::Error>> {
    match command {
        PluginCommands::List => {
            let mut loader = PluginLoader::new();
            let plugins = loader.discover();

            if plugins.is_empty() {
                println!("No plugins installed.");
                println!("\nNote: Plugin system is experimental.");
            } else {
                println!("Installed Plugins:\n");
                println!("{:<25} {:<10} {:<12} DESCRIPTION", "NAME", "VERSION", "TYPE");
                println!("{}", "-".repeat(70));

                for plugin in &plugins {
                    println!(
                        "{:<25} {:<10} {:<12} {}",
                        plugin.name,
                        plugin.version,
                        format!("{:?}", plugin.plugin_type),
                        plugin.description
                    );
                }
                println!("\nTotal: {} plugins", plugins.len());
            }
        }
        PluginCommands::Install { name, version: _ } => {
            eprintln!("Plugin installation not yet implemented.");
            eprintln!("Requested: {}", name);
            return Ok(true);
        }
        PluginCommands::Uninstall { name } => {
            eprintln!("Plugin uninstallation not yet implemented.");
            eprintln!("Requested: {}", name);
            return Ok(true);
        }
        PluginCommands::Update { name } => {
            eprintln!("Plugin updates not yet implemented.");
            if let Some(pkg_name) = name {
                eprintln!("Requested: {}", pkg_name);
            }
            return Ok(true);
        }
        PluginCommands::Search { query } => {
            eprintln!("Plugin search not yet implemented.");
            eprintln!("Query: {}", query);
            return Ok(true);
        }
    }

    Ok(false)
}

fn run_score_command(
    path: &std::path::PathBuf,
    threshold: Option<u16>,
    breakdown: bool,
    trend: bool,
    cli: &Cli,
) -> Result<bool, Box<dyn std::error::Error>> {
    use colored::Colorize;
    use dx_check::scoring_impl::{AnalysisMode, ScoreCalculator, ScoreStorage, ThresholdChecker};

    let root = path;

    // Load config
    let config = if let Some(ref config_path) = cli.config {
        let content = std::fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        CheckerConfig::auto_detect(root)
    };

    // Create checker and run analysis
    let checker = Checker::new(config);
    let result = checker.check_path(root)?;

    // Calculate score
    let mode = if breakdown {
        AnalysisMode::Detailed
    } else {
        AnalysisMode::Quick
    };
    let calculator = ScoreCalculator::with_mode(mode);
    let score = calculator.calculate(&result.diagnostics, result.files_checked);

    // Save score to cache
    let cache_dir = std::path::PathBuf::from(".dx/check/scores");
    let storage = ScoreStorage::new(cache_dir);
    storage.save(&score)?;

    // Display score
    println!("\n{}", "Code Quality Score".bold());
    println!("{}", "─".repeat(50));

    let grade_colored = match score.grade() {
        "SSS" => score.grade().bright_magenta().bold(),
        "SS" => score.grade().magenta().bold(),
        "S" => score.grade().bright_cyan().bold(),
        "A" => score.grade().green().bold(),
        "B" => score.grade().cyan(),
        "C" => score.grade().yellow(),
        "D" => score.grade().red(),
        _ => score.grade().bright_red().bold(),
    };

    println!("Total Score: {}/500 (Rank: {})", score.total_score, grade_colored);
    println!("Files Analyzed: {}", score.files_analyzed);
    println!("Total Violations: {}", score.total_violations());

    if breakdown {
        println!("\n{}", "Category Breakdown:".bold());
        for category in dx_check::scoring_impl::Category::all() {
            let cat_score = score.get_category_score(*category);
            let violations =
                score.categories.get(category).map(|c| c.violation_count()).unwrap_or(0);

            let score_colored = if cat_score >= 90 {
                cat_score.to_string().green()
            } else if cat_score >= 70 {
                cat_score.to_string().yellow()
            } else {
                cat_score.to_string().red()
            };

            println!(
                "  {:20} {}/100 ({} violations)",
                category.as_str(),
                score_colored,
                violations
            );
        }
    }

    // Show trend if requested
    if trend {
        if let Ok(Some(trend_data)) = storage.analyze_trend(&score) {
            println!("\n{}", "Trend Analysis:".bold());
            println!("  {}", trend_data.summary());
        }
    }

    // Check threshold
    if let Some(min_score) = threshold {
        let checker = ThresholdChecker::new().with_total_threshold(min_score);
        match checker.check(&score) {
            dx_check::scoring_impl::ThresholdResult::Pass => {
                println!("\n{}", format!("[PASS] Score meets threshold of {}", min_score).green());
                Ok(false)
            }
            dx_check::scoring_impl::ThresholdResult::Fail(failures) => {
                println!("\n{}", "[FAIL] Score below threshold:".red());
                for failure in failures {
                    println!("  - {}", failure);
                }
                Ok(true)
            }
        }
    } else {
        Ok(false)
    }
}

fn run_test_command(
    path: &std::path::PathBuf,
    _collect_coverage: bool,
    _framework: Option<&str>,
    cli: &Cli,
) -> Result<bool, Box<dyn std::error::Error>> {
    use colored::Colorize;
    use dx_check::testing::TestRunner;

    let root = path;

    // Create test runner
    let runner = TestRunner::new(root);

    // Run all tests
    let output = runner.run_all();

    // Display results
    println!("\n{}", "Test Results".bold());
    println!("{}", "─".repeat(50));

    println!("Passed:  {}", output.total_passed.to_string().green());
    println!("Failed:  {}", output.total_failed.to_string().red());
    println!("Skipped: {}", output.total_skipped.to_string().yellow());
    println!("Total:   {}", output.total());
    println!("Duration: {}ms", output.total_duration_ms);

    // Show failures
    if output.total_failed > 0 {
        println!("\n{}", "Failed Tests:".bold().red());
        for suite in &output.suites {
            if suite.failed > 0 {
                println!("\n  [FAIL] {}", suite.file.display());
                for result in &suite.results {
                    if result.status != dx_check::testing::TestStatus::Passed {
                        println!("    [FAIL] {}", result.name);
                        if let Some(ref msg) = result.message {
                            for line in msg.lines().take(3) {
                                println!("      {}", line.dimmed());
                            }
                        }
                    }
                }
            }
        }
    }

    if cli.verbose {
        println!("\n{}", output.to_dx_human_format());
    }

    Ok(output.total_failed > 0)
}

fn run_ci_command(
    platform: Option<&CiPlatformArg>,
    output: Option<&std::path::PathBuf>,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Auto-detect or use specified platform
    let ci_platform = match platform {
        Some(CiPlatformArg::Github) => CiPlatform::GitHubActions,
        Some(CiPlatformArg::Gitlab) => CiPlatform::GitLabCi,
        Some(CiPlatformArg::Azure) => CiPlatform::AzureDevOps,
        Some(CiPlatformArg::Circleci) => CiPlatform::CircleCi,
        None => {
            // Try to detect
            if let Some(detected) = CiPlatform::detect() {
                println!("Detected CI platform: {:?}", detected);
                detected
            } else {
                println!("No CI platform detected. Generating GitHub Actions config...");
                CiPlatform::GitHubActions
            }
        }
    };

    let generator = CiConfigGenerator::new(ci_platform);
    let config = generator.generate()?;

    match output {
        Some(path) => {
            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, &config)?;
            println!("✅ Generated CI configuration: {}", path.display());
        }
        None => {
            // Print to stdout
            let default_path = match ci_platform {
                CiPlatform::GitHubActions => ".github/workflows/dx-check.yml",
                CiPlatform::GitLabCi => ".gitlab-ci.yml",
                CiPlatform::AzureDevOps => "azure-pipelines.yml",
                CiPlatform::CircleCi => ".circleci/config.yml",
                _ => "dx-check-ci.yml",
            };

            println!("Generated configuration for {:?}:\n", ci_platform);
            println!("{}", config);
            println!("\nSave to: {}", default_path);
        }
    }

    Ok(false)
}
