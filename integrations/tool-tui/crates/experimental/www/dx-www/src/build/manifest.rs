//! # Route Manifest Generator
//!
//! Generates the `manifest.json` file for the build output.
//! This manifest contains all routes, assets, and their metadata.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::build::BinaryObject;
use crate::project::Project;

/// Route manifest for the build output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteManifest {
    /// Manifest version
    pub version: u32,

    /// All routes in the application
    pub routes: Vec<ManifestRoute>,

    /// All layouts
    pub layouts: Vec<ManifestLayout>,

    /// All assets
    pub assets: Vec<ManifestAsset>,

    /// Build timestamp
    pub build_time: u64,

    /// Content hash for cache invalidation
    pub hash: String,
}

/// A route in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestRoute {
    /// Route path (e.g., "/", "/about", "/user/:id")
    pub path: String,

    /// Path to the compiled binary object
    pub binary: PathBuf,

    /// Content hash
    pub hash: String,

    /// Component type (page, layout, error)
    pub component_type: String,

    /// Layout chain (list of layout paths)
    pub layouts: Vec<String>,

    /// Whether this route has a data loader
    pub has_data_loader: bool,

    /// Whether this is a dynamic route
    pub is_dynamic: bool,

    /// Dynamic parameter names
    pub params: Vec<String>,

    /// Whether to pre-render at build time
    pub prerender: bool,

    /// Dependencies (other routes this depends on)
    pub dependencies: Vec<String>,
}

/// A layout in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestLayout {
    /// Layout path (directory path)
    pub path: String,

    /// Path to the compiled binary object
    pub binary: PathBuf,

    /// Content hash
    pub hash: String,

    /// Parent layout (if nested)
    pub parent: Option<String>,
}

/// An asset in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestAsset {
    /// Original path
    pub path: PathBuf,

    /// Output path (with content hash)
    pub output: PathBuf,

    /// MIME type
    pub mime_type: String,

    /// File size in bytes
    pub size: u64,

    /// Content hash
    pub hash: String,
}

impl RouteManifest {
    /// Create a new empty manifest.
    pub fn new() -> Self {
        Self {
            version: 1,
            routes: Vec::new(),
            layouts: Vec::new(),
            assets: Vec::new(),
            build_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            hash: String::new(),
        }
    }

    /// Generate manifest from project and binary objects.
    pub fn generate(project: &Project, binary_objects: &[BinaryObject]) -> Self {
        let mut manifest = Self::new();

        // Build a map of binary objects by path
        let binary_map: HashMap<&Path, &BinaryObject> =
            binary_objects.iter().map(|b| (b.path.as_path(), b)).collect();

        // Process pages
        for page in &project.pages {
            // Find corresponding binary object
            let binary = binary_map.iter().find(|(_, b)| {
                b.path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| {
                        page.path
                            .file_stem()
                            .and_then(|ps| ps.to_str())
                            .map(|ps| s.starts_with(ps))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            });

            let (binary_path, binary_hash) = if let Some((_, obj)) = binary {
                (obj.path.clone(), obj.hash.clone())
            } else {
                continue;
            };

            manifest.routes.push(ManifestRoute {
                path: page.route_path.clone(),
                binary: binary_path,
                hash: binary_hash,
                component_type: "page".to_string(),
                layouts: find_layout_chain(&page.path, project),
                has_data_loader: false, // Determined during parsing
                is_dynamic: page.is_dynamic,
                params: page.params.clone(),
                prerender: !page.is_dynamic,
                dependencies: vec![],
            });
        }

        // Process layouts
        for layout in &project.layouts {
            let rel_path = layout
                .path
                .strip_prefix(&project.root)
                .unwrap_or(&layout.path)
                .to_string_lossy()
                .to_string();

            let binary = binary_map.iter().find(|(_, b)| {
                b.path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.contains("_layout"))
                    .unwrap_or(false)
                    && b.path
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .map(|dir| rel_path.contains(dir))
                        .unwrap_or(false)
            });

            let (binary_path, binary_hash) = if let Some((_, obj)) = binary {
                (obj.path.clone(), obj.hash.clone())
            } else {
                continue;
            };

            let parent = find_parent_layout(&layout.path, project);

            manifest.layouts.push(ManifestLayout {
                path: rel_path,
                binary: binary_path,
                hash: binary_hash,
                parent,
            });
        }

        // Process assets
        for asset in &project.assets {
            let rel_path = asset.path.strip_prefix(&project.root).unwrap_or(&asset.path);

            let hash = compute_hash(rel_path.to_string_lossy().as_ref());
            let ext = asset.path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let mime_type = mime_from_extension(ext);

            let size = std::fs::metadata(&asset.path).map(|m| m.len()).unwrap_or(0);

            // Generate output path with hash
            let stem = asset.path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
            let output_name = format!("{}.{}.{}", stem, &hash[..8], ext);
            let output_path = PathBuf::from("_dx/assets").join(&output_name);

            manifest.assets.push(ManifestAsset {
                path: rel_path.to_path_buf(),
                output: output_path,
                mime_type,
                size,
                hash,
            });
        }

        // Compute overall manifest hash
        manifest.hash = compute_manifest_hash(&manifest);

        manifest
    }

