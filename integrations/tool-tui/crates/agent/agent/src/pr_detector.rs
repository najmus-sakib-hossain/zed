//! # Auto-PR Detection System
//!
//! Detects local changes in DX and automatically creates PRs to share
//! new integrations with the community.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

use crate::Result;

/// A detected local difference from the main repo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDiff {
    /// Type of change
    pub kind: DiffKind,

    /// Path to the changed file
    pub path: PathBuf,

    /// Description of the change
    pub description: String,

    /// When the change was detected
    pub detected_at: DateTime<Utc>,

    /// Content of the change (for new integrations)
    pub content: Option<String>,

    /// Language (for code changes)
    pub language: Option<String>,
}

/// Types of changes that can be detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiffKind {
    /// A new integration was created
    NewIntegration,
    /// A new skill was added
    NewSkill,
    /// A new plugin was created
    NewPlugin,
    /// Configuration changed
    ConfigChange,
    /// Other change
    Other,
}

/// A queued PR to be created
#[derive(Debug, Clone)]
pub struct QueuedPr {
    diff: LocalDiff,
    queued_at: DateTime<Utc>,
}

/// PR Detector - monitors local changes and creates PRs
pub struct PrDetector {
    dx_path: PathBuf,
    upstream_url: String,
    queued_integrations: Vec<QueuedPr>,
    last_check: Option<DateTime<Utc>>,
}

impl PrDetector {
    pub fn new(dx_path: &Path) -> Result<Self> {
        Ok(Self {
            dx_path: dx_path.to_path_buf(),
            upstream_url: "https://github.com/dx-cli/dx".to_string(),
            queued_integrations: Vec::new(),
            last_check: None,
        })
    }

    /// Set the upstream repository URL
    pub fn set_upstream(&mut self, url: &str) {
        self.upstream_url = url.to_string();
    }

    /// Detect local changes that aren't in the upstream repo
    pub async fn detect_local_changes(&self) -> Result<Option<LocalDiff>> {
        info!("Checking for local changes...");

        // Check for new integrations
        let integrations_path = self.dx_path.join("integrations");
        if integrations_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&integrations_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if self.is_new_integration(&path).await? {
                        return Ok(Some(LocalDiff {
                            kind: DiffKind::NewIntegration,
                            path: path.clone(),
                            description: format!("New integration: {:?}", path.file_name()),
                            detected_at: Utc::now(),
                            content: std::fs::read_to_string(&path).ok(),
                            language: self.detect_language(&path),
                        }));
                    }
                }
            }
        }

        // Check for new skills
        let skills_path = self.dx_path.join("skills");
        if skills_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&skills_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if self.is_new_skill(&path).await? {
                        return Ok(Some(LocalDiff {
                            kind: DiffKind::NewSkill,
                            path: path.clone(),
                            description: format!("New skill: {:?}", path.file_name()),
                            detected_at: Utc::now(),
                            content: std::fs::read_to_string(&path).ok(),
                            language: None,
                        }));
                    }
                }
            }
        }

        // Check for new plugins
        let plugins_path = self.dx_path.join("plugins");
        if plugins_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&plugins_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if self.is_new_plugin(&path).await? {
                        return Ok(Some(LocalDiff {
                            kind: DiffKind::NewPlugin,
                            path: path.clone(),
                            description: format!("New plugin: {:?}", path.file_name()),
                            detected_at: Utc::now(),
                            content: None, // WASM binary, no text content
                            language: Some("wasm".to_string()),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check if an integration is new (not in upstream)
    async fn is_new_integration(&self, path: &Path) -> Result<bool> {
        // In a real implementation, this would:
        // 1. Fetch the list of integrations from upstream
        // 2. Compare with local integrations
        // 3. Return true if not found upstream

        // For now, check if there's a .new marker file
        let marker = path.with_extension("new");
        Ok(marker.exists())
    }

    /// Check if a skill is new
    async fn is_new_skill(&self, path: &Path) -> Result<bool> {
        let marker = path.with_extension("new");
        Ok(marker.exists())
    }

    /// Check if a plugin is new
    async fn is_new_plugin(&self, path: &Path) -> Result<bool> {
        let marker = path.with_extension("new");
        Ok(marker.exists())
    }

    /// Detect the programming language of a file
    fn detect_language(&self, path: &Path) -> Option<String> {
        path.extension().and_then(|e| e.to_str()).map(|ext| {
            match ext {
                "py" => "python",
                "js" | "ts" => "javascript",
                "rs" => "rust",
                "go" => "go",
                "sr" => "dx-serializer",
                "wasm" => "wasm",
                _ => ext,
            }
            .to_string()
        })
    }

    /// Queue a new integration for PR creation
    pub async fn queue_new_integration(
        &self,
        name: &str,
        code: &str,
        language: &str,
    ) -> Result<()> {
        info!("Queuing new integration for PR: {}", name);

        // Save the integration with a .new marker
        let path = self
            .dx_path
            .join("integrations")
            .join(format!("{}.{}", name, language));
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, code)?;

        // Create the .new marker
        let marker = path.with_extension("new");
        std::fs::write(&marker, "")?;

        Ok(())
    }

    /// Create a PR for a detected change
    pub async fn create_pr(&self, diff: &LocalDiff) -> Result<()> {
        info!("Creating PR for {:?}: {}", diff.kind, diff.description);

        // In a real implementation, this would:
        // 1. Fork the upstream repo (if not already forked)
        // 2. Create a new branch
        // 3. Add the changed files
        // 4. Commit with a meaningful message
        // 5. Push to the fork
        // 6. Create a PR via GitHub API

        // Generate PR title and body in DX format (token-efficient)
        let pr_title = match diff.kind {
            DiffKind::NewIntegration => format!(
                "feat(integration): add {}",
                diff.path.file_stem().unwrap().to_str().unwrap()
            ),
            DiffKind::NewSkill => format!(
                "feat(skill): add {}",
                diff.path.file_stem().unwrap().to_str().unwrap()
            ),
            DiffKind::NewPlugin => format!(
                "feat(plugin): add {}",
                diff.path.file_stem().unwrap().to_str().unwrap()
            ),
            DiffKind::ConfigChange => format!(
                "chore(config): update {}",
                diff.path.file_stem().unwrap().to_str().unwrap()
            ),
            DiffKind::Other => format!(
                "chore: update {}",
                diff.path.file_stem().unwrap().to_str().unwrap()
            ),
        };

        let pr_body = format!(
            "## Auto-generated PR from DX Agent\n\n\
             **Type**: {:?}\n\
             **Path**: {:?}\n\
             **Description**: {}\n\n\
             This PR was automatically created by the DX Agent when it detected a local \
             change that could benefit the community.\n\n\
             ### DX Serializer Format\n\n\
             ```\n\
             pr:1[type={:?} path={:?} auto=true]\n\
             ```",
            diff.kind, diff.path, diff.description, diff.kind, diff.path
        );

        info!("PR Title: {}", pr_title);
        info!("PR Body:\n{}", pr_body);

        // Remove the .new marker after PR is created
        let marker = diff.path.with_extension("new");
        if marker.exists() {
            std::fs::remove_file(marker)?;
        }

        Ok(())
    }

    /// Get the list of pending PRs
    pub fn pending_prs(&self) -> &[QueuedPr] {
        &self.queued_integrations
    }
}
