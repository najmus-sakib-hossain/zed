//! Expand compact bullets in all .human files in .dx/markdown/
//!
//! Usage: cargo run --example expand_human_bullets

use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markdown_dir = PathBuf::from(".dx/markdown");

    if !markdown_dir.exists() {
        eprintln!("Directory .dx/markdown not found");
        return Ok(());
    }

    let mut processed = 0;
    let mut updated = 0;

    // Find all .human files
    for entry in fs::read_dir(&markdown_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("human") {
            processed += 1;
            println!("Processing: {}", path.display());

            let content = fs::read_to_string(&path)?;
            let expanded = expand_compact_bullets(&content);

            // Only write if content changed
            if expanded != content {
                fs::write(&path, &expanded)?;
                updated += 1;
                println!("  ✓ Updated");
            } else {
                println!("  - No changes needed");
            }
        }
    }

    println!("\nSummary:");
    println!("  Processed: {} files", processed);
    println!("  Updated: {} files", updated);

    Ok(())
}

/// Expand compact bullet lists: `-text -text` → `- text\n- text`
fn expand_compact_bullets(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();

    for line in lines {
        // Check if line has compact bullets (multiple " -" patterns)
        if line.contains(" -") && line.matches(" -").count() > 0 {
            // Check if it's a bullet line or has bullets after a colon
            let has_leading_dash = line.trim_start().starts_with('-');
            let has_colon_bullets = line.contains(": -");

            if has_leading_dash || has_colon_bullets {
                // Find where bullets start
                let (prefix, bullets_part) = if let Some(colon_pos) = line.find(": -") {
                    // Bullets after colon: "Key Innovations: -text -text"
                    let prefix = &line[..colon_pos + 2]; // Include ": "
                    let bullets = &line[colon_pos + 2..];
                    (prefix, bullets)
                } else {
                    // Line starts with bullet
                    ("", line)
                };

                // Split bullets on ` -`
                let parts: Vec<&str> = bullets_part.split(" -").collect();
                let base_indent = line.len() - line.trim_start().len();

                for (i, part) in parts.iter().enumerate() {
                    if i == 0 && !prefix.is_empty() {
                        // First line with prefix (e.g., "Key Innovations:")
                        let trimmed = part.trim_start_matches('-').trim();
                        result.push(format!("{}", prefix));
                        result.push(format!("{}- {}", " ".repeat(base_indent), trimmed));
                    } else if i == 0 {
                        // First bullet without prefix
                        let trimmed = part.trim_start_matches('-').trim();
                        result.push(format!("{}- {}", " ".repeat(base_indent), trimmed));
                    } else {
                        // Subsequent bullets
                        result.push(format!("{}- {}", " ".repeat(base_indent), part.trim()));
                    }
                }
                continue;
            }
        }

        result.push(line.to_string());
    }

    result.join("\n")
}
