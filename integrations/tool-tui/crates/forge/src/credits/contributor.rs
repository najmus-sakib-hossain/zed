//! Contributor data types.
//!
//! Defines the core types for representing contributors and their
//! contributions to DX plugins.

use std::collections::HashMap;

/// A contributor to a DX plugin or project.
#[derive(Debug, Clone)]
pub struct Contributor {
    /// Contributor's display name
    pub name: String,
    /// Email address
    pub email: String,
    /// GitHub username (if known)
    pub github_username: Option<String>,
    /// URL to avatar image
    pub avatar_url: Option<String>,
    /// Role in the project
    pub role: ContributorRole,
    /// Contribution statistics
    pub stats: ContributionStats,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Contributor {
    /// Create a new contributor.
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            github_username: None,
            avatar_url: None,
            role: ContributorRole::Contributor,
            stats: ContributionStats::default(),
            metadata: HashMap::new(),
        }
    }

    /// Set GitHub username.
    pub fn with_github(mut self, username: impl Into<String>) -> Self {
        let username = username.into();
        self.avatar_url = Some(format!("https://github.com/{}.png", username));
        self.github_username = Some(username);
        self
    }

    /// Set contributor role.
    pub fn with_role(mut self, role: ContributorRole) -> Self {
        self.role = role;
        self
    }

    /// Add contribution stats.
    pub fn with_stats(mut self, stats: ContributionStats) -> Self {
        self.stats = stats;
        self
    }

    /// Get GitHub profile URL.
    pub fn github_url(&self) -> Option<String> {
        self.github_username.as_ref().map(|u| format!("https://github.com/{}", u))
    }

    /// Get a unique identifier for this contributor.
    pub fn id(&self) -> String {
        // Use email as primary identifier
        self.email.to_lowercase()
    }

    /// Merge another contributor's stats (same person, different identity).
    pub fn merge(&mut self, other: &Contributor) {
        self.stats.commits += other.stats.commits;
        self.stats.additions += other.stats.additions;
        self.stats.deletions += other.stats.deletions;
        self.stats.files_changed += other.stats.files_changed;

        // Prefer non-empty values
        if self.github_username.is_none() && other.github_username.is_some() {
            self.github_username = other.github_username.clone();
            self.avatar_url = other.avatar_url.clone();
        }

        // Upgrade role if other has higher role
        if other.role > self.role {
            self.role = other.role;
        }
    }
}

/// Role of a contributor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContributorRole {
    /// One-time contributor
    Contributor,
    /// Regular contributor
    RegularContributor,
    /// Core team member
    CoreTeam,
    /// Project maintainer
    Maintainer,
    /// Project owner/creator
    Owner,
}

impl ContributorRole {
    /// Get display string for role.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Contributor => "Contributor",
            Self::RegularContributor => "Regular Contributor",
            Self::CoreTeam => "Core Team",
            Self::Maintainer => "Maintainer",
            Self::Owner => "Owner",
        }
    }

    /// Get emoji for role.
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Contributor => "👤",
            Self::RegularContributor => "👥",
            Self::CoreTeam => "🔧",
            Self::Maintainer => "🛠️",
            Self::Owner => "👑",
        }
    }

    /// Determine role based on contribution count.
    pub fn from_commit_count(commits: usize) -> Self {
        match commits {
            0..=5 => Self::Contributor,
            6..=50 => Self::RegularContributor,
            51..=200 => Self::CoreTeam,
            _ => Self::Maintainer,
        }
    }
}

impl std::fmt::Display for ContributorRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Statistics about a contributor's contributions.
#[derive(Debug, Clone, Default)]
pub struct ContributionStats {
    /// Number of commits
    pub commits: usize,
    /// Lines added
    pub additions: usize,
    /// Lines deleted
    pub deletions: usize,
    /// Files changed
    pub files_changed: usize,
    /// First contribution timestamp
    pub first_contribution: Option<i64>,
    /// Latest contribution timestamp
    pub latest_contribution: Option<i64>,
    /// Primary contribution areas
    pub areas: Vec<String>,
}

impl ContributionStats {
    /// Create new stats with a single commit.
    pub fn single_commit(additions: usize, deletions: usize, files: usize) -> Self {
        Self {
            commits: 1,
            additions,
            deletions,
            files_changed: files,
            ..Default::default()
        }
    }

    /// Get total lines changed.
    pub fn total_changes(&self) -> usize {
        self.additions + self.deletions
    }

    /// Get net lines added.
    pub fn net_additions(&self) -> isize {
        self.additions as isize - self.deletions as isize
    }

    /// Get contribution score (weighted).
    pub fn score(&self) -> usize {
        // Weight: commits * 10 + additions * 1 + deletions * 0.5
        self.commits * 10 + self.additions + self.deletions / 2
    }

    /// Update timestamps.
    pub fn record_timestamp(&mut self, timestamp: i64) {
        match self.first_contribution {
            None => self.first_contribution = Some(timestamp),
            Some(first) if timestamp < first => self.first_contribution = Some(timestamp),
            _ => {}
        }

        match self.latest_contribution {
            None => self.latest_contribution = Some(timestamp),
            Some(latest) if timestamp > latest => self.latest_contribution = Some(timestamp),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contributor_new() {
        let contrib = Contributor::new("Alice", "alice@example.com");
        assert_eq!(contrib.name, "Alice");
        assert_eq!(contrib.email, "alice@example.com");
        assert_eq!(contrib.role, ContributorRole::Contributor);
    }

    #[test]
    fn test_contributor_with_github() {
        let contrib = Contributor::new("Bob", "bob@example.com").with_github("bobdev");
        assert_eq!(contrib.github_username, Some("bobdev".into()));
        assert!(contrib.avatar_url.unwrap().contains("bobdev"));
    }

    #[test]
    fn test_contributor_merge() {
        let mut c1 = Contributor::new("Alice", "alice@example.com");
        c1.stats.commits = 10;

        let c2 = Contributor::new("Alice", "alice@example.com").with_github("alice").with_stats(
            ContributionStats {
                commits: 5,
                ..Default::default()
            },
        );

        c1.merge(&c2);

        assert_eq!(c1.stats.commits, 15);
        assert_eq!(c1.github_username, Some("alice".into()));
    }

    #[test]
    fn test_role_ordering() {
        assert!(ContributorRole::Owner > ContributorRole::Maintainer);
        assert!(ContributorRole::Maintainer > ContributorRole::CoreTeam);
        assert!(ContributorRole::CoreTeam > ContributorRole::RegularContributor);
        assert!(ContributorRole::RegularContributor > ContributorRole::Contributor);
    }

    #[test]
    fn test_role_from_commits() {
        assert_eq!(ContributorRole::from_commit_count(3), ContributorRole::Contributor);
        assert_eq!(ContributorRole::from_commit_count(30), ContributorRole::RegularContributor);
        assert_eq!(ContributorRole::from_commit_count(100), ContributorRole::CoreTeam);
        assert_eq!(ContributorRole::from_commit_count(500), ContributorRole::Maintainer);
    }

    #[test]
    fn test_stats_score() {
        let stats = ContributionStats {
            commits: 5,
            additions: 100,
            deletions: 50,
            ..Default::default()
        };
        // 5*10 + 100 + 50/2 = 50 + 100 + 25 = 175
        assert_eq!(stats.score(), 175);
    }
}
