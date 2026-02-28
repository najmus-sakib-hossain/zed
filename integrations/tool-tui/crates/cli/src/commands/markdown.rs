//! DX Markdown Commands - Beautiful markdown with LLM optimization

use anyhow::Result;
use clap::{Args, Subcommand};
use console::style;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use walkdir::WalkDir;

use markdown::MarkdownBeautifier;
use markdown::convert::llm_to_machine;

#[derive(Args)]
pub struct MarkdownCommand {
    #[command(subcommand)]
    pub command: MarkdownSubcommand,
}

#[derive(Subcommand)]
pub enum MarkdownSubcommand {
    /// Process markdown files (default)
    #[command(visible_alias = "p")]
    Process {
        /// Input file or directory
        #[arg(value_name = "PATH")]
        input: PathBuf,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Auto-fix lint issues
        #[arg(short = 'f', long)]
        autofix: bool,

        /// Skip linting
        #[arg(long)]
        no_lint: bool,
    },

    /// Filter markdown content with presets (revertable)
    #[command(visible_alias = "f")]
    Filter(crate::commands::markdown_filter::MarkdownFilterArgs),
}

impl MarkdownCommand {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            MarkdownSubcommand::Process {
                input,
                recursive,
                verbose,
                autofix,
                no_lint,
            } => process_markdown(input, recursive, verbose, autofix, no_lint).await,
            MarkdownSubcommand::Filter(args) => args.execute().await,
        }
    }
}

async fn process_markdown(
    input: PathBuf,
    mut recursive: bool,
    verbose: bool,
    autofix: bool,
    no_lint: bool,
) -> Result<()> {
    // Validate input
    if !input.exists() {
        eprintln!(
            "\n  {} Path does not exist: {}",
            style("[X]").red().bold(),
            style(input.display()).red()
        );
        return Err(anyhow::anyhow!("Path not found"));
    }

    if input.is_file() {
        beautify_single_file(&input, verbose, autofix, no_lint).await
    } else {
        // For directories, default to recursive (search all subdirectories)
        if !recursive {
            recursive = true; // Always recursive for directories
        }
        beautify_directory(&input, recursive, verbose, autofix, no_lint).await
    }
}