    /// Write the manifest to a JSON file.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

impl Default for RouteManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Find the layout chain for a page.
fn find_layout_chain(page_path: &Path, project: &Project) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = page_path.parent();

    while let Some(dir) = current {
        // Check if there's a layout in this directory
        let has_layout = project.layouts.iter().any(|l| l.path.parent() == Some(dir));

        if has_layout {
            let rel = dir.strip_prefix(&project.root).unwrap_or(dir).to_string_lossy().to_string();
            chain.push(rel);
        }

        // Stop at project root
        if dir == project.root {
            break;
        }

        current = dir.parent();
    }

    // Reverse to get root-first order
    chain.reverse();
    chain
}

/// Find the parent layout for a layout file.
fn find_parent_layout(layout_path: &Path, project: &Project) -> Option<String> {
    let mut current = layout_path.parent()?.parent();

    while let Some(dir) = current {
        // Check if there's a layout in this directory
        if let Some(parent) = project.layouts.iter().find(|l| l.path.parent() == Some(dir)) {
            return Some(
                parent
                    .path
                    .strip_prefix(&project.root)
                    .unwrap_or(&parent.path)
                    .to_string_lossy()
                    .to_string(),
            );
        }

        // Stop at project root
        if dir == project.root {
            break;
        }

        current = dir.parent();
    }

    None
}

/// Compute a hash for content addressing.
fn compute_hash(input: &str) -> String {
    use blake3::Hasher;

    let mut hasher = Hasher::new();
    hasher.update(input.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Compute overall manifest hash.
fn compute_manifest_hash(manifest: &RouteManifest) -> String {
    use blake3::Hasher;

    let mut hasher = Hasher::new();

    // Hash routes
    for route in &manifest.routes {
        hasher.update(route.path.as_bytes());
        hasher.update(route.hash.as_bytes());
    }

    // Hash layouts
    for layout in &manifest.layouts {
        hasher.update(layout.path.as_bytes());
        hasher.update(layout.hash.as_bytes());
    }

    // Hash assets
    for asset in &manifest.assets {
        hasher.update(asset.hash.as_bytes());
    }

    hasher.finalize().to_hex().to_string()
}

/// Get MIME type from file extension.
fn mime_from_extension(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html".to_string(),
        "css" => "text/css".to_string(),
        "js" | "mjs" => "application/javascript".to_string(),
        "json" => "application/json".to_string(),
        "png" => "image/png".to_string(),
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "gif" => "image/gif".to_string(),
        "svg" => "image/svg+xml".to_string(),
        "webp" => "image/webp".to_string(),
        "ico" => "image/x-icon".to_string(),
        "woff" => "font/woff".to_string(),
        "woff2" => "font/woff2".to_string(),
        "ttf" => "font/ttf".to_string(),
        "eot" => "application/vnd.ms-fontobject".to_string(),
        "mp4" => "video/mp4".to_string(),
        "webm" => "video/webm".to_string(),
        "mp3" => "audio/mpeg".to_string(),
        "wav" => "audio/wav".to_string(),
        "pdf" => "application/pdf".to_string(),
        "xml" => "application/xml".to_string(),
        "txt" => "text/plain".to_string(),
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
    fn test_new_manifest() {
        let manifest = RouteManifest::new();
        assert_eq!(manifest.version, 1);
        assert!(manifest.routes.is_empty());
        assert!(manifest.layouts.is_empty());
        assert!(manifest.assets.is_empty());
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash("test");
        let hash2 = compute_hash("test");
        let hash3 = compute_hash("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_mime_from_extension() {
        assert_eq!(mime_from_extension("html"), "text/html");
        assert_eq!(mime_from_extension("css"), "text/css");
        assert_eq!(mime_from_extension("js"), "application/javascript");
        assert_eq!(mime_from_extension("png"), "image/png");
        assert_eq!(mime_from_extension("unknown"), "application/octet-stream");
    }
}
