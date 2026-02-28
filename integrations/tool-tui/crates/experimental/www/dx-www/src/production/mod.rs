//! # Production Build
//!
//! This module handles production build optimization.
//!
//! Features:
//! - Build orchestration
//! - Source map generation
//! - Minification and compression
//! - Deployment target support

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::DxConfig;
use crate::error::{DxError, DxResult};

// =============================================================================
// Production Builder
// =============================================================================

/// Production build orchestrator.
#[derive(Debug)]
pub struct ProductionBuilder {
    /// Build configuration
    config: ProductionConfig,
    /// Collected assets
    assets: Vec<BuildAsset>,
    /// Source maps
    source_maps: HashMap<PathBuf, SourceMap>,
}

/// Production build configuration.
#[derive(Debug, Clone)]
pub struct ProductionConfig {
    /// Output directory
    pub output_dir: PathBuf,
    /// Enable minification
    pub minify: bool,
    /// Generate source maps
    pub source_maps: bool,
    /// Compression level (0-9)
    pub compression_level: u8,
    /// Target deployment platform
    pub target: DeploymentTarget,
    /// Enable tree shaking
    pub tree_shaking: bool,
    /// Enable code splitting
    pub code_splitting: bool,
}

impl Default for ProductionConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(".dx/build"),
            minify: true,
            source_maps: true,
            compression_level: 6,
            target: DeploymentTarget::Static,
            tree_shaking: true,
            code_splitting: true,
        }
    }
}

/// Deployment target platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentTarget {
    /// Static file hosting
    Static,
    /// Node.js server
    Node,
    /// Cloudflare Workers
    CloudflareWorkers,
    /// Vercel
    Vercel,
    /// AWS Lambda
    Lambda,
    /// Docker container
    Docker,
}

impl DeploymentTarget {
    /// Get the target name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Node => "node",
            Self::CloudflareWorkers => "cloudflare-workers",
            Self::Vercel => "vercel",
            Self::Lambda => "lambda",
            Self::Docker => "docker",
        }
    }

    /// Parse target from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "static" => Some(Self::Static),
            "node" => Some(Self::Node),
            "cloudflare" | "cloudflare-workers" | "cf" => Some(Self::CloudflareWorkers),
            "vercel" => Some(Self::Vercel),
            "lambda" | "aws-lambda" => Some(Self::Lambda),
            "docker" => Some(Self::Docker),
            _ => None,
        }
    }
}

/// A build asset.
#[derive(Debug, Clone)]
pub struct BuildAsset {
    /// Source file path
    pub source: PathBuf,
    /// Output file path
    pub output: PathBuf,
    /// Asset type
    pub asset_type: AssetType,
    /// Original size in bytes
    pub original_size: u64,
    /// Compressed size in bytes
    pub compressed_size: Option<u64>,
    /// Content hash for cache busting
    pub hash: String,
}

/// Type of build asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    /// JavaScript bundle
    JavaScript,
    /// CSS stylesheet
    Css,
    /// HTML page
    Html,
    /// Binary template
    BinaryTemplate,
    /// Image asset
    Image,
    /// Font file
    Font,
    /// Other static asset
    Other,
}

/// Source map for debugging.
#[derive(Debug, Clone)]
pub struct SourceMap {
    /// Source map version (always 3)
    pub version: u8,
    /// Source file paths
    pub sources: Vec<String>,
    /// Source file contents (optional)
    pub sources_content: Option<Vec<String>>,
    /// Mapping data
    pub mappings: String,
    /// Symbol names
    pub names: Vec<String>,
}

impl SourceMap {
    /// Create a new source map.
    pub fn new() -> Self {
        Self {
            version: 3,
            sources: Vec::new(),
            sources_content: None,
            mappings: String::new(),
            names: Vec::new(),
        }
    }

