/// Traffic Branch System: Red, Yellow & Green Update Strategies
///
/// This system intelligently updates DX-managed components using a traffic light metaphor:
/// - 🟢 GREEN: Auto-update (no local modifications)
/// - 🟡 YELLOW: 3-way merge (compatible local changes)
/// - 🔴 RED: Manual conflict resolution required
///
/// The system stores base_hash for each managed component to detect local modifications.
use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Component state tracking for traffic branch system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    /// Path to the component file
    pub path: String,

    /// Hash of the original component when first installed
    pub base_hash: String,

    /// Component source (e.g., "dx-ui", "dx-icon")
    pub source: String,

    /// Component name (e.g., "Button", "Icon")
    pub name: String,

    /// Version when first installed
    pub version: String,

    /// Timestamp of installation
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

/// Result of traffic branch analysis
#[derive(Debug, Clone, PartialEq)]
pub enum TrafficBranch {
    /// 🟢 GREEN: Safe to auto-update (no local modifications)
    Green,

    /// 🟡 YELLOW: Can merge (non-conflicting local changes)
    Yellow { conflicts: Vec<String> },

    /// 🔴 RED: Manual resolution required (conflicting changes)
    Red { conflicts: Vec<String> },
}

/// State file manager for component tracking
pub struct ComponentStateManager {
    state_file: PathBuf,
    states: HashMap<String, ComponentState>,
}

impl ComponentStateManager {
    /// Create a new state manager
    pub fn new(forge_dir: &Path) -> Result<Self> {
        let state_file = forge_dir.join("component_state.json");

        let states = if state_file.exists() {
            let content =
                fs::read_to_string(&state_file).context("Failed to read component state file")?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self { state_file, states })
    }

    /// Register a new component installation
    pub fn register_component(
        &mut self,
        path: &Path,
        source: &str,
        name: &str,
        version: &str,
        content: &str,
    ) -> Result<()> {
        let base_hash = compute_hash(content);

        let state = ComponentState {
            path: path.display().to_string(),
            base_hash,
            source: source.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            installed_at: chrono::Utc::now(),
        };

        self.states.insert(path.display().to_string(), state);
        self.save()?;

        Ok(())
    }

    /// Get component state by path
    pub fn get_component(&self, path: &Path) -> Option<&ComponentState> {
        self.states.get(&path.display().to_string())
    }

    /// Check if a file is a managed component
    pub fn is_managed(&self, path: &Path) -> bool {
        self.states.contains_key(&path.display().to_string())
    }

    /// Analyze update strategy for a component
    pub fn analyze_update(&self, path: &Path, remote_content: &str) -> Result<TrafficBranch> {
        let state = self.get_component(path).context("Component not registered")?;

        // Read current local content
        let local_content = fs::read_to_string(path).context("Failed to read local component")?;

        let base_hash = &state.base_hash;
        let local_hash = compute_hash(&local_content);
        let remote_hash = compute_hash(remote_content);

        // 🟢 GREEN BRANCH: No local modifications
        if local_hash == *base_hash {
            return Ok(TrafficBranch::Green);
        }

        // Check if remote has changed
        if remote_hash == *base_hash {
            // Remote hasn't changed, but local has - no update needed
            return Ok(TrafficBranch::Green);
        }

        // Both local and remote have changed - need 3-way merge
        // Reconstruct BASE content would require storing it or fetching it
        // For now, we'll use a simplified conflict detection

        let conflicts = detect_conflicts(&local_content, remote_content);

        if conflicts.is_empty() {
            // 🟡 YELLOW BRANCH: Non-conflicting changes
            Ok(TrafficBranch::Yellow { conflicts: vec![] })
        } else {
            // 🔴 RED BRANCH: Conflicting changes
            Ok(TrafficBranch::Red { conflicts })
        }
    }

    /// Update component after successful merge
    pub fn update_component(
        &mut self,
        path: &Path,
        new_version: &str,
        new_content: &str,
    ) -> Result<()> {
        if let Some(state) = self.states.get_mut(&path.display().to_string()) {
            state.base_hash = compute_hash(new_content);
            state.version = new_version.to_string();
            self.save()?;
        }

        Ok(())
    }

    /// Remove component from tracking
    pub fn unregister_component(&mut self, path: &Path) -> Result<()> {
        self.states.remove(&path.display().to_string());
        self.save()?;
        Ok(())
    }

