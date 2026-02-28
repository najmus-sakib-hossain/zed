//! DX Configuration persistence — saves and loads DX settings to disk.
//!
//! The DX config file lives at `~/.dx/dx_config.json` and stores:
//! - Detected hardware profile (cached so we don't re-detect every launch)
//! - User tier override (if they want to force a specific tier)
//! - Budget configuration (daily/monthly limits, per-provider limits)
//! - Provider API keys (encrypted references, not plaintext)
//! - User preferences (default mood, default profile, voice settings)
//!
//! The config is saved as JSON for easy human inspection and editing.

use crate::cost::BudgetConfig;
use crate::device_tier::{DeviceTier, HardwareProfile};
use crate::mood::Mood;
use crate::profile::AiProfile;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The main DX configuration struct — persisted to `~/.dx/dx_config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxConfig {
    /// Schema version for forward compatibility.
    pub version: u32,

    /// Cached hardware profile from last detection.
    pub hardware: Option<CachedHardwareProfile>,

    /// User override for the device tier (None = auto-detected).
    pub tier_override: Option<DeviceTier>,

    /// Budget configuration for cost control.
    pub budget: BudgetConfig,

    /// Provider API key references (provider_id → key_ref).
    /// Keys are stored as references — the actual secrets should be in the OS keychain
    /// or environment variables. This field stores the *name* of the env var or keychain entry.
    pub provider_keys: HashMap<String, ProviderKeyRef>,

    /// User preferences.
    pub preferences: UserPreferences,

    /// Model download state — which models have been downloaded for which tier.
    pub model_downloads: ModelDownloadState,
}

impl Default for DxConfig {
    fn default() -> Self {
        Self {
            version: 1,
            hardware: None,
            tier_override: None,
            budget: BudgetConfig::default(),
            provider_keys: HashMap::new(),
            preferences: UserPreferences::default(),
            model_downloads: ModelDownloadState::default(),
        }
    }
}

impl DxConfig {
    /// Load config from the default path (`~/.dx/dx_config.json`).
    /// If the file doesn't exist, returns a default config.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }

    /// Load config from a specific path.
    /// If the file doesn't exist, returns a default config.
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            log::info!("DX config not found at {}, using defaults", path.display());
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read DX config from {}", path.display()))?;

        let config: Self = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse DX config from {}", path.display()))?;

        // Handle version migrations here in the future.
        if config.version > 1 {
            log::warn!(
                "DX config version {} is newer than supported (1), some fields may be ignored",
                config.version
            );
        }

        log::info!("DX config loaded from {}", path.display());
        Ok(config)
    }

    /// Save config to the default path (`~/.dx/dx_config.json`).
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        self.save_to(&path)
    }

    /// Save config to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create DX config directory: {}", parent.display())
            })?;
        }

        let contents =
            serde_json::to_string_pretty(self).context("Failed to serialize DX config")?;

        std::fs::write(path, contents)
            .with_context(|| format!("Failed to write DX config to {}", path.display()))?;

        log::info!("DX config saved to {}", path.display());
        Ok(())
    }

    /// Get the default config file path: `~/.dx/dx_config.json`.
    pub fn config_path() -> Result<PathBuf> {
        let home = dx_home_dir()?;
        Ok(home.join("dx_config.json"))
    }

    /// Get the DX home directory: `~/.dx/`.
    pub fn home_dir() -> Result<PathBuf> {
        dx_home_dir()
    }

    /// Get the models directory: `~/.dx/models/`.
    pub fn models_dir() -> Result<PathBuf> {
        let home = dx_home_dir()?;
        Ok(home.join("models"))
    }

    /// Get the cache directory: `~/.dx/cache/`.
    pub fn cache_dir() -> Result<PathBuf> {
        let home = dx_home_dir()?;
        Ok(home.join("cache"))
    }

    /// Update the cached hardware profile and save.
    pub fn update_hardware_profile(&mut self, profile: HardwareProfile) -> Result<()> {
        self.hardware = Some(CachedHardwareProfile {
            profile,
            detected_at: SystemTime::now(),
        });
        self.save()
    }

    /// Get the effective device tier, respecting user override.
    pub fn effective_tier(&self) -> Option<DeviceTier> {
        if let Some(override_tier) = self.tier_override {
            return Some(override_tier);
        }
        self.hardware.as_ref().map(|h| h.profile.effective_tier())
    }

    /// Whether the hardware profile needs re-detection.
    /// Returns true if no profile exists or if it's older than the given max age.
    pub fn needs_hardware_rescan(&self, max_age: std::time::Duration) -> bool {
        match &self.hardware {
            None => true,
            Some(cached) => {
                let age = SystemTime::now()
                    .duration_since(cached.detected_at)
                    .unwrap_or(std::time::Duration::MAX);
                age > max_age
            }
        }
    }

    /// Set or clear the tier override.
    pub fn set_tier_override(&mut self, tier: Option<DeviceTier>) -> Result<()> {
        self.tier_override = tier;
        self.save()
    }

    /// Register a provider API key reference.
    pub fn set_provider_key(&mut self, provider_id: &str, key_ref: ProviderKeyRef) -> Result<()> {
        self.provider_keys.insert(provider_id.to_string(), key_ref);
        self.save()
    }

    /// Remove a provider API key reference.
    pub fn remove_provider_key(&mut self, provider_id: &str) -> Result<()> {
        self.provider_keys.remove(provider_id);
        self.save()
    }

    /// Get the API key for a provider by resolving the key reference.
    pub fn resolve_provider_key(&self, provider_id: &str) -> Option<String> {
        let key_ref = self.provider_keys.get(provider_id)?;
        key_ref.resolve()
    }
}

/// A cached hardware profile with a timestamp of when it was detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedHardwareProfile {
    pub profile: HardwareProfile,
    pub detected_at: SystemTime,
}

/// A reference to a provider API key — not the key itself.
///
/// Keys can be stored in environment variables or (in the future) the OS keychain.
/// We never store raw API keys in the config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderKeyRef {
    /// Key is in an environment variable.
    #[serde(rename = "env")]
    EnvVar {
        /// Name of the environment variable (e.g., "OPENAI_API_KEY").
        var_name: String,
    },

    /// Key is stored in the OS keychain / credential manager.
    #[serde(rename = "keychain")]
    Keychain {
        /// Service name in the keychain.
        service: String,
        /// Account name in the keychain.
        account: String,
    },

    /// Key is provided inline (NOT recommended — only for development/testing).
    /// This will trigger a warning in the UI.
    #[serde(rename = "inline")]
    Inline {
        /// The raw API key (⚠️ stored in plaintext).
        key: String,
    },
}

impl ProviderKeyRef {
    /// Create an env var reference.
    pub fn from_env(var_name: impl Into<String>) -> Self {
        Self::EnvVar {
            var_name: var_name.into(),
        }
    }

    /// Create a keychain reference.
    pub fn from_keychain(service: impl Into<String>, account: impl Into<String>) -> Self {
        Self::Keychain {
            service: service.into(),
            account: account.into(),
        }
    }

    /// Create an inline key (⚠️ not recommended for production).
    pub fn from_inline(key: impl Into<String>) -> Self {
        Self::Inline { key: key.into() }
    }

    /// Resolve the key reference to the actual API key string.
    pub fn resolve(&self) -> Option<String> {
        match self {
            ProviderKeyRef::EnvVar { var_name } => std::env::var(var_name).ok(),
            ProviderKeyRef::Keychain { .. } => {
                // TODO: Integrate with OS keychain (keyring crate or platform-specific APIs).
                log::warn!("Keychain key resolution not yet implemented");
                None
            }
            ProviderKeyRef::Inline { key } => Some(key.clone()),
        }
    }

    /// Whether this key reference stores the key in plaintext (security warning).
    pub fn is_plaintext(&self) -> bool {
        matches!(self, ProviderKeyRef::Inline { .. })
    }
}

/// User preferences for DX behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Default mood when opening DX.
    pub default_mood: Mood,

    /// Default AI profile when opening DX.
    pub default_profile: AiProfile,

    /// Whether DX should speak responses aloud by default.
    pub voice_enabled: bool,

    /// Preferred TTS voice ID (provider-specific).
    pub preferred_voice_id: Option<String>,

    /// Preferred TTS provider ID.
    pub preferred_tts_provider: Option<String>,

    /// TTS speech speed multiplier (1.0 = normal).
    pub tts_speed: f32,

    /// Whether to auto-download models on first launch.
    pub auto_download_models: bool,

    /// Whether to show the Flow Bar on startup.
    pub show_flow_bar: bool,

    /// Whether to enable the background daemon.
    pub daemon_enabled: bool,

    /// Whether the system-wide grammar engine is active.
    pub grammar_enabled: bool,

    /// Whether system-wide edit prediction is active.
    pub system_wide_prediction: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            default_mood: Mood::Text,
            default_profile: AiProfile::Chat,
            voice_enabled: false,
            preferred_voice_id: None,
            preferred_tts_provider: None,
            tts_speed: 1.0,
            auto_download_models: true,
            show_flow_bar: true,
            daemon_enabled: false,
            grammar_enabled: true,
            system_wide_prediction: false,
        }
    }
}

/// Tracks which models have been downloaded for the progressive download strategy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelDownloadState {
    /// Model ID → download status.
    pub models: HashMap<String, ModelDownloadStatus>,
}

/// Download status of a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadStatus {
    /// Model identifier (e.g., "whisper-tiny-en", "piper-tiny-en").
    pub model_id: String,

    /// Whether the download is complete.
    pub complete: bool,

    /// Bytes downloaded so far.
    pub bytes_downloaded: u64,

    /// Total bytes expected.
    pub bytes_total: Option<u64>,

    /// Local file path where the model is stored.
    pub local_path: Option<PathBuf>,

    /// When the download completed (if complete).
    pub completed_at: Option<SystemTime>,

    /// SHA256 hash of the downloaded file (for integrity verification).
    pub sha256: Option<String>,
}

impl ModelDownloadState {
    /// Check if a model has been fully downloaded.
    pub fn is_downloaded(&self, model_id: &str) -> bool {
        self.models.get(model_id).is_some_and(|s| s.complete)
    }

    /// Get the local path of a downloaded model.
    pub fn model_path(&self, model_id: &str) -> Option<&Path> {
        self.models
            .get(model_id)
            .filter(|s| s.complete)
            .and_then(|s| s.local_path.as_deref())
    }

    /// Record that a model download has started.
    pub fn mark_started(&mut self, model_id: &str, bytes_total: Option<u64>) {
        self.models.insert(
            model_id.to_string(),
            ModelDownloadStatus {
                model_id: model_id.to_string(),
                complete: false,
                bytes_downloaded: 0,
                bytes_total,
                local_path: None,
                completed_at: None,
                sha256: None,
            },
        );
    }

    /// Update download progress.
    pub fn update_progress(&mut self, model_id: &str, bytes_downloaded: u64) {
        if let Some(status) = self.models.get_mut(model_id) {
            status.bytes_downloaded = bytes_downloaded;
        }
    }

    /// Record that a model download has completed.
    pub fn mark_complete(&mut self, model_id: &str, local_path: PathBuf, sha256: Option<String>) {
        if let Some(status) = self.models.get_mut(model_id) {
            status.complete = true;
            status.local_path = Some(local_path);
            status.completed_at = Some(SystemTime::now());
            status.sha256 = sha256;
            if let Some(total) = status.bytes_total {
                status.bytes_downloaded = total;
            }
        }
    }

    /// Remove a model download record (e.g., when cleaning up disk space).
    pub fn remove(&mut self, model_id: &str) {
        self.models.remove(model_id);
    }

    /// Total disk space used by downloaded models (bytes).
    pub fn total_downloaded_bytes(&self) -> u64 {
        self.models
            .values()
            .filter(|s| s.complete)
            .filter_map(|s| s.bytes_total)
            .sum()
    }

    /// List all fully downloaded model IDs.
    pub fn downloaded_models(&self) -> Vec<&str> {
        self.models
            .iter()
            .filter(|(_, s)| s.complete)
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// List all in-progress downloads.
    pub fn in_progress_downloads(&self) -> Vec<(&str, f64)> {
        self.models
            .iter()
            .filter(|(_, s)| !s.complete)
            .map(|(id, s)| {
                let progress = match s.bytes_total {
                    Some(total) if total > 0 => s.bytes_downloaded as f64 / total as f64,
                    _ => 0.0,
                };
                (id.as_str(), progress)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the DX home directory: `~/.dx/`.
///
/// Respects `DX_HOME` environment variable if set, otherwise uses `~/.dx/`.
fn dx_home_dir() -> Result<PathBuf> {
    // Check for override via environment variable.
    if let Ok(dx_home) = std::env::var("DX_HOME") {
        return Ok(PathBuf::from(dx_home));
    }

    let home = home_dir().context("Could not determine home directory")?;
    Ok(home.join(".dx"))
}

/// Cross-platform home directory detection.
fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_roundtrip() {
        let config = DxConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: DxConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, 1);
        assert!(parsed.hardware.is_none());
        assert!(parsed.tier_override.is_none());
    }

    #[test]
    fn test_provider_key_resolve_env() {
        std::env::set_var("DX_TEST_KEY_12345", "sk-test-value");
        let key_ref = ProviderKeyRef::from_env("DX_TEST_KEY_12345");
        assert_eq!(key_ref.resolve(), Some("sk-test-value".to_string()));
        assert!(!key_ref.is_plaintext());
        std::env::remove_var("DX_TEST_KEY_12345");
    }

    #[test]
    fn test_provider_key_resolve_inline() {
        let key_ref = ProviderKeyRef::from_inline("sk-inline-key");
        assert_eq!(key_ref.resolve(), Some("sk-inline-key".to_string()));
        assert!(key_ref.is_plaintext());
    }

    #[test]
    fn test_provider_key_resolve_missing_env() {
        let key_ref = ProviderKeyRef::from_env("DX_NONEXISTENT_VAR_99999");
        assert_eq!(key_ref.resolve(), None);
    }

    #[test]
    fn test_model_download_state() {
        let mut state = ModelDownloadState::default();

        assert!(!state.is_downloaded("whisper-tiny-en"));

        state.mark_started("whisper-tiny-en", Some(75_000_000));
        assert!(!state.is_downloaded("whisper-tiny-en"));

        state.update_progress("whisper-tiny-en", 50_000_000);
        let in_progress = state.in_progress_downloads();
        assert_eq!(in_progress.len(), 1);
        assert!((in_progress[0].1 - 0.6667).abs() < 0.01);

        state.mark_complete(
            "whisper-tiny-en",
            PathBuf::from("/home/user/.dx/models/whisper-tiny-en.bin"),
            Some("abc123".to_string()),
        );
        assert!(state.is_downloaded("whisper-tiny-en"));
        assert_eq!(state.downloaded_models(), vec!["whisper-tiny-en"]);
        assert_eq!(state.total_downloaded_bytes(), 75_000_000);
    }

    #[test]
    fn test_effective_tier_with_override() {
        let mut config = DxConfig::default();

        // No hardware, no override → None.
        assert!(config.effective_tier().is_none());

        // Set a tier override.
        config.tier_override = Some(DeviceTier::High);
        assert_eq!(config.effective_tier(), Some(DeviceTier::High));
    }

    #[test]
    fn test_needs_hardware_rescan() {
        let config = DxConfig::default();
        // No hardware profile → needs rescan.
        assert!(config.needs_hardware_rescan(std::time::Duration::from_secs(86400)));

        let mut config = DxConfig::default();
        config.hardware = Some(CachedHardwareProfile {
            profile: HardwareProfile::new(16.0, Some(8.0), 8),
            detected_at: SystemTime::now(),
        });
        // Just detected → does NOT need rescan within 24h.
        assert!(!config.needs_hardware_rescan(std::time::Duration::from_secs(86400)));
        // But does need rescan if max_age is 0.
        assert!(config.needs_hardware_rescan(std::time::Duration::ZERO));
    }

    #[test]
    fn test_config_save_load_roundtrip() {
        let dir = std::env::temp_dir().join("dx_test_config_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dx_config.json");

        let mut config = DxConfig::default();
        config.tier_override = Some(DeviceTier::Mid);
        config.preferences.voice_enabled = true;
        config.preferences.default_mood = Mood::Image;
        config.provider_keys.insert(
            "openai".to_string(),
            ProviderKeyRef::from_env("OPENAI_API_KEY"),
        );

        config.save_to(&path).unwrap();

        let loaded = DxConfig::load_from(&path).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.tier_override, Some(DeviceTier::Mid));
        assert!(loaded.preferences.voice_enabled);
        assert_eq!(loaded.preferences.default_mood, Mood::Image);
        assert!(loaded.provider_keys.contains_key("openai"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
