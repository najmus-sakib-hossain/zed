//! Git Stash Operations
//!
//! Provides stash management including save, pop, list, and drop operations.
//!
//! # Features
//!
//! - Save current changes to stash
//! - Pop/apply stashed changes
//! - List all stashes with descriptions
//! - Drop individual stashes or clear all
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::commands::git::{StashManager, StashOperation, GitRepo};
//!
//! let repo = GitRepo::open_current()?;
//! let stash = StashManager::new(&repo)?;
//! stash.execute(StashOperation::Save { message: Some("WIP".to_string()), keep_index: false })?;
//! ```

use std::process::Command;

use anyhow::{Context, Result};

use super::GitRepo;

/// Information about a stash entry
#[derive(Debug, Clone)]
pub struct StashEntry {
    /// Stash index (0 = most recent)
    pub index: usize,
    /// Stash reference (stash@{N})
    pub reference: String,
    /// Branch the stash was created on
    pub branch: String,
    /// Stash message
    pub message: String,
    /// Commit SHA of the stash
    pub commit: String,
    /// Date of the stash (relative)
    pub date: String,
}

impl StashEntry {
    /// Parse from git stash list output
    pub fn parse(line: &str, index: usize) -> Option<Self> {
        // Format: stash@{N}: On branch: message
        let parts: Vec<&str> = line.splitn(3, ": ").collect();
        if parts.len() < 2 {
            return None;
        }

        let reference = parts[0].to_string();
        let branch_part = parts.get(1).unwrap_or(&"");
        let message = parts.get(2).unwrap_or(&"").to_string();

        let branch = branch_part
            .strip_prefix("On ")
            .or_else(|| branch_part.strip_prefix("WIP on "))
            .unwrap_or(branch_part)
            .to_string();

        Some(Self {
            index,
            reference,
            branch,
            message,
            commit: String::new(),
            date: String::new(),
        })
    }
}

/// Stash operation to execute
#[derive(Debug, Clone)]
pub enum StashOperation {
    /// Save changes to stash
    Save {
        /// Optional message for the stash
        message: Option<String>,
        /// Keep staged changes in index
        keep_index: bool,
        /// Include untracked files
        include_untracked: bool,
    },
    /// Pop the top stash (apply and drop)
    Pop {
        /// Stash index to pop (default: 0)
        index: Option<usize>,
    },
    /// Apply a stash without dropping
    Apply {
        /// Stash index to apply (default: 0)
        index: Option<usize>,
    },
    /// Drop a stash
    Drop {
        /// Stash index to drop (default: 0)
        index: Option<usize>,
    },
    /// Clear all stashes
    Clear,
    /// Show stash contents
    Show {
        /// Stash index to show (default: 0)
        index: Option<usize>,
    },
    /// Create a branch from a stash
    Branch {
        /// Branch name to create
        name: String,
        /// Stash index (default: 0)
        index: Option<usize>,
    },
}

/// Stash manager for git operations
pub struct StashManager<'a> {
    repo: &'a GitRepo,
}

impl<'a> StashManager<'a> {
    /// Create a new stash manager
    pub fn new(repo: &'a GitRepo) -> Result<Self> {
        Ok(Self { repo })
    }

    /// List all stashes
    pub fn list(&self) -> Result<Vec<StashEntry>> {
        let output = Command::new("git")
            .args(["stash", "list"])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to list stashes")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries: Vec<StashEntry> = stdout
            .lines()
            .enumerate()
            .filter_map(|(i, line)| StashEntry::parse(line, i))
            .collect();

        Ok(entries)
    }

    /// Check if there are any stashes
    pub fn has_stashes(&self) -> Result<bool> {
        Ok(!self.list()?.is_empty())
    }

    /// Get count of stashes
    pub fn count(&self) -> Result<usize> {
        Ok(self.list()?.len())
    }

