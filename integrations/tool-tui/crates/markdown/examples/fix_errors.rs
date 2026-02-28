use console::{Emoji, style};
use dx_markdown::convert::{doc_to_human, llm_to_machine};
use dx_markdown::parser::DxmParser;
use dx_markdown::{CompilerConfig, DxMarkdown};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

static WRENCH: Emoji<'_, '_> = Emoji("🔧 ", "[FIX]");

fn main() {
    println!("\n{} {}", WRENCH, style("Fixing errored markdown files").bold().cyan());

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

    // Collect all markdown files (excluding dependencies and build folders)
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

    println!(
        "{} Scanning {} files for errors...\n",
        style("→").cyan(),
        style(md_files.len()).bold()
    );

    // Find errored files with progress bar
    let mut errored_files = Vec::new();
    let start_scan = Instant::now();

    let scan_pb = ProgressBar::new(md_files.len() as u64);
    scan_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} {prefix:>12} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░  ")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    scan_pb.set_prefix(style("Scanning").bold().yellow().to_string());

    for path in &md_files {
        let filename = path.file_name().unwrap().to_string_lossy();
        scan_pb.set_message(format!("{}", filename));

        // Skip very large files to avoid stack overflow
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() > 5_000_000 {
                scan_pb.inc(1);
                continue;
            }
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                scan_pb.inc(1);
                continue;
            }
        };

        // Skip if content is too large
        if content.len() > 2_000_000 {
            scan_pb.inc(1);
            continue;
        }

        if let Ok(compiler) = DxMarkdown::new(config.clone()) {
            if compiler.compile(&content).is_err() {
                errored_files.push(path.clone());
            }
        }

        scan_pb.inc(1);
    }

    scan_pb.finish_and_clear();

    println!(
        "{} Found {} errored files in {:.2}s\n",
        style("✓").green(),
        style(errored_files.len()).bold().yellow(),
        start_scan.elapsed().as_secs_f64()
    );

    if errored_files.is_empty() {
        println!("{} No errors to fix!", style("✓").green());
        return;
    }

    // Setup progress bars
    let multi = MultiProgress::new();

    let pb_human = multi.add(ProgressBar::new(errored_files.len() as u64));
    pb_human.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} {prefix:>12} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░  ")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb_human.set_prefix(style(".human").bold().magenta().to_string());

    let pb_machine = multi.add(ProgressBar::new(errored_files.len() as u64));
    pb_machine.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} {prefix:>12} [{bar:40.green/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░  ")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb_machine.set_prefix(style(".machine").bold().cyan().to_string());

    let mut stats = Stats::default();
    let mut fixed_count = 0;
    let mut still_errored = Vec::new();
    let start_time = Instant::now();

    for path in &errored_files {
        pb_human.set_message(format!("{}", path.file_name().unwrap().to_string_lossy()));
        pb_machine.set_message(format!("{}", path.file_name().unwrap().to_string_lossy()));

        match process_markdown_file(path, &dx_root, &config, &mut stats) {
            Ok(_) => {
                fixed_count += 1;
                pb_human.inc(1);
                pb_machine.inc(1);
            }
            Err(e) => {
                still_errored.push((path.clone(), e.to_string()));
                pb_human.inc(1);
                pb_machine.inc(1);
            }
        }
    }

    pb_human.finish_and_clear();
    pb_machine.finish_and_clear();

    let total_time = start_time.elapsed();

    // Summary
    println!("\n{}", style("═".repeat(60)).dim());
    println!(
        "{} {} files fixed in {}",
        style("✓").green().bold(),
        style(fixed_count).bold().green(),
        style(format!("{:.2}s", total_time.as_secs_f64())).bold()
    );

    if !still_errored.is_empty() {
        println!(
            "{} {} files still have errors",
            style("✗").red().bold(),
            style(still_errored.len()).bold().red()
        );
    }

    println!("\n{} Statistics:", style("📊").bold());
    println!(
        "  {} {} → {}",
        style("Size:").dim(),
        format_bytes(stats.total_human_bytes),
        style(format_bytes(stats.total_machine_bytes)).cyan()
    );

    let token_savings = if stats.total_tokens_before > 0 {
        ((stats.total_tokens_before - stats.total_tokens_after) as f64
            / stats.total_tokens_before as f64)
            * 100.0
    } else {
        0.0
    };

    println!(
        "  {} {} → {} ({:.1}% saved)",
        style("Tokens:").dim(),
        format_number(stats.total_tokens_before),
        style(format_number(stats.total_tokens_after)).cyan(),
        token_savings
    );

    if !still_errored.is_empty() && still_errored.len() <= 10 {
        println!("\n{} Still errored:", style("⚠").yellow().bold());
        for (path, error) in &still_errored {
            let short_error = error.split('\n').next().unwrap_or(error);
            println!(
                "  {} {}: {}",
                style("×").red(),
                path.file_name().unwrap().to_string_lossy(),
                style(short_error).dim()
            );
        }
    } else if still_errored.len() > 10 {
        println!(
            "\n{} Still errored (showing 10 of {}):",
            style("⚠").yellow().bold(),
            still_errored.len()
        );
        for (path, error) in still_errored.iter().take(10) {
            let short_error = error.split('\n').next().unwrap_or(error);
            println!(
                "  {} {}: {}",
                style("×").red(),
                path.file_name().unwrap().to_string_lossy(),
                style(short_error).dim()
            );
        }
    }

    println!();
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[derive(Default)]
struct Stats {
    total_human_bytes: usize,
    total_llm_bytes: usize,
    total_machine_bytes: usize,
    total_tokens_before: usize,
    total_tokens_after: usize,
}

fn process_markdown_file(
    path: &Path,
    workspace_root: &Path,
    config: &CompilerConfig,
    stats: &mut Stats,
) -> Result<(), Box<dyn std::error::Error>> {
    let original_content = fs::read_to_string(path)?;
    stats.total_human_bytes += original_content.len();

    // Maintain directory structure
    let relative_path = path.strip_prefix(workspace_root)?;
    let output_base = workspace_root.join(".dx").join("markdown");

    let file_parent = relative_path.parent().unwrap_or(Path::new(""));
    let output_dir = output_base.join(file_parent);
    fs::create_dir_all(&output_dir)?;

    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let human_path = output_dir.join(format!("{}.human", file_stem));

    // Try multiple fix strategies
    let compiler = DxMarkdown::new(config.clone())?;
    let result = match compiler.compile(&original_content) {
        Ok(r) => r,
        Err(_) => {
            // Strategy 1: Fix tables
            let fixed1 = fix_markdown_tables(&original_content);
            match compiler.compile(&fixed1) {
                Ok(r) => r,
                Err(_) => {
                    // Strategy 2: Fix references
                    let fixed2 = fix_markdown_references(&fixed1);
                    match compiler.compile(&fixed2) {
                        Ok(r) => r,
                        Err(_) => {
                            // Strategy 3: Aggressive fixes
                            let fixed3 = fix_markdown_aggressive(&fixed2);
                            compiler.compile(&fixed3)?
                        }
                    }
                }
            }
        }
    };

    stats.total_llm_bytes += result.output.len();
    stats.total_tokens_before += result.tokens_before;
    stats.total_tokens_after += result.tokens_after;

    // Generate .human file (human-readable format)
    let doc = DxmParser::parse(&result.output)?;
    let human_content = doc_to_human(&doc);
    fs::write(&human_path, &human_content)?;

    // Generate .machine file (binary format)
    let machine_path = output_dir.join(format!("{}.machine", file_stem));
    let machine_bytes = llm_to_machine(&result.output)?;
    fs::write(&machine_path, &machine_bytes)?;

    stats.total_machine_bytes += machine_bytes.len();

    Ok(())
}

fn fix_markdown_tables(content: &str) -> String {
    let mut fixed = content.to_string();

    // Fix empty table cells
    fixed = fixed.replace("||", "| - |");
    fixed = fixed.replace("| |", "| - |");

    let lines: Vec<&str> = fixed.lines().collect();
    let mut result = Vec::new();
    let mut in_table = false;
    let mut expected_cols = 0;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let cols = trimmed.matches('|').count() - 1;

            if trimmed.contains("---") || trimmed.contains("===") || trimmed.contains(":-") {
                in_table = true;
                expected_cols = cols;
                result.push(line.to_string());
                continue;
            }

            if in_table && expected_cols > 0 {
                let cells: Vec<&str> =
                    trimmed.trim_start_matches('|').trim_end_matches('|').split('|').collect();
                let mut fixed_cells: Vec<String> = cells
                    .iter()
                    .map(|s| {
                        let t = s.trim();
                        if t.is_empty() {
                            "-".to_string()
                        } else {
                            t.to_string()
                        }
                    })
                    .collect();

                while fixed_cells.len() < expected_cols {
                    fixed_cells.push("-".to_string());
                }
                if fixed_cells.len() > expected_cols {
                    fixed_cells.truncate(expected_cols);
                }

                result.push(format!("| {} |", fixed_cells.join(" | ")));
                continue;
            }
        } else if in_table && !trimmed.is_empty() && !trimmed.starts_with('|') {
            in_table = false;
        }

        result.push(line.to_string());
    }

    result.join("\n")
}

