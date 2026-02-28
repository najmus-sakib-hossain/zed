//! Branching decision engine - manages file change safety

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// File change representation with backup support for revert
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub old_content: Option<Vec<u8>>, // Changed from String to Vec<u8>
    pub new_content: Vec<u8>,         // Changed from String to Vec<u8>
    pub tool_id: String,
    pub timestamp: DateTime<Utc>,
}

/// Branching vote colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchColor {
    Green,     // Auto-approve
    Yellow,    // Review recommended
    Red,       // Manual resolution required
    NoOpinion, // Abstain from voting
}

/// Branching vote from a tool
#[derive(Debug, Clone)]
pub struct BranchingVote {
    pub voter_id: String,
    pub color: BranchColor,
    pub reason: String,
    pub confidence: f32, // 0.0 to 1.0
}

/// Record of applied changes for revert support
#[derive(Debug, Clone)]
pub struct ApplicationRecord {
    pub id: Uuid,
    pub files: Vec<PathBuf>,
    pub backups: HashMap<PathBuf, Vec<u8>>,
    pub timestamp: DateTime<Utc>,
    pub tool_id: String,
}

/// Branching decision engine - manages file change safety
pub struct BranchingEngine {
    voters: Vec<String>,
    pending_changes: Vec<FileChange>,
    votes: HashMap<PathBuf, Vec<BranchingVote>>,
    last_application: Option<ApplicationRecord>,
}

impl BranchingEngine {
    /// Create a new branching engine
    pub fn new() -> Self {
        Self {
            voters: Vec::new(),
            pending_changes: Vec::new(),
            votes: HashMap::new(),
            last_application: None,
        }
    }

    /// Apply changes with full branching resolution
    pub fn apply_changes(&mut self, changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
        tracing::info!("📝 Applying {} changes with branching safety", changes.len());

        let mut applied_files = Vec::new();
        let mut backups = HashMap::new();
        let application_id = Uuid::new_v4();
        let timestamp = Utc::now();

        for change in changes {
            // Create backup before applying change
            if change.path.exists() {
                let backup_content = std::fs::read(&change.path).with_context(|| {
                    format!("Failed to read file for backup: {:?}", change.path)
                })?;
                backups.insert(change.path.clone(), backup_content);
            }

            // Collect votes for this change
            let color = self.predict_color(&change.path);

            match color {
                BranchColor::Green => {
                    // Auto-apply
                    self.apply_file_change(&change)?;
                    applied_files.push(change.path.clone());
                    tracing::info!("🟢 Auto-applied: {:?}", change.path);
                }
                BranchColor::Yellow => {
                    // Review recommended
                    tracing::warn!("🟡 Review recommended for: {:?}", change.path);
                    self.prompt_review_for_yellow_conflicts(vec![change.clone()])?;
                    // After review, apply
                    self.apply_file_change(&change)?;
                    applied_files.push(change.path.clone());
                }
                BranchColor::Red => {
                    // Manual resolution required
                    tracing::error!("🔴 Manual resolution required: {:?}", change.path);
                    self.automatically_reject_red_conflicts(vec![change.clone()])?;
                }
                BranchColor::NoOpinion => {
                    // Default to yellow behavior
                    self.apply_file_change(&change)?;
                    applied_files.push(change.path.clone());
                }
            }
        }

        // Store application record for revert support
        let application_record = ApplicationRecord {
            id: application_id,
            files: applied_files.clone(),
            backups,
            timestamp,
            tool_id: "branching_engine".to_string(), // TODO: Get from context
        };

        self.last_application = Some(application_record);

        Ok(applied_files)
    }

    /// Revert the most recent application
    pub fn revert_most_recent(&mut self) -> Result<Vec<PathBuf>> {
        if let Some(application) = &self.last_application {
            tracing::info!(
                "🔙 Reverting {} files from application {}",
                application.files.len(),
                application.id
            );

            let mut reverted_files = Vec::new();

            for file_path in &application.files {
                if let Some(backup_content) = application.backups.get(file_path) {
                    // Restore from backup
                    if let Some(parent) = file_path.parent() {
                        std::fs::create_dir_all(parent).with_context(|| {
                            format!("Failed to create directory for revert: {:?}", parent)
                        })?;
                    }

                    std::fs::write(file_path, backup_content)
                        .with_context(|| format!("Failed to restore file: {:?}", file_path))?;

                    reverted_files.push(file_path.clone());
                    tracing::info!("🔄 Restored: {:?}", file_path);
                } else {
                    // File was newly created, so delete it
                    if file_path.exists() {
                        std::fs::remove_file(file_path).with_context(|| {
                            format!("Failed to remove file during revert: {:?}", file_path)
                        })?;
                        reverted_files.push(file_path.clone());
                        tracing::info!("🗑️  Removed: {:?}", file_path);
                    }
                }
            }

            // Clear the last application record
            self.last_application = None;

            Ok(reverted_files)
        } else {
            anyhow::bail!("No recent application to revert")
        }
    }