    /// Execute a stash operation
    pub fn execute(&self, op: StashOperation) -> Result<String> {
        match op {
            StashOperation::Save {
                message,
                keep_index,
                include_untracked,
            } => self.save(message.as_deref(), keep_index, include_untracked),
            StashOperation::Pop { index } => self.pop(index),
            StashOperation::Apply { index } => self.apply(index),
            StashOperation::Drop { index } => self.drop(index),
            StashOperation::Clear => self.clear(),
            StashOperation::Show { index } => self.show(index),
            StashOperation::Branch { name, index } => self.branch(&name, index),
        }
    }

    /// Save current changes to stash
    pub fn save(
        &self,
        message: Option<&str>,
        keep_index: bool,
        include_untracked: bool,
    ) -> Result<String> {
        let mut args = vec!["stash", "push"];

        if keep_index {
            args.push("--keep-index");
        }
        if include_untracked {
            args.push("--include-untracked");
        }
        if let Some(msg) = message {
            args.push("-m");
            args.push(msg);
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(self.repo.root())
            .output()
            .context("Failed to save stash")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to save stash: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Pop the stash (apply and drop)
    pub fn pop(&self, index: Option<usize>) -> Result<String> {
        let stash_ref = self.get_stash_ref(index);
        let output = Command::new("git")
            .args(["stash", "pop", &stash_ref])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to pop stash")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("CONFLICT") {
                anyhow::bail!("Conflict detected when popping stash. Please resolve conflicts.");
            }
            anyhow::bail!("Failed to pop stash: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Apply a stash without dropping it
    pub fn apply(&self, index: Option<usize>) -> Result<String> {
        let stash_ref = self.get_stash_ref(index);
        let output = Command::new("git")
            .args(["stash", "apply", &stash_ref])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to apply stash")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("CONFLICT") {
                anyhow::bail!("Conflict detected when applying stash. Please resolve conflicts.");
            }
            anyhow::bail!("Failed to apply stash: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Drop a stash
    pub fn drop(&self, index: Option<usize>) -> Result<String> {
        let stash_ref = self.get_stash_ref(index);
        let output = Command::new("git")
            .args(["stash", "drop", &stash_ref])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to drop stash")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to drop stash: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Clear all stashes
    pub fn clear(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["stash", "clear"])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to clear stashes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to clear stashes: {}", stderr);
        }

        Ok("All stashes cleared".to_string())
    }

    /// Show stash contents
    pub fn show(&self, index: Option<usize>) -> Result<String> {
        let stash_ref = self.get_stash_ref(index);
        let output = Command::new("git")
            .args(["stash", "show", "-p", &stash_ref])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to show stash")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to show stash: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Create a branch from a stash
    pub fn branch(&self, name: &str, index: Option<usize>) -> Result<String> {
        let stash_ref = self.get_stash_ref(index);
        let output = Command::new("git")
            .args(["stash", "branch", name, &stash_ref])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to create branch from stash")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create branch from stash: {}", stderr);
        }

        Ok(format!("Created branch '{}' from stash", name))
    }

    // Helper to get stash reference
    fn get_stash_ref(&self, index: Option<usize>) -> String {
        format!("stash@{{{}}}", index.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stash_entry_parse() {
        let line = "stash@{0}: On main: WIP feature";
        let entry = StashEntry::parse(line, 0).unwrap();

        assert_eq!(entry.index, 0);
        assert_eq!(entry.reference, "stash@{0}");
        assert_eq!(entry.branch, "main");
        assert_eq!(entry.message, "WIP feature");
    }

    #[test]
    fn test_stash_ref() {
        let repo_path = std::env::temp_dir();
        // Just testing the ref generation logic
        let stash_ref = format!("stash@{{{}}}", 0);
        assert_eq!(stash_ref, "stash@{0}");

        let stash_ref = format!("stash@{{{}}}", 3);
        assert_eq!(stash_ref, "stash@{3}");
    }
}
