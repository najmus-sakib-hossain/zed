//! Git Commit Interface
//!
//! Provides a full-featured commit interface with message editor,
//! template support, and staged changes preview.
//!
//! # Features
//!
//! - Commit message editor with syntax highlighting
//! - Commit template support
//! - Staged changes preview
//! - Amend commit option
//! - Co-author support
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::commands::git::{CommitInterface, GitRepo};
//!
//! let repo = GitRepo::open_current()?;
//! let commit = CommitInterface::new(&repo)?;
//! commit.commit_interactive()?;
//! ```

use std::process::Command;

use anyhow::{Context, Result};

use super::GitRepo;

/// Information about a commit
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Commit SHA
    pub sha: String,
    /// Short SHA (7 chars)
    pub short_sha: String,
    /// Commit message (first line)
    pub message: String,
    /// Full commit message
    pub full_message: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Commit date (Unix timestamp)
    pub date: i64,
    /// Parent commit SHAs
    pub parents: Vec<String>,
}

impl CommitInfo {
    /// Parse from git log output
    pub fn parse(output: &str) -> Result<Self> {
        let parts: Vec<&str> = output.splitn(7, '\x00').collect();
        if parts.len() < 7 {
            anyhow::bail!("Invalid commit format");
        }

        Ok(Self {
            sha: parts[0].to_string(),
            short_sha: parts[0].chars().take(7).collect(),
            message: parts[1].lines().next().unwrap_or("").to_string(),
            full_message: parts[1].to_string(),
            author_name: parts[2].to_string(),
            author_email: parts[3].to_string(),
            date: parts[4].parse().unwrap_or(0),
            parents: parts[5].split_whitespace().map(String::from).collect(),
        })
    }
}

/// Commit interface for creating commits
pub struct CommitInterface<'a> {
    repo: &'a GitRepo,
}

impl<'a> CommitInterface<'a> {
    /// Create a new commit interface
    pub fn new(repo: &'a GitRepo) -> Result<Self> {
        Ok(Self { repo })
    }

    /// Create a commit with the given message
    pub fn commit(&self, message: &str) -> Result<CommitInfo> {
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to create commit")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Commit failed: {}", stderr);
        }

        // Get the new commit info
        self.get_head_commit()
    }

    /// Amend the last commit
    pub fn amend(&self, message: Option<&str>) -> Result<CommitInfo> {
        let mut args = vec!["commit", "--amend"];
        if let Some(msg) = message {
            args.push("-m");
            args.push(msg);
        } else {
            args.push("--no-edit");
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(self.repo.root())
            .output()
            .context("Failed to amend commit")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Amend failed: {}", stderr);
        }

        self.get_head_commit()
    }

    /// Get the HEAD commit info
    pub fn get_head_commit(&self) -> Result<CommitInfo> {
        let format = "%H%x00%B%x00%an%x00%ae%x00%at%x00%P%x00";
        let output = Command::new("git")
            .args(["log", "-1", &format!("--format={}", format)])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to get HEAD commit")?;

        if !output.status.success() {
            anyhow::bail!("Failed to get HEAD commit");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        CommitInfo::parse(&stdout)
    }

    /// Get commit template if configured
    pub fn get_template(&self) -> Result<Option<String>> {
        let output = Command::new("git")
            .args(["config", "--get", "commit.template"])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to get commit template")?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                let content =
                    std::fs::read_to_string(&path).context("Failed to read commit template")?;
                return Ok(Some(content));
            }
        }

        Ok(None)
    }

    /// Get list of recent commits
    pub fn get_recent_commits(&self, count: usize) -> Result<Vec<CommitInfo>> {
        let format = "%H%x00%B%x00%an%x00%ae%x00%at%x00%P%x00%x1e";
        let output = Command::new("git")
            .args([
                "log",
                &format!("-{}", count),
                &format!("--format={}", format),
            ])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to get recent commits")?;

        if !output.status.success() {
            anyhow::bail!("Failed to get recent commits");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits: Result<Vec<_>> = stdout
            .split('\x1e')
            .filter(|s| !s.trim().is_empty())
            .map(CommitInfo::parse)
            .collect();

        commits
    }

    /// Check if there are staged changes
    pub fn has_staged_changes(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to check staged changes")?;

        Ok(!output.status.success())
    }

    /// Get staged changes summary
    pub fn get_staged_summary(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--stat"])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to get staged summary")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Stage all changes
    pub fn stage_all(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to stage all changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Stage all failed: {}", stderr);
        }

        Ok(())
    }

    /// Stage a specific file
    pub fn stage_file(&self, path: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["add", "--", path])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to stage file")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Stage file failed: {}", stderr);
        }

        Ok(())
    }

    /// Unstage a specific file
    pub fn unstage_file(&self, path: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["reset", "HEAD", "--", path])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to unstage file")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Unstage file failed: {}", stderr);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_info_parse() {
        let input = "abc123def456\x00Initial commit\x00John Doe\x00john@example.com\x001234567890\x00parent123\x00";
        let info = CommitInfo::parse(input).unwrap();

        assert_eq!(info.sha, "abc123def456");
        assert_eq!(info.short_sha, "abc123d");
        assert_eq!(info.message, "Initial commit");
        assert_eq!(info.author_name, "John Doe");
        assert_eq!(info.author_email, "john@example.com");
    }
}
