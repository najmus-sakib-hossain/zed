//! Self-Update Engine
//!
//! Provides autonomous configuration generation and capability acquisition
//! for the AI agent, with validation and rollback support.
//!
//! # Self-Update Process
//!
//! 1. **Gap Detection**: Identify missing capabilities
//! 2. **Config Generation**: Use LLM to generate configuration
//! 3. **Validation**: Validate via DX Forge
//! 4. **Application**: Apply configuration changes
//! 5. **Rollback**: Revert on failure
//!
//! # Modes
//!
//! - **Supervised**: Requires human approval for changes
//! - **Autonomous**: Automatically applies validated changes
//! - **Dry-run**: Generates but doesn't apply changes
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::agent::self_update::{SelfUpdateEngine, SelfUpdateConfig, UpdateMode};
//!
//! let config = SelfUpdateConfig {
//!     enabled: true,
//!     mode: UpdateMode::Supervised,
//!     ..Default::default()
//! };
//!
//! let mut engine = SelfUpdateEngine::new(config)?;
//!
//! // Acquire a new capability
//! let gap = CapabilityGap { capability: "weather".to_string(), .. };
//! let result = engine.acquire_capability(&gap).await?;
//!
//! match result {
//!     SelfUpdateResult::Success { config_path } => println!("Applied: {}", config_path),
//!     SelfUpdateResult::PendingApproval { preview } => println!("Needs approval"),
//!     SelfUpdateResult::Failed { error } => println!("Failed: {}", error),
//! }
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::capability::CapabilityGap;

/// Self-update configuration
#[derive(Debug, Clone)]
pub struct SelfUpdateConfig {
    /// Enable self-update
    pub enabled: bool,
    /// Update mode
    pub mode: UpdateMode,
    /// Maximum auto-updates per session
    pub max_updates_per_session: usize,
    /// Configuration directory
    pub config_dir: PathBuf,
    /// Backup directory for rollbacks
    pub backup_dir: PathBuf,
    /// Validation timeout in seconds
    pub validation_timeout: u64,
    /// Trusted sources for configs
    pub trusted_sources: Vec<String>,
}

impl Default for SelfUpdateConfig {
    fn default() -> Self {
        let base_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx");

        Self {
            enabled: true,
            mode: UpdateMode::Supervised,
            max_updates_per_session: 10,
            config_dir: base_dir.join("config"),
            backup_dir: base_dir.join("backups"),
            validation_timeout: 30,
            trusted_sources: vec!["dx-forge".to_string(), "official".to_string()],
        }
    }
}

/// Update modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateMode {
    /// Requires human approval
    Supervised,
    /// Automatically applies validated changes
    Autonomous,
    /// Generates but doesn't apply
    DryRun,
}

impl Default for UpdateMode {
    fn default() -> Self {
        Self::Supervised
    }
}

/// Result of a self-update operation
#[derive(Debug)]
pub enum SelfUpdateResult {
    /// Successfully applied
    Success {
        /// Path to the applied configuration
        config_path: PathBuf,
        /// Rollback state
        rollback: RollbackState,
    },
    /// Pending human approval
    PendingApproval {
        /// Preview of the changes
        preview: ConfigPreview,
    },
    /// Update failed
    Failed {
        /// Error message
        error: String,
        /// Attempted rollback
        rollback_attempted: bool,
    },
    /// Skipped (e.g., already exists)
    Skipped {
        /// Reason for skipping
        reason: String,
    },
}

/// Preview of configuration changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPreview {
    /// Capability being added
    pub capability: String,
    /// Generated configuration
    pub config: String,
    /// Configuration format
    pub format: ConfigFormat,
    /// Target path
    pub target_path: PathBuf,
    /// Explanation of changes
    pub explanation: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

/// Configuration format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigFormat {
    /// DX Serializer format
    Sr,
    /// TOML format
    Toml,
    /// JSON format
    Json,
}

impl ConfigFormat {
    /// Get file extension
    pub fn extension(&self) -> &str {
        match self {
            ConfigFormat::Sr => "sr",
            ConfigFormat::Toml => "toml",
            ConfigFormat::Json => "json",
        }
    }
}

/// State for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackState {
    /// Unique identifier
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Files backed up
    pub backed_up_files: Vec<BackedUpFile>,
    /// New files created
    pub created_files: Vec<PathBuf>,
    /// Can this be rolled back?
    pub can_rollback: bool,
}

/// A backed up file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackedUpFile {
    /// Original path
    pub original_path: PathBuf,
    /// Backup path
    pub backup_path: PathBuf,
}

/// Pending update awaiting approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdate {
    /// Unique identifier
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Preview
    pub preview: ConfigPreview,
    /// Expiry (auto-reject after this)
    pub expires_at: DateTime<Utc>,
}

/// Self-update engine
pub struct SelfUpdateEngine {
    /// Configuration
    config: SelfUpdateConfig,
    /// Updates applied this session
    session_updates: usize,
    /// Rollback states
    rollback_states: HashMap<String, RollbackState>,
    /// Pending updates
    pending_updates: HashMap<String, PendingUpdate>,
}