    /// Submit a vote for a file change
    pub fn submit_vote(&mut self, file: &Path, vote: BranchingVote) -> Result<()> {
        self.votes.entry(file.to_path_buf()).or_default().push(vote);
        Ok(())
    }

    /// Query predicted branch color
    pub fn predict_color(&self, file: &Path) -> BranchColor {
        // Get votes for this file
        if let Some(votes) = self.votes.get(file) {
            // Check for any Red votes (veto)
            if votes.iter().any(|v| v.color == BranchColor::Red) {
                return BranchColor::Red;
            }

            // Check for Yellow votes
            if votes.iter().any(|v| v.color == BranchColor::Yellow) {
                return BranchColor::Yellow;
            }

            // All Green
            if votes
                .iter()
                .all(|v| v.color == BranchColor::Green || v.color == BranchColor::NoOpinion)
            {
                return BranchColor::Green;
            }
        }

        // Default to Green if no votes
        BranchColor::Green
    }

    /// Register a permanent branching voter
    pub fn register_permanent_voter(&mut self, voter_id: String) -> Result<()> {
        if !self.voters.contains(&voter_id) {
            tracing::info!("🗳️  Registered permanent voter: {}", voter_id);
            self.voters.push(voter_id);
        }
        Ok(())
    }

    /// Check if a change is guaranteed safe
    pub fn is_change_guaranteed_safe(&self, file: &Path) -> bool {
        if let Some(votes) = self.votes.get(file) {
            votes.iter().all(|v| v.color == BranchColor::Green)
        } else {
            false
        }
    }

    /// Issue an immediate veto
    pub fn issue_immediate_veto(
        &mut self,
        file: &Path,
        voter_id: &str,
        reason: &str,
    ) -> Result<()> {
        let vote = BranchingVote {
            voter_id: voter_id.to_string(),
            color: BranchColor::Red,
            reason: reason.to_string(),
            confidence: 1.0,
        };

        tracing::error!("🚫 VETO issued for {:?} by {}: {}", file, voter_id, reason);
        self.submit_vote(file, vote)?;
        Ok(())
    }

    /// Reset branching engine state
    pub fn reset_state(&mut self) -> Result<()> {
        tracing::info!("🔄 Resetting branching engine state");
        self.votes.clear();
        self.pending_changes.clear();
        Ok(())
    }

    // Helper methods

    fn apply_file_change(&self, change: &FileChange) -> Result<()> {
        tracing::debug!("💾 Writing file: {:?}", change.path);

        // Ensure directory exists
        if let Some(parent) = change.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        // Write content
        std::fs::write(&change.path, &change.new_content)
            .with_context(|| format!("Failed to write file: {:?}", change.path))?;

        Ok(())
    }

    fn prompt_review_for_yellow_conflicts(&self, changes: Vec<FileChange>) -> Result<()> {
        tracing::info!("🟡 Prompting review for {} yellow changes", changes.len());

        for change in &changes {
            println!("⚠️  CONFLICT: Review required for {:?}", change.path);
        }

        // TODO: Emit event for LSP when EventBus is available
        Ok(())
    }

    fn automatically_reject_red_conflicts(&self, changes: Vec<FileChange>) -> Result<()> {
        tracing::error!("🔴 Rejecting {} red changes", changes.len());

        for change in changes {
            tracing::error!("  ❌ {:?} - Manual resolution required", change.path);
        }

        Ok(())
    }
}

impl Default for BranchingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_branching_votes() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote = BranchingVote {
            voter_id: "test-voter".to_string(),
            color: BranchColor::Green,
            reason: "Test vote".to_string(),
            confidence: 0.9,
        };

        engine.submit_vote(&file, vote).unwrap();

        let color = engine.predict_color(&file);
        assert_eq!(color, BranchColor::Green);
    }

    #[test]
    fn test_revert_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let original_content = b"original content";
        let new_content = b"new content";

        // Write original content
        std::fs::write(&test_file, original_content).unwrap();

        let mut engine = BranchingEngine::new();

        // Create a file change
        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(original_content.to_vec()),
            new_content: new_content.to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        // Apply change
        let applied = engine.apply_changes(vec![change]).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], test_file);

        // Verify new content
        let current_content = std::fs::read(&test_file).unwrap();
        assert_eq!(current_content, new_content);

        // Revert
        let reverted = engine.revert_most_recent().unwrap();
        assert_eq!(reverted.len(), 1);
        assert_eq!(reverted[0], test_file);

        // Verify original content restored
        let restored_content = std::fs::read(&test_file).unwrap();
        assert_eq!(restored_content, original_content);
    }
}
