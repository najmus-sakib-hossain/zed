//! Tree command - Display directory structure with file sizes and code statistics
//!
//! Features:
//! - Parallel directory traversal using rayon
//! - Code statistics (lines of code, comments) using tokei
//! - Smart filtering (ignores .git, node_modules, target, etc.)
//! - Color-coded output with file sizes
//! - Folder summaries with file counts and total sizes

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use human_bytes::human_bytes;
use ignore::WalkBuilder;
use owo_colors::OwoColorize;
use rayon::prelude::*;
use tokei::{Config, Languages};

use crate::cli::TreeArgs;
use crate::ui::theme::Theme;

/// Entry in the file tree
#[derive(Debug, Clone)]
struct TreeEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    size: u64,
    depth: usize,
    code_lines: Option<usize>,
    comment_lines: Option<usize>,
    children: Vec<TreeEntry>,
}

/// Statistics for a directory
#[derive(Debug, Default, Clone)]
struct DirStats {
    file_count: usize,
    total_size: u64,
    code_lines: usize,
    comment_lines: usize,
}

pub async fn run(args: TreeArgs, _theme: &Theme) -> Result<()> {
    let path = args.path.as_deref().unwrap_or(Path::new("."));
    let path = fs::canonicalize(path)?;

    // Clean up Windows UNC path prefix
    let path_str = path.display().to_string();
    let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);

    eprintln!("{} {}", "path:".bright_white().bold(), clean_path.cyan());
    eprintln!();

    // Build the tree - always analyze code
    let tree = build_tree(&path, args.depth, args.all, true)?;

    // Print the tree
    print_tree(&tree, "", true, args.size);

    eprintln!();
    Ok(())
}

/// Build the directory tree structure
fn build_tree(
    root: &Path,
    max_depth: Option<usize>,
    show_hidden: bool,
    analyze_code: bool,
) -> Result<TreeEntry> {
    let mut builder = WalkBuilder::new(root);
    builder
        .max_depth(max_depth)
        .hidden(!show_hidden)
        .git_ignore(false) // Don't ignore so we can see build folders
        .git_global(false)
        .git_exclude(false);

    // Collect all entries
    let entries: Vec<_> = builder.build().filter_map(|e| e.ok()).collect();

    // Build tree structure
    let root_entry = build_tree_recursive(root, &entries, 0, analyze_code)?;

    Ok(root_entry)
}

fn build_tree_recursive(
    path: &Path,
    all_entries: &[ignore::DirEntry],
    depth: usize,
    analyze_code: bool,
) -> Result<TreeEntry> {
    let metadata = fs::metadata(path)?;
    let is_dir = metadata.is_dir();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

    let mut entry = TreeEntry {
        path: path.to_path_buf(),
        name: name.clone(),
        is_dir,
        size: if is_dir { 0 } else { metadata.len() },
        depth,
        code_lines: None,
        comment_lines: None,
        children: Vec::new(),
    };

    if is_dir {
        // Check if this is a build/dependency folder that should be collapsed
        let is_build_folder = matches!(
            name.as_str(),
            "target"
                | "node_modules"
                | ".git"
                | "dist"
                | "build"
                | "out"
                | ".next"
                | ".venv"
                | "__pycache__"
                | ".cache"
                | ".dx-cache"
                | ".rumdl-cache"
        );

        if is_build_folder {
            // Calculate total size but don't show children
            entry.size = calculate_dir_size(path)?;
            return Ok(entry);
        }

        // Get immediate children
        let mut children: Vec<_> = all_entries
            .iter()
            .filter(|e| e.path().parent() == Some(path) && e.path() != path)
            .collect();

        // Sort children: directories first, then by name
        children.sort_by(|a, b| {
            let a_is_dir = a.path().is_dir();
            let b_is_dir = b.path().is_dir();
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.path().cmp(b.path()),
            }
        });

        // Process children in parallel
        let children_entries: Vec<TreeEntry> = children
            .par_iter()
            .filter_map(|child| {
                build_tree_recursive(child.path(), all_entries, depth + 1, analyze_code).ok()
            })
            .collect();

        // Calculate directory size and stats
        let mut total_size = 0u64;
        for child in &children_entries {
            total_size += child.size;
        }
        entry.size = total_size;
        entry.children = children_entries;
    } else if analyze_code {
        // Analyze code statistics for files
        if let Some((code, comments)) = analyze_file(path) {
            entry.code_lines = Some(code);
            entry.comment_lines = Some(comments);
        }
    }

    Ok(entry)
}

/// Calculate total size of a directory recursively
fn calculate_dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    total += calculate_dir_size(&entry.path()).unwrap_or(0);
                } else {
                    total += metadata.len();
                }
            }
        }
    }

    Ok(total)
}

