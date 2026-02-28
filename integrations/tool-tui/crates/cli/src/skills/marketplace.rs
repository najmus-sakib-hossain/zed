//! Skills Marketplace
//!
//! Discover, install, and manage skills from the DX marketplace.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Marketplace skill metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSkill {
    /// Skill ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Author
    pub author: String,
    /// Version
    pub version: String,
    /// Category
    pub category: SkillCategory,
    /// Tags
    pub tags: Vec<String>,
    /// Download count
    pub downloads: u64,
    /// Rating (0-5)
    pub rating: f32,
    /// Repository URL
    pub repository: Option<String>,
    /// License
    pub license: String,
    /// Installation size (bytes)
    pub size: u64,
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Skill categories
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillCategory {
    Automation,
    Communication,
    Development,
    Productivity,
    Entertainment,
    Utilities,
    Integration,
    AI,
}

impl SkillCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Automation => "automation",
            Self::Communication => "communication",
            Self::Development => "development",
            Self::Productivity => "productivity",
            Self::Entertainment => "entertainment",
            Self::Utilities => "utilities",
            Self::Integration => "integration",
            Self::AI => "ai",
        }
    }
}

/// Marketplace client
pub struct Marketplace {
    /// API endpoint
    api_url: String,
    /// Local cache directory
    cache_dir: PathBuf,
}

impl Marketplace {
    /// Create new marketplace client
    pub fn new(api_url: String, cache_dir: PathBuf) -> Self {
        Self { api_url, cache_dir }
    }

    /// Search for skills
    pub async fn search(&self, query: &str) -> Result<Vec<MarketplaceSkill>> {
        let url = format!("{}/skills/search?q={}", self.api_url, query);
        let response = reqwest::get(&url).await.context("Failed to search marketplace")?;

        let skills: Vec<MarketplaceSkill> =
            response.json().await.context("Failed to parse search results")?;

        Ok(skills)
    }

    /// List skills by category
    pub async fn list_by_category(&self, category: SkillCategory) -> Result<Vec<MarketplaceSkill>> {
        let url = format!("{}/skills/category/{}", self.api_url, category.as_str());
        let response = reqwest::get(&url).await.context("Failed to fetch category")?;

        let skills: Vec<MarketplaceSkill> =
            response.json().await.context("Failed to parse category results")?;

        Ok(skills)
    }

    /// Get skill details
    pub async fn get_skill(&self, id: &str) -> Result<MarketplaceSkill> {
        let url = format!("{}/skills/{}", self.api_url, id);
        let response = reqwest::get(&url).await.context("Failed to fetch skill")?;

        let skill: MarketplaceSkill =
            response.json().await.context("Failed to parse skill details")?;

        Ok(skill)
    }

    /// Install a skill
    pub async fn install(&self, id: &str) -> Result<PathBuf> {
        let skill = self.get_skill(id).await?;

        // Download skill package
        let url = format!("{}/skills/{}/download", self.api_url, id);
        let response = reqwest::get(&url).await.context("Failed to download skill")?;

        let bytes = response.bytes().await.context("Failed to read skill package")?;

        // Save to cache
        let skill_dir = self.cache_dir.join(&skill.id);
        std::fs::create_dir_all(&skill_dir)?;

        let package_path = skill_dir.join("skill.tar.gz");
        std::fs::write(&package_path, bytes)?;

        // Extract package
        self.extract_package(&package_path, &skill_dir)?;

        Ok(skill_dir)
    }

    /// Uninstall a skill
    pub async fn uninstall(&self, id: &str) -> Result<()> {
        let skill_dir = self.cache_dir.join(id);
        if skill_dir.exists() {
            std::fs::remove_dir_all(&skill_dir)?;
        }
        Ok(())
    }

    /// Update a skill
    pub async fn update(&self, id: &str) -> Result<PathBuf> {
        self.uninstall(id).await?;
        self.install(id).await
    }

    /// List installed skills
    pub fn list_installed(&self) -> Result<Vec<String>> {
        let mut installed = Vec::new();

        if self.cache_dir.exists() {
            for entry in std::fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        installed.push(name.to_string());
                    }
                }
            }
        }

        Ok(installed)
    }

    /// Extract skill package
    fn extract_package(&self, package: &PathBuf, dest: &PathBuf) -> Result<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let file = std::fs::File::open(package)?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);
        archive.unpack(dest)?;

        Ok(())
    }

    /// Get popular skills
    pub async fn get_popular(&self, limit: usize) -> Result<Vec<MarketplaceSkill>> {
        let url = format!("{}/skills/popular?limit={}", self.api_url, limit);
        let response = reqwest::get(&url).await.context("Failed to fetch popular skills")?;

        let skills: Vec<MarketplaceSkill> =
            response.json().await.context("Failed to parse popular skills")?;

        Ok(skills)
    }

    /// Get trending skills
    pub async fn get_trending(&self, limit: usize) -> Result<Vec<MarketplaceSkill>> {
        let url = format!("{}/skills/trending?limit={}", self.api_url, limit);
        let response = reqwest::get(&url).await.context("Failed to fetch trending skills")?;

        let skills: Vec<MarketplaceSkill> =
            response.json().await.context("Failed to parse trending skills")?;

        Ok(skills)
    }
}

impl Default for Marketplace {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx")
            .join("skills");

        Self::new("https://marketplace.dx.dev".to_string(), cache_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_category_str() {
        assert_eq!(SkillCategory::Automation.as_str(), "automation");
        assert_eq!(SkillCategory::AI.as_str(), "ai");
    }

    #[test]
    fn test_marketplace_creation() {
        let marketplace = Marketplace::default();
        assert!(marketplace.cache_dir.to_string_lossy().contains("dx"));
    }
}
