//! Icon processing for DX-WWW build pipeline
//!
//! This module handles icon extraction from components and resolution via dx-icon CLI.
//! It scans component files for `<dx-icon>` usage, resolves icons at compile time,
//! and inlines optimized SVG data for zero runtime overhead.

use crate::error::{BuildError, Result};
use crate::hash::content_hash;
use crate::{BuildArtifact, BuildCache, CacheEntry, CacheKey};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Icon processor for extracting and resolving icons at build time
pub struct IconProcessor {
    /// Configuration for icon processing
    config: IconConfig,
}

/// Configuration for icon processing
#[derive(Debug, Clone)]
pub struct IconConfig {
    /// Directory containing component files to scan
    pub components_dir: PathBuf,
    /// Output directory for generated icon data
    pub output_dir: PathBuf,
    /// File extensions to scan for icon usage
    pub file_extensions: Vec<String>,
    /// Whether to enable tree-shaking (only include used icons)
    pub tree_shaking: bool,
}

impl Default for IconConfig {
    fn default() -> Self {
        Self {
            components_dir: PathBuf::from("www/components"),
            output_dir: PathBuf::from("dist/icons"),
            file_extensions: vec!["rs".to_string(), "pg".to_string(), "html".to_string()],
            tree_shaking: true,
        }
    }
}

/// Resolved icon with SVG data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedIcon {
    /// Icon name in "set:name" format
    pub name: String,
    /// Icon set (e.g., "heroicons", "mdi")
    pub set: String,
    /// Icon identifier within the set
    pub icon_id: String,
    /// Optimized SVG markup
    pub svg: String,
    /// Size in bytes
    pub size: usize,
    /// Content hash
    pub hash: String,
}

/// Manifest of all resolved icons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconManifest {
    /// All resolved icons
    pub icons: Vec<ResolvedIcon>,
    /// Total number of icons
    pub total_count: usize,
    /// Total size in bytes
    pub total_size: usize,
    /// Icons grouped by set
    pub by_set: HashMap<String, Vec<String>>,
}

impl IconProcessor {
    /// Create a new icon processor
    pub fn new(config: IconConfig) -> Self {
        Self { config }
    }

    /// Process icons from component files
    ///
    /// This method:
    /// 1. Scans component files for `<dx-icon>` usage
    /// 2. Extracts unique icon names
    /// 3. Resolves icons via dx-icon library
    /// 4. Generates optimized SVG data
    /// 5. Creates build artifacts with caching
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Component directory doesn't exist
    /// - Icon resolution fails
    /// - Output files cannot be written
    pub fn process_icons(
        &self,
        cache: &mut BuildCache,
    ) -> Result<(IconManifest, Vec<BuildArtifact>)> {
        // Check if components directory exists
        if !self.config.components_dir.exists() {
            // If directory doesn't exist, return empty results (no icons to process)
            return Ok((
                IconManifest {
                    icons: Vec::new(),
                    total_count: 0,
                    total_size: 0,
                    by_set: HashMap::new(),
                },
                Vec::new(),
            ));
        }

        // Extract icon names from component files
        let icon_names = self.extract_icon_names()?;

        if icon_names.is_empty() {
            // No icons found, return empty results
            return Ok((
                IconManifest {
                    icons: Vec::new(),
                    total_count: 0,
                    total_size: 0,
                    by_set: HashMap::new(),
                },
                Vec::new(),
            ));
        }

        // Create cache key from icon names
        let cache_key = self.create_cache_key(&icon_names)?;

        // Check cache for existing icons
        if let Some(cached) = cache.get(&cache_key)
            && self.verify_cached_outputs(&cached.output_path)
        {
            return self.load_cached_icons(&cached.output_path);
        }

        // Resolve icons via dx-icon library
        let resolved_icons = self.resolve_icons(&icon_names)?;

        // Generate manifest
        let manifest = self.create_manifest(&resolved_icons);

        // Write manifest and create artifacts
        let artifacts = self.write_manifest_and_artifacts(&manifest, cache, cache_key)?;

        Ok((manifest, artifacts))
    }

    /// Extract icon names from component files
    fn extract_icon_names(&self) -> Result<Vec<String>> {
        let mut icon_names = HashSet::new();

        // Walk through component directory
        for entry in WalkDir::new(&self.config.components_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Check if file has a valid extension
            if !path.is_file() {
                continue;
            }

            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if !self.config.file_extensions.iter().any(|e| e == &ext_str) {
                    continue;
                }
            } else {
                continue;
            }

            // Read file content
            let content = std::fs::read_to_string(path).map_err(|e| BuildError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;

            // Extract icon names from content
            let names = self.parse_icon_names(&content);
            icon_names.extend(names);
        }

        Ok(icon_names.into_iter().collect())
    }