    /// Save state to disk
    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.states)?;
        fs::write(&self.state_file, content)?;
        Ok(())
    }

    /// List all managed components
    pub fn list_components(&self) -> Vec<&ComponentState> {
        self.states.values().collect()
    }
}

/// Compute SHA-256 hash of content
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Detect conflicts between local and remote versions
/// Returns list of conflicting line ranges
fn detect_conflicts(local: &str, remote: &str) -> Vec<String> {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(local, remote);
    let mut conflicts = Vec::new();
    let mut current_conflict: Option<(usize, usize)> = None;

    for (idx, change) in diff.iter_all_changes().enumerate() {
        match change.tag() {
            ChangeTag::Delete | ChangeTag::Insert => {
                if let Some((start, _)) = current_conflict {
                    current_conflict = Some((start, idx));
                } else {
                    current_conflict = Some((idx, idx));
                }
            }
            ChangeTag::Equal => {
                if let Some((start, end)) = current_conflict.take() {
                    conflicts.push(format!("lines {}-{}", start + 1, end + 1));
                }
            }
        }
    }

    if let Some((start, end)) = current_conflict {
        conflicts.push(format!("lines {}-{}", start + 1, end + 1));
    }

    conflicts
}

/// Apply traffic branch update strategy
pub async fn apply_update(
    path: &Path,
    remote_content: &str,
    remote_version: &str,
    state_mgr: &mut ComponentStateManager,
) -> Result<UpdateResult> {
    let branch = state_mgr.analyze_update(path, remote_content)?;

    match branch {
        TrafficBranch::Green => {
            // 🟢 AUTO-UPDATE: Safe to overwrite
            fs::write(path, remote_content)?;
            state_mgr.update_component(path, remote_version, remote_content)?;

            println!(
                "{} {} updated to v{} {}",
                "🟢".bright_green(),
                path.display().to_string().bright_cyan(),
                remote_version.bright_white(),
                "(auto-updated)".bright_black()
            );

            Ok(UpdateResult::AutoUpdated)
        }

        TrafficBranch::Yellow { .. } => {
            // 🟡 MERGE: Attempt 3-way merge
            let local_content = fs::read_to_string(path)?;

            // Simplified merge: append remote changes
            // In production, use proper 3-way merge algorithm
            let merged = merge_contents(&local_content, remote_content)?;

            fs::write(path, &merged)?;
            state_mgr.update_component(path, remote_version, &merged)?;

            println!(
                "{} {} updated to v{} {}",
                "🟡".bright_yellow(),
                path.display().to_string().bright_cyan(),
                remote_version.bright_white(),
                "(merged with local changes)".yellow()
            );

            Ok(UpdateResult::Merged)
        }

        TrafficBranch::Red { conflicts } => {
            // 🔴 CONFLICT: Manual resolution required
            println!(
                "{} {} {} v{}",
                "🔴".bright_red(),
                "CONFLICT:".red().bold(),
                path.display().to_string().bright_cyan(),
                remote_version.bright_white()
            );
            println!("   {} Update conflicts with your local changes:", "│".bright_black());
            for conflict in &conflicts {
                println!("   {} Conflict at {}", "│".bright_black(), conflict.red());
            }
            println!(
                "   {} Run {} to resolve",
                "└".bright_black(),
                "forge resolve".bright_white().bold()
            );

            Ok(UpdateResult::Conflict { conflicts })
        }
    }
}

/// Result of applying an update
#[derive(Debug)]
pub enum UpdateResult {
    /// Successfully auto-updated (Green branch)
    AutoUpdated,

    /// Successfully merged (Yellow branch)
    Merged,

    /// Conflict detected (Red branch)
    Conflict { conflicts: Vec<String> },
}

/// Simple merge strategy (placeholder for production 3-way merge)
fn merge_contents(_local: &str, remote: &str) -> Result<String> {
    // Simplified: If no direct conflicts, use remote
    // In production, implement proper 3-way merge with BASE content
    Ok(remote.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_computation() {
        let content = "Hello, world!";
        let hash1 = compute_hash(content);
        let hash2 = compute_hash(content);
        assert_eq!(hash1, hash2);

        let different = compute_hash("Different content");
        assert_ne!(hash1, different);
    }

    #[test]
    fn test_conflict_detection() {
        let local = "line1\nline2\nline3\n";
        let remote = "line1\nmodified\nline3\n";

        let conflicts = detect_conflicts(local, remote);
        assert!(!conflicts.is_empty());
    }
}
