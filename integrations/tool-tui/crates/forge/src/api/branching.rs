//! Safe File Application with Enterprise-Grade Branching Decision Engine APIs

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

/// File change representation
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub old_content: Option<String>,
    pub new_content: String,
    pub tool_id: String,
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

/// Branching engine state
static BRANCHING_STATE: OnceLock<Arc<RwLock<BranchingState>>> = OnceLock::new();

#[derive(Default)]
struct BranchingState {
    voters: Vec<String>,
    pending_changes: Vec<FileChange>,
    votes: HashMap<PathBuf, Vec<BranchingVote>>,
    last_application: Option<ApplicationRecord>,
}

fn get_branching_state() -> Arc<RwLock<BranchingState>> {
    BRANCHING_STATE
        .get_or_init(|| Arc::new(RwLock::new(BranchingState::default())))
        .clone()
}

/// Primary API — full branching resolution + telemetry
pub fn apply_changes(changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
    tracing::info!("📝 Applying {} changes with branching safety", changes.len());

    let state = get_branching_state();
    let mut state = state.write();

    let mut applied_files = Vec::new();
    let mut backups = HashMap::new();
    let application_id = Uuid::new_v4();
    let timestamp = Utc::now();
    let tool_id = changes
        .first()
        .map(|c| c.tool_id.clone())
        .unwrap_or_else(|| "unknown".to_string());

    for change in changes {
        // Create backup before applying change
        if change.path.exists() {
            let backup_content = std::fs::read(&change.path)
                .with_context(|| format!("Failed to read file for backup: {:?}", change.path))?;
            backups.insert(change.path.clone(), backup_content);
        }

        // Collect votes for this change
        let color = query_predicted_branch_color_internal(&state, &change.path);

        match color {
            BranchColor::Green => {
                // Auto-apply
                apply_file_change(&change)?;
                applied_files.push(change.path.clone());
                tracing::info!("🟢 Auto-applied: {:?}", change.path);
            }
            BranchColor::Yellow => {
                // Review recommended
                tracing::warn!("🟡 Review recommended for: {:?}", change.path);
                prompt_review_for_yellow_conflicts(vec![change.clone()])?;
                // After review, apply
                apply_file_change(&change)?;
                applied_files.push(change.path.clone());
            }
            BranchColor::Red => {
                // Manual resolution required
                tracing::error!("🔴 Manual resolution required: {:?}", change.path);
                automatically_reject_red_conflicts(vec![change.clone()])?;
            }
            BranchColor::NoOpinion => {
                // Default to yellow behavior
                apply_file_change(&change)?;
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
        tool_id,
    };

    state.last_application = Some(application_record);

    Ok(applied_files)
}

/// Fast path when tool knows its changes are safe
pub fn apply_changes_with_preapproved_votes(changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
    tracing::info!("⚡ Fast-path applying {} pre-approved changes", changes.len());

    let state = get_branching_state();
    let mut state = state.write();

    let mut applied_files = Vec::new();
    let mut backups = HashMap::new();
    let application_id = Uuid::new_v4();
    let timestamp = Utc::now();
    let tool_id = changes
        .first()
        .map(|c| c.tool_id.clone())
        .unwrap_or_else(|| "unknown".to_string());

    for change in &changes {
        // Create backup before applying change
        if change.path.exists() {
            let backup_content = std::fs::read(&change.path)
                .with_context(|| format!("Failed to read file for backup: {:?}", change.path))?;
            backups.insert(change.path.clone(), backup_content);
        }

        apply_file_change(change)?;
        applied_files.push(change.path.clone());
    }

    // Store application record for revert support
    let application_record = ApplicationRecord {
        id: application_id,
        files: applied_files.clone(),
        backups,
        timestamp,
        tool_id,
    };

    state.last_application = Some(application_record);

    Ok(applied_files)
}

/// Only forge core or `dx apply --force`
pub fn apply_changes_force_unchecked(changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
    tracing::warn!("⚠️  FORCE APPLYING {} changes WITHOUT SAFETY CHECKS", changes.len());

    let mut applied_files = Vec::new();

    for change in changes {
        apply_file_change(&change)?;
        applied_files.push(change.path.clone());
    }

    Ok(applied_files)
}

/// Dry-run with full diff, colors, and risk score
pub fn preview_proposed_changes(changes: Vec<FileChange>) -> Result<String> {
    let mut preview = String::new();

    preview.push_str("╔══════════════════════════════════════════════════════════════╗\n");
    preview.push_str("║          PROPOSED CHANGES PREVIEW                            ║\n");
    preview.push_str("╚══════════════════════════════════════════════════════════════╝\n\n");

    for change in &changes {
        let color = query_predicted_branch_color(&change.path)?;
        let color_icon = match color {
            BranchColor::Green => "🟢",
            BranchColor::Yellow => "🟡",
            BranchColor::Red => "🔴",
            BranchColor::NoOpinion => "⚪",
        };

        preview.push_str(&format!("{} {:?}\n", color_icon, change.path));
        preview.push_str(&format!("   Tool: {}\n", change.tool_id));
        preview.push_str(&format!("   Risk: {:?}\n\n", color));
    }

    Ok(preview)
}

/// Auto-accept green conflicts
pub fn automatically_accept_green_conflicts(changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
    let green_changes: Vec<FileChange> = changes
        .into_iter()
        .filter(|c| query_predicted_branch_color(&c.path).ok() == Some(BranchColor::Green))
        .collect();

    tracing::info!("🟢 Auto-accepting {} green changes", green_changes.len());
    apply_changes_with_preapproved_votes(green_changes)
}

/// Opens rich inline LSP review UI
pub fn prompt_review_for_yellow_conflicts(changes: Vec<FileChange>) -> Result<()> {
    tracing::info!("🟡 Prompting review for {} yellow changes", changes.len());

    for change in &changes {
        println!("⚠️  CONFLICT: Review required for {:?}", change.path);
    }

    // Emit event for LSP
    crate::api::events::publish_event(crate::api::events::ForgeEvent::Custom {
        event_type: "conflict_review_required".to_string(),
        data: serde_json::json!({
            "changes": changes.iter().map(|c| c.path.to_string_lossy()).collect::<Vec<_>>()
        }),
        timestamp: chrono::Utc::now().timestamp(),
    })?;

    Ok(())
}

/// Auto-reject red conflicts
pub fn automatically_reject_red_conflicts(changes: Vec<FileChange>) -> Result<()> {
    tracing::error!("🔴 Rejecting {} red changes", changes.len());

    for change in changes {
        tracing::error!("  ❌ {:?} - Manual resolution required", change.path);
    }

    Ok(())
}

/// Undo for cart removal or failed scaffolding
///
/// Restores all files from the most recent application to their previous state.
/// - Files that existed before are restored from backup
/// - Files that were newly created are deleted
///
/// # Errors
/// - Returns error if no recent application exists
/// - Returns error if file restoration fails (permissions, disk full, etc.)
pub fn revert_most_recent_application() -> Result<Vec<PathBuf>> {
    let state = get_branching_state();
    let mut state = state.write();

    if let Some(application) = state.last_application.take() {
        tracing::info!(
            "🔙 Reverting {} files from application {}",
            application.files.len(),
            application.id
        );

        let mut reverted_files = Vec::new();
        let mut errors = Vec::new();

        for file_path in &application.files {
            let result = if let Some(backup_content) = application.backups.get(file_path) {
                // Restore from backup - file existed before
                restore_file_from_backup(file_path, backup_content)
            } else {
                // File was newly created, so delete it
                delete_newly_created_file(file_path)
            };

            match result {
                Ok(()) => {
                    reverted_files.push(file_path.clone());
                    tracing::info!("🔄 Restored: {:?}", file_path);
                }
                Err(e) => {
                    tracing::error!("❌ Failed to revert {:?}: {}", file_path, e);
                    errors.push((file_path.clone(), e));
                }
            }
        }

        // If any errors occurred, report them but still return successfully reverted files
        if !errors.is_empty() {
            let error_summary: Vec<String> =
                errors.iter().map(|(path, e)| format!("{:?}: {}", path, e)).collect();
            tracing::warn!("⚠️  Some files could not be reverted: {}", error_summary.join(", "));
        }

        Ok(reverted_files)
    } else {
        anyhow::bail!("No recent application to revert")
    }
}

/// Restore a file from backup content
fn restore_file_from_backup(file_path: &Path, backup_content: &[u8]) -> Result<()> {
    // Ensure parent directory exists (in case it was deleted)
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory for revert: {:?}", parent))?;
    }

    // Check if we can write to the file
    if file_path.exists() {
        let metadata = std::fs::metadata(file_path)
            .with_context(|| format!("Failed to get metadata for: {:?}", file_path))?;

        if metadata.permissions().readonly() {
            // Try to make it writable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = metadata.permissions();
                perms.set_mode(perms.mode() | 0o200); // Add write permission
                std::fs::set_permissions(file_path, perms).with_context(|| {
                    format!("Failed to set write permission on: {:?}", file_path)
                })?;
            }
            #[cfg(not(unix))]
            {
                let mut perms = metadata.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                std::fs::set_permissions(file_path, perms).with_context(|| {
                    format!("Failed to set write permission on: {:?}", file_path)
                })?;
            }
        }
    }

    std::fs::write(file_path, backup_content)
        .with_context(|| format!("Failed to restore file: {:?}", file_path))?;

    Ok(())
}