async fn beautify_single_file(
    input: &PathBuf,
    _verbose: bool,
    autofix: bool,
    no_lint: bool,
) -> Result<()> {
    let file_name = input.file_name().unwrap().to_string_lossy().to_string();

    // Check if it's a markdown file
    if input.extension().and_then(|s| s.to_str()) != Some("md") {
        eprintln!(
            "\n  {} Not a markdown file: {}",
            style("[!]").yellow().bold(),
            style(&file_name).yellow()
        );
        return Ok(());
    }

    println!("\n  {} Processing: {}\n", style("[*]").cyan().bold(), style(&file_name).cyan());

    // Create spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("  {spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.enable_steady_tick(Duration::from_millis(80));

    // Read file
    spinner.set_message(format!("{} Reading file...", style("→").dim()));
    let content = std::fs::read_to_string(input)?;

    // Lint if enabled
    if !no_lint {
        spinner.set_message(format!("{} Linting...", style("→").dim()));
        let issues = markdown::lint_markdown(&content);
        if !issues.is_empty() {
            spinner.finish_and_clear();
            println!("  {} Found {} lint issue(s):", style("⚠").yellow().bold(), issues.len());
            for issue in &issues {
                println!("    {} {}", style("•").yellow(), style(issue).dim());
            }
            println!();
            spinner.reset();
            spinner.enable_steady_tick(Duration::from_millis(80));
        }
    }

    // Auto-fix if enabled
    let processed_content = if autofix {
        spinner.set_message(format!("{} Auto-fixing...", style("→").dim()));
        markdown::autofix_markdown(&content)
    } else {
        content
    };

    // Beautify
    spinner.set_message(format!("{} Beautifying...", style("→").dim()));
    let beautifier = MarkdownBeautifier::new();
    let beautified = beautifier.beautify(&processed_content)?;

    // Create .dx/markdown directory if it doesn't exist
    let dx_markdown_dir = std::path::Path::new(".dx/markdown");
    std::fs::create_dir_all(dx_markdown_dir)?;

    // Save to .human file in .dx/markdown/
    let file_stem = input.file_stem().unwrap().to_string_lossy();
    let human_path = dx_markdown_dir.join(format!("{}.human", file_stem));
    spinner.set_message(format!("{} Writing .human file...", style("→").dim()));
    std::fs::write(&human_path, &beautified)?;

    // Convert to LLM format
    spinner.set_message(format!("{} Optimizing for LLMs...", style("→").dim()));
    let compiler = markdown::DxMarkdown::new(markdown::CompilerConfig::default())?;
    let result = compiler.compile(&beautified)?;

    // Generate .machine format (binary) from the LLM output
    spinner.set_message(format!("{} Generating .machine format...", style("→").dim()));
    let machine_path = dx_markdown_dir.join(format!("{}.machine", file_stem));
    match llm_to_machine(&result.output) {
        Ok(machine_bytes) => {
            std::fs::write(&machine_path, machine_bytes)?;
        }
        Err(e) => {
            eprintln!("Warning: Failed to generate .machine format: {}", e);
        }
    }

    // Write back to .md (human format is the source)
    spinner.set_message(format!("{} Writing .md file...", style("→").dim()));
    std::fs::write(input, &beautified)?;

    spinner.finish_and_clear();

    // Print success
    let counter = serializer::TokenCounter::new();
    let human_tokens = counter.count(&beautified, serializer::ModelType::ClaudeSonnet4).count;
    let llm_tokens = counter.count(&result.output, serializer::ModelType::ClaudeSonnet4).count;

    let savings = if human_tokens > 0 && llm_tokens <= human_tokens {
        ((human_tokens - llm_tokens) as f64 / human_tokens as f64) * 100.0
    } else {
        0.0
    };

    // Calculate dynamic box width based on plain text content (without ANSI codes)
    let file_name_plain = format!("[✓] {}", &file_name);
    let tokens_plain =
        format!("  Tokens:   {} →   {} (  {:.1}% saved)", human_tokens, llm_tokens, savings);
    let human_plain = format!("  Human:    {}", input.display());
    let llm_plain = format!("  LLM:      .dx/markdown/{}.llm", file_stem);
    let machine_plain = format!("  Machine:  {}", machine_path.display());

    // Find the longest line
    let max_width = file_name_plain
        .len()
        .max(tokens_plain.len())
        .max(human_plain.len())
        .max(llm_plain.len())
        .max(machine_plain.len())
        .max(60); // minimum width

    // Create border with exact width (accounting for │ space content space │)
    let border_line = "─".repeat(max_width + 2);

    println!();
    println!("  ┌{}┐", border_line);
    println!("  │ {:width$} │", file_name_plain, width = max_width);
    println!("  ├{}┤", border_line);
    println!("  │ {:width$} │", tokens_plain, width = max_width);
    println!("  │ {:width$} │", human_plain, width = max_width);
    println!("  │ {:width$} │", llm_plain, width = max_width);
    println!("  │ {:width$} │", machine_plain, width = max_width);
    println!("  └{}┘", border_line);
    println!();

    Ok(())
}

