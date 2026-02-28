//! Style processing for DX-WWW build pipeline
//!
//! This module handles CSS compilation to Binary Dawn format using the dx-style CLI tool.
//! It discovers all CSS files in www/styles/, compiles them to .dxbd binary format,
//! and implements caching with content hashing to avoid recompiling unchanged CSS.

use crate::error::{BuildError, Result};
use crate::hash::content_hash;
use crate::{ArtifactType, BuildArtifact, BuildCache, CacheEntry, CacheKey};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Style processor for compiling CSS to Binary Dawn format
pub struct StyleProcessor {
    /// Configuration for style processing
    config: StyleConfig,
}

/// Configuration for style processing
#[derive(Debug, Clone)]
pub struct StyleConfig {
    /// Input directory containing CSS files (e.g., www/styles/)
    pub input_dir: PathBuf,
    /// Output directory for Binary Dawn CSS files
    pub output_dir: PathBuf,
    /// Enable auto-grouping with similarity detection
    pub auto_grouping: bool,
    /// Similarity threshold for auto-grouping (0.0 to 1.0)
    pub similarity_threshold: f32,
    /// Compression level (0-9, where 9 is maximum compression)
    pub compression_level: u8,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("www/styles"),
            output_dir: PathBuf::from("dist/styles"),
            auto_grouping: true,
            similarity_threshold: 0.7,
            compression_level: 9,
        }
    }
}

/// Bundle of compiled Binary Dawn CSS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStyleBundle {
    /// Binary CSS data
    pub data: Vec<u8>,
    /// Content hash of the binary data
    pub hash: String,
    /// Size in bytes
    pub size: usize,
    /// Source CSS files that were compiled
    pub sources: Vec<PathBuf>,
}

/// Metadata about a compiled style file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleArtifactMetadata {
    /// Source CSS file path
    pub source: PathBuf,
    /// Output Binary Dawn CSS file path
    pub output: PathBuf,
    /// Content hash of source
    pub source_hash: String,
    /// Content hash of output
    pub output_hash: String,
    /// Size of output in bytes
    pub size: usize,
}

impl StyleProcessor {
    /// Create a new style processor
    pub fn new(config: StyleConfig) -> Self {
        Self { config }
    }

    /// Compile all CSS files to Binary Dawn format
    ///
    /// This method:
    /// 1. Discovers all CSS files in the input directory
    /// 2. Checks cache for each file
    /// 3. Compiles changed files using dx-style CLI
    /// 4. Generates .dxbd binary format files
    /// 5. Updates cache with new artifacts
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input directory doesn't exist
    /// - CSS files cannot be read
    /// - dx-style CLI fails
    /// - Output files cannot be written
    pub fn compile_to_binary(&self, cache: &mut BuildCache) -> Result<Vec<BuildArtifact>> {
        // Ensure input directory exists
        if !self.config.input_dir.exists() {
            return Err(BuildError::FileNotFound(self.config.input_dir.clone()));
        }

        // Create output directory
        std::fs::create_dir_all(&self.config.output_dir).map_err(|e| BuildError::Io {
            path: self.config.output_dir.clone(),
            source: e,
        })?;

        // Discover all CSS files
        let css_files = self.discover_css_files()?;

        if css_files.is_empty() {
            // No CSS files found, return empty artifacts
            return Ok(Vec::new());
        }

        let mut artifacts = Vec::new();

        // Process each CSS file
        for css_file in css_files {
            let artifact = self.process_css_file(&css_file, cache)?;
            artifacts.push(artifact);
        }

        Ok(artifacts)
    }

    /// Discover all CSS files in the input directory
    pub fn discover_css_files(&self) -> Result<Vec<PathBuf>> {
        let mut css_files = Vec::new();

        for entry in WalkDir::new(&self.config.input_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("css") {
                css_files.push(path.to_path_buf());
            }
        }

        Ok(css_files)
    }

