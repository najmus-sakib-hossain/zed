//! Package Storage Backend

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::protocol::PackageMetadata;

/// Package storage backend
pub struct PackageStorage {
    root: PathBuf,
    /// In-memory index: hash -> (name, version, path)
    index: HashMap<u64, (String, String, PathBuf)>,
}

impl PackageStorage {
    /// Create new storage backend
    pub fn new(root: PathBuf) -> Result<Self> {
        // Create storage directory
        fs::create_dir_all(&root)?;

        let mut storage = Self {
            root: root.clone(),
            index: HashMap::new(),
        };

        // Build index
        storage.scan_packages()?;

        println!("📚 Indexed {} packages", storage.index.len());

        Ok(storage)
    }

    /// Scan storage directory and build index
    fn scan_packages(&mut self) -> Result<()> {
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("dxp") {
                // Parse filename: <name>@<version>.dxp
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                    && let Some((name, version)) = stem.split_once('@')
                {
                    let hash = hash_package_name(name);
                    self.index.insert(hash, (name.to_string(), version.to_string(), path.clone()));
                }
            }
        }

        Ok(())
    }

    /// Get package metadata
    pub async fn get_metadata(&self, name_hash: u64, _version: u64) -> Result<PackageMetadata> {
        let (name, version, path) = self.index.get(&name_hash).context("Package not found")?;

        let size = fs::metadata(path)?.len();
        let file_data = fs::read(path)?;
        let hash_bytes = blake3::hash(&file_data);
        let hash = format!("{}", hash_bytes.to_hex());

        // Parse dependencies from DXP file
        let dependencies = self.parse_dependencies_from_dxp(path).unwrap_or_default();

        Ok(PackageMetadata {
            name: name.clone(),
            version: version.clone(),
            dependencies,
            size,
            hash,
        })
    }

    /// Parse dependencies from a DXP file
    fn parse_dependencies_from_dxp(&self, path: &Path) -> Result<Vec<(String, String)>> {
        let file_data = fs::read(path)?;

        // Check magic number
        if file_data.len() < 128 || &file_data[0..4] != b"DXP\0" {
            return Ok(Vec::new());
        }

        // Read header to get metadata offset and size
        let metadata_offset =
            u64::from_le_bytes(file_data[48..56].try_into().unwrap_or([0; 8])) as usize;
        let metadata_size =
            u32::from_le_bytes(file_data[40..44].try_into().unwrap_or([0; 4])) as usize;

        if metadata_offset == 0
            || metadata_size == 0
            || metadata_offset + metadata_size > file_data.len()
        {
            return Ok(Vec::new());
        }

        // Parse metadata JSON
        let metadata_bytes = &file_data[metadata_offset..metadata_offset + metadata_size];
        let metadata: serde_json::Value = serde_json::from_slice(metadata_bytes)?;

        // Extract dependencies
        let deps = metadata
            .get("dependencies")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|dep| {
                        let name = dep.get("name").and_then(|n| n.as_str())?;
                        let version = dep.get("version").and_then(|v| v.as_str())?;
                        Some((name.to_string(), version.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(deps)
    }

    /// Get package data
    pub async fn get_package(&self, name_hash: u64, _version: u64) -> Result<Vec<u8>> {
        let (_name, _version, path) = self.index.get(&name_hash).context("Package not found")?;

        let mut file = File::open(path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;

        Ok(data)
    }

    /// Get storage path
    pub fn path(&self) -> &Path {
        &self.root
    }
}

/// Hash package name using blake3
pub fn hash_package_name(name: &str) -> u64 {
    let hash = blake3::hash(name.as_bytes());
    u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap_or([0u8; 8]))
}