fn fix_markdown_references(content: &str) -> String {
    let mut fixed = content.to_string();

    // Remove standalone empty brackets that cause undefined references
    fixed = fixed.replace("[]", "");
    fixed = fixed.replace("[ ]", "");

    // Fix common reference patterns
    let lines: Vec<&str> = fixed.lines().collect();
    let mut result = Vec::new();

    for line in lines {
        let mut line_fixed = line.to_string();

        // Fix table cells with just numbers or special chars that become references
        if line.trim().starts_with('|') {
            // Replace cells that are just numbers or underscores
            line_fixed = line_fixed
                .replace("| TOTAL |", "| Total |")
                .replace("| _ |", "| - |")
                .replace("| 3 |", "| Three |")
                .replace("| 2 |", "| Two |")
                .replace("| 1 |", "| One |");
        }

        result.push(line_fixed);
    }

    result.join("\n")
}

fn fix_markdown_aggressive(content: &str) -> String {
    let fixed = content.to_string();

    // Remove problematic patterns
    let lines: Vec<&str> = fixed.lines().collect();
    let mut result = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        // Skip lines that are just reference definitions with empty content
        if trimmed.starts_with('[') && trimmed.contains("]:") && trimmed.len() < 10 {
            continue;
        }

        result.push(line.to_string());
    }

    result.join("\n")
}
