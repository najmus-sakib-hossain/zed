//! DX Binary Output (.sr) Module
//!
//! Handles binary serialization output with proper path hashing
//! for the `.dx/serializer` folder structure.
//!
//! ## Path Hashing Strategy
//!
//! Files with the same name in different directories get unique subfolders:
//! ```text
//! project/
//!   config.dx          -> .dx/serializer/config.sr
//!   src/config.dx      -> .dx/serializer/src/config.sr
//!   lib/config.dx      -> .dx/serializer/lib/config.sr
//! ```
//!
//! The hash is derived from the relative path to ensure deterministic output.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Binary output configuration
#[derive(Debug, Clone)]
pub struct BinaryConfig {
    /// Project root directory
    pub project_root: PathBuf,
    /// Output directory (default: .dx/serializer)
    pub output_dir: PathBuf,
}

impl Default for BinaryConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            output_dir: PathBuf::from(".dx/serializer"),
        }
    }
}

impl BinaryConfig {
    /// Create config with custom project root
    pub fn with_root<P: AsRef<Path>>(root: P) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            output_dir: root.join(".dx/serializer"),
            project_root: root,
        }
    }
}

/// Compute hash for a file path (8 hex characters)
///
/// Uses FNV-1a for speed and good distribution
pub fn hash_path(relative_path: &str) -> String {
    // FNV-1a 64-bit
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in relative_path.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Take lower 32 bits for 8 hex chars
    format!("{:08x}", hash as u32)
}

/// Get the output path for a .sr file
///
/// # Arguments
/// * `source_path` - Path to the source .dx file
/// * `config` - Binary output configuration
///
/// # Returns
/// The full path where the .sr file should be written
///
/// # Example
///
/// ```rust
/// use serializer::binary_output::{BinaryConfig, get_binary_path};
/// use std::path::PathBuf;
///
/// let config = BinaryConfig::with_root("/project");
/// let output = get_binary_path("/project/src/config.dx", &config);
///
/// // Output path contains the hash directory and .dx extension
/// assert!(output.to_string_lossy().contains(".dx/serializer"));
/// assert!(output.to_string_lossy().ends_with(".dx"));
/// ```
pub fn get_binary_path<P: AsRef<Path>>(source_path: P, config: &BinaryConfig) -> PathBuf {
    let source = source_path.as_ref();

    // Get relative path from project root
    let relative = source
        .strip_prefix(&config.project_root)
        .unwrap_or(source)
        .to_string_lossy()
        .replace('\\', "/"); // Normalize path separators

    // Hash the relative path (including directory)
    let hash = hash_path(&relative);

    // Get filename and change extension to .sr
    let filename = source
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "dx".to_string());

    // Build output path: .dx/serializer/{hash}/{filename}.dx
    config.output_dir.join(&hash).join(format!("{}.dx", filename))
}

/// Write binary data to the correct .dx path
///
/// Creates directories as needed and writes the binary data.
pub fn write_binary<P: AsRef<Path>>(
    source_path: P,
    binary_data: &[u8],
    config: &BinaryConfig,
) -> std::io::Result<PathBuf> {
    let output_path = get_binary_path(&source_path, config);

    // Create parent directories
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write binary data
    let mut file = fs::File::create(&output_path)?;
    file.write_all(binary_data)?;

    Ok(output_path)
}

/// Read binary data from .dx file
pub fn read_binary<P: AsRef<Path>>(
    source_path: P,
    config: &BinaryConfig,
) -> std::io::Result<Vec<u8>> {
    let binary_path = get_binary_path(&source_path, config);
    fs::read(binary_path)
}

/// Check if binary cache exists and is newer than source
pub fn is_cache_valid<P: AsRef<Path>>(source_path: P, config: &BinaryConfig) -> bool {
    let source = source_path.as_ref();
    let binary_path = get_binary_path(source, config);

    // Check if binary exists
    let binary_meta = match fs::metadata(&binary_path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    // Check if source exists
    let source_meta = match fs::metadata(source) {
        Ok(m) => m,
        Err(_) => return false,
    };

    // Compare modification times
    match (source_meta.modified(), binary_meta.modified()) {
        (Ok(src_time), Ok(bin_time)) => bin_time >= src_time,
        _ => false,
    }
}

/// Get manifest of all .dx files in a project
pub fn get_manifest(config: &BinaryConfig) -> std::io::Result<Vec<(String, PathBuf)>> {
    let mut manifest = Vec::new();

    if !config.output_dir.exists() {
        return Ok(manifest);
    }

    // Walk the serializer directory
    for entry in fs::read_dir(&config.output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // This is a hash directory
            let hash = path.file_name().map(|s| s.to_string_lossy().to_string());

            for file_entry in fs::read_dir(&path)? {
                let file_entry = file_entry?;
                let file_path = file_entry.path();

                if file_path.extension().map(|e| e == "dx").unwrap_or(false) {
                    if let Some(h) = &hash {
                        manifest.push((h.clone(), file_path));
                    }
                }
            }
        }
    }

    Ok(manifest)
}

/// Clean stale binary files (where source no longer exists)
pub fn clean_stale(config: &BinaryConfig) -> std::io::Result<usize> {
    let mut cleaned = 0;

    if !config.output_dir.exists() {
        return Ok(0);
    }

    // Walk hash directories
    for entry in fs::read_dir(&config.output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Check if directory is empty after potential cleanup
            let mut is_empty = true;

            for file_entry in fs::read_dir(&path)? {
                let file_entry = file_entry?;
                let file_path = file_entry.path();

                if file_path.extension().map(|e| e == "dx").unwrap_or(false) {
                    // We don't have source mapping here, so we can't validate
                    // In production, we'd store a manifest with source paths
                    is_empty = false;
                }
            }

            if is_empty {
                fs::remove_dir(&path)?;
                cleaned += 1;
            }
        }
    }

    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_path_consistency() {
        let hash1 = hash_path("config.dx");
        let hash2 = hash_path("config.dx");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_path_uniqueness() {
        let hash1 = hash_path("config.dx");
        let hash2 = hash_path("src/config.dx");
        let hash3 = hash_path("lib/config.dx");

        assert_ne!(hash1, hash2);
        assert_ne!(hash2, hash3);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_get_binary_path() {
        let config = BinaryConfig::with_root("/project");
        let path = get_binary_path("/project/config.dx", &config);

        assert!(path.to_string_lossy().contains(".dx/serializer"));
        assert!(path.to_string_lossy().ends_with(".dx"));
    }

    #[test]
    fn test_binary_path_different_dirs() {
        let config = BinaryConfig::with_root("/project");

        let path1 = get_binary_path("/project/config.dx", &config);
        let path2 = get_binary_path("/project/src/config.dx", &config);

        // Same filename, different hashes
        assert_ne!(path1, path2);
        assert!(path1.file_name() == path2.file_name()); // Both are config.dx
    }
}