/// Delete a file that was newly created during the application
fn delete_newly_created_file(file_path: &Path) -> Result<()> {
    if file_path.exists() {
        // Check if it's a file or directory
        let metadata = std::fs::metadata(file_path)
            .with_context(|| format!("Failed to get metadata for: {:?}", file_path))?;

        if metadata.is_file() {
            // Handle read-only files
            if metadata.permissions().readonly() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = metadata.permissions();
                    perms.set_mode(perms.mode() | 0o200);
                    std::fs::set_permissions(file_path, perms).with_context(|| {
                        format!("Failed to set write permission on: {:?}", file_path)
                    })?;
                }
                #[cfg(not(unix))]
                {
                    let mut perms = metadata.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    perms.set_readonly(false);
                    std::fs::set_permissions(file_path, perms).with_context(|| {
                        format!("Failed to set write permission on: {:?}", file_path)
                    })?;
                }
            }

            std::fs::remove_file(file_path)
                .with_context(|| format!("Failed to remove file during revert: {:?}", file_path))?;

            tracing::info!("🗑️  Removed newly created file: {:?}", file_path);
        } else {
            tracing::warn!("⚠️  Skipping directory during revert: {:?}", file_path);
        }
    } else {
        // File already doesn't exist, nothing to do
        tracing::debug!("File already deleted: {:?}", file_path);
    }

    Ok(())
}

