//! DX-WWW Build Pipeline
//!
//! This crate provides the build pipeline orchestrator for DX-WWW applications.
//! It coordinates asset processing (media, styles, icons, fonts, i18n, serialization)
//! and implements a caching layer with content hashing to avoid redundant work.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

mod cache;
mod error;
mod hash;
mod icon;
mod media;
mod style;

pub use cache::{BuildCache, CacheEntry, CacheKey};
pub use error::{BuildError, Result};
pub use hash::content_hash;
pub use icon::{IconConfig, IconManifest, IconProcessor, ResolvedIcon};
pub use media::{FaviconManifest, FaviconSize, MediaConfig, MediaProcessor};
pub use style::{BinaryStyleBundle, StyleArtifactMetadata, StyleConfig, StyleProcessor};

/// Main build pipeline orchestrator
///
/// Coordinates all asset processing and implements caching to avoid redundant work.
pub struct BuildPipeline {
    /// Root directory of the project
    root_dir: PathBuf,
    /// Cache directory for build artifacts
    cache_dir: PathBuf,
    /// Build cache for tracking processed assets
    cache: BuildCache,
    /// Configuration for the build pipeline
    config: BuildConfig,
}

/// Configuration for the build pipeline
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Enable caching of build artifacts
    pub enable_cache: bool,
    /// Enable media processing (favicons, images)
    pub enable_media: bool,
    /// Enable style processing (Binary Dawn CSS)
    pub enable_styles: bool,
    /// Enable icon processing
    pub enable_icons: bool,
    /// Enable font processing
    pub enable_fonts: bool,
    /// Enable i18n processing
    pub enable_i18n: bool,
    /// Enable serialization processing
    pub enable_serialization: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            enable_media: true,
            enable_styles: true,
            enable_icons: true,
            enable_fonts: true,
            enable_i18n: true,
            enable_serialization: true,
        }
    }
}

/// Result of a build operation
#[derive(Debug)]
pub struct BuildResult {
    /// Artifacts produced by the build
    pub artifacts: Vec<BuildArtifact>,
    /// Statistics about the build
    pub stats: BuildStats,
}

/// A single build artifact
#[derive(Debug, Clone)]
pub struct BuildArtifact {
    /// Type of artifact
    pub artifact_type: ArtifactType,
    /// Path to the artifact file
    pub path: PathBuf,
    /// Content hash of the artifact
    pub hash: String,
    /// Size in bytes
    pub size: usize,
}

/// Type of build artifact
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactType {
    /// Media assets (favicons, images)
    Media,
    /// Style assets (Binary Dawn CSS)
    Style,
    /// Icon assets
    Icon,
    /// Font assets
    Font,
    /// i18n translation bundles
    I18n,
    /// Serialized data
    Serialization,
    /// WASM bundle
    Wasm,
}

/// Statistics about a build
#[derive(Debug, Default)]
pub struct BuildStats {
    /// Number of artifacts processed
    pub artifacts_processed: usize,
    /// Number of artifacts from cache
    pub artifacts_cached: usize,
    /// Total build time in milliseconds
    pub build_time_ms: u64,
    /// Breakdown by artifact type
    pub by_type: HashMap<ArtifactType, TypeStats>,
}

/// Statistics for a specific artifact type
#[derive(Debug, Default, Clone)]
pub struct TypeStats {
    /// Number of artifacts of this type
    pub count: usize,
    /// Total size in bytes
    pub total_size: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

impl BuildPipeline {
    /// Create a new build pipeline
    ///
    /// # Arguments
    ///
    /// * `root_dir` - Root directory of the project
    /// * `config` - Build configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created
    pub fn new(root_dir: impl Into<PathBuf>, config: BuildConfig) -> Result<Self> {
        let root_dir = root_dir.into();
        let cache_dir = root_dir.join(".dx").join("cache");

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir).map_err(|e| BuildError::Io {
            path: cache_dir.clone(),
            source: e,
        })?;

        let cache = BuildCache::new(&cache_dir)?;

        Ok(Self {
            root_dir,
            cache_dir,
            cache,
            config,
        })
    }

    /// Run the complete build pipeline
    ///
    /// # Errors
    ///
    /// Returns an error if any stage of the build fails
    pub fn build(&mut self) -> Result<BuildResult> {
        let start_time = SystemTime::now();
        let mut artifacts = Vec::new();
        let mut stats = BuildStats::default();

        // Process each asset type in order
        if self.config.enable_media {
            let media_artifacts = self.process_media()?;
            self.update_stats(&mut stats, ArtifactType::Media, &media_artifacts);
            artifacts.extend(media_artifacts);
        }

        if self.config.enable_styles {
            let style_artifacts = self.process_styles()?;
            self.update_stats(&mut stats, ArtifactType::Style, &style_artifacts);
            artifacts.extend(style_artifacts);
        }

        if self.config.enable_icons {
            let icon_artifacts = self.process_icons()?;
            self.update_stats(&mut stats, ArtifactType::Icon, &icon_artifacts);
            artifacts.extend(icon_artifacts);
        }

        if self.config.enable_fonts {
            let font_artifacts = self.process_fonts()?;
            self.update_stats(&mut stats, ArtifactType::Font, &font_artifacts);
            artifacts.extend(font_artifacts);
        }

        if self.config.enable_i18n {
            let i18n_artifacts = self.process_i18n()?;
            self.update_stats(&mut stats, ArtifactType::I18n, &i18n_artifacts);
            artifacts.extend(i18n_artifacts);
        }

        if self.config.enable_serialization {
            let serialization_artifacts = self.process_serialization()?;
            self.update_stats(&mut stats, ArtifactType::Serialization, &serialization_artifacts);
            artifacts.extend(serialization_artifacts);
        }

        // Calculate total build time
        stats.build_time_ms = start_time.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);

