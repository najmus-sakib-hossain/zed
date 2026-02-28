//! Credits manager for tracking and managing contributors.
//!
//! Provides a unified interface for extracting, storing, and
//! querying contributor information.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::contributor::{Contributor, ContributorRole};
use super::git::{GitError, GitExtractor};
use super::markdown::ContributorsMarkdown;

/// Manager for contributor credits.
#[derive(Debug)]
pub struct CreditsManager {
    /// Repository path
    repo_path: PathBuf,
    /// Cached contributors
    contributors: Vec<Contributor>,
    /// GitHub username mappings (email -> username)
    github_mappings: HashMap<String, String>,
    /// Manual role overrides
    role_overrides: HashMap<String, ContributorRole>,
}

impl CreditsManager {
    /// Create a new credits manager for a git repository.
    pub fn from_git_repo(path: &Path) -> Result<Self, GitError> {
        let extractor = GitExtractor::new(path)?;
        let authors = extractor.extract_authors()?;
        let contributors = extractor.to_contributors(authors);

        Ok(Self {
            repo_path: path.to_path_buf(),
            contributors,
            github_mappings: HashMap::new(),
            role_overrides: HashMap::new(),
        })
    }

    /// Create an empty credits manager.
    pub fn new(path: &Path) -> Self {
        Self {
            repo_path: path.to_path_buf(),
            contributors: Vec::new(),
            github_mappings: HashMap::new(),
            role_overrides: HashMap::new(),
        }
    }

    /// Add a GitHub username mapping.
    pub fn add_github_mapping(&mut self, email: &str, github: &str) {
        self.github_mappings.insert(email.to_lowercase(), github.to_string());
    }

    /// Set a role override for a contributor.
    pub fn set_role_override(&mut self, email: &str, role: ContributorRole) {
        self.role_overrides.insert(email.to_lowercase(), role);
    }

    /// Get all contributors.
    pub fn get_contributors(&self) -> &[Contributor] {
        &self.contributors
    }

    /// Get contributors with enriched GitHub info.
    pub fn get_enriched_contributors(&self) -> Vec<Contributor> {
        self.contributors
            .iter()
            .map(|c| {
                let mut enriched = c.clone();

                // Apply GitHub mapping
                if let Some(github) = self.github_mappings.get(&c.email.to_lowercase()) {
                    enriched.github_username = Some(github.clone());
                    enriched.avatar_url = Some(format!("https://github.com/{}.png", github));
                }

                // Try to guess GitHub username from email
                if enriched.github_username.is_none() {
                    if let Some(github) = GitExtractor::guess_github_username(&c.email) {
                        enriched.github_username = Some(github.clone());
                        enriched.avatar_url = Some(format!("https://github.com/{}.png", github));
                    }
                }

                // Apply role override
                if let Some(role) = self.role_overrides.get(&c.email.to_lowercase()) {
                    enriched.role = *role;
                }

                enriched
            })
            .collect()
    }

    /// Get contributors by role.
    pub fn get_by_role(&self, role: ContributorRole) -> Vec<&Contributor> {
        self.contributors.iter().filter(|c| c.role == role).collect()
    }

    /// Get the top N contributors by score.
    pub fn top_contributors(&self, n: usize) -> Vec<&Contributor> {
        self.contributors.iter().take(n).collect()
    }

    /// Find a contributor by email.
    pub fn find_by_email(&self, email: &str) -> Option<&Contributor> {
        let email_lower = email.to_lowercase();
        self.contributors.iter().find(|c| c.email.to_lowercase() == email_lower)
    }

    /// Add a manual contributor.
    pub fn add_contributor(&mut self, contributor: Contributor) {
        // Check if already exists
        let email_lower = contributor.email.to_lowercase();
        if let Some(existing) =
            self.contributors.iter_mut().find(|c| c.email.to_lowercase() == email_lower)
        {
            existing.merge(&contributor);
        } else {
            self.contributors.push(contributor);
        }
    }

    /// Refresh contributors from git history.
    pub fn refresh(&mut self) -> Result<(), GitError> {
        let extractor = GitExtractor::new(&self.repo_path)?;
        let authors = extractor.extract_authors()?;
        self.contributors = extractor.to_contributors(authors);
        Ok(())
    }