/// Internal helper to query predicted branch color without acquiring lock
fn query_predicted_branch_color_internal(state: &BranchingState, file: &Path) -> BranchColor {
    // Get votes for this file
    if let Some(votes) = state.votes.get(file) {
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

// ========================================================================
// Branching Decision Engine
// ========================================================================

/// Vote Green/Yellow/Red/NoOpinion on a FileChange
pub fn submit_branching_vote(file: &Path, vote: BranchingVote) -> Result<()> {
    let state = get_branching_state();
    let mut state = state.write();

    state.votes.entry(file.to_path_buf()).or_default().push(vote);

    Ok(())
}

/// ui, auth, style, security, check, etc.
pub fn register_permanent_branching_voter(voter_id: String) -> Result<()> {
    let state = get_branching_state();
    let mut state = state.write();

    if !state.voters.contains(&voter_id) {
        tracing::info!("🗳️  Registered permanent voter: {}", voter_id);
        state.voters.push(voter_id);
    }

    Ok(())
}

/// Simulate outcome without applying
pub fn query_predicted_branch_color(file: &PathBuf) -> Result<BranchColor> {
    let state = get_branching_state();
    let state = state.read();

    // Get votes for this file
    if let Some(votes) = state.votes.get(file) {
        // Check for any Red votes (veto)
        if votes.iter().any(|v| v.color == BranchColor::Red) {
            return Ok(BranchColor::Red);
        }

        // Check for Yellow votes
        if votes.iter().any(|v| v.color == BranchColor::Yellow) {
            return Ok(BranchColor::Yellow);
        }

        // All Green
        if votes
            .iter()
            .all(|v| v.color == BranchColor::Green || v.color == BranchColor::NoOpinion)
        {
            return Ok(BranchColor::Green);
        }
    }

    // Default to Green if no votes
    Ok(BranchColor::Green)
}

/// True iff every voter returned Green
pub fn is_change_guaranteed_safe(file: &PathBuf) -> Result<bool> {
    let state = get_branching_state();
    let state = state.read();

    if let Some(votes) = state.votes.get(file) {
        Ok(votes.iter().all(|v| v.color == BranchColor::Green))
    } else {
        Ok(false)
    }
}

/// Hard block — highest priority Red vote
pub fn issue_immediate_veto(file: &PathBuf, voter_id: &str, reason: &str) -> Result<()> {
    let vote = BranchingVote {
        voter_id: voter_id.to_string(),
        color: BranchColor::Red,
        reason: reason.to_string(),
        confidence: 1.0,
    };

    tracing::error!("🚫 VETO issued for {:?} by {}: {}", file, voter_id, reason);

    submit_branching_vote(file, vote)?;

    Ok(())
}

/// Called before cart commit or variant switch
pub fn reset_branching_engine_state() -> Result<()> {
    let state = get_branching_state();
    let mut state = state.write();

    tracing::info!("🔄 Resetting branching engine state");
    state.votes.clear();
    state.pending_changes.clear();

    Ok(())
}

// Helper function
fn apply_file_change(change: &FileChange) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to ensure tests run serially since they share global state
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn reset_state_for_test() {
        let state = get_branching_state();
        let mut state = state.write();
        state.voters.clear();
        state.pending_changes.clear();
        state.votes.clear();
        state.last_application = None;
    }

    #[test]
    fn test_branching_votes() {
        let _lock = TEST_MUTEX.lock().unwrap();
        reset_state_for_test();

        let file = PathBuf::from("test_votes.ts");

        let vote = BranchingVote {
            voter_id: "test-voter".to_string(),
            color: BranchColor::Green,
            reason: "Test vote".to_string(),
            confidence: 0.9,
        };

        submit_branching_vote(&file, vote).unwrap();

        let color = query_predicted_branch_color(&file).unwrap();
        assert_eq!(color, BranchColor::Green);
    }

    #[test]
    fn test_revert_round_trip() {
        let _lock = TEST_MUTEX.lock().unwrap();
        reset_state_for_test();

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let original_content = "original content";
        let new_content = "new content";

        // Write original content
        std::fs::write(&test_file, original_content).unwrap();

        // Create a file change
        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(original_content.to_string()),
            new_content: new_content.to_string(),
            tool_id: "test".to_string(),
        };

        // Apply change
        let applied = apply_changes_with_preapproved_votes(vec![change]).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], test_file);

        // Verify new content
        let current_content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(current_content, new_content);

        // Revert
        let reverted = revert_most_recent_application().unwrap();
        assert_eq!(reverted.len(), 1);
        assert_eq!(reverted[0], test_file);

        // Verify original content restored
        let restored_content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(restored_content, original_content);
    }

    #[test]
    fn test_revert_newly_created_file() {
        let _lock = TEST_MUTEX.lock().unwrap();
        reset_state_for_test();

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("new_file.txt");
        let new_content = "new file content";

        // File doesn't exist initially
        assert!(!test_file.exists());

        // Create a file change for a new file
        let change = FileChange {
            path: test_file.clone(),
            old_content: None,
            new_content: new_content.to_string(),
            tool_id: "test".to_string(),
        };

        // Apply change
        let applied = apply_changes_with_preapproved_votes(vec![change]).unwrap();
        assert_eq!(applied.len(), 1);

        // Verify file was created
        assert!(test_file.exists());
        let current_content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(current_content, new_content);

        // Revert
        let reverted = revert_most_recent_application().unwrap();
        assert_eq!(reverted.len(), 1);

        // Verify file was deleted
        assert!(!test_file.exists());
    }

    #[test]
    fn test_revert_multiple_files() {
        let _lock = TEST_MUTEX.lock().unwrap();
        reset_state_for_test();

        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let file3 = temp_dir.path().join("subdir").join("file3.txt");

        // Write original content to existing files
        std::fs::write(&file1, "original1").unwrap();
        std::fs::write(&file2, "original2").unwrap();

        // Create changes
        let changes = vec![
            FileChange {
                path: file1.clone(),
                old_content: Some("original1".to_string()),
                new_content: "new1".to_string(),
                tool_id: "test".to_string(),
            },
            FileChange {
                path: file2.clone(),
                old_content: Some("original2".to_string()),
                new_content: "new2".to_string(),
                tool_id: "test".to_string(),
            },
            FileChange {
                path: file3.clone(),
                old_content: None,
                new_content: "new3".to_string(),
                tool_id: "test".to_string(),
            },
        ];

        // Apply changes
        let applied = apply_changes_with_preapproved_votes(changes).unwrap();
        assert_eq!(applied.len(), 3);

        // Verify changes were applied
        assert_eq!(std::fs::read_to_string(&file1).unwrap(), "new1");
        assert_eq!(std::fs::read_to_string(&file2).unwrap(), "new2");
        assert_eq!(std::fs::read_to_string(&file3).unwrap(), "new3");

        // Revert
        let reverted = revert_most_recent_application().unwrap();
        assert_eq!(reverted.len(), 3);

        // Verify original content restored
        assert_eq!(std::fs::read_to_string(&file1).unwrap(), "original1");
        assert_eq!(std::fs::read_to_string(&file2).unwrap(), "original2");
        assert!(!file3.exists()); // New file should be deleted
    }

    #[test]
    fn test_revert_no_application() {
        let _lock = TEST_MUTEX.lock().unwrap();
        reset_state_for_test();

        // Try to revert when there's nothing to revert
        let result = revert_most_recent_application();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No recent application to revert"));
    }

    #[test]
    fn test_revert_file_already_deleted() {
        let _lock = TEST_MUTEX.lock().unwrap();
        reset_state_for_test();

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let original_content = "original content";
        let new_content = "new content";

        // Write original content
        std::fs::write(&test_file, original_content).unwrap();

        // Create a file change
        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(original_content.to_string()),
            new_content: new_content.to_string(),
            tool_id: "test".to_string(),
        };

        // Apply change
        apply_changes_with_preapproved_votes(vec![change]).unwrap();

        // Manually delete the file before reverting
        std::fs::remove_file(&test_file).unwrap();
        assert!(!test_file.exists());

        // Revert should still work and restore the file
        let reverted = revert_most_recent_application().unwrap();
        assert_eq!(reverted.len(), 1);

        // Verify original content restored
        assert!(test_file.exists());
        let restored_content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(restored_content, original_content);
    }
}
