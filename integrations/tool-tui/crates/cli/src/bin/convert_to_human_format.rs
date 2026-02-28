#!/usr/bin/env rust
//! Converts all DX Serializer and DX Markdown files to the new format:
//! - Front-facing files (.md, .sr, .dx) → Human format
//! - .dx/{markdown,serializer}/*.llm → LLM format (moved from .human)
//! - .dx/{markdown,serializer}/*.machine → Machine format (unchanged)
//! - Remove old .human files

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    println!("🔄 Converting DX files to new Human-on-disk format...\n");

    let workspace_root = std::env::current_dir()?;

    // Statistics
    let mut stats = ConversionStats::default();

    // Step 1: Rename .human files to .llm in .dx folders
    println!("📝 Step 1: Renaming .human files to .llm in .dx folders...");
    rename_human_to_llm(&workspace_root, &mut stats)?;

    // Step 2: Process all .md files (DX Markdown)
    println!("\n📝 Step 2: Processing Markdown files...");
    process_markdown_files(&workspace_root, &mut stats)?;

    // Step 3: Process all .sr and .dx files (DX Serializer)
    println!("\n📦 Step 3: Processing Serializer files...");
    process_serializer_files(&workspace_root, &mut stats)?;

    // Print summary
    println!("\n✅ Conversion Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Renamed .human → .llm: {}", stats.human_renamed);
    println!("\nMarkdown files:");
    println!("  - Converted to human format: {}", stats.md_converted);
    println!("  - Created .llm files: {}", stats.md_llm_created);
    println!("  - Skipped (already human): {}", stats.md_skipped);
    println!("\nSerializer files:");
    println!("  - Converted to human format: {}", stats.sr_converted);
    println!("  - Created .llm files: {}", stats.sr_llm_created);
    println!("  - Skipped (already human): {}", stats.sr_skipped);
    println!(
        "\nTotal files processed: {}",
        stats.md_converted + stats.md_skipped + stats.sr_converted + stats.sr_skipped
    );

    Ok(())
}

#[derive(Default)]
struct ConversionStats {
    human_renamed: usize,
    md_converted: usize,
    md_llm_created: usize,
    md_skipped: usize,
    sr_converted: usize,
    sr_llm_created: usize,
    sr_skipped: usize,
}

fn rename_human_to_llm(root: &Path, stats: &mut ConversionStats) -> io::Result<()> {
    let dx_markdown = root.join(".dx/markdown");
    let dx_serializer = root.join(".dx/serializer");

    for dx_dir in [dx_markdown, dx_serializer] {
        if dx_dir.exists() {
            rename_human_files_in_dir(&dx_dir, stats)?;
        }
    }

    Ok(())
}

fn rename_human_files_in_dir(dir: &Path, stats: &mut ConversionStats) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            rename_human_files_in_dir(&path, stats)?;
        } else if let Some(ext) = path.extension() {
            if ext == "human" {
                let llm_path = path.with_extension("llm");
                fs::rename(&path, &llm_path)?;
                stats.human_renamed += 1;
                println!("  ✓ Renamed: {} → {}", path.display(), llm_path.display());
            }
        }
    }

    Ok(())
}

fn process_markdown_files(root: &Path, stats: &mut ConversionStats) -> io::Result<()> {
    let md_files =
        find_files(root, &["md"], &["node_modules", "target", "trash", "integrations", ".dx"])?;

    for md_path in md_files {
        process_markdown_file(&md_path, root, stats)?;
    }

    Ok(())
}