    /// Parse icon names from source code
    fn parse_icon_names(&self, source: &str) -> Vec<String> {
        use once_cell::sync::Lazy;
        use regex::Regex;

        static ICON_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"<dx-icon\s+(?:[^>]*\s+)?name="([^"]+)"[^>]*/?>"#)
                .expect("Invalid regex pattern")
        });

        let mut names = Vec::new();
        for cap in ICON_REGEX.captures_iter(source) {
            if let Some(name) = cap.get(1) {
                names.push(name.as_str().to_string());
            }
        }

        names
    }

    /// Resolve icons via dx-icon library
    fn resolve_icons(&self, icon_names: &[String]) -> Result<Vec<ResolvedIcon>> {
        let mut resolved = Vec::new();
        let mut reader = dx_icon::icons();

        for name in icon_names {
            // Parse icon name into set and id
            let (set, icon_id) = self.parse_icon_name(name);

            // Get icon from dx-icon library
            if let Some(icon) = reader.get(set, icon_id) {
                // Generate optimized SVG
                let svg = icon.to_svg(24);
                let size = svg.len();
                let hash = content_hash(svg.as_bytes());

                resolved.push(ResolvedIcon {
                    name: name.clone(),
                    set: set.to_string(),
                    icon_id: icon_id.to_string(),
                    svg,
                    size,
                    hash,
                });
            } else {
                // Icon not found - log warning but continue
                eprintln!("Warning: Icon not found: {} (set: {}, id: {})", name, set, icon_id);
            }
        }

        Ok(resolved)
    }