    /// Add a source file.
    pub fn add_source(&mut self, path: &str, content: Option<&str>) {
        self.sources.push(path.to_string());
        if let Some(c) = content {
            let contents = self.sources_content.get_or_insert_with(Vec::new);
            contents.push(c.to_string());
        }
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> String {
        let sources_json: Vec<String> = self
            .sources
            .iter()
            .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();

        let names_json: Vec<String> = self
            .names
            .iter()
            .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();

        let sources_content = if let Some(ref contents) = self.sources_content {
            let content_json: Vec<String> = contents
                .iter()
                .map(|s| {
                    format!(
                        "\"{}\"",
                        s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
                    )
                })
                .collect();
            format!(",\"sourcesContent\":[{}]", content_json.join(","))
        } else {
            String::new()
        };

        format!(
            r#"{{"version":{},"sources":[{}],"mappings":"{}","names":[{}]{}}}"#,
            self.version,
            sources_json.join(","),
            self.mappings,
            names_json.join(","),
            sources_content
        )
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductionBuilder {
    /// Create a new production builder.
    pub fn new() -> Self {
        Self {
            config: ProductionConfig::default(),
            assets: Vec::new(),
            source_maps: HashMap::new(),
        }
    }

    /// Create with configuration.
    pub fn with_config(config: ProductionConfig) -> Self {
        Self {
            config,
            assets: Vec::new(),
            source_maps: HashMap::new(),
        }
    }

    /// Create from DxConfig.
    pub fn from_dx_config(config: &DxConfig) -> Self {
        let prod_config = ProductionConfig {
            output_dir: PathBuf::from(&config.build.output_dir),
            minify: config.build.minify,
            source_maps: config.build.source_maps,
            compression_level: 6,
            target: DeploymentTarget::Static,
            tree_shaking: true,
            code_splitting: true,
        };
        Self::with_config(prod_config)
    }

    /// Run the production build.
    pub fn build(&mut self, source_dir: &Path) -> DxResult<BuildResult> {
        let start = std::time::Instant::now();

        // Ensure output directory exists
        std::fs::create_dir_all(&self.config.output_dir).map_err(|e| DxError::IoError {
            path: Some(self.config.output_dir.clone()),
            message: e.to_string(),
        })?;

        // Collect source files
        let files = self.collect_files(source_dir)?;

        // Process each file
        for file in files {
            self.process_file(&file)?;
        }

        // Generate manifest
        let manifest = self.generate_manifest()?;
        let manifest_path = self.config.output_dir.join("manifest.json");
        std::fs::write(&manifest_path, &manifest).map_err(|e| DxError::IoError {
            path: Some(manifest_path.clone()),
            message: e.to_string(),
        })?;

        // Write source maps
        if self.config.source_maps {
            self.write_source_maps()?;
        }

        let duration = start.elapsed();

        let total_original: u64 = self.assets.iter().map(|a| a.original_size).sum();
        let total_compressed: u64 = self.assets.iter().filter_map(|a| a.compressed_size).sum();

        Ok(BuildResult {
            output_dir: self.config.output_dir.clone(),
            assets: self.assets.clone(),
            total_size: total_original,
            compressed_size: if total_compressed > 0 {
                Some(total_compressed)
            } else {
                None
            },
            duration,
        })
    }

    /// Collect all source files.
    fn collect_files(&self, dir: &Path) -> DxResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        if !dir.exists() {
            return Ok(files);
        }

        self.collect_files_recursive(dir, &mut files)?;
        Ok(files)
    }

    /// Recursively collect files.
    fn collect_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> DxResult<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| DxError::IoError {
            path: Some(dir.to_path_buf()),
            message: e.to_string(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| DxError::IoError {
                path: Some(dir.to_path_buf()),
                message: e.to_string(),
            })?;
            let path = entry.path();

            if path.is_dir() {
                // Skip hidden directories
                if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.')) {
                    continue;
                }
                self.collect_files_recursive(&path, files)?;
            } else {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Process a single file.
    fn process_file(&mut self, path: &Path) -> DxResult<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let asset_type = match ext {
            "js" | "mjs" => AssetType::JavaScript,
            "css" => AssetType::Css,
            "html" | "htm" => AssetType::Html,
            "pg" | "cp" => AssetType::BinaryTemplate,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => AssetType::Image,
            "woff" | "woff2" | "ttf" | "otf" | "eot" => AssetType::Font,
            _ => AssetType::Other,
        };

        let content = std::fs::read(path).map_err(|e| DxError::IoError {
            path: Some(path.to_path_buf()),
            message: e.to_string(),
        })?;

        let original_size = content.len() as u64;

        // Compute hash
        let hash = compute_hash(&content);

        // Process based on type
        let (output_content, source_map) = match asset_type {
            AssetType::JavaScript if self.config.minify => {
                self.minify_javascript(&content, path)?
            }
            AssetType::Css if self.config.minify => self.minify_css(&content, path)?,
            _ => (content, None),
        };

        // Determine output path
        let relative = path.file_name().unwrap_or_default();
        let output_name = if matches!(asset_type, AssetType::JavaScript | AssetType::Css) {
            // Add hash to filename for cache busting
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            format!("{}.{}.{}", stem, &hash[..8], ext)
        } else {
            relative.to_string_lossy().to_string()
        };
        let output_path = self.config.output_dir.join(&output_name);

        // Write output
        std::fs::write(&output_path, &output_content).map_err(|e| DxError::IoError {
            path: Some(output_path.clone()),
            message: e.to_string(),
        })?;

        // Store source map
        if let Some(sm) = source_map {
            self.source_maps.insert(output_path.clone(), sm);
        }

        // Record asset
        self.assets.push(BuildAsset {
            source: path.to_path_buf(),
            output: output_path,
            asset_type,
            original_size,
            compressed_size: Some(output_content.len() as u64),
            hash,
        });

        Ok(())
    }

    /// Minify JavaScript.
    fn minify_javascript(
        &self,
        content: &[u8],
        path: &Path,
    ) -> DxResult<(Vec<u8>, Option<SourceMap>)> {
        let source = String::from_utf8_lossy(content);

        // Simple minification: remove comments and excess whitespace
        let mut result = String::new();
        let mut in_string = false;
        let mut string_char = ' ';
        let mut in_single_comment = false;
        let mut in_multi_comment = false;
        let mut prev_char = ' ';
        let mut last_was_space = false;

        for c in source.chars() {
            if in_single_comment {
                if c == '\n' {
                    in_single_comment = false;
                    if !last_was_space && !result.is_empty() {
                        result.push(' ');
                        last_was_space = true;
                    }
                }
                prev_char = c;
                continue;
            }

            if in_multi_comment {
                if prev_char == '*' && c == '/' {
                    in_multi_comment = false;
                }
                prev_char = c;
                continue;
            }

            if in_string {
                result.push(c);
                if c == string_char && prev_char != '\\' {
                    in_string = false;
                }
                prev_char = c;
                last_was_space = false;
                continue;
            }

            match c {
                '"' | '\'' | '`' => {
                    in_string = true;
                    string_char = c;
                    result.push(c);
                    last_was_space = false;
                }
                '/' if prev_char == '/' => {
                    in_single_comment = true;
                    result.pop(); // Remove the first /
                }
                '*' if prev_char == '/' => {
                    in_multi_comment = true;
                    result.pop(); // Remove the /
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if !last_was_space && !result.is_empty() {
                        result.push(' ');
                        last_was_space = true;
                    }
                }
                _ => {
                    result.push(c);
                    last_was_space = false;
                }
            }
            prev_char = c;
        }

        let result = result.trim().to_string();

        // Generate source map
        let source_map = if self.config.source_maps {
            let mut sm = SourceMap::new();
            sm.add_source(
                path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown"),
                Some(&source),
            );
            Some(sm)
        } else {
            None
        };

        Ok((result.into_bytes(), source_map))
    }

    /// Minify CSS.
    fn minify_css(&self, content: &[u8], path: &Path) -> DxResult<(Vec<u8>, Option<SourceMap>)> {
        let source = String::from_utf8_lossy(content);

        // Simple CSS minification
        let mut result = String::new();
        let mut in_string = false;
        let mut string_char = ' ';
        let mut in_comment = false;
        let mut prev_char = ' ';
        let mut last_was_space = false;

        for c in source.chars() {
            if in_comment {
                if prev_char == '*' && c == '/' {
                    in_comment = false;
                }
                prev_char = c;
                continue;
            }

            if in_string {
                result.push(c);
                if c == string_char && prev_char != '\\' {
                    in_string = false;
                }
                prev_char = c;
                last_was_space = false;
                continue;
            }

            match c {
                '"' | '\'' => {
                    in_string = true;
                    string_char = c;
                    result.push(c);
                    last_was_space = false;
                }
                '*' if prev_char == '/' => {
                    in_comment = true;
                    result.pop();
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if !last_was_space && !result.is_empty() {
                        // Only keep space if needed
                        let last = result.chars().last().unwrap_or(' ');
                        if !matches!(last, '{' | '}' | ';' | ':' | ',') {
                            result.push(' ');
                            last_was_space = true;
                        }
                    }
                }
                '{' | '}' | ';' | ':' | ',' => {
                    // Remove trailing space before these chars
                    if last_was_space && !result.is_empty() {
                        result.pop();
                    }
                    result.push(c);
                    last_was_space = false;
                }
                _ => {
                    result.push(c);
                    last_was_space = false;
                }
            }
            prev_char = c;
        }

        let result = result.trim().to_string();

        // Generate source map
        let source_map = if self.config.source_maps {
            let mut sm = SourceMap::new();
            sm.add_source(
                path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown"),
                Some(&source),
            );
            Some(sm)
        } else {
            None
        };

        Ok((result.into_bytes(), source_map))
    }

    /// Generate build manifest.
    fn generate_manifest(&self) -> DxResult<String> {
        let mut entries = Vec::new();

        for asset in &self.assets {
            let entry = format!(
                r#"    "{}": {{"output": "{}", "hash": "{}", "size": {}}}"#,
                asset.source.display().to_string().replace('\\', "/"),
                asset.output.display().to_string().replace('\\', "/"),
                asset.hash,
                asset.compressed_size.unwrap_or(asset.original_size)
            );
            entries.push(entry);
        }

        Ok(format!("{{\n{}\n}}", entries.join(",\n")))
    }

    /// Write source maps to disk.
    fn write_source_maps(&self) -> DxResult<()> {
        for (path, source_map) in &self.source_maps {
            let map_path = PathBuf::from(format!("{}.map", path.display()));
            let json = source_map.to_json();
            std::fs::write(&map_path, json).map_err(|e| DxError::IoError {
                path: Some(map_path),
                message: e.to_string(),
            })?;
        }
        Ok(())
    }

    /// Get collected assets.
    pub fn assets(&self) -> &[BuildAsset] {
        &self.assets
    }
}

impl Default for ProductionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Build result.
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// Output directory
    pub output_dir: PathBuf,
    /// Built assets
    pub assets: Vec<BuildAsset>,
    /// Total original size
    pub total_size: u64,
    /// Total compressed size
    pub compressed_size: Option<u64>,
    /// Build duration
    pub duration: std::time::Duration,
}

impl BuildResult {
    /// Print build summary.
    pub fn print_summary(&self) {
        println!("Build complete in {:?}", self.duration);
        println!();
        println!("Output: {}", self.output_dir.display());
        println!("Files:  {}", self.assets.len());
        println!("Size:   {} bytes", self.total_size);
        if let Some(compressed) = self.compressed_size {
            let ratio = (compressed as f64 / self.total_size as f64) * 100.0;
            println!("Minified: {} bytes ({:.1}%)", compressed, ratio);
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Compute content hash using BLAKE3.
fn compute_hash(content: &[u8]) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(content);
    let hash = hasher.finalize();
    // Manual hex encoding
    hash.as_bytes()[..16].iter().map(|b| format!("{:02x}", b)).collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_builder_new() {
        let builder = ProductionBuilder::new();
        assert!(builder.assets.is_empty());
        assert!(builder.source_maps.is_empty());
    }

    #[test]
    fn test_production_config_default() {
        let config = ProductionConfig::default();
        assert!(config.minify);
        assert!(config.source_maps);
        assert_eq!(config.compression_level, 6);
        assert!(config.tree_shaking);
        assert!(config.code_splitting);
    }

    #[test]
    fn test_deployment_target_from_str() {
        assert_eq!(DeploymentTarget::from_str("static"), Some(DeploymentTarget::Static));
        assert_eq!(DeploymentTarget::from_str("node"), Some(DeploymentTarget::Node));
        assert_eq!(
            DeploymentTarget::from_str("cloudflare"),
            Some(DeploymentTarget::CloudflareWorkers)
        );
        assert_eq!(DeploymentTarget::from_str("vercel"), Some(DeploymentTarget::Vercel));
        assert_eq!(DeploymentTarget::from_str("lambda"), Some(DeploymentTarget::Lambda));
        assert_eq!(DeploymentTarget::from_str("docker"), Some(DeploymentTarget::Docker));
        assert_eq!(DeploymentTarget::from_str("unknown"), None);
    }

    #[test]
    fn test_deployment_target_name() {
        assert_eq!(DeploymentTarget::Static.name(), "static");
        assert_eq!(DeploymentTarget::Node.name(), "node");
        assert_eq!(DeploymentTarget::CloudflareWorkers.name(), "cloudflare-workers");
    }

    #[test]
    fn test_source_map_new() {
        let sm = SourceMap::new();
        assert_eq!(sm.version, 3);
        assert!(sm.sources.is_empty());
        assert!(sm.names.is_empty());
    }

    #[test]
    fn test_source_map_add_source() {
        let mut sm = SourceMap::new();
        sm.add_source("test.js", Some("console.log('hello');"));
        assert_eq!(sm.sources.len(), 1);
        assert_eq!(sm.sources[0], "test.js");
        assert!(sm.sources_content.is_some());
    }

    #[test]
    fn test_source_map_to_json() {
        let mut sm = SourceMap::new();
        sm.add_source("test.js", None);
        let json = sm.to_json();
        assert!(json.contains("\"version\":3"));
        assert!(json.contains("\"test.js\""));
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash(b"hello");
        let hash2 = compute_hash(b"hello");
        let hash3 = compute_hash(b"world");
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 32); // 16 bytes = 32 hex chars
    }
}
