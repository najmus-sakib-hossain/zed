//! Git Status View
//!
//! Interactive status display with staged/unstaged/untracked sections
//! and interactive staging capabilities.
//!
//! # Features
//!
//! - Categorized file display (staged, modified, untracked)
//! - Interactive staging/unstaging
//! - Keyboard navigation
//! - Diff preview on selection
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::commands::git::{GitStatus, GitRepo};
//!
//! let repo = GitRepo::open_current()?;
//! let status = GitStatus::new(&repo)?;
//! status.display_interactive()?;
//! ```

use std::fmt;

use anyhow::Result;
use owo_colors::OwoColorize;

use super::GitRepo;

/// File status in git
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// File is unmodified
    Unmodified,
    /// File has been modified
    Modified,
    /// File has been added (new file)
    Added,
    /// File has been deleted
    Deleted,
    /// File has been renamed
    Renamed,
    /// File has been copied
    Copied,
    /// File is untracked
    Untracked,
    /// File is ignored
    Ignored,
    /// File has merge conflicts
    Conflict,
    /// File type changed
    TypeChange,
}

impl FileStatus {
    /// Parse from git status short format
    pub fn from_status_char(c: char) -> Self {
        match c {
            ' ' => Self::Unmodified,
            'M' => Self::Modified,
            'A' => Self::Added,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            '?' => Self::Untracked,
            '!' => Self::Ignored,
            'U' => Self::Conflict,
            'T' => Self::TypeChange,
            _ => Self::Unmodified,
        }
    }

    /// Get display character
    pub const fn as_char(&self) -> char {
        match self {
            Self::Unmodified => ' ',
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Copied => 'C',
            Self::Untracked => '?',
            Self::Ignored => '!',
            Self::Conflict => 'U',
            Self::TypeChange => 'T',
        }
    }

    /// Get color for this status
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::Modified => (255, 200, 50),    // Yellow
            Self::Added => (100, 255, 100),      // Green
            Self::Deleted => (255, 100, 100),    // Red
            Self::Renamed => (150, 150, 255),    // Blue
            Self::Copied => (150, 150, 255),     // Blue
            Self::Untracked => (180, 180, 180),  // Gray
            Self::Ignored => (100, 100, 100),    // Dark gray
            Self::Conflict => (255, 100, 255),   // Magenta
            Self::TypeChange => (255, 200, 50),  // Yellow
            Self::Unmodified => (255, 255, 255), // White
        }
    }
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

/// Category of files in status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCategory {
    /// Staged changes
    Staged,
    /// Unstaged changes (modified but not staged)
    Unstaged,
    /// Untracked files
    Untracked,
    /// Files with merge conflicts
    Conflicts,
}

impl fmt::Display for StatusCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Staged => write!(f, "Changes to be committed"),
            Self::Unstaged => write!(f, "Changes not staged for commit"),
            Self::Untracked => write!(f, "Untracked files"),
            Self::Conflicts => write!(f, "Unmerged paths"),
        }
    }
}

/// A single file entry in status
#[derive(Debug, Clone)]
pub struct StatusEntry {
    /// File path (relative to repo root)
    pub path: String,
    /// Index status (staged)
    pub index_status: FileStatus,
    /// Working tree status
    pub worktree_status: FileStatus,
    /// Original path (for renames)
    pub orig_path: Option<String>,
}

