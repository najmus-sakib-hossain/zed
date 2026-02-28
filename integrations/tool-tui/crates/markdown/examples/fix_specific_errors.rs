use console::{Emoji, style};
use dx_markdown::convert::{doc_to_human, llm_to_machine};
use dx_markdown::parser::DxmParser;
use dx_markdown::{CompilerConfig, DxMarkdown};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

static WRENCH: Emoji<'_, '_> = Emoji("🔧 ", "[FIX]");

fn main() {
    println!("\n{} {}", WRENCH, style("Fixing Specific Errored Files").bold().cyan());
    println!("{}", style("─".repeat(60)).dim());

    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let config = CompilerConfig {
        strip_urls: false,
        strip_images: false,
        strip_badges: false,
        tables_to_tsv: false,
        minify_code: false,
        collapse_whitespace: false,
        strip_filler: false,
        dictionary: false,
        ..CompilerConfig::default()
    };

    // List of errored files from the last run
    let errored_paths = vec![
        "crates/driven/docs/ARCHITECTURE.md",
        "crates/dcp/.kiro/specs/dcp-protocol/design.md",
        "crates/forge/vscode-dx-hologram/README.md",
        "crates/check/docs/CONFIGURATION.md",
        "crates/check/docs/COVERAGE.md",
        "crates/python/package-manager/docs/MIGRATION.md",
        "crates/check/docs/RULES.md",
        "crates/media/.bmad/bmm/testarch/knowledge/ci-burn-in.md",
        "LLM_FORMAT.md",
        "crates/forge/.kiro/specs/2/design.md",
        "crates/style/docs/INCREMENTAL_PARSING.md",
        "crates/forge/.kiro/specs/2/tasks.md",
        "crates/serializer/docs/API.md",
        "MARKDOWN_VISUAL.md",
        "crates/javascript/.kiro/specs/production-readiness/requirements.md",
        "trash/specs/dx-check-production/design.md",
        "crates/forge/docs/component-injection-examples.md",
        "crates/media/.bmad/bmm/testarch/knowledge/selective-testing.md",
        "crates/forge/docs/DX_TOOLS_INTEGRATION_GUIDE.md",
        "crates/media/.bmad/bmm/testarch/knowledge/test-healing-patterns.md",
        "crates/javascript/docs/API_REFERENCE.md",
        "crates/python/.kiro/specs/6/tasks.md",
        "crates/javascript/docs/MIGRATION.md",
        "crates/javascript/package-manager/README.md",
        "trash/DCP.md",
        "crates/javascript/project-manager/README.md",
        "trash/DXM_DRAFT.md",
        "trash/DXP.md",
        "trash/specs/dxm-human-format/tasks.md",
        "trash/DX_MARKDOWN.md",
        "trash/specs/dxm-human-format/requirements.md",
        "trash/specs/dx-www-production-ready/design.md",
        "trash/MARKDOWN_DRAFT.md",
        "trash/FORGE_DRAFT.md",
        "trash/specs/2/requirements.md",
        "trash/PROBLEMS.md",
        "trash/MARKDOWN.md",
        "trash/specs/serializer-production-hardening/requirements.md",
    ];

    let errored_files: Vec<PathBuf> =
        errored_paths.iter().map(|p| dx_root.join(p)).filter(|p| p.exists()).collect();

    println!(
        "{} Processing {} errored files\n",
        style("→").cyan(),
        style(errored_files.len()).bold().yellow()
    );

    // Setup progress bars
    let multi = MultiProgress::new();

    let pb_human = multi.add(ProgressBar::new(errored_files.len() as u64));
    pb_human.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.magenta} {prefix:>12} [{bar:40.magenta/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░  ")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb_human.set_prefix(style(".human").bold().magenta().to_string());

    let pb_machine = multi.add(ProgressBar::new(errored_files.len() as u64));
    pb_machine.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} {prefix:>12} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
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
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        pb_human.set_message(format!("{}", filename));
        pb_machine.set_message(format!("{}", filename));

        match process_markdown_file(path, &dx_root, &config, &mut stats) {
            Ok(_) => {
                fixed_count += 1;
            }
            Err(e) => {
                still_errored.push((path.clone(), e.to_string()));
            }
        }

        pb_human.inc(1);
        pb_machine.inc(1);
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

    if !still_errored.is_empty() {
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

    // Pre-fix the content before compilation
    let fixed_content = fix_markdown_issues(&original_content);

    let compiler = DxMarkdown::new(config.clone())?;
    let result = compiler.compile(&fixed_content)?;

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

fn fix_markdown_issues(content: &str) -> String {
    let mut fixed = content.to_string();

    // Fix empty table cells
    fixed = fixed.replace("||", "| - |");

    // Fix DX serializer syntax references that are being documented (not in code blocks)
    // These patterns appear in documentation ABOUT the syntax
    fixed = fixed.replace(" - ^ref", " - `^ref`");
    fixed = fixed.replace("(^ref)", "(`^ref`)");
    fixed = fixed.replace(" ^ref ", " `^ref` ");
    fixed = fixed.replace("References (^ref)", "References (`^ref`)");
    fixed = fixed.replace(" - ^", " - `^`");

    // Fix hash-based references
    fixed = fixed.replace("#:key", "`#:key`");
    fixed = fixed.replace("#<letter>", "`#<letter>`");

    // Fix standalone brackets that might be interpreted as references
    fixed = fixed.replace("[key=value]", "`[key=value]`");
    fixed = fixed.replace("[key value]", "`[key value]`");
    fixed = fixed.replace("[rows]", "`[rows]`");
    fixed = fixed.replace("[count]", "`[count]`");
    fixed = fixed.replace("[filename]", "`[filename]`");

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
            if trimmed == "```yaml" {
                in_yaml_block = true;
            } else if in_code_block == false {
                in_yaml_block = false;
            }
            result.push(line.to_string());
            continue;
        }

        // Fix @ symbols in YAML blocks (pulldown-cmark bug: processes references inside code blocks)
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

        // Fix inline code with special characters
        if fixed_line.contains('`') && !fixed_line.contains("``") {
            let parts: Vec<&str> = fixed_line.split('`').collect();
            let mut new_parts = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                if i % 2 == 1 {
                    // Inside inline code - escape special characters
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
