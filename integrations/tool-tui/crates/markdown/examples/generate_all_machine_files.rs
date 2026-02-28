use console::{Emoji, style};
use dx_markdown::convert::{doc_to_human, llm_to_machine};
use dx_markdown::markdown::MarkdownParser;
use dx_markdown::{CompilerConfig, DxMarkdown};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use walkdir::WalkDir;

static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");
static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", ">>>");

fn main() {
    // Professional header
    println!("\n{} {}", SPARKLE, style("DX Markdown Compiler").bold().cyan());
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

    let errors = Arc::new(Mutex::new(Vec::new()));
    let start_time = Instant::now();

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
    println!("{} Starting compilation...\n", style("→").cyan());

    // Setup progress bars
    let multi = MultiProgress::new();

    let pb_human = multi.add(ProgressBar::new(md_files.len() as u64));
    pb_human.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.magenta} {prefix:>12} [{bar:40.magenta/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░  ")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb_human.set_prefix(style(".human").bold().magenta().to_string());

    let pb_machine = multi.add(ProgressBar::new(md_files.len() as u64));
    pb_machine.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} {prefix:>12} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░  ")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb_machine.set_prefix(style(".machine").bold().cyan().to_string());

    let pb_human_clone = pb_human.clone();
    let pb_machine_clone = pb_machine.clone();

    let processed: Vec<_> = md_files
        .par_iter()
        .map(|path| {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();

            let mut local_stats = Stats::default();
            let result = process_markdown_file(path, &dx_root, &config, &mut local_stats);

            match result {
                Ok(_) => {
                    local_stats.success_count = 1;
                    pb_human_clone.set_message(format!("{}", filename));
                    pb_machine_clone.set_message(format!("{}", filename));
                }
                Err(e) => {
                    local_stats.error_count = 1;
                    let error_msg = format!("{}: {}", path.display(), e);
                    errors.lock().unwrap().push(error_msg);
                }
            }

            pb_human_clone.inc(1);
            pb_machine_clone.inc(1);
            local_stats
        })
        .collect();

    pb_human.finish_and_clear();
    pb_machine.finish_and_clear();

    let mut final_stats = Stats::default();
    for local_stats in processed {
        final_stats.success_count += local_stats.success_count;
        final_stats.error_count += local_stats.error_count;
        final_stats.total_human_bytes += local_stats.total_human_bytes;
        final_stats.total_llm_bytes += local_stats.total_llm_bytes;
        final_stats.total_machine_bytes += local_stats.total_machine_bytes;
        final_stats.total_tokens_before += local_stats.total_tokens_before;
        final_stats.total_tokens_after += local_stats.total_tokens_after;
    }

    let stats = final_stats;
    let total_time = start_time.elapsed();

    // Professional summary
    println!("\n{}", style("═".repeat(60)).dim());
    println!(
        "{} {} files in {}",
        style("✓").green().bold(),
        style(format!("Compiled {}", stats.success_count)).bold(),
        style(format!("{:.2}s", total_time.as_secs_f64())).bold()
    );

    if stats.error_count > 0 {
        println!(
            "{} {} files failed",
            style("⚠").yellow().bold(),
            style(stats.error_count).bold().yellow()
        );
    }

    // Statistics
    println!("\n{} Statistics:", style("📊").bold());

    let size_savings = ((stats.total_human_bytes - stats.total_machine_bytes) as f64
        / stats.total_human_bytes as f64)
        * 100.0;
    println!(
        "  {} {} → {} ({:.1}% smaller)",
        style("Size:").dim(),
        format_bytes(stats.total_human_bytes),
        style(format_bytes(stats.total_machine_bytes)).cyan(),
        size_savings
    );

    let token_savings = ((stats.total_tokens_before - stats.total_tokens_after) as f64
        / stats.total_tokens_before as f64)
        * 100.0;
    println!(
        "  {} {} → {} ({:.1}% saved)",
        style("Tokens:").dim(),
        format_number(stats.total_tokens_before),
        style(format_number(stats.total_tokens_after)).cyan(),
        token_savings
    );

    let error_list = errors.lock().unwrap();
    if !error_list.is_empty() {
        println!("\n{} All Errors:", style("⚠").yellow().bold());
        for error in error_list.iter() {
            let short_error = error.split('\n').next().unwrap_or(error);
            println!("  {} {}", style("×").red(), style(short_error).dim());
        }
    }

    println!("\n{} Output: {}", ROCKET, style(".dx/markdown/").cyan().bold());
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
    success_count: usize,
    error_count: usize,
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

    // Create subdirectories matching source structure
    let file_parent = relative_path.parent().unwrap_or(Path::new(""));
    let output_dir = output_base.join(file_parent);
    fs::create_dir_all(&output_dir)?;

    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let human_path = output_dir.join(format!("{}.human", file_stem));

    // Pre-fix the content before compilation
    let fixed_content = fix_markdown_issues(&original_content);

    let compiler = DxMarkdown::new(config.clone())?;
    let result = compiler.compile(&fixed_content)?;

    stats.total_llm_bytes += result.output.len();
    stats.total_tokens_before += result.tokens_before;
    stats.total_tokens_after += result.tokens_after;

    // Generate .human file (human-readable format) - parse ORIGINAL markdown, not LLM format!
    let doc = MarkdownParser::parse(&fixed_content)?;
    let human_content = doc_to_human(&doc);
    fs::write(&human_path, &human_content)?;

    // Generate .machine file (binary format)
    let machine_path = output_dir.join(format!("{}.machine", file_stem));
    let machine_bytes = llm_to_machine(&result.output)?;
    fs::write(&machine_path, &machine_bytes)?;

    stats.total_machine_bytes += machine_bytes.len();

    Ok(())
}