    /// Parse icon name into set and id
    fn parse_icon_name<'a>(&self, name: &'a str) -> (&'a str, &'a str) {
        if let Some(colon_pos) = name.find(':') {
            let (set, rest) = name.split_at(colon_pos);
            (set, &rest[1..])
        } else {
            // Default to lucide if no set specified
            ("lucide", name)
        }
    }

    /// Create manifest from resolved icons
    fn create_manifest(&self, resolved_icons: &[ResolvedIcon]) -> IconManifest {
        let mut by_set: HashMap<String, Vec<String>> = HashMap::new();
        let mut total_size = 0;

        for icon in resolved_icons {
            by_set.entry(icon.set.clone()).or_default().push(icon.icon_id.clone());
            total_size += icon.size;
        }

        IconManifest {
            icons: resolved_icons.to_vec(),
            total_count: resolved_icons.len(),
            total_size,
            by_set,
        }
    }

    /// Write manifest and create artifacts
    fn write_manifest_and_artifacts(
        &self,
        manifest: &IconManifest,
        cache: &mut BuildCache,
        cache_key: CacheKey,
    ) -> Result<Vec<BuildArtifact>> {
        // Create output directory
        std::fs::create_dir_all(&self.config.output_dir).map_err(|e| BuildError::Io {
            path: self.config.output_dir.clone(),
            source: e,
        })?;

        // Write manifest to file
        let manifest_path = self.config.output_dir.join("icons.json");
        let manifest_data = serde_json::to_vec_pretty(manifest)
            .map_err(|e| BuildError::Icon(format!("Failed to serialize manifest: {}", e)))?;

        std::fs::write(&manifest_path, &manifest_data).map_err(|e| BuildError::Io {
            path: manifest_path.clone(),
            source: e,
        })?;

        let manifest_hash = content_hash(&manifest_data);

        // Cache the manifest
        let cache_entry = CacheEntry::new(
            cache_key,
            manifest_path.clone(),
            manifest_hash.clone(),
            manifest_data.len(),
        );
        cache.insert(cache_entry)?;

        // Create artifacts
        let mut artifacts = vec![BuildArtifact {
            artifact_type: crate::ArtifactType::Icon,
            path: manifest_path,
            hash: manifest_hash,
            size: manifest_data.len(),
        }];

        // Write individual icon files (optional, for debugging)
        for icon in &manifest.icons {
            let icon_path =
                self.config.output_dir.join(format!("{}_{}.svg", icon.set, icon.icon_id));

            std::fs::write(&icon_path, &icon.svg).map_err(|e| BuildError::Io {
                path: icon_path.clone(),
                source: e,
            })?;

            artifacts.push(BuildArtifact {
                artifact_type: crate::ArtifactType::Icon,
                path: icon_path,
                hash: icon.hash.clone(),
                size: icon.size,
            });
        }

        Ok(artifacts)
    }

    /// Create cache key from icon names
    fn create_cache_key(&self, icon_names: &[String]) -> Result<CacheKey> {
        // Sort names for consistent hashing
        let mut sorted_names = icon_names.to_vec();
        sorted_names.sort();

        // Create hash from sorted names
        let names_str = sorted_names.join(",");
        let hash = content_hash(names_str.as_bytes());

        // Use a virtual path for the cache key
        let virtual_path = PathBuf::from("icons.manifest");

        Ok(CacheKey {
            source_path: virtual_path,
            content_hash: hash,
            processor: "icon".to_string(),
        })
    }

    /// Verify that cached outputs still exist
    fn verify_cached_outputs(&self, manifest_path: &Path) -> bool {
        manifest_path.exists()
    }

    /// Load cached icons from manifest
    fn load_cached_icons(
        &self,
        manifest_path: &Path,
    ) -> Result<(IconManifest, Vec<BuildArtifact>)> {
        let data = std::fs::read(manifest_path).map_err(|e| BuildError::Io {
            path: manifest_path.to_path_buf(),
            source: e,
        })?;

        let manifest: IconManifest = serde_json::from_slice(&data)
            .map_err(|e| BuildError::Icon(format!("Failed to parse manifest: {}", e)))?;

        let artifacts = self.create_artifacts_from_manifest(&manifest)?;

        Ok((manifest, artifacts))
    }

    /// Create artifacts from manifest
    fn create_artifacts_from_manifest(
        &self,
        manifest: &IconManifest,
    ) -> Result<Vec<BuildArtifact>> {
        let mut artifacts = Vec::new();

        // Add manifest artifact
        let manifest_path = self.config.output_dir.join("icons.json");
        if manifest_path.exists() {
            let data = std::fs::read(&manifest_path).map_err(|e| BuildError::Io {
                path: manifest_path.clone(),
                source: e,
            })?;

            artifacts.push(BuildArtifact {
                artifact_type: crate::ArtifactType::Icon,
                path: manifest_path,
                hash: content_hash(&data),
                size: data.len(),
            });
        }

        // Add individual icon artifacts
        for icon in &manifest.icons {
            let icon_path =
                self.config.output_dir.join(format!("{}_{}.svg", icon.set, icon.icon_id));

            if icon_path.exists() {
                artifacts.push(BuildArtifact {
                    artifact_type: crate::ArtifactType::Icon,
                    path: icon_path,
                    hash: icon.hash.clone(),
                    size: icon.size,
                });
            }
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_icon_config_default() {
        let config = IconConfig::default();
        assert_eq!(config.components_dir, PathBuf::from("www/components"));
        assert!(config.tree_shaking);
        assert!(config.file_extensions.contains(&"rs".to_string()));
    }

    #[test]
    fn test_parse_icon_name() {
        let config = IconConfig::default();
        let processor = IconProcessor::new(config);

        let (set, id) = processor.parse_icon_name("heroicons:home");
        assert_eq!(set, "heroicons");
        assert_eq!(id, "home");

        let (set, id) = processor.parse_icon_name("mdi:star");
        assert_eq!(set, "mdi");
        assert_eq!(id, "star");

        let (set, id) = processor.parse_icon_name("home");
        assert_eq!(set, "lucide");
        assert_eq!(id, "home");
    }

    #[test]
    fn test_parse_icon_names_from_source() {
        let config = IconConfig::default();
        let processor = IconProcessor::new(config);

        let source = r#"
            <dx-icon name="heroicons:home" />
            <dx-icon name="mdi:star" size="32" />
            <dx-icon name="lucide:heart" color="red" />
        "#;

        let names = processor.parse_icon_names(source);
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"heroicons:home".to_string()));
        assert!(names.contains(&"mdi:star".to_string()));
        assert!(names.contains(&"lucide:heart".to_string()));
    }

    #[test]
    fn test_process_icons_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = IconConfig {
            components_dir: temp_dir.path().join("nonexistent"),
            output_dir: temp_dir.path().join("icons"),
            file_extensions: vec!["rs".to_string()],
            tree_shaking: true,
        };

        let processor = IconProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        let result = processor.process_icons(&mut cache);
        assert!(result.is_ok());

        let (manifest, artifacts) = result.unwrap();
        assert_eq!(manifest.total_count, 0);
        assert_eq!(artifacts.len(), 0);
    }

    #[test]
    fn test_process_icons_with_components() {
        let temp_dir = TempDir::new().unwrap();
        let components_dir = temp_dir.path().join("components");
        std::fs::create_dir_all(&components_dir).unwrap();

        // Create a component file with icon usage
        let component_file = components_dir.join("button.rs");
        std::fs::write(
            &component_file,
            r#"
            fn render() {
                <dx-icon name="heroicons:home" />
            }
            "#,
        )
        .unwrap();

        let config = IconConfig {
            components_dir,
            output_dir: temp_dir.path().join("icons"),
            file_extensions: vec!["rs".to_string()],
            tree_shaking: true,
        };

        let processor = IconProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        let result = processor.process_icons(&mut cache);
        assert!(result.is_ok());

        let (manifest, artifacts) = result.unwrap();
        assert!(manifest.total_count > 0);
        assert!(!artifacts.is_empty());
    }
}
