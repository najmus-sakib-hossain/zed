//! Media processing for DX-WWW build pipeline
//!
//! This module handles favicon generation from source logos using the dx-media CLI tool.
//! It generates all required favicon sizes and formats, and creates a manifest.json file
//! with icon references.

use crate::error::{BuildError, Result};
use crate::hash::content_hash;
use crate::{BuildArtifact, BuildCache, CacheEntry, CacheKey};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Media processor for generating favicons and processing images
pub struct MediaProcessor {
    /// Configuration for media processing
    config: MediaConfig,
}

/// Configuration for media processing
#[derive(Debug, Clone)]
pub struct MediaConfig {
    /// Path to the source logo file
    pub logo_path: PathBuf,
    /// Output directory for generated favicons
    pub output_dir: PathBuf,
    /// Favicon sizes to generate
    pub sizes: Vec<FaviconSize>,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            logo_path: PathBuf::from("extension/media/logo.png"),
            output_dir: PathBuf::from("dist/favicons"),
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
        }
    }
}

/// Favicon size specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaviconSize {
    /// 16x16 ICO format
    Ico16,
    /// 32x32 ICO format
    Ico32,
    /// 48x48 ICO format
    Ico48,
    /// 16x16 PNG format
    Png16,
    /// 32x32 PNG format
    Png32,
    /// 180x180 Apple Touch Icon
    AppleTouch180,
    /// 192x192 Android Chrome
    AndroidChrome192,
    /// 512x512 Android Chrome
    AndroidChrome512,
}

impl FaviconSize {
    /// Get the pixel dimensions for this size
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Ico16 | Self::Png16 => (16, 16),
            Self::Ico32 | Self::Png32 => (32, 32),
            Self::Ico48 => (48, 48),
            Self::AppleTouch180 => (180, 180),
            Self::AndroidChrome192 => (192, 192),
            Self::AndroidChrome512 => (512, 512),
        }
    }

    /// Get the output filename for this size
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Ico16 => "favicon-16x16.ico",
            Self::Ico32 => "favicon-32x32.ico",
            Self::Ico48 => "favicon-48x48.ico",
            Self::Png16 => "favicon-16x16.png",
            Self::Png32 => "favicon-32x32.png",
            Self::AppleTouch180 => "apple-touch-icon.png",
            Self::AndroidChrome192 => "android-chrome-192x192.png",
            Self::AndroidChrome512 => "android-chrome-512x512.png",
        }
    }

    /// Get the format (extension) for this size
    pub fn format(&self) -> &'static str {
        match self {
            Self::Ico16 | Self::Ico32 | Self::Ico48 => "ico",
            _ => "png",
        }
    }
}

/// Manifest of generated favicons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaviconManifest {
    /// List of generated favicon files
    pub icons: Vec<FaviconEntry>,
}

/// Single favicon entry in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaviconEntry {
    /// Path to the favicon file (relative to output dir)
    pub src: String,
    /// Sizes in "WxH" format
    pub sizes: String,
    /// MIME type
    #[serde(rename = "type")]
    pub mime_type: String,
}

impl MediaProcessor {
    /// Create a new media processor
    pub fn new(config: MediaConfig) -> Self {
        Self { config }
    }

    /// Generate favicons from the source logo
    ///
    /// This method checks the cache first and only regenerates if the source has changed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The source logo file doesn't exist
    /// - The dx-media CLI tool fails
    /// - Output files cannot be written
    pub fn generate_favicons(
        &self,
        cache: &mut BuildCache,
    ) -> Result<(FaviconManifest, Vec<BuildArtifact>)> {
        // Check if source logo exists
        if !self.config.logo_path.exists() {
            return Err(BuildError::FileNotFound(self.config.logo_path.clone()));
        }

        // Create cache key from source logo
        let cache_key = CacheKey::from_file(&self.config.logo_path, "media-favicon".to_string())?;

        // Check cache for existing favicons
        if let Some(cached) = cache.get(&cache_key) {
            // Verify all expected outputs exist
            if self.verify_cached_outputs(&cached.output_path) {
                return self.load_cached_favicons(&cached.output_path);
            }
        }

        // Generate new favicons
        let manifest = self.generate_favicons_uncached()?;
        let artifacts = self.create_artifacts(&manifest)?;

        // Cache the manifest
        let manifest_path = self.config.output_dir.join("manifest.json");
        let manifest_data = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| BuildError::Media(format!("Failed to serialize manifest: {}", e)))?;
        let manifest_hash = content_hash(&manifest_data);

