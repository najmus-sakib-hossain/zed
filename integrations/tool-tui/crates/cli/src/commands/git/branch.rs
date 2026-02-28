//! Git Branch Management
//!
//! Provides branch operations including create, switch, delete, and merge.
//!
//! # Features
//!
//! - Branch listing with tracking info
//! - Create, switch, delete branches
//! - Merge operations with conflict detection
//! - Branch comparison
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::commands::git::{BranchManager, BranchOperation, GitRepo};
//!
//! let repo = GitRepo::open_current()?;
//! let branches = BranchManager::new(&repo)?;
//! branches.execute(BranchOperation::Create { name: "feature".to_string(), checkout: true })?;
//! ```

use std::process::Command;

use anyhow::{Context, Result};

use super::GitRepo;

/// Information about a git branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current branch
    pub is_current: bool,
    /// Whether this is a remote branch
    pub is_remote: bool,
    /// Upstream tracking branch (if set)
    pub upstream: Option<String>,
    /// Commit SHA at branch tip
    pub commit_sha: String,
    /// Commit message at branch tip
    pub commit_message: String,
    /// Commits ahead of upstream
    pub ahead: u32,
    /// Commits behind upstream
    pub behind: u32,
    /// Last commit author
    pub author: String,
    /// Last commit date (relative)
    pub date: String,
}

/// Branch operation to execute
#[derive(Debug, Clone)]
pub enum BranchOperation {
    /// Create a new branch
    Create {
        name: String,
        /// Also checkout the new branch
        checkout: bool,
        /// Start point (commit or branch)
        start_point: Option<String>,
    },
    /// Switch to a branch
    Switch {
        name: String,
        /// Create if doesn't exist
        create: bool,
    },
    /// Delete a branch
    Delete {
        name: String,
        /// Force delete even if not merged
        force: bool,
    },
    /// Rename a branch
    Rename { old_name: String, new_name: String },
    /// Merge a branch into current
    Merge {
        name: String,
        /// Don't fast-forward
        no_ff: bool,
        /// Squash commits
        squash: bool,
    },
    /// Set upstream tracking branch
    SetUpstream { branch: String, upstream: String },
}

/// Branch manager for git operations
pub struct BranchManager<'a> {
    repo: &'a GitRepo,
}

impl<'a> BranchManager<'a> {
    /// Create a new branch manager
    pub fn new(repo: &'a GitRepo) -> Result<Self> {
        Ok(Self { repo })
    }

    /// Get all local branches
    pub fn list_local(&self) -> Result<Vec<BranchInfo>> {
        self.list_branches(false)
    }

    /// Get all remote branches
    pub fn list_remote(&self) -> Result<Vec<BranchInfo>> {
        self.list_branches(true)
    }

    /// Get all branches (local and remote)
    pub fn list_all(&self) -> Result<Vec<BranchInfo>> {
        let mut branches = self.list_local()?;
        branches.extend(self.list_remote()?);
        Ok(branches)
    }

