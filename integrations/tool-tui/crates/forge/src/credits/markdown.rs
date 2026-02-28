//! CONTRIBUTORS.md generation.
//!
//! Generates beautiful CONTRIBUTORS.md files with contributor
//! information, avatars, and statistics.

use super::contributor::{Contributor, ContributorRole};

/// CONTRIBUTORS.md generator.
#[derive(Debug)]
pub struct ContributorsMarkdown<'a> {
    /// Contributors to include
    contributors: &'a [Contributor],
    /// Project name
    project_name: String,
    /// Show avatars
    show_avatars: bool,
    /// Show statistics
    show_stats: bool,
    /// Group by role
    group_by_role: bool,
}

impl<'a> ContributorsMarkdown<'a> {
    /// Create a new markdown generator.
    pub fn new(contributors: &'a [Contributor]) -> Self {
        Self {
            contributors,
            project_name: String::from("DX"),
            show_avatars: true,
            show_stats: true,
            group_by_role: true,
        }
    }

    /// Set project name.
    pub fn with_project_name(mut self, name: impl Into<String>) -> Self {
        self.project_name = name.into();
        self
    }

    /// Disable avatars.
    pub fn no_avatars(mut self) -> Self {
        self.show_avatars = false;
        self
    }

    /// Disable statistics.
    pub fn no_stats(mut self) -> Self {
        self.show_stats = false;
        self
    }

    /// Disable role grouping.
    pub fn no_grouping(mut self) -> Self {
        self.group_by_role = false;
        self
    }

    /// Generate the CONTRIBUTORS.md content.
    pub fn generate(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!("# {} Contributors\n\n", self.project_name));
        output.push_str(
            "Thanks to all the wonderful people who have contributed to this project! 🎉\n\n",
        );

        if self.group_by_role {
            self.generate_grouped(&mut output);
        } else {
            self.generate_flat(&mut output);
        }

        // Footer
        output.push_str("\n---\n\n");
        output.push_str(&format!(
            "*This file was automatically generated. Total contributors: {}*\n",
            self.contributors.len()
        ));

        output
    }

    /// Generate grouped by role.
    fn generate_grouped(&self, output: &mut String) {
        let roles = [
            (ContributorRole::Owner, "👑 Owners"),
            (ContributorRole::Maintainer, "🛠️ Maintainers"),
            (ContributorRole::CoreTeam, "🔧 Core Team"),
            (ContributorRole::RegularContributor, "👥 Regular Contributors"),
            (ContributorRole::Contributor, "👤 Contributors"),
        ];

        for (role, title) in roles {
            let members: Vec<&Contributor> =
                self.contributors.iter().filter(|c| c.role == role).collect();

            if members.is_empty() {
                continue;
            }

            output.push_str(&format!("## {}\n\n", title));

            if self.show_avatars {
                self.generate_avatar_grid(output, &members);
            } else {
                self.generate_list(output, &members);
            }

            output.push('\n');
        }
    }

    /// Generate flat list.
    fn generate_flat(&self, output: &mut String) {
        output.push_str("## All Contributors\n\n");

        if self.show_avatars {
            self.generate_avatar_grid(output, &self.contributors.iter().collect::<Vec<_>>());
        } else {
            self.generate_list(output, &self.contributors.iter().collect::<Vec<_>>());
        }
    }

    /// Generate avatar grid.
    fn generate_avatar_grid(&self, output: &mut String, contributors: &[&Contributor]) {
        output.push_str("<table>\n");
        output.push_str("  <tbody>\n");

        // 6 contributors per row
        let per_row = 6;
        for chunk in contributors.chunks(per_row) {
            output.push_str("    <tr>\n");

            for contrib in chunk {
                output.push_str("      <td align=\"center\" valign=\"top\" width=\"14.28%\">\n");

                // Avatar
                if let Some(ref github) = contrib.github_username {
                    output
                        .push_str(&format!("        <a href=\"https://github.com/{}\">\n", github));
                    output.push_str(&format!(
                        "          <img src=\"https://github.com/{}.png?size=100\" width=\"100px;\" alt=\"{}\"/>\n",
                        github, contrib.name
                    ));
                    output
                        .push_str(&format!("          <br /><sub><b>{}</b></sub>\n", contrib.name));
                    output.push_str("        </a>\n");
                } else {
                    output.push_str(&format!("        <sub><b>{}</b></sub>\n", contrib.name));
                }

                // Stats badge
                if self.show_stats {
                    output.push_str(&format!(
                        "        <br /><sub>{} commits</sub>\n",
                        contrib.stats.commits
                    ));
                }

                output.push_str("      </td>\n");
            }

            // Fill empty cells
            for _ in 0..(per_row - chunk.len()) {
                output.push_str("      <td></td>\n");
            }

            output.push_str("    </tr>\n");
        }

        output.push_str("  </tbody>\n");
        output.push_str("</table>\n\n");
    }

    /// Generate simple list.
    fn generate_list(&self, output: &mut String, contributors: &[&Contributor]) {
        for contrib in contributors {
            let github_link = contrib
                .github_username
                .as_ref()
                .map(|g| format!(" ([@{}](https://github.com/{}))", g, g))
                .unwrap_or_default();

            let stats = if self.show_stats {
                format!(
                    " - {} commits, +{}/-{}",
                    contrib.stats.commits, contrib.stats.additions, contrib.stats.deletions
                )
            } else {
                String::new()
            };

            output.push_str(&format!("- **{}**{}{}\n", contrib.name, github_link, stats));
        }
    }
}

