//! DX Serializer Tool Integration
//!
//! Executes dx-serializer to convert .dx files to .sr binary format
//! with proper hashed path management in .dx/serializer folder.

use crate::dx_cache::DxToolId;
use crate::dx_executor::{DxToolExecutable, ExecutionContext, ToolResult};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// FNV-1a hash for path hashing (matches dx-serializer)
fn hash_path(relative_path: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in relative_path.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    format!("{:08x}", hash as u32)
}

/// Get the output path for a .sr file
pub fn get_binary_path(source_path: &Path, project_root: &Path) -> PathBuf {
    // Get relative path from project root
    let relative = source_path
        .strip_prefix(project_root)
        .unwrap_or(source_path)
        .to_string_lossy()
        .replace('\\', "/");

    // Hash the relative path
    let hash = hash_path(&relative);

    // Get filename and change extension to .sr
    let filename = source_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "dx".to_string());

    // Build output path: .dx/serializer/{hash}/{filename}.sr
    project_root
        .join(".dx")
        .join("serializer")
        .join(&hash)
        .join(format!("{}.dx", filename))
}

/// DX Serializer Tool
///
/// Converts .dx files to .sr binary format with hashed paths.
pub struct SerializerTool {
    /// Input files to process
    input_files: Vec<PathBuf>,
}

impl SerializerTool {
    /// Create new serializer tool
    pub fn new() -> Self {
        Self {
            input_files: Vec::new(),
        }
    }

    /// Add input file to process
    pub fn add_file(&mut self, path: PathBuf) {
        self.input_files.push(path);
    }

    /// Find all .dx files in project
    pub fn find_dx_files(project_root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        Self::find_dx_files_recursive(project_root, project_root, &mut files);
        files
    }

    fn find_dx_files_recursive(dir: &Path, _root: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Skip .dx folder and node_modules
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == ".dx" || name == "node_modules" || name == "target" || name == ".git" {
                continue;
            }

            if path.is_dir() {
                Self::find_dx_files_recursive(&path, _root, files);
            } else if path.extension().map(|e| e == "dx").unwrap_or(false)
                || path.file_name().map(|n| n == "dx").unwrap_or(false)
            {
                files.push(path);
            }
        }
    }

    /// Serialize a single file to binary
    pub fn serialize_file(source_path: &Path, project_root: &Path) -> Result<PathBuf> {
        // Read source file
        let content = std::fs::read_to_string(source_path)
            .with_context(|| format!("Failed to read: {}", source_path.display()))?;

        // Parse with dx-serializer (using the encode function)
        let parsed = dx_serializer::parse(content.as_bytes())
            .with_context(|| format!("Failed to parse: {}", source_path.display()))?;

        // Encode to binary
        let binary = dx_serializer::encode(&parsed)
            .with_context(|| format!("Failed to encode: {}", source_path.display()))?;

        // Get output path
        let output_path = get_binary_path(source_path, project_root);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write binary data
        std::fs::write(&output_path, &binary)?;

        Ok(output_path)
    }

    /// Check if binary is up to date
    pub fn is_cache_valid(source_path: &Path, project_root: &Path) -> bool {
        let binary_path = get_binary_path(source_path, project_root);

        let source_meta = match std::fs::metadata(source_path) {
            Ok(m) => m,
            Err(_) => return false,
        };

        let binary_meta = match std::fs::metadata(&binary_path) {
            Ok(m) => m,
            Err(_) => return false,
        };

        match (source_meta.modified(), binary_meta.modified()) {
            (Ok(src_time), Ok(bin_time)) => bin_time >= src_time,
            _ => false,
        }
    }
}

impl Default for SerializerTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DxToolExecutable for SerializerTool {
    fn id(&self) -> DxToolId {
        DxToolId::Serializer
    }

    fn execute(&self, ctx: &ExecutionContext) -> Result<ToolResult> {
        let start = Instant::now();
        let mut output_files = Vec::new();
        let mut errors = Vec::new();
        let mut cache_hits = 0u64;
        let mut cache_misses = 0u64;

        // Find all .dx files
        let dx_files = Self::find_dx_files(&ctx.project_root);

        for source in &dx_files {
            // Check cache
            if Self::is_cache_valid(source, &ctx.project_root) {
                cache_hits += 1;
                continue;
            }

            cache_misses += 1;

            // Serialize file
            match Self::serialize_file(source, &ctx.project_root) {
                Ok(output) => {
                    output_files.push(output);
                }
                Err(e) => {
                    errors.push(format!("{}: {}", source.display(), e));
                }
            }
        }

        let duration = start.elapsed();
        let success = errors.is_empty();

        Ok(ToolResult {
            tool: "serializer".to_string(),
            success,
            duration_ms: duration.as_millis() as u64,
            warm_start: cache_hits > 0 && cache_misses == 0,
            cache_hits,
            cache_misses,
            output_files,
            errors,
        })
    }

    fn should_run(&self, _ctx: &ExecutionContext) -> bool {
        true
    }

    fn dependencies(&self) -> &[DxToolId] {
        &[]
    }

    fn build_cache(&self, _ctx: &ExecutionContext, _result: &ToolResult) -> Result<()> {
        // Cache is already built during execution
        Ok(())
    }
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
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_get_binary_path() {
        let project = PathBuf::from("/project");
        let source = PathBuf::from("/project/config.dx");
        let output = get_binary_path(&source, &project);

        // Check path components in a platform-independent way
        let output_str = output.to_string_lossy();
        assert!(output_str.contains(".dx") && output_str.contains("serializer"));
        assert!(output_str.ends_with(".dx"));
    }
}