impl StatusEntry {
    /// Parse from git status porcelain v2 line
    fn from_porcelain_v2(line: &str) -> Option<Self> {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() < 4 {
            return None;
        }

        match chars[0] {
            '1' | '2' => {
                // Regular or rename/copy entry
                let index = FileStatus::from_status_char(chars[2]);
                let worktree = FileStatus::from_status_char(chars[3]);

                // Parse path (after tab)
                let parts: Vec<&str> = line.splitn(9, ' ').collect();
                if parts.len() >= 9 {
                    let path_part = parts[8];
                    let (path, orig_path) = if chars[0] == '2' {
                        // Rename has original path after tab
                        let paths: Vec<&str> = path_part.split('\t').collect();
                        if paths.len() == 2 {
                            (paths[0].to_string(), Some(paths[1].to_string()))
                        } else {
                            (path_part.to_string(), None)
                        }
                    } else {
                        (path_part.to_string(), None)
                    };

                    Some(Self {
                        path,
                        index_status: index,
                        worktree_status: worktree,
                        orig_path,
                    })
                } else {
                    None
                }
            }
            '?' => {
                // Untracked file
                let path = line[2..].to_string();
                Some(Self {
                    path,
                    index_status: FileStatus::Untracked,
                    worktree_status: FileStatus::Untracked,
                    orig_path: None,
                })
            }
            '!' => {
                // Ignored file
                let path = line[2..].to_string();
                Some(Self {
                    path,
                    index_status: FileStatus::Ignored,
                    worktree_status: FileStatus::Ignored,
                    orig_path: None,
                })
            }
            'u' => {
                // Merge conflict
                let parts: Vec<&str> = line.splitn(11, ' ').collect();
                if parts.len() >= 11 {
                    Some(Self {
                        path: parts[10].to_string(),
                        index_status: FileStatus::Conflict,
                        worktree_status: FileStatus::Conflict,
                        orig_path: None,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Parse from git status porcelain v1 (short format)
    fn from_porcelain_v1(line: &str) -> Option<Self> {
        if line.len() < 4 {
            return None;
        }

        let chars: Vec<char> = line.chars().collect();
        let index = FileStatus::from_status_char(chars[0]);
        let worktree = FileStatus::from_status_char(chars[1]);
        let path = line[3..].trim_start().to_string();

        // Handle rename
        let (path, orig_path) = if path.contains(" -> ") {
            let parts: Vec<&str> = path.split(" -> ").collect();
            (parts[1].to_string(), Some(parts[0].to_string()))
        } else {
            (path, None)
        };

        Some(Self {
            path,
            index_status: index,
            worktree_status: worktree,
            orig_path,
        })
    }

    /// Get the category for this entry
    pub fn category(&self) -> StatusCategory {
        if self.index_status == FileStatus::Conflict || self.worktree_status == FileStatus::Conflict
        {
            StatusCategory::Conflicts
        } else if self.index_status == FileStatus::Untracked {
            StatusCategory::Untracked
        } else if self.index_status != FileStatus::Unmodified {
            StatusCategory::Staged
        } else {
            StatusCategory::Unstaged
        }
    }

    /// Is this entry staged?
    pub fn is_staged(&self) -> bool {
        self.index_status != FileStatus::Unmodified
            && self.index_status != FileStatus::Untracked
            && self.index_status != FileStatus::Ignored
    }

    /// Is this entry modified in working tree?
    pub fn is_modified(&self) -> bool {
        self.worktree_status != FileStatus::Unmodified
    }
}

/// Git status with categorized entries
pub struct GitStatus<'a> {
    /// Reference to the repository
    repo: &'a GitRepo,
    /// All status entries
    entries: Vec<StatusEntry>,
    /// Current branch name
    branch: String,
    /// Upstream branch (if tracking)
    upstream: Option<String>,
    /// Commits ahead of upstream
    ahead: u32,
    /// Commits behind upstream
    behind: u32,
}

impl<'a> GitStatus<'a> {
    /// Create new status from repository
    pub fn new(repo: &'a GitRepo) -> Result<Self> {
        let mut status = Self {
            repo,
            entries: Vec::new(),
            branch: String::new(),
            upstream: None,
            ahead: 0,
            behind: 0,
        };

        status.refresh()?;
        Ok(status)
    }

    /// Refresh status from git
    pub fn refresh(&mut self) -> Result<()> {
        // Get branch info
        self.branch = self.repo.current_branch().unwrap_or_else(|_| "HEAD".into());

        // Get upstream tracking
        if let Ok(upstream) = self.repo.exec(&[
            "rev-parse",
            "--abbrev-ref",
            &format!("{}@{{upstream}}", self.branch),
        ]) {
            self.upstream = Some(upstream.trim().to_string());

            // Get ahead/behind
            if let Ok(counts) = self.repo.exec(&[
                "rev-list",
                "--count",
                "--left-right",
                &format!("{}...{}", self.branch, self.upstream.as_ref().unwrap()),
            ]) {
                let parts: Vec<&str> = counts.split_whitespace().collect();
                if parts.len() == 2 {
                    self.ahead = parts[0].parse().unwrap_or(0);
                    self.behind = parts[1].parse().unwrap_or(0);
                }
            }
        }

        // Get status entries
        let output = self.repo.exec(&["status", "--porcelain=v2", "-uall"])?;

        self.entries.clear();
        for line in output.lines() {
            if line.starts_with('#') {
                continue; // Skip header lines
            }
            if let Some(entry) = StatusEntry::from_porcelain_v2(line) {
                self.entries.push(entry);
            }
        }

        Ok(())
    }

    /// Get entries by category
    pub fn by_category(&self, category: StatusCategory) -> Vec<&StatusEntry> {
        self.entries.iter().filter(|e| e.category() == category).collect()
    }

    /// Get all staged entries
    pub fn staged(&self) -> Vec<&StatusEntry> {
        self.by_category(StatusCategory::Staged)
    }

    /// Get all unstaged entries
    pub fn unstaged(&self) -> Vec<&StatusEntry> {
        self.by_category(StatusCategory::Unstaged)
    }

    /// Get all untracked entries
    pub fn untracked(&self) -> Vec<&StatusEntry> {
        self.by_category(StatusCategory::Untracked)
    }

    /// Get all conflict entries
    pub fn conflicts(&self) -> Vec<&StatusEntry> {
        self.by_category(StatusCategory::Conflicts)
    }

    /// Is working directory clean?
    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    /// Stage a file
    pub fn stage(&self, path: &str) -> Result<()> {
        self.repo.exec(&["add", "--", path])?;
        Ok(())
    }

    /// Unstage a file
    pub fn unstage(&self, path: &str) -> Result<()> {
        self.repo.exec(&["reset", "HEAD", "--", path])?;
        Ok(())
    }

    /// Discard changes in working directory
    pub fn discard(&self, path: &str) -> Result<()> {
        self.repo.exec(&["checkout", "--", path])?;
        Ok(())
    }

    /// Stage all changes
    pub fn stage_all(&self) -> Result<()> {
        self.repo.exec(&["add", "-A"])?;
        Ok(())
    }

    /// Unstage all changes
    pub fn unstage_all(&self) -> Result<()> {
        self.repo.exec(&["reset", "HEAD"])?;
        Ok(())
    }

    /// Display status in simple format
    pub fn display(&self) -> Result<()> {
        println!();

        // Header
        let branch_display = if let Some(ref upstream) = self.upstream {
            let mut extra = String::new();
            if self.ahead > 0 {
                extra.push_str(&format!(" ↑{}", self.ahead));
            }
            if self.behind > 0 {
                extra.push_str(&format!(" ↓{}", self.behind));
            }
            format!(
                "On branch {} → {}{}",
                self.branch.bright_green().bold(),
                upstream.bright_blue(),
                extra.yellow()
            )
        } else {
            format!("On branch {}", self.branch.bright_green().bold())
        };
        println!("  {}", branch_display);
        println!();

        if self.is_clean() {
            println!("  {}", "Nothing to commit, working tree clean".bright_green());
            println!();
            return Ok(());
        }

        // Conflicts
        let conflicts = self.conflicts();
        if !conflicts.is_empty() {
            println!(
                "  {} (fix conflicts and run \"git commit\")",
                StatusCategory::Conflicts.to_string().bright_red().bold()
            );
            for entry in conflicts {
                println!("      {} {}", "both modified:".bright_red(), entry.path.white());
            }
            println!();
        }

        // Staged
        let staged = self.staged();
        if !staged.is_empty() {
            println!(
                "  {} (use \"git restore --staged <file>...\" to unstage)",
                StatusCategory::Staged.to_string().bright_green().bold()
            );
            for entry in staged {
                let (r, g, b) = entry.index_status.color();
                let status_str = match entry.index_status {
                    FileStatus::Added => "new file:  ".to_string(),
                    FileStatus::Modified => "modified:  ".to_string(),
                    FileStatus::Deleted => "deleted:   ".to_string(),
                    FileStatus::Renamed => {
                        if let Some(ref orig) = entry.orig_path {
                            format!("renamed:   {} -> ", orig)
                        } else {
                            "renamed:   ".to_string()
                        }
                    }
                    _ => format!("{}: ", entry.index_status.as_char()),
                };
                println!(
                    "      {}{}",
                    status_str.truecolor(r, g, b),
                    entry.path.truecolor(r, g, b)
                );
            }
            println!();
        }

        // Unstaged
        let unstaged = self.unstaged();
        if !unstaged.is_empty() {
            println!(
                "  {} (use \"git add <file>...\" to stage)",
                StatusCategory::Unstaged.to_string().bright_yellow().bold()
            );
            for entry in unstaged {
                let (r, g, b) = entry.worktree_status.color();
                let status_str = match entry.worktree_status {
                    FileStatus::Modified => "modified:  ".to_string(),
                    FileStatus::Deleted => "deleted:   ".to_string(),
                    FileStatus::TypeChange => "typechange:".to_string(),
                    _ => format!("{}: ", entry.worktree_status.as_char()),
                };
                println!(
                    "      {}{}",
                    status_str.truecolor(r, g, b),
                    entry.path.truecolor(r, g, b)
                );
            }
            println!();
        }

        // Untracked
        let untracked = self.untracked();
        if !untracked.is_empty() {
            println!(
                "  {} (use \"git add <file>...\" to include)",
                StatusCategory::Untracked.to_string().bright_black().bold()
            );
            for entry in untracked {
                println!("      {}", entry.path.bright_black());
            }
            println!();
        }

        Ok(())
    }

    /// Display interactive status (with selection)
    pub fn display_interactive(&self) -> Result<()> {
        // For now, display simple status
        // TODO: Implement full TUI with selection
        self.display()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_status_parsing() {
        assert_eq!(FileStatus::from_status_char('M'), FileStatus::Modified);
        assert_eq!(FileStatus::from_status_char('A'), FileStatus::Added);
        assert_eq!(FileStatus::from_status_char('D'), FileStatus::Deleted);
        assert_eq!(FileStatus::from_status_char('?'), FileStatus::Untracked);
    }

    #[test]
    fn test_status_entry_porcelain_v1() {
        let entry = StatusEntry::from_porcelain_v1("M  src/main.rs").unwrap();
        assert_eq!(entry.path, "src/main.rs");
        assert_eq!(entry.index_status, FileStatus::Modified);

        let entry = StatusEntry::from_porcelain_v1("?? new_file.txt").unwrap();
        assert_eq!(entry.path, "new_file.txt");
        assert_eq!(entry.index_status, FileStatus::Untracked);
    }

    #[test]
    fn test_status_category() {
        let staged = StatusEntry {
            path: "test.rs".to_string(),
            index_status: FileStatus::Modified,
            worktree_status: FileStatus::Unmodified,
            orig_path: None,
        };
        assert_eq!(staged.category(), StatusCategory::Staged);

        let unstaged = StatusEntry {
            path: "test.rs".to_string(),
            index_status: FileStatus::Unmodified,
            worktree_status: FileStatus::Modified,
            orig_path: None,
        };
        assert_eq!(unstaged.category(), StatusCategory::Unstaged);

        let untracked = StatusEntry {
            path: "test.rs".to_string(),
            index_status: FileStatus::Untracked,
            worktree_status: FileStatus::Untracked,
            orig_path: None,
        };
        assert_eq!(untracked.category(), StatusCategory::Untracked);
    }
}