    /// Generate CONTRIBUTORS.md content.
    pub fn generate_contributors_md(&self) -> String {
        let enriched = self.get_enriched_contributors();
        let markdown = ContributorsMarkdown::new(&enriched);
        markdown.generate()
    }

    /// Generate CONTRIBUTORS.md and write to file.
    pub fn write_contributors_md(&self, path: &Path) -> std::io::Result<()> {
        let content = self.generate_contributors_md();
        std::fs::write(path, content)
    }

    /// Get total statistics across all contributors.
    pub fn total_stats(&self) -> TotalStats {
        let mut stats = TotalStats::default();

        for contrib in &self.contributors {
            stats.total_contributors += 1;
            stats.total_commits += contrib.stats.commits;
            stats.total_additions += contrib.stats.additions;
            stats.total_deletions += contrib.stats.deletions;

            match contrib.role {
                ContributorRole::Owner => stats.owners += 1,
                ContributorRole::Maintainer => stats.maintainers += 1,
                ContributorRole::CoreTeam => stats.core_team += 1,
                ContributorRole::RegularContributor => stats.regular_contributors += 1,
                ContributorRole::Contributor => stats.contributors += 1,
            }
        }

        stats
    }
}

/// Total statistics across all contributors.
#[derive(Debug, Default)]
pub struct TotalStats {
    /// Total number of contributors
    pub total_contributors: usize,
    /// Total commits
    pub total_commits: usize,
    /// Total lines added
    pub total_additions: usize,
    /// Total lines deleted
    pub total_deletions: usize,
    /// Number of owners
    pub owners: usize,
    /// Number of maintainers
    pub maintainers: usize,
    /// Number of core team members
    pub core_team: usize,
    /// Number of regular contributors
    pub regular_contributors: usize,
    /// Number of one-time contributors
    pub contributors: usize,
}

impl TotalStats {
    /// Get total lines changed.
    pub fn total_changes(&self) -> usize {
        self.total_additions + self.total_deletions
    }

    /// Get average commits per contributor.
    pub fn avg_commits(&self) -> f64 {
        if self.total_contributors == 0 {
            0.0
        } else {
            self.total_commits as f64 / self.total_contributors as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::contributor::ContributionStats;
    use super::*;

    #[test]
    fn test_credits_manager_new() {
        let manager = CreditsManager::new(Path::new("."));
        assert!(manager.get_contributors().is_empty());
    }

    #[test]
    fn test_add_contributor() {
        let mut manager = CreditsManager::new(Path::new("."));

        let contrib =
            Contributor::new("Alice", "alice@example.com").with_stats(ContributionStats {
                commits: 10,
                ..Default::default()
            });

        manager.add_contributor(contrib);
        assert_eq!(manager.get_contributors().len(), 1);
    }

    #[test]
    fn test_github_mapping() {
        let mut manager = CreditsManager::new(Path::new("."));
        manager.add_contributor(Contributor::new("Alice", "alice@example.com"));
        manager.add_github_mapping("alice@example.com", "alicedev");

        let enriched = manager.get_enriched_contributors();
        assert_eq!(enriched[0].github_username, Some("alicedev".into()));
    }

    #[test]
    fn test_role_override() {
        let mut manager = CreditsManager::new(Path::new("."));
        manager.add_contributor(Contributor::new("Bob", "bob@example.com"));
        manager.set_role_override("bob@example.com", ContributorRole::Maintainer);

        let enriched = manager.get_enriched_contributors();
        assert_eq!(enriched[0].role, ContributorRole::Maintainer);
    }

    #[test]
    fn test_total_stats() {
        let mut manager = CreditsManager::new(Path::new("."));

        manager.add_contributor(Contributor::new("A", "a@example.com").with_stats(
            ContributionStats {
                commits: 5,
                additions: 100,
                deletions: 50,
                ..Default::default()
            },
        ));

        manager.add_contributor(Contributor::new("B", "b@example.com").with_stats(
            ContributionStats {
                commits: 3,
                additions: 50,
                deletions: 25,
                ..Default::default()
            },
        ));

        let stats = manager.total_stats();
        assert_eq!(stats.total_contributors, 2);
        assert_eq!(stats.total_commits, 8);
        assert_eq!(stats.total_additions, 150);
        assert_eq!(stats.total_deletions, 75);
    }
}