async fn beautify_directory(
    input: &PathBuf,
    recursive: bool,
    verbose: bool,
    autofix: bool,
    no_lint: bool,
) -> Result<()> {
    // Find all markdown files
    let walker = if recursive {
        WalkDir::new(input).follow_links(false)
    } else {
        WalkDir::new(input).max_depth(1).follow_links(false)
    };

    let mut files: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .map(|e| e.path().to_path_buf())
        .collect();

    files.sort();

    if files.is_empty() {
        println!(
            "\n  {} No markdown files found in {}",
            style("[i]").yellow().bold(),
            style(input.display()).cyan()
        );
        return Ok(());
    }

    println!(
        "\n  {} Found {} markdown file(s) - processing in parallel...\n",
        style("[*]").magenta().bold(),
        style(files.len()).cyan().bold()
    );

    // Create .dx/markdown directory if it doesn't exist
    let dx_markdown_dir = std::path::Path::new(".dx/markdown");
    std::fs::create_dir_all(dx_markdown_dir)?;

    // Atomic counters for statistics
    let total_tokens_before = Arc::new(AtomicUsize::new(0));
    let total_tokens_after = Arc::new(AtomicUsize::new(0));
    let files_processed = Arc::new(AtomicUsize::new(0));
    let files_with_issues = Arc::new(AtomicUsize::new(0));
    let files_failed = Arc::new(AtomicUsize::new(0));

    // Progress bar with professional styling - updates in place with steady refresh
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "  [{elapsed_precise}] [{bar:50.cyan/blue}] {pos}/{len} ({percent}%) {per_sec}",
            )
            .unwrap()
            .progress_chars("█▓▒░ "),
    );
    pb.set_draw_target(indicatif::ProgressDrawTarget::stderr());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // Process files in parallel using rayon
    files.par_iter().progress_with(pb.clone()).for_each(|file| {
        let file_name = file.file_name().unwrap().to_string_lossy().to_string();

        // Read file
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "  {} Failed to read {}: {}",
                    style("[-]").red().bold(),
                    style(&file_name).red(),
                    style(e).dim()
                );
                files_failed.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        // Lint
        if !no_lint {
            let issues = markdown::lint_markdown(&content);
            if !issues.is_empty() {
                files_with_issues.fetch_add(1, Ordering::Relaxed);
                if verbose {
                    eprintln!(
                        "  {} {} has {} lint issue(s)",
                        style("[!]").yellow().bold(),
                        style(&file_name).yellow(),
                        issues.len()
                    );
                }
            }
        }

        // Auto-fix
        let processed_content = if autofix {
            markdown::autofix_markdown(&content)
        } else {
            content
        };

        // Beautify
        let beautifier = MarkdownBeautifier::new();
        let beautified = match beautifier.beautify(&processed_content) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "  {} Failed to beautify {}: {}",
                    style("[-]").red().bold(),
                    style(&file_name).red(),
                    style(e).dim()
                );
                files_failed.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        // Preserve directory structure relative to where the command was run
        // Files from WalkDir are already relative to cwd, so just use them directly
        let relative_path = file.as_path();

        // Create corresponding path in .dx/markdown/
        let human_path = dx_markdown_dir.join(relative_path).with_extension("human");
        let machine_path = dx_markdown_dir.join(relative_path).with_extension("machine");

        // Create parent directories if needed
        if let Some(parent) = human_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            eprintln!(
                "  {} Failed to create directory for {}: {}",
                style("[-]").red().bold(),
                style(&file_name).red(),
                style(e).dim()
            );
            files_failed.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Save .human file with directory structure
        if let Err(e) = std::fs::write(&human_path, &beautified) {
            eprintln!(
                "  {} Failed to write .human for {}: {}",
                style("[-]").red().bold(),
                style(&file_name).red(),
                style(e).dim()
            );
            files_failed.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Convert to LLM
        let compiler = match markdown::DxMarkdown::new(markdown::CompilerConfig::default()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "  {} Failed to create compiler for {}: {}",
                    style("[-]").red().bold(),
                    style(&file_name).red(),
                    style(e).dim()
                );
                files_failed.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        let result = match compiler.compile(&beautified) {
            Ok(r) => r,
            Err(e) => {
                eprintln!(
                    "  {} Failed to compile {}: {}",
                    style("[-]").red().bold(),
                    style(&file_name).red(),
                    style(e).dim()
                );
                files_failed.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        // Save .llm file with directory structure
        let llm_path = dx_markdown_dir.join(relative_path).with_extension("llm");
        if let Err(e) = std::fs::write(&llm_path, &result.output) {
            eprintln!(
                "  {} Failed to write .llm for {}: {}",
                style("[-]").red().bold(),
                style(&file_name).red(),
                style(e).dim()
            );
            files_failed.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Generate .machine format (binary) with directory structure
        match llm_to_machine(&result.output) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&machine_path, bytes) {
                    eprintln!(
                        "  {} Failed to write .machine for {}: {}",
                        style("[!]").yellow().bold(),
                        style(&file_name).yellow(),
                        style(e).dim()
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "  {} Failed to generate .machine for {}: {}",
                    style("[!]").yellow().bold(),
                    style(&file_name).yellow(),
                    style(e).dim()
                );
            }
        }

        // Write .md back with human format (source)
        if let Err(e) = std::fs::write(file, &beautified) {
            eprintln!(
                "  {} Failed to write .md for {}: {}",
                style("[-]").red().bold(),
                style(&file_name).red(),
                style(e).dim()
            );
            files_failed.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Calculate token counts: .human (beautified) vs .md (LLM-optimized)
        // This shows the true savings of dx-markdown optimization
        // Using Claude Sonnet 4 tokenizer as it shows better savings with no-line-gap format
        let counter = serializer::TokenCounter::new();
        let human_tokens = counter.count(&beautified, serializer::ModelType::ClaudeSonnet4).count;
        let llm_tokens = counter.count(&result.output, serializer::ModelType::ClaudeSonnet4).count;

        // Update stats atomically
        total_tokens_before.fetch_add(human_tokens, Ordering::Relaxed);
        total_tokens_after.fetch_add(llm_tokens, Ordering::Relaxed);
        files_processed.fetch_add(1, Ordering::Relaxed);

        if verbose {
            let savings = if human_tokens > 0 && llm_tokens <= human_tokens {
                ((human_tokens - llm_tokens) as f64 / human_tokens as f64) * 100.0
            } else {
                0.0
            };
            eprintln!(
                "  {} {} {} {} → {} {}",
                style("[+]").green().bold(),
                style(&file_name).cyan(),
                style("|").dim(),
                style(format!("{}", human_tokens)).yellow(),
                style(format!("{}", llm_tokens)).green(),
                style(format!("({:.1}%)", savings)).cyan()
            );
        }
    });

    pb.finish_and_clear();

    // Get final statistics
    let processed = files_processed.load(Ordering::Relaxed);
    let failed = files_failed.load(Ordering::Relaxed);
    let with_issues = files_with_issues.load(Ordering::Relaxed);
    let tokens_before = total_tokens_before.load(Ordering::Relaxed);
    let tokens_after = total_tokens_after.load(Ordering::Relaxed);

    let savings = if tokens_before > 0 && tokens_after <= tokens_before {
        ((tokens_before - tokens_after) as f64 / tokens_before as f64) * 100.0
    } else {
        0.0
    };
    let tokens_saved = tokens_before.saturating_sub(tokens_after);

    // Build all content lines first to calculate proper width
    let mut lines = Vec::new();
    lines.push("  [*] Processing Summary".to_string());
    lines.push(String::new()); // separator
    lines.push("  [>] Files:".to_string());
    lines.push(format!("      [+] Processed:        {}", processed));
    if failed > 0 {
        lines.push(format!("      [-] Failed:           {}", failed));
    }
    if with_issues > 0 {
        lines.push(format!("      [!] With lint issues: {}", with_issues));
    }
    lines.push(String::new());
    lines.push("  [>] Token Optimization:".to_string());
    lines.push(format!("      Before: {:>10} tokens", tokens_before));
    lines.push(format!("      After:  {:>10} tokens", tokens_after));
    lines.push(format!("      Saved:  {:>10} tokens ({:>5.1}%)", tokens_saved, savings));
    lines.push(String::new());
    lines.push("  [>] Output:".to_string());
    lines.push("      *.md                   (human-readable)".to_string());
    lines.push("      .dx/markdown/*.llm     (LLM-optimized)".to_string());
    lines.push("      .dx/markdown/*.machine (binary)".to_string());

    // Calculate max width from plain text (strip ANSI codes for accurate measurement)
    let max_width = lines
        .iter()
        .map(|line| {
            // Remove ANSI escape sequences for width calculation
            let plain = console::strip_ansi_codes(line);
            plain.len()
        })
        .max()
        .unwrap_or(60)
        .max(60);

    // Print box with proper alignment
    println!();
    let border = "═".repeat(max_width + 2);
    println!("  {}", style(format!("╔{}╗", border)).cyan().bold());

    for (i, line) in lines.iter().enumerate() {
        if i == 1 {
            // Separator line
            println!("  {}", style(format!("╠{}╣", border)).cyan().bold());
        } else if line.is_empty() {
            // Empty line
            println!(
                "  {} {:width$} {}",
                style("║").cyan(),
                "",
                style("║").cyan(),
                width = max_width
            );
        } else {
            // Content line with proper coloring
            let plain = console::strip_ansi_codes(line);
            let padding = max_width - plain.len();

            // Apply colors based on content
            let colored_line = if line.contains("[*]") {
                style(line.clone()).white().bold().to_string()
            } else if line.contains("[>]") {
                style(line.clone()).cyan().bold().to_string()
            } else if line.contains("[+]") {
                let parts: Vec<&str> = line.split("[+]").collect();
                format!("{}{}{}", parts[0], style("[+]").green().bold(), style(parts[1]).green())
            } else if line.contains("[-]") {
                let parts: Vec<&str> = line.split("[-]").collect();
                format!("{}{}{}", parts[0], style("[-]").red().bold(), style(parts[1]).red())
            } else if line.contains("[!]") {
                let parts: Vec<&str> = line.split("[!]").collect();
                format!("{}{}{}", parts[0], style("[!]").yellow().bold(), style(parts[1]).yellow())
            } else if line.contains("Before:") {
                style(line.clone()).yellow().to_string()
            } else if line.contains("After:") {
                style(line.clone()).green().to_string()
            } else if line.contains("Saved:") {
                style(line.clone()).cyan().bold().to_string()
            } else {
                style(line.clone()).dim().to_string()
            };

            println!(
                "  {} {}{} {}",
                style("║").cyan(),
                colored_line,
                " ".repeat(padding),
                style("║").cyan()
            );
        }
    }

    println!("  {}", style(format!("╚{}╝", border)).cyan().bold());
    println!();

    Ok(())
}
