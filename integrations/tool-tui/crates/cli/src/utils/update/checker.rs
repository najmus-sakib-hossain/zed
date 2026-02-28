//! Update checker implementation

use super::types::{GitHubAsset, GitHubRelease, UpdateInfo};
use super::{CURRENT_VERSION, RELEASES_API_URL};
use crate::utils::error::DxError;

/// Update checker for the DX CLI
pub struct UpdateChecker {
    /// GitHub releases API URL
    api_url: String,
    /// Current version
    current_version: String,
}

impl UpdateChecker {
    /// Create a new update checker
    pub fn new() -> Self {
        Self {
            api_url: RELEASES_API_URL.to_string(),
            current_version: CURRENT_VERSION.to_string(),
        }
    }

    /// Create an update checker with custom settings (for testing)
    #[cfg(test)]
    pub fn with_config(api_url: String, current_version: String) -> Self {
        Self {
            api_url,
            current_version,
        }
    }

    /// Get the current version
    pub fn current_version(&self) -> &str {
        &self.current_version
    }

    /// Check for available updates
    pub async fn check(&self) -> Result<Option<UpdateInfo>, DxError> {
        let release = self.fetch_latest_release().await?;

        if release.prerelease || release.draft {
            return Ok(None);
        }

        let new_version = release.tag_name.trim_start_matches('v').to_string();

        if !is_newer_version(&new_version, &self.current_version) {
            return Ok(None);
        }

        let platform = get_platform_identifier();
        let (download_url, full_size) = self.find_binary_asset(&release.assets, &platform)?;
        let (delta_url, delta_size) = self.find_delta_asset(&release.assets, &platform);
        let signature = self.find_signature(&release.assets, &platform)?;

        let release_notes =
            release.body.as_deref().map(summarize_release_notes).unwrap_or_default();

        Ok(Some(UpdateInfo {
            current_version: self.current_version.clone(),
            new_version,
            release_notes,
            download_url,
            delta_url,
            full_size,
            delta_size,
            signature,
        }))
    }

    async fn fetch_latest_release(&self) -> Result<GitHubRelease, DxError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(format!("dx-cli/{}", super::CURRENT_VERSION))
            .build()
            .map_err(|e| DxError::Network {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        let response = client.get(&self.api_url).send().await.map_err(|e| DxError::Network {
            message: format!("Failed to fetch release info: {}", e),
        })?;

        if !response.status().is_success() {
            return Err(DxError::Network {
                message: format!("GitHub API returned status: {}", response.status()),
            });
        }

        response.json().await.map_err(|e| DxError::Network {
            message: format!("Failed to parse release info: {}", e),
        })
    }

    fn find_binary_asset(
        &self,
        assets: &[GitHubAsset],
        platform: &str,
    ) -> Result<(String, u64), DxError> {
        let patterns = [
            format!("dx-{}.exe", platform),
            format!("dx-{}.tar.gz", platform),
            format!("dx-{}.zip", platform),
            format!("dx-{}", platform),
        ];

        for pattern in &patterns {
            if let Some(asset) = assets.iter().find(|a| a.name == *pattern) {
                return Ok((asset.browser_download_url.clone(), asset.size));
            }
        }

        Err(DxError::UpdateDownloadFailed {
            message: format!("No binary found for platform: {}", platform),
        })
    }

    fn find_delta_asset(
        &self,
        assets: &[GitHubAsset],
        platform: &str,
    ) -> (Option<String>, Option<u64>) {
        let patterns = [
            format!("dx-{}.patch", platform),
            format!("dx-{}.delta", platform),
            format!("dx-{}-{}.patch", platform, self.current_version),
        ];

        for pattern in &patterns {
            if let Some(asset) = assets.iter().find(|a| a.name == *pattern) {
                return (Some(asset.browser_download_url.clone()), Some(asset.size));
            }
        }

        (None, None)
    }

    fn find_signature(&self, assets: &[GitHubAsset], platform: &str) -> Result<String, DxError> {
        let patterns = [
            format!("dx-{}.sig", platform),
            format!("dx-{}.asc", platform),
        ];

        for pattern in &patterns {
            if let Some(asset) = assets.iter().find(|a| a.name == *pattern) {
                return Ok(asset.browser_download_url.clone());
            }
        }

        Err(DxError::UpdateDownloadFailed {
            message: format!("No signature found for platform: {}", platform),
        })
    }
}

impl Default for UpdateChecker {
    fn default() -> Self {
        Self::new()
    }
}

fn get_platform_identifier() -> String {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "unknown"
    };

    format!("{}-{}", os, arch)
}

fn is_newer_version(new: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };

    let new_v = parse_version(new);
    let current_v = parse_version(current);

    new_v > current_v
}

fn summarize_release_notes(notes: &str) -> String {
    let first_para = notes.split("\n\n").next().unwrap_or(notes).trim();

    if first_para.len() > 200 {
        format!("{}...", &first_para[..197])
    } else {
        first_para.to_string()
    }
}
