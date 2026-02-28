//! # Static Assets
//!
//! This module handles static asset serving and optimization.
//!
//! Features:
//! - Serve files from `public/` directory
//! - Asset optimization (images, etc.)
//! - Content hashing for cache busting
//! - URL resolution for imports

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{DxError, DxResult};

// =============================================================================
// Asset Server
// =============================================================================

/// Serves and manages static assets.
#[derive(Debug)]
pub struct AssetServer {
    /// Root directory for assets
    pub root: PathBuf,
    /// Asset manifest (path -> hash)
    manifest: HashMap<String, AssetEntry>,
    /// Public path prefix
    public_path: String,
}

impl AssetServer {
    /// Create a new asset server.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            manifest: HashMap::new(),
            public_path: "/".to_string(),
        }
    }

    /// Create with a custom public path.
    pub fn with_public_path(root: PathBuf, public_path: String) -> Self {
        Self {
            root,
            manifest: HashMap::new(),
            public_path,
        }
    }

    /// Scan the assets directory.
    pub fn scan(&mut self) -> DxResult<()> {
        if !self.root.exists() {
            return Ok(());
        }

        self.scan_directory(&self.root.clone())?;
        Ok(())
    }

    /// Scan a directory recursively.
    fn scan_directory(&mut self, dir: &Path) -> DxResult<()> {
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
                self.scan_directory(&path)?;
            } else {
                self.register_asset(&path)?;
            }
        }

        Ok(())
    }

    /// Register an asset file.
    fn register_asset(&mut self, path: &Path) -> DxResult<()> {
        let relative = path.strip_prefix(&self.root).map_err(|_| DxError::IoError {
            path: Some(path.to_path_buf()),
            message: "Failed to get relative path".to_string(),
        })?;

        let url_path =
            format!("{}{}", self.public_path, relative.to_string_lossy().replace('\\', "/"));

        // Read file and compute hash
        let content = std::fs::read(path).map_err(|e| DxError::IoError {
            path: Some(path.to_path_buf()),
            message: e.to_string(),
        })?;

        let hash = compute_hash(&content);
        let size = content.len();
        let mime_type = mime_from_extension(path);

        let entry = AssetEntry {
            path: path.to_path_buf(),
            url_path: url_path.clone(),
            hash,
            size,
            mime_type,
            optimized: false,
        };

        self.manifest.insert(url_path, entry);
        Ok(())
    }

    /// Get an asset by URL path.
    pub fn get(&self, url_path: &str) -> Option<&AssetEntry> {
        self.manifest.get(url_path)
    }

    /// Get all assets.
    pub fn assets(&self) -> &HashMap<String, AssetEntry> {
        &self.manifest
    }

    /// Resolve an asset URL with content hash.
    pub fn resolve_url(&self, url_path: &str) -> Option<String> {
        self.manifest.get(url_path).map(|entry| {
            let ext_idx = url_path.rfind('.').unwrap_or(url_path.len());
            let (base, ext) = url_path.split_at(ext_idx);
            format!("{}.{}{}", base, &entry.hash[..8], ext)
        })
    }

    /// Read asset content.
    pub fn read(&self, url_path: &str) -> DxResult<Vec<u8>> {
        let entry = self.manifest.get(url_path).ok_or_else(|| DxError::IoError {
            path: Some(PathBuf::from(url_path)),
            message: "Asset not found".to_string(),
        })?;

        std::fs::read(&entry.path).map_err(|e| DxError::IoError {
            path: Some(entry.path.clone()),
            message: e.to_string(),
        })
    }
}

// =============================================================================
// Asset Entry
// =============================================================================

/// An asset file entry.
#[derive(Debug, Clone)]
pub struct AssetEntry {
    /// File path on disk
    pub path: PathBuf,
    /// URL path
    pub url_path: String,
    /// Content hash
    pub hash: String,
    /// File size in bytes
    pub size: usize,
    /// MIME type
    pub mime_type: String,
    /// Whether the asset has been optimized
    pub optimized: bool,
}

// =============================================================================
// Asset Optimizer
// =============================================================================

/// Optimizes static assets.
#[derive(Debug)]
pub struct AssetOptimizer {
    /// Image quality (0-100)
    pub image_quality: u8,
    /// Enable image compression
    pub compress_images: bool,
    /// Enable SVG optimization
    pub optimize_svg: bool,
}

impl Default for AssetOptimizer {
    fn default() -> Self {
        Self {
            image_quality: 80,
            compress_images: true,
            optimize_svg: true,
        }
    }
}

