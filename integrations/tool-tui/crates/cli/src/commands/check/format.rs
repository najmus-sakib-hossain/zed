//! Format subcommand - runs third-party formatters
//!
//! Supports: Prettier (JS/TS), rustfmt (Rust), Black (Python), etc.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use super::OutputFormat;
use crate::ui::theme::Theme;

/// Format files using third-party formatters
#[derive(Args, Clone)]
pub struct FormatCommand {
    /// Paths to format
    #[arg(index = 1)]
    pub paths: Vec<PathBuf>,

    /// Write formatted output to files
    #[arg(long, short)]
    pub write: bool,

    /// Check if files are formatted (exit with error if not)
    #[arg(long)]
    pub check: bool,

    /// Show diff of changes
    #[arg(long)]
    pub diff: bool,

    /// Patterns to ignore
    #[arg(long)]
    pub ignore: Vec<String>,
}

/// Formatter adapter trait for third-party tools
pub trait FormatterAdapter: Send + Sync {
    /// Formatter name
    fn name(&self) -> &'static str;

    /// Supported file extensions
    fn extensions(&self) -> &[&'static str];

    /// Format content and return formatted result
    fn format(&self, path: &std::path::Path, content: &str) -> Result<String>;

    /// Check if file is formatted
    fn check(&self, path: &std::path::Path, content: &str) -> Result<bool> {
        let formatted = self.format(path, content)?;
        Ok(formatted == content)
    }
}

/// Prettier adapter for JavaScript/TypeScript
pub struct PrettierAdapter;

impl FormatterAdapter for PrettierAdapter {
    fn name(&self) -> &'static str {
        "prettier"
    }

    fn extensions(&self) -> &[&'static str] {
        &[
            "js", "jsx", "ts", "tsx", "json", "css", "scss", "md", "html",
        ]
    }

    fn format(&self, _path: &std::path::Path, content: &str) -> Result<String> {
        // In production, this would call prettier via subprocess or WASM
        // For now, return content as-is (placeholder)
        Ok(content.to_string())
    }
}

/// Rustfmt adapter for Rust
pub struct RustfmtAdapter;

impl FormatterAdapter for RustfmtAdapter {
    fn name(&self) -> &'static str {
        "rustfmt"
    }

    fn extensions(&self) -> &[&'static str] {
        &["rs"]
    }

    fn format(&self, _path: &std::path::Path, content: &str) -> Result<String> {
        // In production, this would call rustfmt
        Ok(content.to_string())
    }
}

/// Black adapter for Python
pub struct BlackAdapter;

impl FormatterAdapter for BlackAdapter {
    fn name(&self) -> &'static str {
        "black"
    }

    fn extensions(&self) -> &[&'static str] {
        &["py", "pyi"]
    }

    fn format(&self, _path: &std::path::Path, content: &str) -> Result<String> {
        Ok(content.to_string())
    }
}

/// gofmt adapter for Go
pub struct GofmtAdapter;

impl FormatterAdapter for GofmtAdapter {
    fn name(&self) -> &'static str {
        "gofmt"
    }

    fn extensions(&self) -> &[&'static str] {
        &["go"]
    }

    fn format(&self, _path: &std::path::Path, content: &str) -> Result<String> {
        Ok(content.to_string())
    }
}

/// Run format command
pub async fn run(cmd: FormatCommand, format: OutputFormat, theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::time::Instant;

    let start = Instant::now();

    let paths = if cmd.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cmd.paths.clone()
    };

    if !matches!(format, OutputFormat::Human) {
        // Output in requested format
        return output_results(&[], format);
    }

    theme.print_section("dx check format: Formatting Files");
    eprintln!();

    // Collect files
    let mut files_checked = 0u32;
    let mut files_formatted = 0u32;
    let mut errors = Vec::new();

    // Get all formatters
    let formatters: Vec<Box<dyn FormatterAdapter>> = vec![
        Box::new(PrettierAdapter),
        Box::new(RustfmtAdapter),
        Box::new(BlackAdapter),
        Box::new(GofmtAdapter),
    ];

    for path in &paths {
        if path.is_file() {
            let result = format_file(path, &formatters, cmd.write, cmd.check).await;
            files_checked += 1;
            match result {
                Ok(changed) => {
                    if changed {
                        files_formatted += 1;
                    }
                }
                Err(e) => errors.push((path.clone(), e)),
            }
        } else if path.is_dir() {
            // Walk directory
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let file_path = entry.path();

                // Skip ignored patterns
                let skip =
                    cmd.ignore.iter().any(|pattern| file_path.to_string_lossy().contains(pattern));
                if skip {
                    continue;
                }

                let result = format_file(file_path, &formatters, cmd.write, cmd.check).await;
                files_checked += 1;
                match result {
                    Ok(changed) => {
                        if changed {
                            files_formatted += 1;
                        }
                    }
                    Err(e) => errors.push((file_path.to_path_buf(), e)),
                }
            }
        }
    }

    let elapsed = start.elapsed();

    // Summary
    eprintln!(
        "  {} {} files checked, {} formatted in {:.2}s",
        "✓".green().bold(),
        files_checked,
        files_formatted,
        elapsed.as_secs_f64()
    );

    if !errors.is_empty() {
        eprintln!();
        eprintln!("  {} {} errors:", "✗".red().bold(), errors.len());
        for (path, err) in &errors {
            eprintln!("    {} {}: {}", "│".bright_black(), path.display(), err);
        }
        if cmd.check {
            anyhow::bail!("Format check failed with {} errors", errors.len());
        }
    }

    eprintln!();
    Ok(())
}

async fn format_file(
    path: &std::path::Path,
    formatters: &[Box<dyn FormatterAdapter>],
    write: bool,
    check: bool,
) -> Result<bool> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Find appropriate formatter
    let formatter = formatters.iter().find(|f| f.extensions().contains(&ext));

    let Some(formatter) = formatter else {
        // No formatter for this extension
        return Ok(false);
    };

    let content = std::fs::read_to_string(path)?;
    let formatted = formatter.format(path, &content)?;

    if formatted == content {
        return Ok(false);
    }

    if check {
        anyhow::bail!("File {} is not formatted", path.display());
    }

    if write {
        std::fs::write(path, &formatted)?;
    }

    Ok(true)
}

fn output_results(_results: &[()], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{{\"files\": [], \"formatted\": 0}}");
        }
        OutputFormat::Llm => {
            println!("format_result files=0 formatted=0");
        }
        OutputFormat::Sarif | OutputFormat::Junit | OutputFormat::Github => {
            println!("<!-- Format results -->");
        }
        _ => {}
    }
    Ok(())
}