fn process_markdown_file(
    md_path: &Path,
    root: &Path,
    stats: &mut ConversionStats,
) -> io::Result<()> {
    let content = fs::read_to_string(md_path)?;

    // Check if already in human format
    if is_human_format_markdown(&content) {
        stats.md_skipped += 1;

        // Still need to ensure .llm file exists in .dx folder
        ensure_llm_file_exists(md_path, root, &content, "markdown")?;
        return Ok(());
    }

    // Content is in LLM format, convert to human
    let human_content = llm_to_human_markdown(&content);

    // Write human format to the .md file
    fs::write(md_path, &human_content)?;
    stats.md_converted += 1;

    // Create .dx/markdown/{relative-path}/{filename}.llm file
    let relative_path = md_path.strip_prefix(root).unwrap_or(md_path);
    let llm_dir = root.join(".dx/markdown").join(relative_path.parent().unwrap_or(Path::new("")));
    fs::create_dir_all(&llm_dir)?;

    let filename = md_path.file_stem().unwrap().to_str().unwrap();
    let llm_path = llm_dir.join(format!("{}.llm", filename));
    fs::write(&llm_path, &content)?; // Original LLM content
    stats.md_llm_created += 1;

    println!("  ✓ {}", relative_path.display());

    Ok(())
}

fn ensure_llm_file_exists(
    file_path: &Path,
    root: &Path,
    content: &str,
    folder: &str,
) -> io::Result<()> {
    let relative_path = file_path.strip_prefix(root).unwrap_or(file_path);
    let llm_dir = root
        .join(format!(".dx/{}", folder))
        .join(relative_path.parent().unwrap_or(Path::new("")));
    fs::create_dir_all(&llm_dir)?;

    let filename = file_path.file_stem().unwrap().to_str().unwrap();
    let llm_path = llm_dir.join(format!("{}.llm", filename));

    if !llm_path.exists() {
        // Convert human to LLM and save
        let llm_content = if folder == "markdown" {
            human_to_llm_markdown(content)
        } else {
            human_to_llm_serializer(content)
        };
        fs::write(&llm_path, &llm_content)?;
    }

    Ok(())
}

fn process_serializer_files(root: &Path, stats: &mut ConversionStats) -> io::Result<()> {
    let sr_files = find_files(
        root,
        &["sr", "dx"],
        &["node_modules", "target", "trash", "integrations", ".dx"],
    )?;

    for sr_path in sr_files {
        // Skip files without extension named 'dx' (binary files)
        if sr_path.extension().is_none() && sr_path.file_name().unwrap() == "dx" {
            continue;
        }

        process_serializer_file(&sr_path, root, stats)?;
    }

    Ok(())
}

fn process_serializer_file(
    sr_path: &Path,
    root: &Path,
    stats: &mut ConversionStats,
) -> io::Result<()> {
    let content = fs::read_to_string(sr_path)?;

    // Check if already in human format
    if is_human_format_serializer(&content) {
        stats.sr_skipped += 1;

        // Still need to ensure .llm file exists in .dx folder
        ensure_llm_file_exists(sr_path, root, &content, "serializer")?;
        return Ok(());
    }

    // Content is in LLM format, convert to human
    let human_content = llm_to_human_serializer(&content);

    // Write human format to the .sr/.dx file
    fs::write(sr_path, &human_content)?;
    stats.sr_converted += 1;

    // Create .dx/serializer/{relative-path}/{filename}.llm file
    let relative_path = sr_path.strip_prefix(root).unwrap_or(sr_path);
    let llm_dir = root
        .join(".dx/serializer")
        .join(relative_path.parent().unwrap_or(Path::new("")));
    fs::create_dir_all(&llm_dir)?;

    let filename = sr_path.file_stem().unwrap().to_str().unwrap();
    let llm_path = llm_dir.join(format!("{}.llm", filename));
    fs::write(&llm_path, &content)?; // Original LLM content
    stats.sr_llm_created += 1;

    println!("  ✓ {}", relative_path.display());

    Ok(())
}

fn find_files(root: &Path, extensions: &[&str], exclude_dirs: &[&str]) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    find_files_recursive(root, root, extensions, exclude_dirs, &mut files)?;
    Ok(files)
}

