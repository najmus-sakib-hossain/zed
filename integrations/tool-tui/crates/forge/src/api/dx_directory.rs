//! .dx/ Directory — The Transparent, Version-Controlled Brain APIs
//!
//! This module provides APIs for managing the `.dx/` directory, which stores
//! tool configurations, cached binaries, and state history.

use anyhow::Result;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// Returns the path to the `.dx/` directory for the current workspace.
///
/// # Returns
///
/// The path to the `.dx/` directory within the detected workspace root.
pub fn get_dx_directory_path() -> Result<PathBuf> {
    let root = crate::api::cicd::detect_workspace_root()?;
    Ok(root.join(".dx"))
}

/// Returns the path to the binary storage directory within `.dx/`.
///
/// # Returns
///
/// The path to `.dx/binaries/` where tool binaries are cached.
pub fn get_dx_binary_storage_path() -> Result<PathBuf> {
    Ok(get_dx_directory_path()?.join("binaries"))
}

/// Caches a tool's binary for offline use.
///
/// Stores the binary data in `.dx/binaries/{tool_name}.bin`.
///
/// # Arguments
///
/// * `tool_name` - The name of the tool
/// * `binary_data` - The raw binary data to cache
///
/// # Returns
///
/// `Ok(())` on successful caching.
pub fn cache_tool_offline_binary(tool_name: &str, binary_data: &[u8]) -> Result<()> {
    let path = get_dx_binary_storage_path()?.join(format!("{}.bin", tool_name));
    let parent_dir = path.parent().ok_or_else(|| {
        anyhow::anyhow!("Invalid binary cache path: {:?} has no parent directory", path)
    })?;
    std::fs::create_dir_all(parent_dir)?;
    std::fs::write(&path, binary_data)?;
    tracing::info!("💾 Cached binary for {}: {:?}", tool_name, path);
    Ok(())
}

/// Loads a cached tool binary from the `.dx/binaries/` directory.
///
/// # Arguments
///
/// * `tool_name` - The name of the tool whose binary to load
///
/// # Returns
///
/// The raw binary data if found.
pub fn load_tool_offline_binary(tool_name: &str) -> Result<Vec<u8>> {
    let path = get_dx_binary_storage_path()?.join(format!("{}.bin", tool_name));
    Ok(std::fs::read(&path)?)
}

/// Commits the current `.dx/` state with a message.
///
/// Creates a snapshot of the current tool configuration and state, storing it
/// in `.dx/state/{commit_id}.json`.
///
/// # Arguments
///
/// * `message` - A description of the state being committed
///
/// # Returns
///
/// The unique commit ID (UUID) for the saved state.
pub fn commit_current_dx_state(message: &str) -> Result<String> {
    tracing::info!("💾 Committing dx state: {}", message);
    let commit_id = uuid::Uuid::new_v4().to_string();

    let dx_dir = get_dx_directory_path()?;
    let state_dir = dx_dir.join("state");
    fs::create_dir_all(&state_dir)?;

    let timestamp = chrono::Utc::now().to_rfc3339();

    let state = json!({
        "id": commit_id,
        "message": message,
        "timestamp": timestamp,
        "tools": crate::core::Forge::new(crate::api::cicd::detect_workspace_root()?)?.list_tools(),
    });

    let state_file = state_dir.join(format!("{}.json", commit_id));
    fs::write(&state_file, serde_json::to_string_pretty(&state)?)?;

    tracing::info!("✅ State committed to {:?}", state_file);

    Ok(commit_id)
}

/// Checks out a previously committed `.dx/` state.
///
/// # Status
///
/// **Not yet implemented** - Currently logs the checkout request and returns `Ok(())`.
/// Future implementation will restore tool configurations and state from the specified
/// commit.
///
/// # Arguments
///
/// * `state_id` - The commit ID of the state to restore
///
/// # Returns
///
/// Currently always returns `Ok(())`. Will return checkout confirmation when implemented.
pub fn checkout_dx_state(state_id: &str) -> Result<()> {
    tracing::info!("🔄 Checking out dx state: {}", state_id);
    Ok(())
}

/// Lists the history of `.dx/` state commits.
///
/// # Status
///
/// **Not yet implemented** - Currently returns an empty vector.
/// Future implementation will scan `.dx/state/` and return commit history.
///
/// # Returns
///
/// Currently returns an empty `Vec`. Will return tuples of (commit_id, message, timestamp)
/// when implemented.
pub fn list_dx_history() -> Result<Vec<(String, String, i64)>> {
    // Returns (commit_id, message, timestamp)
    Ok(Vec::new())
}

/// Shows the diff between two `.dx/` state commits.
///
/// # Status
///
/// **Partially implemented** - Currently returns a placeholder diff string.
/// Future implementation will compute and display actual differences between states.
///
/// # Arguments
///
/// * `from_state` - The source commit ID
/// * `to_state` - The target commit ID
///
/// # Returns
///
/// Currently returns a placeholder string. Will return formatted diff output when implemented.
pub fn show_dx_state_diff(from_state: &str, to_state: &str) -> Result<String> {
    Ok(format!("Diff from {} to {}", from_state, to_state))
}

/// Pushes the current `.dx/` state to a remote location.
///
/// # Status
///
/// **Not yet implemented** - Currently logs the push request and returns `Ok(())`.
/// Future implementation will sync state to remote storage (R2, S3, or git remote).
///
/// # Arguments
///
/// * `remote_url` - The URL of the remote to push to
///
/// # Returns
///
/// Currently always returns `Ok(())`. Will return push confirmation when implemented.
pub fn push_dx_state_to_remote(remote_url: &str) -> Result<()> {
    tracing::info!("☁️  Pushing dx state to: {}", remote_url);
    Ok(())
}

/// Pulls `.dx/` state from a remote location.
///
/// # Status
///
/// **Not yet implemented** - Currently logs the pull request and returns `Ok(())`.
/// Future implementation will fetch and merge state from remote storage.
///
/// # Arguments
///
/// * `remote_url` - The URL of the remote to pull from
///
/// # Returns
///
/// Currently always returns `Ok(())`. Will return pull confirmation when implemented.
pub fn pull_dx_state_from_remote(remote_url: &str) -> Result<()> {
    tracing::info!("☁️  Pulling dx state from: {}", remote_url);
    Ok(())
}
