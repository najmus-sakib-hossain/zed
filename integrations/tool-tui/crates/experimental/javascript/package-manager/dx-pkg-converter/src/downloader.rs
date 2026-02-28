//! npm Registry Downloader

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Clone)]
pub struct NpmDownloader {
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct NpmPackageMetadata {
    versions: std::collections::HashMap<String, NpmVersionInfo>,
    #[serde(rename = "dist-tags")]
    dist_tags: std::collections::HashMap<String, String>,
}

#[derive(Deserialize)]
struct NpmVersionInfo {
    dist: NpmDist,
}

#[derive(Deserialize)]
struct NpmDist {
    tarball: String,
}

impl Default for NpmDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl NpmDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Download package tarball from npm
    pub async fn download(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        // Get package metadata
        let metadata_url = format!("https://registry.npmjs.org/{}", name);
        let metadata: NpmPackageMetadata = self
            .client
            .get(&metadata_url)
            .send()
            .await?
            .json()
            .await
            .context("Failed to fetch package metadata")?;

        // Resolve version
        let resolved_version = if version == "latest" {
            metadata.dist_tags.get("latest").context("No latest version found")?
        } else {
            version
        };

        // Get tarball URL
        let tarball_url = &metadata
            .versions
            .get(resolved_version)
            .context(format!("Version {} not found", resolved_version))?
            .dist
            .tarball;

        println!("   Version: {}", resolved_version);
        println!("   Downloading: {}", tarball_url);

        // Download tarball
        let bytes = self.client.get(tarball_url).send().await?.bytes().await?;

        Ok(bytes.to_vec())
    }
}