        stats.artifacts_processed = artifacts.len();

        Ok(BuildResult { artifacts, stats })
    }

    /// Process media assets (favicons, images)
    fn process_media(&mut self) -> Result<Vec<BuildArtifact>> {
        let media_config = MediaConfig {
            logo_path: self.root_dir.join("extension/media/logo.png"),
            output_dir: self.root_dir.join("dist/favicons"),
            sizes: vec![
                FaviconSize::Ico16,
                FaviconSize::Ico32,
                FaviconSize::Ico48,
                FaviconSize::Png16,
                FaviconSize::Png32,
                FaviconSize::AppleTouch180,
                FaviconSize::AndroidChrome192,
                FaviconSize::AndroidChrome512,
            ],
        };

        let processor = MediaProcessor::new(media_config);

        let (_manifest, artifacts) = processor.generate_favicons(&mut self.cache)?;

        Ok(artifacts)
    }

    /// Process style assets (Binary Dawn CSS)
    fn process_styles(&mut self) -> Result<Vec<BuildArtifact>> {
        let style_config = StyleConfig {
            input_dir: self.root_dir.join("www/styles"),
            output_dir: self.root_dir.join("dist/styles"),
            auto_grouping: true,
            similarity_threshold: 0.7,
            compression_level: 9,
        };

        let processor = StyleProcessor::new(style_config);
        processor.compile_to_binary(&mut self.cache)
    }

    /// Process icon assets
    fn process_icons(&mut self) -> Result<Vec<BuildArtifact>> {
        let icon_config = IconConfig {
            components_dir: self.root_dir.join("www/components"),
            output_dir: self.root_dir.join("dist/icons"),
            file_extensions: vec!["rs".to_string(), "pg".to_string(), "html".to_string()],
            tree_shaking: true,
        };

        let processor = IconProcessor::new(icon_config);
        let (_manifest, artifacts) = processor.process_icons(&mut self.cache)?;
        Ok(artifacts)
    }

    /// Process font assets
    fn process_fonts(&mut self) -> Result<Vec<BuildArtifact>> {
        // TODO: Implement font processing in task 6.1
        Ok(Vec::new())
    }

    /// Process i18n translation bundles
    fn process_i18n(&mut self) -> Result<Vec<BuildArtifact>> {
        // TODO: Implement i18n processing in task 7.1
        Ok(Vec::new())
    }

    /// Process serialization data
    fn process_serialization(&mut self) -> Result<Vec<BuildArtifact>> {
        // TODO: Implement serialization processing in task 8.1
        Ok(Vec::new())
    }

    /// Update statistics for a specific artifact type
    fn update_stats(
        &self,
        stats: &mut BuildStats,
        artifact_type: ArtifactType,
        artifacts: &[BuildArtifact],
    ) {
        let type_stats = stats.by_type.entry(artifact_type).or_default();
        type_stats.count += artifacts.len();
        type_stats.total_size += artifacts.iter().map(|a| a.size).sum::<usize>();
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get the root directory
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Get a reference to the build cache
    pub fn cache(&self) -> &BuildCache {
        &self.cache
    }

    /// Get a mutable reference to the build cache
    pub fn cache_mut(&mut self) -> &mut BuildCache {
        &mut self.cache
    }

    /// Clear the build cache
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be cleared
    pub fn clear_cache(&mut self) -> Result<()> {
        self.cache.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_config_default() {
        let config = BuildConfig::default();
        assert!(config.enable_cache);
        assert!(config.enable_media);
        assert!(config.enable_styles);
        assert!(config.enable_icons);
        assert!(config.enable_fonts);
        assert!(config.enable_i18n);
        assert!(config.enable_serialization);
    }

    #[test]
    fn test_build_pipeline_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pipeline = BuildPipeline::new(temp_dir.path(), BuildConfig::default());
        assert!(pipeline.is_ok());
    }

    #[test]
    fn test_build_pipeline_empty_build() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Disable media and styles processing to avoid file not found errors
        let config = BuildConfig {
            enable_media: false,
            enable_styles: false,
            ..Default::default()
        };

        let mut pipeline = BuildPipeline::new(temp_dir.path(), config).unwrap();
        let result = pipeline.build();
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.artifacts.len(), 0);
    }
}
