use console::{Emoji, style};
use dx_markdown::{CompilerConfig, DxMarkdown};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use walkdir::WalkDir;

static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");
static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", ">>>");
static CHART: Emoji<'_, '_> = Emoji("📊 ", ">>>");

#[derive(Default)]
struct Stats {
    original_size: usize,
    compressed_size: usize,
    original_tokens: usize,
    compressed_tokens: usize,
    files_processed: usize,
}

fn main() {
    // Professional header
    println!("\n{} {}", SPARKLE, style("DX Markdown LLM Format Converter").bold().cyan());
    println!("{}", style("─".repeat(60)).dim());

    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let config = CompilerConfig {
        strip_urls: true,
        strip_images: true,
        strip_badges: true,
        tables_to_tsv: true,
        minify_code: false,
        collapse_whitespace: true,
        strip_filler: true,
        dictionary: false,
        ..CompilerConfig::default()
    };

    let stats = Arc::new(Mutex::new(Stats::default()));
    let errors = Arc::new(Mutex::new(Vec::new()));
    let start_time = Instant::now();

    // Find all markdown files
    let md_files: Vec<PathBuf> = WalkDir::new(&dx_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .filter(|e| {
            let path_str = e.path().to_string_lossy();
            !path_str.contains("node_modules")
                && !path_str.contains("/target/")
                && !path_str.contains("\\target\\")
                && !path_str.contains("\\.git\\")
                && !path_str.contains("/.git/")
                && !path_str.contains("\\.venv\\")
                && !path_str.contains("/.venv/")
                && !path_str.contains("/submodules/")
                && !path_str.contains("\\submodules\\")
                && !path_str.contains("/.changeset/")
                && !path_str.contains("\\.changeset\\")
                && !path_str.contains("/pkg/")
                && !path_str.contains("\\pkg\\")
                && !path_str.contains("/dist/")
                && !path_str.contains("\\dist\\")
                && !path_str.contains("/build/")
                && !path_str.contains("\\build\\")
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    println!("{} Found {} markdown files", style("→").cyan(), style(md_files.len()).bold());
    println!("{} Converting to LLM format...\n", style("→").cyan());

    // Setup progress bar with block characters
    let pb = ProgressBar::new(md_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.green/dim}] {pos}/{len} {percent}% {msg}")
            .unwrap()
            .progress_chars("█▓░")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );

    let pb_clone = pb.clone();
    let stats_clone = Arc::clone(&stats);
    let errors_clone = Arc::clone(&errors);

    // Process files in parallel
    md_files.par_iter().for_each(|path| match process_file(path, &config) {
        Ok((original_size, compressed_size, original_tokens, compressed_tokens)) => {
            let mut stats = stats_clone.lock().unwrap();
            stats.original_size += original_size;
            stats.compressed_size += compressed_size;
            stats.original_tokens += original_tokens;
            stats.compressed_tokens += compressed_tokens;
            stats.files_processed += 1;

            let file_name = path.file_name().unwrap().to_string_lossy();
            pb_clone.set_message(file_name.to_string());
            pb_clone.inc(1);
        }
        Err(e) => {
            let mut errors = errors_clone.lock().unwrap();
            errors.push((path.clone(), e));
            pb_clone.inc(1);
        }
    });

    pb.finish_and_clear();

    let elapsed = start_time.elapsed();
    let stats = stats.lock().unwrap();
    let errors = errors.lock().unwrap();

    // Print results
    println!("\n{}", style("═".repeat(60)).dim());
    println!(
        "{} Converted {} files in {:.2}s\n",
        style("✓").green().bold(),
        style(stats.files_processed).bold(),
        elapsed.as_secs_f64()
    );

    println!("{} {}", CHART, style("Statistics:").bold().cyan());

    // Size comparison
    let size_saved = stats.original_size.saturating_sub(stats.compressed_size);
    let size_percent = if stats.original_size > 0 {
        (size_saved as f64 / stats.original_size as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "  {}: {} → {} ({:.1}% smaller)",
        style("Size").bold(),
        format_bytes(stats.original_size),
        format_bytes(stats.compressed_size),
        size_percent
    );

    // Token comparison with visual bar
    let tokens_saved = stats.original_tokens.saturating_sub(stats.compressed_tokens);
    let token_percent = if stats.original_tokens > 0 {
        (tokens_saved as f64 / stats.original_tokens as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "  {}: {} → {} ({:.1}% saved)",
        style("Tokens").bold(),
        format_number(stats.original_tokens),
        format_number(stats.compressed_tokens),
        token_percent
    );

    // Visual token savings bar
    let bar_width = 40;
    let filled = ((stats.compressed_tokens as f64 / stats.original_tokens as f64)
        * bar_width as f64) as usize;
    let empty = bar_width - filled;
    let bar = format!(
        "{}{} {:.1}%",
        "█".repeat(filled),
        "░".repeat(empty),
        (stats.compressed_tokens as f64 / stats.original_tokens as f64) * 100.0
    );
    println!("  {}: {}", style("Efficiency").bold(), style(bar).green());

    if !errors.is_empty() {
        println!("\n{} {} errors:", style("⚠").yellow(), errors.len());
        for (path, err) in errors.iter() {
            println!(
                "  {} {}",
                style("×").red(),
                path.strip_prefix(&dx_root).unwrap_or(path).display()
            );
            println!("    {}", style(err).dim());
        }
    }

    println!("\n{} All markdown files converted to LLM format!", ROCKET);
    println!();
}

fn process_file(
    path: &Path,
    config: &CompilerConfig,
) -> Result<(usize, usize, usize, usize), String> {
    // Read original markdown
    let original_content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let original_size = original_content.len();
    let original_tokens = estimate_tokens(&original_content);

    // Compile to LLM format
    let compiler =
        DxMarkdown::new(config.clone()).map_err(|e| format!("Failed to create compiler: {}", e))?;
    let result = compiler
        .compile(&original_content)
        .map_err(|e| format!("Compilation failed: {}", e))?;

    let compressed_size = result.output.len();
    let compressed_tokens = estimate_tokens(&result.output);

    // Write back to the same file
    fs::write(path, result.output).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok((original_size, compressed_size, original_tokens, compressed_tokens))
}

/// Estimate token count (rough approximation: ~4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Format bytes into human-readable format
fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format number with thousands separator
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;

    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }

    result.chars().rev().collect()
}
