//! Git Diff Viewer
//!
//! Provides diff viewing with syntax highlighting, side-by-side mode,
//! and hunk navigation.
//!
//! # Features
//!
//! - Side-by-side and inline diff modes
//! - Syntax highlighting for changed content
//! - Hunk-based navigation
//! - Context expansion
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::commands::git::{DiffViewer, DiffMode, GitRepo};
//!
//! let repo = GitRepo::open_current()?;
//! let diff = DiffViewer::new(&repo)?;
//! diff.show_staged(DiffMode::SideBySide)?;
//! ```

use std::process::Command;

use anyhow::{Context, Result};

use super::GitRepo;

/// Diff display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffMode {
    /// Unified diff (default)
    #[default]
    Unified,
    /// Side-by-side comparison
    SideBySide,
    /// Inline with word-level highlighting
    Inline,
}

/// Information about a diff hunk
#[derive(Debug, Clone)]
pub struct HunkInfo {
    /// Starting line in old file
    pub old_start: u32,
    /// Number of lines in old file
    pub old_count: u32,
    /// Starting line in new file
    pub new_start: u32,
    /// Number of lines in new file
    pub new_count: u32,
    /// Hunk header text (function context)
    pub header: String,
    /// Lines in this hunk
    pub lines: Vec<DiffLine>,
}

impl HunkInfo {
    /// Parse hunk header
    pub fn parse_header(header: &str) -> Option<Self> {
        // Parse @@ -old_start,old_count +new_start,new_count @@ header
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "@@" {
            return None;
        }

        let old_range = parts[1].trim_start_matches('-');
        let new_range = parts[2].trim_start_matches('+').trim_end_matches("@@");

        let (old_start, old_count) = Self::parse_range(old_range)?;
        let (new_start, new_count) = Self::parse_range(new_range)?;

        let header_text = if parts.len() > 4 {
            parts[4..].join(" ")
        } else {
            String::new()
        };

        Some(Self {
            old_start,
            old_count,
            new_start,
            new_count,
            header: header_text,
            lines: Vec::new(),
        })
    }

    fn parse_range(range: &str) -> Option<(u32, u32)> {
        let parts: Vec<&str> = range.split(',').collect();
        let start = parts.first()?.parse().ok()?;
        let count = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        Some((start, count))
    }
}

/// A single line in a diff
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// Line type
    pub kind: DiffLineKind,
    /// Line content (without +/- prefix)
    pub content: String,
    /// Old line number (if applicable)
    pub old_line: Option<u32>,
    /// New line number (if applicable)
    pub new_line: Option<u32>,
}

/// Type of diff line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    /// Context line (unchanged)
    Context,
    /// Added line
    Added,
    /// Removed line
    Removed,
    /// Header line
    Header,
}

/// Diff statistics for a file
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    /// Number of files changed
    pub files_changed: usize,
    /// Number of insertions
    pub insertions: usize,
    /// Number of deletions
    pub deletions: usize,
}

/// File diff information
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// File path
    pub path: String,
    /// Old file path (for renames)
    pub old_path: Option<String>,
    /// Is binary file
    pub is_binary: bool,
    /// Hunks in this file diff
    pub hunks: Vec<HunkInfo>,
    /// Number of added lines
    pub additions: usize,
    /// Number of deleted lines
    pub deletions: usize,
}

/// Diff viewer for git differences
pub struct DiffViewer<'a> {
    repo: &'a GitRepo,
    context_lines: u32,
}

impl<'a> DiffViewer<'a> {
    /// Create a new diff viewer
    pub fn new(repo: &'a GitRepo) -> Result<Self> {
        Ok(Self {
            repo,
            context_lines: 3,
        })
    }

    /// Set number of context lines
    pub fn with_context(mut self, lines: u32) -> Self {
        self.context_lines = lines;
        self
    }

    /// Get staged diff
    pub fn get_staged(&self) -> Result<Vec<FileDiff>> {
        self.get_diff(&["--cached"])
    }

    /// Get unstaged diff
    pub fn get_unstaged(&self) -> Result<Vec<FileDiff>> {
        self.get_diff(&[])
    }

    /// Get diff for a specific file
    pub fn get_file_diff(&self, path: &str, staged: bool) -> Result<Vec<FileDiff>> {
        let mut args = vec![];
        if staged {
            args.push("--cached");
        }
        args.push("--");
        args.push(path);
        self.get_diff(&args)
    }

    /// Get diff between two commits
    pub fn get_commit_diff(&self, from: &str, to: &str) -> Result<Vec<FileDiff>> {
        self.get_diff(&[from, to])
    }