impl AssetOptimizer {
    /// Create a new optimizer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Optimize an asset.
    pub fn optimize(&self, entry: &AssetEntry) -> DxResult<OptimizedAsset> {
        let content = std::fs::read(&entry.path).map_err(|e| DxError::IoError {
            path: Some(entry.path.clone()),
            message: e.to_string(),
        })?;

        let original_size = content.len();
        let optimized = match entry.mime_type.as_str() {
            "image/png" | "image/jpeg" | "image/webp" if self.compress_images => {
                self.optimize_image(&content, &entry.mime_type)?
            }
            "image/svg+xml" if self.optimize_svg => self.optimize_svg(&content)?,
            _ => content,
        };

        let hash = compute_hash(&optimized);
        let savings = if original_size > optimized.len() {
            original_size - optimized.len()
        } else {
            0
        };

        Ok(OptimizedAsset {
            content: optimized,
            hash,
            original_size,
            optimized_size: original_size.saturating_sub(savings),
            savings,
        })
    }

    /// Optimize an image.
    fn optimize_image(&self, _content: &[u8], _mime_type: &str) -> DxResult<Vec<u8>> {
        // Placeholder - would use image crate
        // For now, return original
        Ok(_content.to_vec())
    }

    /// Optimize SVG.
    fn optimize_svg(&self, content: &[u8]) -> DxResult<Vec<u8>> {
        // Basic SVG optimization - remove whitespace
        let svg = String::from_utf8_lossy(content);
        let optimized = svg.lines().map(|line| line.trim()).collect::<Vec<_>>().join("");
        Ok(optimized.into_bytes())
    }
}

/// An optimized asset.
#[derive(Debug)]
pub struct OptimizedAsset {
    /// Optimized content
    pub content: Vec<u8>,
    /// Content hash
    pub hash: String,
    /// Original size
    pub original_size: usize,
    /// Optimized size
    pub optimized_size: usize,
    /// Bytes saved
    pub savings: usize,
}

// =============================================================================
// Helpers
// =============================================================================

/// Compute content hash using Blake3.
fn compute_hash(content: &[u8]) -> String {
    let hash = blake3::hash(content);
    hash.to_hex().to_string()
}

/// Get MIME type from file extension.
fn mime_from_extension(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        // Images
        Some("png") => "image/png".to_string(),
        Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
        Some("gif") => "image/gif".to_string(),
        Some("webp") => "image/webp".to_string(),
        Some("svg") => "image/svg+xml".to_string(),
        Some("ico") => "image/x-icon".to_string(),
        Some("avif") => "image/avif".to_string(),
        // Fonts
        Some("woff") => "font/woff".to_string(),
        Some("woff2") => "font/woff2".to_string(),
        Some("ttf") => "font/ttf".to_string(),
        Some("otf") => "font/otf".to_string(),
        Some("eot") => "application/vnd.ms-fontobject".to_string(),
        // Web
        Some("html") | Some("htm") => "text/html".to_string(),
        Some("css") => "text/css".to_string(),
        Some("js") => "application/javascript".to_string(),
        Some("json") => "application/json".to_string(),
        Some("xml") => "application/xml".to_string(),
        // Documents
        Some("pdf") => "application/pdf".to_string(),
        Some("txt") => "text/plain".to_string(),
        Some("md") => "text/markdown".to_string(),
        // Media
        Some("mp3") => "audio/mpeg".to_string(),
        Some("wav") => "audio/wav".to_string(),
        Some("ogg") => "audio/ogg".to_string(),
        Some("mp4") => "video/mp4".to_string(),
        Some("webm") => "video/webm".to_string(),
        // Archives
        Some("zip") => "application/zip".to_string(),
        Some("gz") | Some("gzip") => "application/gzip".to_string(),
        // Default
        _ => "application/octet-stream".to_string(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_server_new() {
        let server = AssetServer::new(PathBuf::from("public"));
        assert_eq!(server.root, PathBuf::from("public"));
        assert_eq!(server.public_path, "/");
    }

    #[test]
    fn test_asset_server_with_public_path() {
        let server = AssetServer::with_public_path(PathBuf::from("public"), "/assets/".to_string());
        assert_eq!(server.public_path, "/assets/");
    }

    #[test]
    fn test_mime_from_extension() {
        assert_eq!(mime_from_extension(Path::new("image.png")), "image/png");
        assert_eq!(mime_from_extension(Path::new("style.css")), "text/css");
        assert_eq!(mime_from_extension(Path::new("script.js")), "application/javascript");
        assert_eq!(mime_from_extension(Path::new("data.json")), "application/json");
        assert_eq!(mime_from_extension(Path::new("font.woff2")), "font/woff2");
        assert_eq!(mime_from_extension(Path::new("unknown")), "application/octet-stream");
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash(b"hello");
        let hash2 = compute_hash(b"hello");
        let hash3 = compute_hash(b"world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // Blake3 hex length
    }

    #[test]
    fn test_asset_optimizer_default() {
        let optimizer = AssetOptimizer::default();
        assert_eq!(optimizer.image_quality, 80);
        assert!(optimizer.compress_images);
        assert!(optimizer.optimize_svg);
    }
}