    /// Process a single CSS file
    fn process_css_file(&self, css_file: &Path, cache: &mut BuildCache) -> Result<BuildArtifact> {
        // Create cache key from source CSS file
        let cache_key = CacheKey::from_file(css_file, "style-binary-dawn".to_string())?;

        // Check cache for existing compiled output
        if let Some(cached) = cache.get(&cache_key) {
            // Verify cached output still exists and is valid
            if cached.is_valid() {
                return Ok(BuildArtifact {
                    artifact_type: ArtifactType::Style,
                    path: cached.output_path.clone(),
                    hash: cached.output_hash.clone(),
                    size: cached.size,
                });
            }
        }

        // Compile CSS to Binary Dawn format
        let output_path = self.get_output_path(css_file)?;
        self.compile_css_file(css_file, &output_path)?;

        // Read compiled output
        let output_data = std::fs::read(&output_path).map_err(|e| BuildError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        let output_hash = content_hash(&output_data);
        let size = output_data.len();

        // Cache the compiled output
        let cache_entry =
            CacheEntry::new(cache_key, output_path.clone(), output_hash.clone(), size);
        cache.insert(cache_entry)?;

        Ok(BuildArtifact {
            artifact_type: ArtifactType::Style,
            path: output_path,
            hash: output_hash,
            size,
        })
    }

    /// Get the output path for a CSS file
    fn get_output_path(&self, css_file: &Path) -> Result<PathBuf> {
        // Get relative path from input dir
        let relative = pathdiff::diff_paths(css_file, &self.config.input_dir).ok_or_else(|| {
            BuildError::Style(format!("Failed to compute relative path for {:?}", css_file))
        })?;

        // Change extension to .dxbd
        let mut output_path = self.config.output_dir.join(relative);
        output_path.set_extension("dxbd");

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| BuildError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        Ok(output_path)
    }

    /// Compile a CSS file to Binary Dawn format using dx-style CLI
    fn compile_css_file(&self, input: &Path, output: &Path) -> Result<()> {
        // Build dx-style command
        let mut cmd = Command::new("cargo");
        cmd.arg("run")
            .arg("--bin")
            .arg("dx-style")
            .arg("--")
            .arg("compile")
            .arg(input)
            .arg("--output")
            .arg(output)
            .arg("--format")
            .arg("binary");

        // Add auto-grouping if enabled
        if self.config.auto_grouping {
            cmd.arg("--auto-group")
                .arg("--similarity-threshold")
                .arg(self.config.similarity_threshold.to_string());
        }

        // Add compression level
        cmd.arg("--compression").arg(self.config.compression_level.to_string());

        // Execute command
        let output_result = cmd
            .output()
            .map_err(|e| BuildError::Style(format!("Failed to execute dx-style CLI: {}", e)))?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            return Err(BuildError::Style(format!(
                "dx-style compilation failed for {:?}: {}",
                input, stderr
            )));
        }

        Ok(())
    }

    /// Generate a runtime loader for Binary Dawn CSS
    ///
    /// This generates JavaScript/WASM code that can decode and apply
    /// Binary Dawn CSS at runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the loader template cannot be generated
    pub fn generate_loader(&self) -> Result<String> {
        // Generate a simple loader stub
        // In production, this would generate actual WASM loader code
        Ok(r#"
// Binary Dawn CSS Runtime Loader
// This loader decodes .dxbd format and applies styles to the DOM

export async function loadBinaryStyles(url) {
    const response = await fetch(url);
    const buffer = await response.arrayBuffer();
    const styles = decodeBinaryDawnCSS(buffer);
    applyStylesToDOM(styles);
}

function decodeBinaryDawnCSS(buffer) {
    // TODO: Implement Binary Dawn CSS decoder
    // This would decode the binary format to CSS rules
    return [];
}

function applyStylesToDOM(styles) {
    // TODO: Apply decoded styles to DOM
    // This would create style elements or use CSSOM
}
"#
        .to_string())
    }