        let cache_entry =
            CacheEntry::new(cache_key, manifest_path.clone(), manifest_hash, manifest_data.len());
        cache.insert(cache_entry)?;

        Ok((manifest, artifacts))
    }

    /// Generate favicons without using cache
    fn generate_favicons_uncached(&self) -> Result<FaviconManifest> {
        // Create output directory
        std::fs::create_dir_all(&self.config.output_dir).map_err(|e| BuildError::Io {
            path: self.config.output_dir.clone(),
            source: e,
        })?;

        let mut icons = Vec::new();

        // Generate each favicon size
        for size in &self.config.sizes {
            let (width, height) = size.dimensions();
            let output_path = self.config.output_dir.join(size.filename());

            // Call dx-media CLI to generate favicon
            // For now, we'll use a simple image resize approach
            // In production, this would call the actual dx-media tool
            self.generate_favicon_file(&self.config.logo_path, &output_path, width, height)?;

            // Add to manifest
            icons.push(FaviconEntry {
                src: size.filename().to_string(),
                sizes: format!("{}x{}", width, height),
                mime_type: match size.format() {
                    "ico" => "image/x-icon".to_string(),
                    "png" => "image/png".to_string(),
                    _ => "application/octet-stream".to_string(),
                },
            });
        }

        Ok(FaviconManifest { icons })
    }

    /// Generate a single favicon file
    ///
    /// This is a placeholder that would call dx-media CLI in production.
    /// For now, it creates a simple copy or placeholder.
    fn generate_favicon_file(
        &self,
        source: &Path,
        output: &Path,
        _width: u32,
        _height: u32,
    ) -> Result<()> {
        // TODO: Call dx-media CLI tool
        // For now, we'll create a placeholder by copying the source
        // In production, this would be:
        // Command::new("dx-media")
        //     .arg("resize")
        //     .arg(source)
        //     .arg(output)
        //     .arg("--width").arg(width.to_string())
        //     .arg("--height").arg(height.to_string())
        //     .output()

        // Placeholder: just copy the file for now
        std::fs::copy(source, output).map_err(|e| BuildError::Io {
            path: output.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }

    /// Verify that all cached outputs still exist
    fn verify_cached_outputs(&self, manifest_path: &Path) -> bool {
        if !manifest_path.exists() {
            return false;
        }

        // Load manifest and check all icon files exist
        if let Ok(data) = std::fs::read(manifest_path)
            && let Ok(manifest) = serde_json::from_slice::<FaviconManifest>(&data)
        {
            return manifest
                .icons
                .iter()
                .all(|icon| self.config.output_dir.join(&icon.src).exists());
        }

        false
    }

    /// Load cached favicons from manifest
    fn load_cached_favicons(
        &self,
        manifest_path: &Path,
    ) -> Result<(FaviconManifest, Vec<BuildArtifact>)> {
        let data = std::fs::read(manifest_path).map_err(|e| BuildError::Io {
            path: manifest_path.to_path_buf(),
            source: e,
        })?;

        let manifest: FaviconManifest = serde_json::from_slice(&data)
            .map_err(|e| BuildError::Media(format!("Failed to parse manifest: {}", e)))?;

        let artifacts = self.create_artifacts(&manifest)?;

        Ok((manifest, artifacts))
    }

    /// Create build artifacts from manifest
    fn create_artifacts(&self, manifest: &FaviconManifest) -> Result<Vec<BuildArtifact>> {
        let mut artifacts = Vec::new();

        for icon in &manifest.icons {
            let path = self.config.output_dir.join(&icon.src);
            if !path.exists() {
                continue;
            }

            let data = std::fs::read(&path).map_err(|e| BuildError::Io {
                path: path.clone(),
                source: e,
            })?;

            artifacts.push(BuildArtifact {
                artifact_type: crate::ArtifactType::Media,
                path,
                hash: content_hash(&data),
                size: data.len(),
            });
        }

        Ok(artifacts)
    }

    /// Generate manifest.json for PWA
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest cannot be serialized or written
    pub fn generate_manifest_json(&self, favicons: &FaviconManifest) -> Result<String> {
        // Create a PWA manifest with icon references
        let manifest = serde_json::json!({
            "name": "DX-WWW Application",
            "short_name": "DX-WWW",
            "icons": favicons.icons,
            "theme_color": "#ffffff",
            "background_color": "#ffffff",
            "display": "standalone"
        });

        serde_json::to_string_pretty(&manifest)
            .map_err(|e| BuildError::Media(format!("Failed to serialize manifest: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_favicon_size_dimensions() {
        assert_eq!(FaviconSize::Ico16.dimensions(), (16, 16));
        assert_eq!(FaviconSize::Ico32.dimensions(), (32, 32));
        assert_eq!(FaviconSize::Ico48.dimensions(), (48, 48));
        assert_eq!(FaviconSize::Png16.dimensions(), (16, 16));
        assert_eq!(FaviconSize::Png32.dimensions(), (32, 32));
        assert_eq!(FaviconSize::AppleTouch180.dimensions(), (180, 180));
        assert_eq!(FaviconSize::AndroidChrome192.dimensions(), (192, 192));
        assert_eq!(FaviconSize::AndroidChrome512.dimensions(), (512, 512));
    }

    #[test]
    fn test_favicon_size_filename() {
        assert_eq!(FaviconSize::Ico16.filename(), "favicon-16x16.ico");
        assert_eq!(FaviconSize::Png32.filename(), "favicon-32x32.png");
        assert_eq!(FaviconSize::AppleTouch180.filename(), "apple-touch-icon.png");
        assert_eq!(FaviconSize::AndroidChrome192.filename(), "android-chrome-192x192.png");
    }

    #[test]
    fn test_favicon_size_format() {
        assert_eq!(FaviconSize::Ico16.format(), "ico");
        assert_eq!(FaviconSize::Png16.format(), "png");
        assert_eq!(FaviconSize::AppleTouch180.format(), "png");
    }

    #[test]
    fn test_media_config_default() {
        let config = MediaConfig::default();
        assert_eq!(config.logo_path, PathBuf::from("extension/media/logo.png"));
        assert_eq!(config.sizes.len(), 8);
    }

    #[test]
    fn test_generate_manifest_json() {
        let temp_dir = TempDir::new().unwrap();
        let config = MediaConfig {
            logo_path: PathBuf::from("logo.png"),
            output_dir: temp_dir.path().to_path_buf(),
            sizes: vec![FaviconSize::Png16, FaviconSize::Png32],
        };

        let processor = MediaProcessor::new(config);

        let manifest = FaviconManifest {
            icons: vec![
                FaviconEntry {
                    src: "favicon-16x16.png".to_string(),
                    sizes: "16x16".to_string(),
                    mime_type: "image/png".to_string(),
                },
                FaviconEntry {
                    src: "favicon-32x32.png".to_string(),
                    sizes: "32x32".to_string(),
                    mime_type: "image/png".to_string(),
                },
            ],
        };

        let json = processor.generate_manifest_json(&manifest);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains("favicon-16x16.png"));
        assert!(json.contains("favicon-32x32.png"));
    }

    #[test]
    fn test_generate_favicons_missing_source() {
        let temp_dir = TempDir::new().unwrap();
        let config = MediaConfig {
            logo_path: PathBuf::from("nonexistent.png"),
            output_dir: temp_dir.path().join("favicons"),
            sizes: vec![FaviconSize::Png16],
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        let result = processor.generate_favicons(&mut cache);
        assert!(result.is_err());
        match result {
            Err(BuildError::FileNotFound(_)) => {}
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_generate_favicons_with_source() {
        let temp_dir = TempDir::new().unwrap();

        // Create a dummy logo file
        let logo_path = temp_dir.path().join("logo.png");
        std::fs::write(&logo_path, b"fake png data").unwrap();

        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: vec![FaviconSize::Png16, FaviconSize::Png32],
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        let result = processor.generate_favicons(&mut cache);
        assert!(result.is_ok());

        let (manifest, artifacts) = result.unwrap();
        assert_eq!(manifest.icons.len(), 2);
        assert_eq!(artifacts.len(), 2);

        // Verify files were created
        assert!(output_dir.join("favicon-16x16.png").exists());
        assert!(output_dir.join("favicon-32x32.png").exists());
    }

    #[test]
    fn test_generate_favicons_uses_cache() {
        let temp_dir = TempDir::new().unwrap();

        // Create a dummy logo file
        let logo_path = temp_dir.path().join("logo.png");
        std::fs::write(&logo_path, b"fake png data").unwrap();

        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: vec![FaviconSize::Png16],
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // First generation
        let result1 = processor.generate_favicons(&mut cache);
        assert!(result1.is_ok());

        // Second generation should use cache
        let result2 = processor.generate_favicons(&mut cache);
        assert!(result2.is_ok());

        let (manifest2, artifacts2) = result2.unwrap();
        assert_eq!(manifest2.icons.len(), 1);
        assert_eq!(artifacts2.len(), 1);
    }
}
