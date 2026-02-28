//! Update-related type definitions

use serde::{Deserialize, Serialize};

/// Represents an available update
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateInfo {
    /// Current installed version
    pub current_version: String,
    /// New available version
    pub new_version: String,
    /// Release notes summary
    pub release_notes: String,
    /// Download URL for the full binary
    pub download_url: String,
    /// Download URL for delta patch (if available)
    pub delta_url: Option<String>,
    /// Size of the full binary in bytes
    pub full_size: u64,
    /// Size of the delta patch in bytes (if available)
    pub delta_size: Option<u64>,
    /// Ed25519 signature for verification
    pub signature: String,
}

impl UpdateInfo {
    /// Check if a delta patch is available
    pub fn has_delta(&self) -> bool {
        self.delta_url.is_some()
    }

    /// Get the preferred download URL (delta if available, otherwise full)
    pub fn preferred_url(&self) -> &str {
        self.delta_url.as_deref().unwrap_or(&self.download_url)
    }

    /// Get the size of the preferred download
    pub fn preferred_size(&self) -> u64 {
        self.delta_size.unwrap_or(self.full_size)
    }

    /// Format the version display string
    pub fn version_display(&self) -> String {
        format!("{} → {}", self.current_version, self.new_version)
    }
}

/// GitHub release asset information
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    /// Asset name
    pub name: String,
    /// Download URL
    pub browser_download_url: String,
    /// Size in bytes
    pub size: u64,
}

/// GitHub release information
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    /// Release tag name (version)
    pub tag_name: String,
    /// Release name
    pub name: Option<String>,
    /// Release body (notes)
    pub body: Option<String>,
    /// Release assets
    pub assets: Vec<GitHubAsset>,
    /// Whether this is a prerelease
    pub prerelease: bool,
    /// Whether this is a draft
    pub draft: bool,
}
