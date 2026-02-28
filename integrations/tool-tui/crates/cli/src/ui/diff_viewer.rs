use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use similar::{ChangeTag, TextDiff};
use std::path::Path;

/// Diff viewer for comparing files or text (Professional Git-style)
pub struct DiffViewer;

impl DiffViewer {
    /// Show diff between two files
    pub fn show_file_diff(old_path: &Path, new_path: &Path) -> Result<()> {
        let old_content = std::fs::read_to_string(old_path)
            .with_context(|| format!("Failed to read file: {}", old_path.display()))?;
        let new_content = std::fs::read_to_string(new_path)
            .with_context(|| format!("Failed to read file: {}", new_path.display()))?;

        Self::show_diff(
            &old_content,
            &new_content,
            &old_path.display().to_string(),
            &new_path.display().to_string(),
        )
    }

    /// Show diff between two strings (Professional Git-style)
    pub fn show_diff(old: &str, new: &str, old_label: &str, new_label: &str) -> Result<()> {
        let diff = TextDiff::from_lines(old, new);

        // Professional header with box
        const BOX_WIDTH: usize = 70;
        println!();
        println!("{}", format!("┌{}┐", "─".repeat(BOX_WIDTH)).bright_black());

        let header_text = format!("📝 diff --git a/{} b/{}", old_label, new_label);
        let padding_needed = BOX_WIDTH.saturating_sub(header_text.len() + 2);
        println!(
            "{}{}{}{}",
            "│ ".bright_black(),
            header_text.white().bold(),
            " ".repeat(padding_needed),
            "│".bright_black()
        );

        println!("{}", format!("├{}┤", "─".repeat(BOX_WIDTH)).bright_black());

        let old_text = format!("─ a/{}", old_label);
        let padding_needed = BOX_WIDTH.saturating_sub(old_text.len() + 2);
        println!(
            "{}{}{}{}",
            "│ ".bright_black(),
            old_text.red().bold(),
            " ".repeat(padding_needed),
            "│".bright_black()
        );

        let new_text = format!("+ b/{}", new_label);
        let padding_needed = BOX_WIDTH.saturating_sub(new_text.len() + 2);
        println!(
            "{}{}{}{}",
            "│ ".bright_black(),
            new_text.green().bold(),
            " ".repeat(padding_needed),
            "│".bright_black()
        );

        println!("{}", format!("└{}┘", "─".repeat(BOX_WIDTH)).bright_black());
        println!();

        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                println!("{}", "  ⋮".bright_black());
                println!();
            }

            // Calculate line ranges for hunk header
            let mut old_start = 0;
            let mut old_count = 0;
            let mut new_start = 0;
            let mut new_count = 0;

            for op in group {
                for change in diff.iter_changes(op) {
                    match change.tag() {
                        ChangeTag::Delete => {
                            if old_count == 0 {
                                old_start = change.old_index().unwrap_or(0) + 1;
                            }
                            old_count += 1;
                        }
                        ChangeTag::Insert => {
                            if new_count == 0 {
                                new_start = change.new_index().unwrap_or(0) + 1;
                            }
                            new_count += 1;
                        }
                        ChangeTag::Equal => {
                            if old_count == 0 {
                                old_start = change.old_index().unwrap_or(0) + 1;
                            }
                            if new_count == 0 {
                                new_start = change.new_index().unwrap_or(0) + 1;
                            }
                            old_count += 1;
                            new_count += 1;
                        }
                    }
                }
            }

            // Print hunk header with professional styling
            println!(
                "  {}",
                format!("@@ -{},{} +{},{} @@", old_start, old_count, new_start, new_count)
                    .on_bright_black()
                    .cyan()
                    .bold()
            );

