//! Git Integration Commands
//!
//! This module provides a comprehensive Git interface for the DX CLI with:
//! - Status view with interactive staging
//! - Commit interface with message editor
//! - Diff viewer with syntax highlighting
//! - Branch management
//! - Stash operations
//!
//! # Architecture
//!
//! All git operations use a combination of:
//! - Direct git command execution for reliability
//! - Streaming output for large diffs
//! - Zero-copy parsing where possible
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::commands::git::{GitStatus, GitDiff, GitBranch};
//!
//! // Show interactive status
//! let status = GitStatus::new()?;
//! status.show_interactive()?;
//!
//! // View diff
//! let diff = GitDiff::new()?;
//! diff.show_staged()?;
//! ```

pub mod args;
pub mod branch;
pub mod commit;
pub mod diff;
pub mod stash;
pub mod status;

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

// Re-exports for public API
#[allow(unused_imports)]
pub use args::{BranchSubcommands, GitArgs, GitCommands, StashSubcommands};
#[allow(unused_imports)]
pub use branch::{BranchInfo, BranchManager, BranchOperation};
#[allow(unused_imports)]
pub use commit::{CommitInfo, CommitInterface};
#[allow(unused_imports)]
pub use diff::{DiffMode, DiffViewer, HunkInfo};
#[allow(unused_imports)]
pub use stash::{StashEntry, StashManager, StashOperation};
#[allow(unused_imports)]
pub use status::{FileStatus, GitStatus, StatusCategory};

/// Git repository wrapper providing common operations
pub struct GitRepo {
    /// Repository root path
    root: PathBuf,
}

impl GitRepo {
    /// Open a git repository
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Find repository root
        let root = Self::find_repo_root(path)?;

        Ok(Self { root })
    }

    /// Open the current directory's repository
    pub fn open_current() -> Result<Self> {
        Self::open(std::env::current_dir()?)
    }

    /// Find the repository root from a path
    fn find_repo_root(path: &Path) -> Result<PathBuf> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(path)
            .output()
            .context("Failed to execute git")?;

        if !output.status.success() {
            anyhow::bail!("Not a git repository: {}", path.display());
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(PathBuf::from(root))
    }

    /// Get the repository root path
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Execute a git command and return output
    pub fn exec(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git command failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Execute a git command, allowing failure
    pub fn exec_maybe(&self, args: &[&str]) -> Option<String> {
        self.exec(args).ok()
    }

    /// Get current branch name
    pub fn current_branch(&self) -> Result<String> {
        self.exec(&["rev-parse", "--abbrev-ref", "HEAD"]).map(|s| s.trim().to_string())
    }

    /// Get current HEAD commit hash (short)
    pub fn head_short(&self) -> Result<String> {
        self.exec(&["rev-parse", "--short", "HEAD"]).map(|s| s.trim().to_string())
    }

    /// Check if working directory is clean
    pub fn is_clean(&self) -> Result<bool> {
        let output = self.exec(&["status", "--porcelain"])?;
        Ok(output.trim().is_empty())
    }

    /// Check if there are staged changes
    pub fn has_staged(&self) -> Result<bool> {
        let output = self.exec(&["diff", "--cached", "--name-only"])?;
        Ok(!output.trim().is_empty())
    }

    /// Get remote URL for origin
    pub fn remote_url(&self) -> Option<String> {
        self.exec_maybe(&["remote", "get-url", "origin"]).map(|s| s.trim().to_string())
    }
}

/// Git subcommand routing
#[derive(Debug, Clone)]
pub enum GitCommand {
    /// Show status
    Status,
    /// Show diff
    Diff { staged: bool, file: Option<String> },
    /// Commit changes
    Commit {
        message: Option<String>,
        amend: bool,
    },
    /// Branch operations
    Branch(BranchOperation),
    /// Stash operations
    Stash(StashOperation),
    /// Add files
    Add { files: Vec<String>, all: bool },
    /// Reset files
    Reset { files: Vec<String>, hard: bool },
    /// Checkout
    Checkout { target: String },
    /// Pull
    Pull { rebase: bool },
    /// Push
    Push {
        force: bool,
        upstream: Option<String>,
    },
    /// Log
    Log { count: usize },
}

/// Run a git subcommand
pub async fn run(cmd: GitCommand) -> Result<()> {
    let repo = GitRepo::open_current()?;

    match cmd {
        GitCommand::Status => {
            let status = GitStatus::new(&repo)?;
            status.display_interactive()?;
        }
        GitCommand::Diff { staged, file } => {
            let viewer = DiffViewer::new(&repo)?;
            let diffs = if staged {
                if let Some(ref path) = file {
                    viewer.get_file_diff(path, true)?
                } else {
                    viewer.get_staged()?
                }
            } else if let Some(ref path) = file {
                viewer.get_file_diff(path, false)?
            } else {
                viewer.get_unstaged()?
            };
            // Print diff summary
            for diff in &diffs {
                println!("{}  +{} -{}", diff.path, diff.additions, diff.deletions);
            }
        }
        GitCommand::Commit { message, amend } => {
            let interface = CommitInterface::new(&repo)?;
            if amend {
                interface.amend(message.as_deref())?;
            } else if let Some(msg) = message {
                interface.commit(&msg)?;
            } else {
                anyhow::bail!("Commit message required");
            }
        }
        GitCommand::Branch(op) => {
            let manager = BranchManager::new(&repo)?;
            manager.execute(op)?;
        }
        GitCommand::Stash(op) => {
            let manager = StashManager::new(&repo)?;
            manager.execute(op)?;
        }
        GitCommand::Add { files, all } => {
            if all {
                repo.exec(&["add", "-A"])?;
            } else {
                let args: Vec<&str> =
                    std::iter::once("add").chain(files.iter().map(|s| s.as_str())).collect();
                repo.exec(&args)?;
            }
        }
        GitCommand::Reset { files, hard } => {
            if files.is_empty() {
                if hard {
                    repo.exec(&["reset", "--hard"])?;
                } else {
                    repo.exec(&["reset"])?;
                }
            } else {
                let args: Vec<&str> =
                    std::iter::once("reset").chain(files.iter().map(|s| s.as_str())).collect();
                repo.exec(&args)?;
            }
        }
        GitCommand::Checkout { target } => {
            repo.exec(&["checkout", &target])?;
        }
        GitCommand::Pull { rebase } => {
            if rebase {
                repo.exec(&["pull", "--rebase"])?;
            } else {
                repo.exec(&["pull"])?;
            }
        }
        GitCommand::Push { force, upstream } => {
            let mut args = vec!["push"];
            if force {
                args.push("--force");
            }
            if let Some(ref up) = upstream {
                args.push("-u");
                args.push(up);
            }
            repo.exec(&args)?;
        }
        GitCommand::Log { count } => {
            let count_str = count.to_string();
            repo.exec(&["log", "--oneline", "--graph", &format!("-{}", count_str)])?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_repo_detection() {
        // This test assumes we're in a git repository
        if let Ok(repo) = GitRepo::open_current() {
            assert!(repo.root().exists());
            assert!(repo.current_branch().is_ok());
        }
    }
}