impl SelfUpdateEngine {
    /// Create a new self-update engine
    pub fn new(config: SelfUpdateConfig) -> Result<Self> {
        // Ensure directories exist
        std::fs::create_dir_all(&config.config_dir)?;
        std::fs::create_dir_all(&config.backup_dir)?;

        Ok(Self {
            config,
            session_updates: 0,
            rollback_states: HashMap::new(),
            pending_updates: HashMap::new(),
        })
    }

    /// Acquire a capability (generate and apply configuration)
    pub async fn acquire_capability(&mut self, gap: &CapabilityGap) -> Result<SelfUpdateResult> {
        // Check if enabled
        if !self.config.enabled {
            return Ok(SelfUpdateResult::Skipped {
                reason: "Self-update is disabled".to_string(),
            });
        }

        // Check session limit
        if self.session_updates >= self.config.max_updates_per_session {
            return Ok(SelfUpdateResult::Skipped {
                reason: format!(
                    "Session limit reached ({}/{})",
                    self.session_updates, self.config.max_updates_per_session
                ),
            });
        }

        // Check if config already exists
        let config_path = self.config.config_dir.join(format!("{}.sr", gap.capability));
        if config_path.exists() {
            return Ok(SelfUpdateResult::Skipped {
                reason: format!("Configuration already exists: {}", config_path.display()),
            });
        }

        // Generate configuration
        let preview = self.generate_config(gap).await?;

        // Validate configuration
        if let Err(e) = self.validate_config(&preview) {
            return Ok(SelfUpdateResult::Failed {
                error: format!("Validation failed: {}", e),
                rollback_attempted: false,
            });
        }

        // Apply based on mode
        match self.config.mode {
            UpdateMode::Supervised => {
                // Store as pending
                let pending = PendingUpdate {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    preview: preview.clone(),
                    expires_at: Utc::now() + chrono::Duration::hours(24),
                };
                let id = pending.id.clone();
                self.pending_updates.insert(id, pending);

                Ok(SelfUpdateResult::PendingApproval { preview })
            }
            UpdateMode::Autonomous => {
                // Apply directly
                self.apply_config(&preview).await
            }
            UpdateMode::DryRun => {
                // Just return preview
                Ok(SelfUpdateResult::PendingApproval { preview })
            }
        }
    }

    /// Generate configuration for a capability
    async fn generate_config(&self, gap: &CapabilityGap) -> Result<ConfigPreview> {
        // Get skill requirement if available
        let config_content = if let Some(ref req) = gap.requirement {
            // Generate based on requirement
            self.generate_from_requirement(req)
        } else {
            // Generate generic config
            self.generate_generic_config(&gap.capability)
        };

        let target_path = self.config.config_dir.join(format!("{}.sr", gap.capability));

        Ok(ConfigPreview {
            capability: gap.capability.clone(),
            config: config_content,
            format: ConfigFormat::Sr,
            target_path,
            explanation: format!(
                "Generated configuration for '{}' capability. {}",
                gap.capability, gap.reason
            ),
            confidence: gap.confidence,
        })
    }

    /// Generate config from skill requirement
    fn generate_from_requirement(&self, req: &super::capability::SkillRequirement) -> String {
        let mut lines = vec![
            format!("# {} Configuration", req.name),
            format!("# Auto-generated by DX Agent"),
            String::new(),
            format!("{} {{", req.name),
        ];

        // Add required fields
        for field in &req.required_config {
            lines.push(format!("    {} \"\"  # Required", field));
        }

        // Add optional fields
        for field in &req.optional_config {
            lines.push(format!("    # {} \"\"  # Optional", field));
        }

        // Add example values if available
        if let Some(ref example) = req.example_config {
            lines.push(String::new());
            lines.push("    # Example configuration:".to_string());
            if let serde_json::Value::Object(map) = example {
                for (key, value) in map {
                    let value_str = match value {
                        serde_json::Value::String(s) => format!("\"{}\"", s),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => format!("{}", value),
                    };
                    lines.push(format!("    # {} {}", key, value_str));
                }
            }
        }

        lines.push("}".to_string());
        lines.join("\n")
    }

    /// Generate generic config
    fn generate_generic_config(&self, capability: &str) -> String {
        format!(
            r#"# {} Configuration
# Auto-generated by DX Agent

{} {{
    enabled true
    
    # Add your configuration here
}}
"#,
            capability, capability
        )
    }

    /// Validate configuration
    fn validate_config(&self, preview: &ConfigPreview) -> Result<()> {
        // Basic syntax validation
        // In a full implementation, this would use DX Forge
        if preview.config.is_empty() {
            return Err(anyhow::anyhow!("Empty configuration"));
        }

        // Check for balanced braces
        let open_braces = preview.config.matches('{').count();
        let close_braces = preview.config.matches('}').count();
        if open_braces != close_braces {
            return Err(anyhow::anyhow!("Unbalanced braces"));
        }

        Ok(())
    }