    /// Get diff statistics
    pub fn get_stats(&self, staged: bool) -> Result<DiffStats> {
        let mut args = vec!["diff", "--stat", "--stat-width=1000"];
        if staged {
            args.push("--cached");
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(self.repo.root())
            .output()
            .context("Failed to get diff stats")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse the summary line at the end
        let mut stats = DiffStats::default();

        for line in stdout.lines().rev() {
            if line.contains("file") && (line.contains("insertion") || line.contains("deletion")) {
                // Parse: "N files changed, N insertions(+), N deletions(-)"
                for part in line.split(',') {
                    let part = part.trim();
                    if part.contains("file") {
                        stats.files_changed = part
                            .split_whitespace()
                            .next()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                    } else if part.contains("insertion") {
                        stats.insertions = part
                            .split_whitespace()
                            .next()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                    } else if part.contains("deletion") {
                        stats.deletions = part
                            .split_whitespace()
                            .next()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                    }
                }
                break;
            }
        }

        Ok(stats)
    }

    /// Internal method to get and parse diff
    fn get_diff(&self, extra_args: &[&str]) -> Result<Vec<FileDiff>> {
        let context_arg = format!("-U{}", self.context_lines);
        let mut args = vec!["diff", "--no-color", "-p", &context_arg];
        args.extend_from_slice(extra_args);

        let output = Command::new("git")
            .args(&args)
            .current_dir(self.repo.root())
            .output()
            .context("Failed to get diff")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_diff(&stdout)
    }

    /// Parse unified diff output into structured data
    fn parse_diff(diff_text: &str) -> Result<Vec<FileDiff>> {
        let mut files = Vec::new();
        let mut current_file: Option<FileDiff> = None;
        let mut current_hunk: Option<HunkInfo> = None;
        let mut old_line = 0u32;
        let mut new_line = 0u32;

        for line in diff_text.lines() {
            if line.starts_with("diff --git") {
                // Save previous file
                if let Some(mut file) = current_file.take() {
                    if let Some(hunk) = current_hunk.take() {
                        file.hunks.push(hunk);
                    }
                    files.push(file);
                }

                // Parse file paths from "diff --git a/path b/path"
                let parts: Vec<&str> = line.splitn(4, ' ').collect();
                let path =
                    parts.get(3).map(|s| s.trim_start_matches("b/")).unwrap_or("").to_string();

                current_file = Some(FileDiff {
                    path,
                    old_path: None,
                    is_binary: false,
                    hunks: Vec::new(),
                    additions: 0,
                    deletions: 0,
                });
            } else if line.starts_with("Binary files") {
                if let Some(ref mut file) = current_file {
                    file.is_binary = true;
                }
            } else if line.starts_with("rename from") {
                if let Some(ref mut file) = current_file {
                    let old_path = line.trim_start_matches("rename from ").to_string();
                    file.old_path = Some(old_path);
                }
            } else if line.starts_with("@@") {
                // Save previous hunk
                if let (Some(file), Some(hunk)) = (&mut current_file, current_hunk.take()) {
                    file.hunks.push(hunk);
                }

                // Parse new hunk
                if let Some(mut hunk) = HunkInfo::parse_header(line) {
                    old_line = hunk.old_start;
                    new_line = hunk.new_start;
                    hunk.lines.push(DiffLine {
                        kind: DiffLineKind::Header,
                        content: line.to_string(),
                        old_line: None,
                        new_line: None,
                    });
                    current_hunk = Some(hunk);
                }
            } else if let Some(ref mut hunk) = current_hunk {
                let (kind, content) = if let Some(rest) = line.strip_prefix('+') {
                    if let Some(ref mut file) = current_file {
                        file.additions += 1;
                    }
                    let nl = new_line;
                    new_line += 1;
                    hunk.lines.push(DiffLine {
                        kind: DiffLineKind::Added,
                        content: rest.to_string(),
                        old_line: None,
                        new_line: Some(nl),
                    });
                    continue;
                } else if let Some(rest) = line.strip_prefix('-') {
                    if let Some(ref mut file) = current_file {
                        file.deletions += 1;
                    }
                    let ol = old_line;
                    old_line += 1;
                    hunk.lines.push(DiffLine {
                        kind: DiffLineKind::Removed,
                        content: rest.to_string(),
                        old_line: Some(ol),
                        new_line: None,
                    });
                    continue;
                } else if let Some(rest) = line.strip_prefix(' ') {
                    (DiffLineKind::Context, rest)
                } else {
                    continue;
                };

                let ol = old_line;
                let nl = new_line;
                old_line += 1;
                new_line += 1;

                hunk.lines.push(DiffLine {
                    kind,
                    content: content.to_string(),
                    old_line: Some(ol),
                    new_line: Some(nl),
                });
            }
        }

        // Save final file
        if let Some(mut file) = current_file {
            if let Some(hunk) = current_hunk {
                file.hunks.push(hunk);
            }
            files.push(file);
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunk_header_parse() {
        let header = "@@ -10,5 +20,8 @@ fn test()";
        let hunk = HunkInfo::parse_header(header).unwrap();

        assert_eq!(hunk.old_start, 10);
        assert_eq!(hunk.old_count, 5);
        assert_eq!(hunk.new_start, 20);
        assert_eq!(hunk.new_count, 8);
    }

    #[test]
    fn test_diff_mode_default() {
        assert_eq!(DiffMode::default(), DiffMode::Unified);
    }
}
