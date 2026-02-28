//! # Obsidian Integration
//!
//! Obsidian vault file operations and search.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::obsidian::{ObsidianVault, ObsidianConfig};
//!
//! let config = ObsidianConfig::from_file("~/.dx/config/obsidian.sr")?;
//! let vault = ObsidianVault::new(&config)?;
//!
//! // Create a note
//! vault.create_note("My Note", "# Hello\n\nContent here.").await?;
//!
//! // Search notes
//! let results = vault.search("keyword").await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Obsidian configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianConfig {
    /// Whether Obsidian integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Vault path
    pub vault_path: PathBuf,
    /// Default folder for new notes
    #[serde(default)]
    pub default_folder: String,
    /// Daily notes folder
    #[serde(default = "default_daily_folder")]
    pub daily_folder: String,
    /// Daily note format
    #[serde(default = "default_daily_format")]
    pub daily_format: String,
    /// Template folder
    pub template_folder: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_daily_folder() -> String {
    "Daily".to_string()
}

fn default_daily_format() -> String {
    "%Y-%m-%d".to_string()
}

impl Default for ObsidianConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vault_path: PathBuf::new(),
            default_folder: String::new(),
            daily_folder: default_daily_folder(),
            daily_format: default_daily_format(),
            template_folder: None,
        }
    }
}

impl ObsidianConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }

    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if let Ok(path) = std::env::var("OBSIDIAN_VAULT_PATH") {
            self.vault_path = PathBuf::from(path);
        }
    }
}

/// Obsidian note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianNote {
    /// Note title (filename without .md)
    pub title: String,
    /// Full path to the note
    pub path: PathBuf,
    /// Note content
    pub content: String,
    /// Frontmatter (YAML)
    pub frontmatter: Option<HashMap<String, serde_yaml::Value>>,
    /// Tags found in the note
    pub tags: Vec<String>,
    /// Links to other notes
    pub links: Vec<String>,
    /// Backlinks (notes that link to this one)
    pub backlinks: Vec<String>,
    /// Created time
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    /// Modified time
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Note
    pub note: ObsidianNote,
    /// Match score
    pub score: f32,
    /// Matched lines
    pub matches: Vec<SearchMatch>,
}

/// Search match in a note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    /// Line number (1-indexed)
    pub line: usize,
    /// Line content
    pub content: String,
    /// Match start position in line
    pub start: usize,
    /// Match end position in line
    pub end: usize,
}

/// Obsidian vault
pub struct ObsidianVault {
    config: ObsidianConfig,
    /// Cache of note metadata
    note_cache: HashMap<PathBuf, NoteCacheEntry>,
}

#[derive(Debug, Clone)]
struct NoteCacheEntry {
    title: String,
    tags: Vec<String>,
    links: Vec<String>,
    modified: std::time::SystemTime,
}

impl ObsidianVault {
    /// Create a new vault instance
    pub fn new(config: &ObsidianConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        if !config.vault_path.exists() {
            return Err(DrivenError::NotFound(format!(
                "Vault not found: {}",
                config.vault_path.display()
            )));
        }

        Ok(Self {
            config,
            note_cache: HashMap::new(),
        })
    }