    /// Apply configuration
    async fn apply_config(&mut self, preview: &ConfigPreview) -> Result<SelfUpdateResult> {
        let rollback_id = uuid::Uuid::new_v4().to_string();
        let mut rollback = RollbackState {
            id: rollback_id.clone(),
            timestamp: Utc::now(),
            backed_up_files: Vec::new(),
            created_files: Vec::new(),
            can_rollback: true,
        };

        // Backup existing file if it exists
        if preview.target_path.exists() {
            let backup_path = self.config.backup_dir.join(format!(
                "{}_{}.bak",
                preview.capability,
                Utc::now().format("%Y%m%d_%H%M%S")
            ));

            std::fs::copy(&preview.target_path, &backup_path)
                .context("Failed to backup existing config")?;

            rollback.backed_up_files.push(BackedUpFile {
                original_path: preview.target_path.clone(),
                backup_path,
            });
        }

        // Write new config
        std::fs::write(&preview.target_path, &preview.config)
            .context("Failed to write configuration")?;

        rollback.created_files.push(preview.target_path.clone());

        // Store rollback state
        self.rollback_states.insert(rollback_id.clone(), rollback.clone());
        self.session_updates += 1;

        Ok(SelfUpdateResult::Success {
            config_path: preview.target_path.clone(),
            rollback,
        })
    }

    /// Approve a pending update
    pub async fn approve_update(&mut self, update_id: &str) -> Result<SelfUpdateResult> {
        let pending = self.pending_updates.remove(update_id).context("Pending update not found")?;

        // Check expiry
        if Utc::now() > pending.expires_at {
            return Ok(SelfUpdateResult::Failed {
                error: "Update expired".to_string(),
                rollback_attempted: false,
            });
        }

        // Apply the update
        self.apply_config(&pending.preview).await
    }

    /// Reject a pending update
    pub fn reject_update(&mut self, update_id: &str) -> Result<()> {
        self.pending_updates.remove(update_id).context("Pending update not found")?;
        Ok(())
    }

    /// Rollback an applied update
    pub fn rollback(&mut self, rollback_id: &str) -> Result<()> {
        let state = self.rollback_states.remove(rollback_id).context("Rollback state not found")?;

        if !state.can_rollback {
            return Err(anyhow::anyhow!("Rollback not available for this update"));
        }

        // Remove created files
        for path in &state.created_files {
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }

        // Restore backed up files
        for backup in &state.backed_up_files {
            if backup.backup_path.exists() {
                std::fs::copy(&backup.backup_path, &backup.original_path)?;
            }
        }

        Ok(())
    }

    /// List pending updates
    pub fn list_pending(&self) -> Vec<&PendingUpdate> {
        self.pending_updates.values().collect()
    }

    /// List rollback states
    pub fn list_rollbacks(&self) -> Vec<&RollbackState> {
        self.rollback_states.values().collect()
    }

    /// Clean up expired pending updates
    pub fn cleanup_expired(&mut self) -> usize {
        let now = Utc::now();
        let expired: Vec<_> = self
            .pending_updates
            .iter()
            .filter(|(_, p)| now > p.expires_at)
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired.len();
        for id in expired {
            self.pending_updates.remove(&id);
        }
        count
    }

    /// Get configuration for self-update
    pub fn config(&self) -> &SelfUpdateConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_mode(&mut self, mode: UpdateMode) {
        self.config.mode = mode;
    }

    /// Enable/disable self-update
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_default() {
        let config = SelfUpdateConfig::default();
        assert!(config.enabled);
        assert_eq!(config.mode, UpdateMode::Supervised);
    }

    #[test]
    fn test_engine_creation() {
        let dir = tempdir().unwrap();
        let config = SelfUpdateConfig {
            config_dir: dir.path().join("config"),
            backup_dir: dir.path().join("backups"),
            ..Default::default()
        };
        let engine = SelfUpdateEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_generate_generic_config() {
        let dir = tempdir().unwrap();
        let config = SelfUpdateConfig {
            config_dir: dir.path().join("config"),
            backup_dir: dir.path().join("backups"),
            ..Default::default()
        };
        let engine = SelfUpdateEngine::new(config).unwrap();

        let generated = engine.generate_generic_config("weather");
        assert!(generated.contains("weather"));
        assert!(generated.contains("enabled"));
    }

    #[test]
    fn test_validate_config() {
        let dir = tempdir().unwrap();
        let config = SelfUpdateConfig {
            config_dir: dir.path().join("config"),
            backup_dir: dir.path().join("backups"),
            ..Default::default()
        };
        let engine = SelfUpdateEngine::new(config).unwrap();

        let preview = ConfigPreview {
            capability: "test".to_string(),
            config: "test { enabled true }".to_string(),
            format: ConfigFormat::Sr,
            target_path: dir.path().join("test.sr"),
            explanation: "Test".to_string(),
            confidence: 0.8,
        };

        assert!(engine.validate_config(&preview).is_ok());

        let bad_preview = ConfigPreview {
            config: "test { ".to_string(),
            ..preview
        };
        assert!(engine.validate_config(&bad_preview).is_err());
    }
}