fn find_files_recursive(
    root: &Path,
    current: &Path,
    extensions: &[&str],
    exclude_dirs: &[&str],
    files: &mut Vec<PathBuf>,
) -> io::Result<()> {
    if !current.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap();
            if !exclude_dirs.contains(&dir_name) {
                find_files_recursive(root, &path, extensions, exclude_dirs, files)?;
            }
        } else if let Some(ext) = path.extension() {
            if extensions.contains(&ext.to_str().unwrap()) {
                files.push(path);
            }
        } else {
            // Check for files without extension (like 'dx')
            if let Some(name) = path.file_name() {
                if extensions.contains(&name.to_str().unwrap()) {
                    files.push(path);
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// Format Detection
// ============================================================================

fn is_human_format_markdown(content: &str) -> bool {
    // Human format has:
    // - Multiple blank lines
    // - Padded spacing
    // - Full words
    content.contains("\n\n\n")
        || content.contains("  ") && content.len() > 500 && content.matches('\n').count() > 20
}

fn is_human_format_serializer(content: &str) -> bool {
    // Human format has:
    // - Indentation (2+ spaces at line start)
    // - key = value with spacing
    // - Section headers with blank lines
    let has_indentation =
        content.lines().any(|line| line.starts_with("  ") && !line.trim().is_empty());
    let has_spaced_equals = content.contains(" = ");

    has_indentation || has_spaced_equals
}

// ============================================================================
// Conversion Functions (Simplified)
// ============================================================================

fn llm_to_human_markdown(llm: &str) -> String {
    // Add spacing and formatting
    let mut human = String::with_capacity(llm.len() * 2);
    let mut prev_was_header = false;

    for line in llm.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            // Headers get extra spacing
            if !prev_was_header {
                human.push('\n');
            }
            human.push_str(line);
            human.push_str("\n\n");
            prev_was_header = true;
        } else if trimmed.is_empty() {
            human.push('\n');
            prev_was_header = false;
        } else {
            human.push_str(line);
            human.push('\n');
            prev_was_header = false;
        }
    }

    human
}

fn human_to_llm_markdown(human: &str) -> String {
    // Remove extra spacing
    let mut llm = String::with_capacity(human.len());
    let mut prev_empty = false;

    for line in human.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !prev_empty {
                llm.push('\n');
                prev_empty = true;
            }
        } else {
            llm.push_str(trimmed);
            llm.push('\n');
            prev_empty = false;
        }
    }

    llm.trim_end().to_string()
}

fn llm_to_human_serializer(llm: &str) -> String {
    // Add indentation and spacing
    let mut human = String::with_capacity(llm.len() * 2);
    let mut indent_level: i32 = 0;

    for line in llm.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            human.push('\n');
            continue;
        }

        // Adjust indent for closing brackets
        if trimmed.starts_with(']') || trimmed.starts_with(')') {
            indent_level = indent_level.saturating_sub(1);
        }

        // Add indentation
        for _ in 0..indent_level {
            human.push_str("  ");
        }

        // Add spacing around =
        if let Some(eq_pos) = trimmed.find('=') {
            let (key, value) = trimmed.split_at(eq_pos);
            human.push_str(key.trim());
            human.push_str(" = ");
            human.push_str(value[1..].trim());
        } else {
            human.push_str(trimmed);
        }
        human.push('\n');

        // Adjust indent for opening brackets
        if trimmed.ends_with('[') || trimmed.ends_with('(') {
            indent_level += 1;
        }
    }

    human
}

fn human_to_llm_serializer(human: &str) -> String {
    // Remove indentation and extra spacing
    let mut llm = String::with_capacity(human.len());

    for line in human.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Remove spacing around =
        if let Some(eq_pos) = trimmed.find('=') {
            let (key, value) = trimmed.split_at(eq_pos);
            llm.push_str(key.trim());
            llm.push('=');
            llm.push_str(value[1..].trim());
        } else {
            llm.push_str(trimmed);
        }
        llm.push('\n');
    }

    llm.trim_end().to_string()
}