    /// Get the current branch name
    pub fn current(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to get current branch")?;

        if !output.status.success() {
            anyhow::bail!("Failed to get current branch");
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Execute a branch operation
    pub fn execute(&self, op: BranchOperation) -> Result<()> {
        match op {
            BranchOperation::Create {
                name,
                checkout,
                start_point,
            } => self.create_branch(&name, checkout, start_point.as_deref()),
            BranchOperation::Switch { name, create } => self.switch_branch(&name, create),
            BranchOperation::Delete { name, force } => self.delete_branch(&name, force),
            BranchOperation::Rename { old_name, new_name } => {
                self.rename_branch(&old_name, &new_name)
            }
            BranchOperation::Merge {
                name,
                no_ff,
                squash,
            } => self.merge_branch(&name, no_ff, squash),
            BranchOperation::SetUpstream { branch, upstream } => {
                self.set_upstream(&branch, &upstream)
            }
        }
    }

    /// Check if a branch exists
    pub fn exists(&self, name: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", &format!("refs/heads/{}", name)])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to check branch existence")?;

        Ok(output.status.success())
    }

    /// Get branch ahead/behind counts relative to upstream
    pub fn get_ahead_behind(&self, branch: &str) -> Result<(u32, u32)> {
        let output = Command::new("git")
            .args([
                "rev-list",
                "--left-right",
                "--count",
                &format!("{}@{{u}}...{}", branch, branch),
            ])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to get ahead/behind counts")?;

        if !output.status.success() {
            return Ok((0, 0));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split('\t').collect();

        let behind = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let ahead = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

        Ok((ahead, behind))
    }

    // Internal methods

    fn list_branches(&self, remote: bool) -> Result<Vec<BranchInfo>> {
        let format = "%(HEAD)%(refname:short)%(upstream:short)%(objectname:short)%(subject)%(authorname)%(creatordate:relative)";
        let sep = "\x00";
        let full_format = format.replace(")(", &format!("){sep}("));
        let format_arg = format!("--format={}", full_format);

        let mut args = vec!["branch", &format_arg, "-v"];

        if remote {
            args.push("-r");
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(self.repo.root())
            .output()
            .context("Failed to list branches")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut branches = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(7, sep).collect();
            if parts.len() < 6 {
                continue;
            }

            let is_current = parts[0] == "*";
            let name = parts[1].to_string();
            let upstream = if parts[2].is_empty() {
                None
            } else {
                Some(parts[2].to_string())
            };
            let commit_sha = parts[3].to_string();
            let commit_message = parts[4].to_string();
            let author = parts[5].to_string();
            let date = parts.get(6).unwrap_or(&"").to_string();

            let (ahead, behind) = if upstream.is_some() {
                self.get_ahead_behind(&name).unwrap_or((0, 0))
            } else {
                (0, 0)
            };

            branches.push(BranchInfo {
                name,
                is_current,
                is_remote: remote,
                upstream,
                commit_sha,
                commit_message,
                ahead,
                behind,
                author,
                date,
            });
        }

        Ok(branches)
    }

    fn create_branch(&self, name: &str, checkout: bool, start_point: Option<&str>) -> Result<()> {
        let mut args = if checkout {
            vec!["checkout", "-b", name]
        } else {
            vec!["branch", name]
        };

        if let Some(point) = start_point {
            args.push(point);
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(self.repo.root())
            .output()
            .context("Failed to create branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create branch: {}", stderr);
        }

        Ok(())
    }

    fn switch_branch(&self, name: &str, create: bool) -> Result<()> {
        let args = if create {
            vec!["switch", "-c", name]
        } else {
            vec!["switch", name]
        };

        let output = Command::new("git")
            .args(&args)
            .current_dir(self.repo.root())
            .output()
            .context("Failed to switch branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to switch branch: {}", stderr);
        }

        Ok(())
    }

    fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        let flag = if force { "-D" } else { "-d" };
        let output = Command::new("git")
            .args(["branch", flag, name])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to delete branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to delete branch: {}", stderr);
        }

        Ok(())
    }

    fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["branch", "-m", old_name, new_name])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to rename branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to rename branch: {}", stderr);
        }

        Ok(())
    }

    fn merge_branch(&self, name: &str, no_ff: bool, squash: bool) -> Result<()> {
        let mut args = vec!["merge", name];

        if no_ff {
            args.push("--no-ff");
        }
        if squash {
            args.push("--squash");
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(self.repo.root())
            .output()
            .context("Failed to merge branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("CONFLICT") {
                anyhow::bail!("Merge conflict detected. Please resolve conflicts and commit.");
            }
            anyhow::bail!("Failed to merge branch: {}", stderr);
        }

        Ok(())
    }

    fn set_upstream(&self, branch: &str, upstream: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["branch", "-u", upstream, branch])
            .current_dir(self.repo.root())
            .output()
            .context("Failed to set upstream")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to set upstream: {}", stderr);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_operation_create() {
        let op = BranchOperation::Create {
            name: "feature".to_string(),
            checkout: true,
            start_point: None,
        };

        match op {
            BranchOperation::Create { name, checkout, .. } => {
                assert_eq!(name, "feature");
                assert!(checkout);
            }
            _ => panic!("Expected Create operation"),
        }
    }
}