fn fix_markdown_issues(content: &str) -> String {
    let mut fixed = content.to_string();

    // Fix empty table cells
    fixed = fixed.replace("||", "| - |");

    // Fix DX serializer syntax references that are being documented
    fixed = fixed.replace(" - ^ref", " - `^ref`");
    fixed = fixed.replace("(^ref)", "(`^ref`)");
    fixed = fixed.replace(" ^ref ", " `^ref` ");
    fixed = fixed.replace("References (^ref)", "References (`^ref`)");
    fixed = fixed.replace(" - ^", " - `^`");

    // Fix hash-based references
    fixed = fixed.replace("#:key", "`#:key`");
    fixed = fixed.replace("#<letter>", "`#<letter>`");

    // Fix standalone brackets
    fixed = fixed.replace("[key=value]", "`[key=value]`");
    fixed = fixed.replace("[key value]", "`[key value]`");
    fixed = fixed.replace("[rows]", "`[rows]`");
    fixed = fixed.replace("[count]", "`[count]`");
    fixed = fixed.replace("[filename]", "`[filename]`");

    // Fix array/range syntax that looks like references
    fixed = fixed.replace("[0..8]:", "`[0..8]:`");
    fixed = fixed.replace("[8..16]:", "`[8..16]:`");
    fixed = fixed.replace("[16..24]:", "`[16..24]:`");
    fixed = fixed.replace("[24..1024]:", "`[24..1024]:`");
    fixed = fixed.replace("[1024..]:", "`[1024..]:`");
    fixed = fixed.replace("friends[3]:", "`friends[3]:`");
    fixed = fixed.replace("hikes[3]", "`hikes[3]`");
    fixed = fixed.replace("slot[0]:", "`slot[0]:`");
    fixed = fixed.replace("slot[1]:", "`slot[1]:`");
    fixed = fixed.replace("slot[2]:", "`slot[2]:`");

    let lines: Vec<&str> = fixed.lines().collect();
    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut in_yaml_block = false;
    let mut in_table = false;
    let mut expected_cols = 0;

    for line in lines {
        let trimmed = line.trim();

        // Track code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            if trimmed == "```yaml"
                || trimmed == "```json"
                || trimmed == "```rust"
                || trimmed == "```javascript"
                || trimmed == "```typescript"
            {
                in_yaml_block = true;
            } else if in_code_block == false {
                in_yaml_block = false;
            }
            result.push(line.to_string());
            continue;
        }

        // Fix @ symbols in code blocks (pulldown-cmark bug)
        if in_yaml_block && line.contains('@') {
            result.push(line.replace('@', "&#64;"));
            continue;
        }

        // Skip processing inside code blocks
        if in_code_block {
            result.push(line.to_string());
            continue;
        }

        let mut fixed_line = line.to_string();

        // Fix bold text followed by colon (e.g., "**Key**: value")
        if fixed_line.contains("**:") {
            fixed_line = fixed_line.replace("**:", "**\\:");
        }

        // Fix patterns that look like link references outside code blocks
        // Pattern: [something]: text (but not [text](url) or ![alt](url))
        if fixed_line.contains("]:") && !fixed_line.contains("](") {
            // Common specific patterns
            fixed_line = fixed_line.replace("[ref]:", "`[ref]:`");
            fixed_line = fixed_line.replace("[age]:", "`[age]:`");
            fixed_line = fixed_line.replace("[key]:", "`[key]:`");
            fixed_line = fixed_line.replace("[build]:", "`[build]:`");
            fixed_line = fixed_line.replace("[doc_for_details]:", "`[doc_for_details]:`");

            // Numeric patterns
            for i in 0..100 {
                let pattern = format!("[{}]:", i);
                let replacement = format!("`[{}]:`", i);
                if fixed_line.contains(&pattern) {
                    fixed_line = fixed_line.replace(&pattern, &replacement);
                }
            }
        }

        // Fix inline code with special characters
        if fixed_line.contains('`') && !fixed_line.contains("``") {
            let parts: Vec<&str> = fixed_line.split('`').collect();
            let mut new_parts = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                if i % 2 == 1 {
                    let fixed_part = part.replace('^', "\\^").replace('#', "\\#");
                    new_parts.push(fixed_part);
                } else {
                    new_parts.push(part.to_string());
                }
            }
            fixed_line = new_parts.join("`");
        }

        // Detect and fix tables
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let cols = trimmed.matches('|').count() - 1;

            if trimmed.contains("---") || trimmed.contains("===") || trimmed.contains(":-") {
                in_table = true;
                expected_cols = cols;
                result.push(fixed_line);
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
                            "n/a".to_string()
                        } else if t.starts_with("**") && t.ends_with("**") {
                            // Bold text in table cells - escape to prevent reference interpretation
                            t.replace("**", "\\*\\*")
                        } else if t == "TOTAL" || t == "_" || t == "Overall" {
                            format!("`{}`", t)
                        } else if t.len() <= 3 && t.chars().all(|c| c.is_numeric()) {
                            format!("`{}`", t)
                        } else {
                            t.to_string()
                        }
                    })
                    .collect();

                while fixed_cells.len() < expected_cols {
                    fixed_cells.push("n/a".to_string());
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

        result.push(fixed_line);
    }

    result.join("\n")
}