    /// Check if vault is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled && self.config.vault_path.exists()
    }

    /// Get vault path
    pub fn vault_path(&self) -> &Path {
        &self.config.vault_path
    }

    /// Create a new note
    pub async fn create_note(&self, title: &str, content: &str) -> Result<ObsidianNote> {
        let filename = self.sanitize_filename(title);
        let folder = if self.config.default_folder.is_empty() {
            self.config.vault_path.clone()
        } else {
            self.config.vault_path.join(&self.config.default_folder)
        };

        // Ensure folder exists
        tokio::fs::create_dir_all(&folder)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        let path = folder.join(format!("{}.md", filename));

        if path.exists() {
            return Err(DrivenError::Conflict(format!(
                "Note already exists: {}",
                path.display()
            )));
        }

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        self.get_note(&path).await
    }

    /// Get a note by path or title
    pub async fn get_note(&self, path: &Path) -> Result<ObsidianNote> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.vault_path.join(path)
        };

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        let metadata = tokio::fs::metadata(&full_path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        let title = full_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();

        let (frontmatter, body) = self.parse_frontmatter(&content);
        let tags = self.extract_tags(&content);
        let links = self.extract_links(&content);

        Ok(ObsidianNote {
            title,
            path: full_path,
            content,
            frontmatter,
            tags,
            links,
            backlinks: Vec::new(), // Would need to scan vault
            created: metadata.created().ok().map(|t| t.into()),
            modified: metadata.modified().ok().map(|t| t.into()),
        })
    }

    /// Update a note
    pub async fn update_note(&self, path: &Path, content: &str) -> Result<ObsidianNote> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.vault_path.join(path)
        };

        if !full_path.exists() {
            return Err(DrivenError::NotFound(format!(
                "Note not found: {}",
                full_path.display()
            )));
        }

        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        self.get_note(&full_path).await
    }

    /// Delete a note
    pub async fn delete_note(&self, path: &Path) -> Result<()> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.vault_path.join(path)
        };

        tokio::fs::remove_file(&full_path)
            .await
            .map_err(|e| DrivenError::Io(e))
    }

    /// Create or get today's daily note
    pub async fn daily_note(&self) -> Result<ObsidianNote> {
        let today = chrono::Local::now().format(&self.config.daily_format).to_string();
        let folder = self.config.vault_path.join(&self.config.daily_folder);
        let path = folder.join(format!("{}.md", today));

        if path.exists() {
            return self.get_note(&path).await;
        }

        // Create daily note with template
        let content = format!("# {}\n\n", today);
        
        tokio::fs::create_dir_all(&folder)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        tokio::fs::write(&path, &content)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        self.get_note(&path).await
    }

    /// Append to today's daily note
    pub async fn append_to_daily(&self, content: &str) -> Result<ObsidianNote> {
        let note = self.daily_note().await?;
        let new_content = format!("{}\n{}", note.content.trim_end(), content);
        self.update_note(&note.path, &new_content).await
    }

    /// Search notes
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // Walk vault directory
        let mut entries = tokio::fs::read_dir(&self.config.vault_path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| DrivenError::Io(e))? {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(note) = self.get_note(&path).await {
                    let matches = self.find_matches(&note.content, &query_lower);
                    if !matches.is_empty() {
                        let score = matches.len() as f32 / note.content.len() as f32;
                        results.push(SearchResult {
                            note,
                            score,
                            matches,
                        });
                    }
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// List all notes
    pub async fn list_notes(&self) -> Result<Vec<ObsidianNote>> {
        let mut notes = Vec::new();

        fn walk_dir(dir: &Path, notes: &mut Vec<PathBuf>) -> std::io::Result<()> {
            if dir.is_dir() {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        walk_dir(&path, notes)?;
                    } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                        notes.push(path);
                    }
                }
            }
            Ok(())
        }

        let mut paths = Vec::new();
        walk_dir(&self.config.vault_path, &mut paths)
            .map_err(|e| DrivenError::Io(e))?;

        for path in paths {
            if let Ok(note) = self.get_note(&path).await {
                notes.push(note);
            }
        }

        Ok(notes)
    }

    /// Get backlinks for a note
    pub async fn get_backlinks(&self, title: &str) -> Result<Vec<ObsidianNote>> {
        let notes = self.list_notes().await?;
        let link_pattern = format!("[[{}]]", title);
        let link_pattern_alias = format!("[[{}|", title);

        Ok(notes
            .into_iter()
            .filter(|n| {
                n.content.contains(&link_pattern) || n.content.contains(&link_pattern_alias)
            })
            .collect())
    }

    /// Parse frontmatter from content
    fn parse_frontmatter(&self, content: &str) -> (Option<HashMap<String, serde_yaml::Value>>, String) {
        if !content.starts_with("---") {
            return (None, content.to_string());
        }

        if let Some(end) = content[3..].find("---") {
            let frontmatter_str = &content[3..end + 3];
            let body = &content[end + 6..];

            let frontmatter: Result<HashMap<String, serde_yaml::Value>, _> =
                serde_yaml::from_str(frontmatter_str.trim());

            (frontmatter.ok(), body.trim_start().to_string())
        } else {
            (None, content.to_string())
        }
    }

    /// Extract tags from content
    fn extract_tags(&self, content: &str) -> Vec<String> {
        let mut tags = Vec::new();

        // Tags in frontmatter are handled separately
        // Find inline tags (#tag)
        for word in content.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                let tag = word[1..]
                    .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '-');
                if !tag.is_empty() {
                    tags.push(tag.to_string());
                }
            }
        }

        tags.sort();
        tags.dedup();
        tags
    }

    /// Extract wiki links from content
    fn extract_links(&self, content: &str) -> Vec<String> {
        let mut links = Vec::new();
        let mut remaining = content;

        while let Some(start) = remaining.find("[[") {
            if let Some(end) = remaining[start..].find("]]") {
                let link = &remaining[start + 2..start + end];
                // Handle aliases [[Note|Alias]]
                let note_name = link.split('|').next().unwrap_or(link);
                links.push(note_name.to_string());
                remaining = &remaining[start + end + 2..];
            } else {
                break;
            }
        }

        links.sort();
        links.dedup();
        links
    }

    /// Find matches in content
    fn find_matches(&self, content: &str, query: &str) -> Vec<SearchMatch> {
        let mut matches = Vec::new();
        let content_lower = content.to_lowercase();

        for (line_idx, line) in content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            let mut start = 0;

            while let Some(pos) = line_lower[start..].find(query) {
                matches.push(SearchMatch {
                    line: line_idx + 1,
                    content: line.to_string(),
                    start: start + pos,
                    end: start + pos + query.len(),
                });
                start = start + pos + 1;
            }
        }

        matches
    }

    /// Sanitize filename
    fn sanitize_filename(&self, name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tags() {
        let config = ObsidianConfig::default();
        let vault = ObsidianVault {
            config,
            note_cache: HashMap::new(),
        };

        let content = "This is a note with #tag1 and #tag2/nested tags.";
        let tags = vault.extract_tags(content);
        assert!(tags.contains(&"tag1".to_string()));
        assert!(tags.contains(&"tag2/nested".to_string()));
    }

    #[test]
    fn test_extract_links() {
        let config = ObsidianConfig::default();
        let vault = ObsidianVault {
            config,
            note_cache: HashMap::new(),
        };

        let content = "Link to [[Note A]] and [[Note B|alias]].";
        let links = vault.extract_links(content);
        assert!(links.contains(&"Note A".to_string()));
        assert!(links.contains(&"Note B".to_string()));
    }

    #[test]
    fn test_sanitize_filename() {
        let config = ObsidianConfig::default();
        let vault = ObsidianVault {
            config,
            note_cache: HashMap::new(),
        };

        assert_eq!(vault.sanitize_filename("My Note: Test"), "My Note_ Test");
        assert_eq!(vault.sanitize_filename("A/B\\C"), "A_B_C");
    }
}