/// Generate a contributor badge (for README).
#[allow(dead_code)]
pub fn generate_badge(contributor_count: usize) -> String {
    format!(
        "[![Contributors](https://img.shields.io/badge/contributors-{}-blue.svg)](CONTRIBUTORS.md)",
        contributor_count
    )
}

/// Generate all-contributors style emoji key.
#[allow(dead_code)]
pub fn generate_emoji_key() -> &'static str {
    r#"### Emoji Key

| Emoji | Type |
|-------|------|
| 💻 | Code |
| 📖 | Documentation |
| 🎨 | Design |
| 🐛 | Bug reports |
| 💡 | Ideas |
| 🔧 | Tools |
| 📦 | Packages |
| 🚧 | Maintenance |
"#
}

#[cfg(test)]
mod tests {
    use super::super::contributor::ContributionStats;
    use super::*;

    fn sample_contributors() -> Vec<Contributor> {
        vec![
            Contributor::new("Alice", "alice@example.com")
                .with_github("alicedev")
                .with_role(ContributorRole::Owner)
                .with_stats(ContributionStats {
                    commits: 100,
                    additions: 5000,
                    deletions: 2000,
                    ..Default::default()
                }),
            Contributor::new("Bob", "bob@example.com")
                .with_github("bobdev")
                .with_role(ContributorRole::Contributor)
                .with_stats(ContributionStats {
                    commits: 5,
                    additions: 50,
                    deletions: 10,
                    ..Default::default()
                }),
        ]
    }

    #[test]
    fn test_generate_markdown() {
        let contributors = sample_contributors();
        let markdown = ContributorsMarkdown::new(&contributors).generate();

        assert!(markdown.contains("# DX Contributors"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));
        assert!(markdown.contains("alicedev"));
    }

    #[test]
    fn test_generate_grouped() {
        let contributors = sample_contributors();
        let markdown = ContributorsMarkdown::new(&contributors).generate();

        assert!(markdown.contains("👑 Owners"));
        assert!(markdown.contains("👤 Contributors"));
    }

    #[test]
    fn test_generate_no_avatars() {
        let contributors = sample_contributors();
        let markdown = ContributorsMarkdown::new(&contributors).no_avatars().generate();

        assert!(!markdown.contains("<table>"));
        assert!(markdown.contains("- **Alice**"));
    }

    #[test]
    fn test_badge() {
        let badge = generate_badge(42);
        assert!(badge.contains("contributors-42"));
        assert!(badge.contains("CONTRIBUTORS.md"));
    }
}
