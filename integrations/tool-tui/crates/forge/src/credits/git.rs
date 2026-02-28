//! Git history extraction for contributor information.
//!
//! Extracts author information from git commit history using
//! git2 or command-line git.

use super::contributor::{Contributor, ContributorRole};
use std::path::Path;
use std::process::Command;

/// Author information extracted from git.
#[derive(Debug, Clone)]
pub struct GitAuthor {
    /// Author name from commit
    pub name: String,
    /// Author email from commit
    pub email: String,
    /// Commit hash
    pub commit_hash: String,
    /// Commit timestamp
    pub timestamp: i64,
    /// Lines added in commit
    pub additions: usize,
    /// Lines deleted in commit
    pub deletions: usize,
    /// Files changed in commit
    pub files_changed: usize,
}

/// Git history extractor.
#[derive(Debug)]
pub struct GitExtractor {
    /// Repository path
    repo_path: std::path::PathBuf,
}

impl GitExtractor {
    /// Create a new git extractor for the given repository.
    pub fn new(repo_path: &Path) -> Result<Self, GitError> {
        // Check if it's a git repository
        let git_dir = repo_path.join(".git");
        if !git_dir.exists() {
            return Err(GitError::NotARepository(repo_path.to_path_buf()));
        }

        Ok(Self {
            repo_path: repo_path.to_path_buf(),
        })
    }

    /// Extract all authors from git history.
    pub fn extract_authors(&self) -> Result<Vec<GitAuthor>, GitError> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["log", "--format=%H|%an|%ae|%at", "--numstat"])
            .output()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_git_log(&stdout)
    }

    /// Parse git log output.
    fn parse_git_log(&self, log: &str) -> Result<Vec<GitAuthor>, GitError> {
        let mut authors = Vec::new();
        let mut current_commit: Option<(String, String, String, i64)> = None;
        let mut additions = 0usize;
        let mut deletions = 0usize;
        let mut files = 0usize;

        for line in log.lines() {
            let line = line.trim();

            if line.is_empty() {
                // End of commit stats, create author entry
                if let Some((hash, name, email, timestamp)) = current_commit.take() {
                    authors.push(GitAuthor {
                        name,
                        email,
                        commit_hash: hash,
                        timestamp,
                        additions,
                        deletions,
                        files_changed: files,
                    });
                    additions = 0;
                    deletions = 0;
                    files = 0;
                }
                continue;
            }

            // Check if this is a commit header line
            if line.contains('|') {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    // Save any pending commit first
                    if let Some((hash, name, email, timestamp)) = current_commit.take() {
                        authors.push(GitAuthor {
                            name,
                            email,
                            commit_hash: hash,
                            timestamp,
                            additions,
                            deletions,
                            files_changed: files,
                        });
                        additions = 0;
                        deletions = 0;
                        files = 0;
                    }

                    current_commit = Some((
                        parts[0].to_string(),
                        parts[1].to_string(),
                        parts[2].to_string(),
                        parts[3].parse().unwrap_or(0),
                    ));
                }
            } else {
                // This is a numstat line: additions\tdeletions\tfile
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    if let (Ok(add), Ok(del)) =
                        (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                    {
                        additions += add;
                        deletions += del;
                        files += 1;
                    }
                }
            }
        }

        // Don't forget the last commit
        if let Some((hash, name, email, timestamp)) = current_commit {
            authors.push(GitAuthor {
                name,
                email,
                commit_hash: hash,
                timestamp,
                additions,
                deletions,
                files_changed: files,
            });
        }

        Ok(authors)
    }

    /// Extract the current user's git configuration.
    pub fn get_current_user(&self) -> Result<(String, String), GitError> {
        let name = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["config", "user.name"])
            .output()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        let email = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["config", "user.email"])
            .output()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        Ok((
            String::from_utf8_lossy(&name.stdout).trim().to_string(),
            String::from_utf8_lossy(&email.stdout).trim().to_string(),
        ))
    }

    /// Convert git authors to contributors with aggregated stats.
    pub fn to_contributors(&self, authors: Vec<GitAuthor>) -> Vec<Contributor> {
        use std::collections::HashMap;

        let mut contributor_map: HashMap<String, Contributor> = HashMap::new();

        for author in authors {
            let key = author.email.to_lowercase();

            let entry = contributor_map
                .entry(key)
                .or_insert_with(|| Contributor::new(&author.name, &author.email));

            // Update stats
            entry.stats.commits += 1;
            entry.stats.additions += author.additions;
            entry.stats.deletions += author.deletions;
            entry.stats.files_changed += author.files_changed;
            entry.stats.record_timestamp(author.timestamp);
        }

        // Update roles based on contribution count
        let mut contributors: Vec<Contributor> = contributor_map.into_values().collect();
        for contrib in &mut contributors {
            contrib.role = ContributorRole::from_commit_count(contrib.stats.commits);
        }

        // Sort by score (highest first)
        contributors.sort_by(|a, b| b.stats.score().cmp(&a.stats.score()));

        contributors
    }

    /// Try to find GitHub username from email.
    pub fn guess_github_username(email: &str) -> Option<String> {
        // Common patterns:
        // - username@users.noreply.github.com
        // - username@gmail.com (can't guess)

        if email.ends_with("@users.noreply.github.com") {
            let parts: Vec<&str> = email.split('@').collect();
            if let Some(username_part) = parts.first() {
                // Handle GitHub's ID+username format
                if let Some(pos) = username_part.find('+') {
                    return Some(username_part[pos + 1..].to_string());
                }
                return Some(username_part.to_string());
            }
        }

        None
    }
}

/// Errors that can occur during git extraction.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotARepository(std::path::PathBuf),

    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Failed to parse git output: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_username_from_noreply() {
        let email = "12345+username@users.noreply.github.com";
        assert_eq!(GitExtractor::guess_github_username(email), Some("username".into()));
    }

    #[test]
    fn test_github_username_simple_noreply() {
        let email = "username@users.noreply.github.com";
        assert_eq!(GitExtractor::guess_github_username(email), Some("username".into()));
    }

    #[test]
    fn test_github_username_regular_email() {
        let email = "user@example.com";
        assert_eq!(GitExtractor::guess_github_username(email), None);
    }

    #[test]
    fn test_git_author_struct() {
        let author = GitAuthor {
            name: "Test".into(),
            email: "test@example.com".into(),
            commit_hash: "abc123".into(),
            timestamp: 1234567890,
            additions: 100,
            deletions: 50,
            files_changed: 5,
        };

        assert_eq!(author.name, "Test");
        assert_eq!(author.additions, 100);
    }
}