            // Print changes with better formatting
            for op in group {
                for change in diff.iter_changes(op) {
                    let line = change.value();
                    let trimmed = line.trim_end_matches('\n');

                    match change.tag() {
                        ChangeTag::Delete => {
                            // Darker red background with white text
                            print!("  {}", "─".red().bold());
                            print!(" {}", trimmed.on_truecolor(52, 0, 0).bright_red());
                            println!();
                        }
                        ChangeTag::Insert => {
                            // Darker green background with white text
                            print!("  {}", "+".green().bold());
                            print!(" {}", trimmed.on_truecolor(0, 52, 0).bright_green());
                            println!();
                        }
                        ChangeTag::Equal => {
                            // Subtle gray for context
                            print!("  {}", "│".bright_black());
                            print!(" {}", trimmed.bright_black());
                            println!();
                        }
                    }
                }
            }
        }

        // Professional footer with stats
        let stats = diff.ratio();
        let insertions = diff.iter_all_changes().filter(|c| c.tag() == ChangeTag::Insert).count();
        let deletions = diff.iter_all_changes().filter(|c| c.tag() == ChangeTag::Delete).count();

        println!();
        println!("{}", format!("┌{}┐", "─".repeat(BOX_WIDTH)).bright_black());

        let stats_text =
            format!("📊 +{} -{} │ Similarity: {:.1}%", insertions, deletions, stats * 100.0);
        let padding_needed = BOX_WIDTH.saturating_sub(stats_text.len() + 2);

        println!(
            "{}{}{}{}",
            "│ ".bright_black(),
            stats_text.bright_white(),
            " ".repeat(padding_needed),
            "│".bright_black()
        );

        println!("{}", format!("└{}┘", "─".repeat(BOX_WIDTH)).bright_black());
        println!();

        Ok(())
    }
}

/// Demo function showing diff capabilities
pub fn demo_diff_viewer() -> Result<()> {
    const TITLE_WIDTH: usize = 70;
    println!();
    println!("{}", format!("╔{}╗", "═".repeat(TITLE_WIDTH)).bright_cyan());

    let title = "⚡ DX CLI Professional Diff Viewer";
    let padding_needed = TITLE_WIDTH.saturating_sub(title.len() + 2);
    println!(
        "{}{}{}{}",
        "║ ".bright_cyan(),
        title.bright_white().bold(),
        " ".repeat(padding_needed),
        "║".bright_cyan()
    );

    println!("{}", format!("╚{}╝", "═".repeat(TITLE_WIDTH)).bright_cyan());

    // Example 1: Simple text diff
    let old_text = r#"Hello World
This is a test
Some unchanged line
Old content here
Another line
"#;

    let new_text = r#"Hello World!
This is a test
Some unchanged line
New content here
Another line
Extra line added
"#;

    DiffViewer::show_diff(old_text, new_text, "old.txt", "new.txt")?;

    // Example 2: Code diff
    let old_code = r#"function greet(name) {
    console.log("Hello " + name);
}

greet("World");
"#;

    let new_code = r#"function greet(name) {
    console.log(`Hello ${name}!`);
}

function farewell(name) {
    console.log(`Goodbye ${name}!`);
}

greet("World");
farewell("World");
"#;

    DiffViewer::show_diff(old_code, new_code, "old.js", "new.js")?;

    // Example 3: Rust code diff
    let old_rust = r#"fn main() {
    let x = 5;
    println!("x = {}", x);
}
"#;

    let new_rust = r#"fn main() {
    let x = 5;
    let y = 10;
    println!("x = {}, y = {}", x, y);
}
"#;

    DiffViewer::show_diff(old_rust, new_rust, "old.rs", "new.rs")?;

    println!("{}", format!("╔{}╗", "═".repeat(TITLE_WIDTH)).bright_cyan());

    let msg = "✓ Professional Git-style diff with modern UI";
    let padding_needed = TITLE_WIDTH.saturating_sub(msg.len() + 2);
    println!(
        "{}{}{}{}",
        "║ ".bright_cyan(),
        msg.bright_white(),
        " ".repeat(padding_needed),
        "║".bright_cyan()
    );

    println!("{}", format!("╚{}╝", "═".repeat(TITLE_WIDTH)).bright_cyan());
    println!();

    Ok(())
}