    /// Get all tracked style artifacts
    ///
    /// Returns metadata about all compiled style files
    pub fn get_artifacts(&self, _cache: &BuildCache) -> Vec<StyleArtifactMetadata> {
        // This would query the cache for all style artifacts
        // For now, return empty vector
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_style_config_default() {
        let config = StyleConfig::default();
        assert_eq!(config.input_dir, PathBuf::from("www/styles"));
        assert_eq!(config.output_dir, PathBuf::from("dist/styles"));
        assert!(config.auto_grouping);
        assert_eq!(config.similarity_threshold, 0.7);
        assert_eq!(config.compression_level, 9);
    }

    #[test]
    fn test_style_processor_creation() {
        let config = StyleConfig::default();
        let processor = StyleProcessor::new(config);
        assert!(processor.config.auto_grouping);
    }

    #[test]
    fn test_discover_css_files_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        std::fs::create_dir(&styles_dir).unwrap();

        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };

        let processor = StyleProcessor::new(config);
        let files = processor.discover_css_files();
        assert!(files.is_ok());
        assert_eq!(files.unwrap().len(), 0);
    }

    #[test]
    fn test_discover_css_files_with_css() {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        std::fs::create_dir(&styles_dir).unwrap();

        // Create some CSS files
        std::fs::write(styles_dir.join("main.css"), "body { color: red; }").unwrap();
        std::fs::write(styles_dir.join("theme.css"), ".theme { background: blue; }").unwrap();

        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };

        let processor = StyleProcessor::new(config);
        let files = processor.discover_css_files();
        assert!(files.is_ok());
        let files = files.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_discover_css_files_nested() {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        std::fs::create_dir(&styles_dir).unwrap();

        // Create nested directory structure
        let components_dir = styles_dir.join("components");
        std::fs::create_dir(&components_dir).unwrap();

        std::fs::write(styles_dir.join("main.css"), "body { color: red; }").unwrap();
        std::fs::write(components_dir.join("button.css"), ".btn { padding: 10px; }").unwrap();

        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };

        let processor = StyleProcessor::new(config);
        let files = processor.discover_css_files();
        assert!(files.is_ok());
        let files = files.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_get_output_path() {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        let output_dir = temp_dir.path().join("dist");

        let config = StyleConfig {
            input_dir: styles_dir.clone(),
            output_dir: output_dir.clone(),
            ..Default::default()
        };

        let processor = StyleProcessor::new(config);

        let input = styles_dir.join("main.css");
        let output = processor.get_output_path(&input);
        assert!(output.is_ok());
        let output = output.unwrap();
        assert_eq!(output, output_dir.join("main.dxbd"));
    }

    #[test]
    fn test_get_output_path_nested() {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        let output_dir = temp_dir.path().join("dist");

        let config = StyleConfig {
            input_dir: styles_dir.clone(),
            output_dir: output_dir.clone(),
            ..Default::default()
        };

        let processor = StyleProcessor::new(config);

        let input = styles_dir.join("components").join("button.css");
        let output = processor.get_output_path(&input);
        assert!(output.is_ok());
        let output = output.unwrap();
        assert_eq!(output, output_dir.join("components").join("button.dxbd"));
    }

    #[test]
    fn test_generate_loader() {
        let config = StyleConfig::default();
        let processor = StyleProcessor::new(config);
        let loader = processor.generate_loader();
        assert!(loader.is_ok());
        let loader = loader.unwrap();
        assert!(loader.contains("loadBinaryStyles"));
        assert!(loader.contains("decodeBinaryDawnCSS"));
    }

    #[test]
    fn test_compile_to_binary_missing_input_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = StyleConfig {
            input_dir: temp_dir.path().join("nonexistent"),
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };

        let processor = StyleProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        let result = processor.compile_to_binary(&mut cache);
        assert!(result.is_err());
        match result {
            Err(BuildError::FileNotFound(_)) => {}
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_compile_to_binary_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        std::fs::create_dir(&styles_dir).unwrap();

        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };

        let processor = StyleProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        let result = processor.compile_to_binary(&mut cache);
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 0);
    }
}