/// Analyze a file for code and comment statistics
fn analyze_file(path: &Path) -> Option<(usize, usize)> {
    // Check if file has a recognized extension
    let ext = path.extension()?.to_str()?;

    // Skip dx-specific and binary files that cause tokei warnings
    let ignored_extensions = [
        // DX formats
        "dx",
        "sr",
        "bcss",
        "dpb",
        "machine",
        "human",
        "llm",
        // Fonts
        "woff",
        "woff2",
        "ttf",
        "otf",
        // Images
        "png",
        "jpg",
        "jpeg",
        "gif",
        "svg",
        "ico",
        "webp",
        "avif",
        "icns",
        "bmp",
        // Audio
        "mp3",
        "wav",
        "ogg",
        "aac",
        "flac",
        "m4a",
        // Video
        "mp4",
        "webm",
        "avi",
        "mov",
        "mkv",
        // 3D Models
        "glb",
        "gltf",
        "obj",
        "fbx",
        // Archives
        "zip",
        "tar",
        "gz",
        "bz2",
        "xz",
        "7z",
        "rar",
        // Documents
        "pdf",
        "doc",
        "docx",
        "xls",
        "xlsx",
        "ppt",
        "pptx",
        "prose",
        // Data files
        "csv",
        "tsv",
        "parquet",
        // Binaries
        "exe",
        "dll",
        "so",
        "dylib",
        "a",
        "lib",
        "o",
        "bin",
        // Rust build artifacts
        "rlib",
        "rmeta",
        // Lock/config files
        "lock",
        "sum",
        "vsix",
        "map",
        "info",
        "resolved",
        "json5",
        // Web/manifest
        "webmanifest",
        // Test/snapshot files
        "example",
        "proptest-regressions",
        "snap",
        // Type definitions
        "typed",
        // Sandbox
        "sandbox",
        "sandbox-browser",
        "qr-import",
        // Services
        "service",
        "timer",
        // Java
        "jar",
        "properties",
        // Apple
        "plist",
        // Patches
        "patch",
        // HTML (causes MIME warnings)
        "html",
        "htm",
        // Temporary/partial files
        "part",
        "part00",
        "tmp",
        "temp",
        "cache",
        // Metadata/timestamps
        "timestamp",
        "tag",
        "isle",
        // Expression/export files
        "expr",
        "exp",
        // Other
        "wasm",
        "wat",
        // Build files
        "log",
        "xcfilelist",
        // C++ modules
        "cppm",
        "c3",
        // JSON variants
        "jsonl",
        // Other
        "kk",
    ];

    if ignored_extensions.contains(&ext) {
        return None;
    }

    // Also skip files without extensions or with weird names
    if ext.is_empty() || ext.len() > 10 {
        return None;
    }

    let mut languages = Languages::new();
    let config = Config::default();

    languages.get_statistics(&[path], &[], &config);

    let mut total_code = 0;
    let mut total_comments = 0;

    for (_, language) in languages.iter() {
        for report in &language.reports {
            total_code += report.stats.code;
            total_comments += report.stats.comments;
        }
    }

    if total_code > 0 || total_comments > 0 {
        Some((total_code, total_comments))
    } else {
        None
    }
}

/// Print the tree structure
fn print_tree(entry: &TreeEntry, prefix: &str, is_last: bool, sort_by_size: bool) {
    print_tree_internal(entry, prefix, is_last, sort_by_size, true);
}

fn print_tree_internal(
    entry: &TreeEntry,
    prefix: &str,
    is_last: bool,
    sort_by_size: bool,
    is_root: bool,
) {
    // Only the root entry gets no connector
    let (connector, spacing) = if is_root {
        ("", " ")
    } else if is_last {
        ("└── ", "")
    } else {
        ("├── ", "")
    };

    // Check if this is a collapsed build folder
    let is_build_folder = entry.is_dir && entry.children.is_empty() && entry.size > 0;

    let name_colored = if entry.is_dir {
        if is_build_folder {
            format!("{} {}", entry.name, "(collapsed)".bright_black())
                .cyan()
                .bold()
                .to_string()
        } else {
            entry.name.cyan().bold().to_string()
        }
    } else {
        entry.name.white().to_string()
    };

    // Format size
    let size_str = if entry.is_dir {
        if entry.children.is_empty() && !is_build_folder {
            "(empty)".to_string()
        } else if is_build_folder {
            // For collapsed folders, just show size
            "".to_string()
        } else {
            let file_count = count_files(entry);
            format!("({} files)", file_count).green().to_string()
        }
    } else {
        format!("[{}]", human_bytes(entry.size as f64)).bright_black().to_string()
    };

    // Format code stats - always show for files with cyan color
    let stats_str = if !entry.is_dir {
        if let (Some(code), Some(comments)) = (entry.code_lines, entry.comment_lines) {
            format!(" {{code: {}, comments: {}}}", code, comments).cyan().to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Print size in orange/yellow for large directories
    let size_display = if entry.is_dir && entry.size > 1_000_000 {
        format!("[{}]", human_bytes(entry.size as f64))
            .bright_yellow()
            .bold()
            .to_string()
    } else if entry.is_dir {
        format!("[{}]", human_bytes(entry.size as f64)).bright_black().to_string()
    } else {
        size_str.clone()
    };

    eprintln!(
        "{}{}{}{}{}{}",
        prefix,
        connector.bright_black(),
        name_colored,
        spacing,
        if entry.is_dir {
            if is_build_folder {
                size_display
            } else {
                format!("{} {}", size_str, size_display)
            }
        } else {
            size_display
        },
        stats_str
    );

    if entry.is_dir && !entry.children.is_empty() {
        // Root's children should have no prefix, but their children need indentation
        let new_prefix = if is_root {
            // Root level: children start with no prefix
            String::new()
        } else if prefix.is_empty() {
            // First level children: add indentation for their children
            if is_last {
                "    ".to_string()
            } else {
                format!("{}   ", "│".bright_black())
            }
        } else {
            // Nested levels: continue the tree structure
            let continuation = if is_last {
                "    ".to_string()
            } else {
                format!("{}   ", "│".bright_black())
            };
            format!("{}{}", prefix, continuation)
        };

        let mut children = entry.children.clone();
        if sort_by_size {
            children.sort_by(|a, b| b.size.cmp(&a.size));
        }

        for (i, child) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;
            print_tree_internal(child, &new_prefix, is_last_child, sort_by_size, false);
        }
    }
}

/// Count total files in a directory tree
fn count_files(entry: &TreeEntry) -> usize {
    if !entry.is_dir {
        return 1;
    }

    entry.children.iter().map(count_files).sum()
}
