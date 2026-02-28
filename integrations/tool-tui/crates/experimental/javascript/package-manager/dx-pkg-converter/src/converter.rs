//! Package Converter Implementation
//! Converts npm .tgz tarballs to DXP binary format
//! This is where we gain performance - convert once, use forever!

use anyhow::Result;
use flate2::read::GzDecoder;
use lz4_flex::compress_prepend_size;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;

use crate::format::{DxpFile, DxpFileEntry};

#[derive(Clone)]
pub struct PackageConverter {
    /// Minimum size to compress (bytes)
    compress_threshold: usize,
}

impl PackageConverter {
    pub fn new() -> Self {
        Self {
            compress_threshold: 1024, // 1KB
        }
    }

    /// Convert a .tgz file to .dxp
    pub async fn convert_file(&self, input: &Path, output: Option<&PathBuf>) -> Result<PathBuf> {
        // Read .tgz
        let tgz_data = std::fs::read(input)?;

        // Extract package name and version from path or tar contents
        let name = input.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        // Generate output path
        let output_path = output.cloned().unwrap_or_else(|| PathBuf::from(format!("{}.dxp", name)));

        // Convert
        self.convert_tgz(&tgz_data, &output_path).await?;

        Ok(output_path)
    }

    /// Convert package bytes to .dxp
    pub async fn convert_bytes(
        &self,
        name: &str,
        version: &str,
        tgz_data: &[u8],
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let output_path = output_dir.join(format!("{}@{}.dxp", name, version));
        std::fs::create_dir_all(output_dir)?;

        self.convert_tgz(tgz_data, &output_path).await?;

        Ok(output_path)
    }

    /// Core conversion logic: npm .tgz → DXP binary format
    async fn convert_tgz(&self, tgz_data: &[u8], output: &Path) -> Result<()> {
        // Decompress gzip
        let gz = GzDecoder::new(tgz_data);
        let mut archive = Archive::new(gz);

        // Extract all files and package.json
        let mut entries = Vec::new();
        let mut package_json: Option<serde_json::Value> = None;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();
            let path_str = path.to_string_lossy().to_string();

            // npm tarballs have "package/" prefix - strip it
            let clean_path = path_str.strip_prefix("package/").unwrap_or(&path_str).to_string();

            // Skip directories
            if entry.header().entry_type().is_dir() {
                continue;
            }

            // Read file contents
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents)?;

            // Capture package.json for metadata extraction
            if clean_path == "package.json"
                && let Ok(json) = serde_json::from_slice::<serde_json::Value>(&contents)
            {
                package_json = Some(json);
            }

            // Determine if compression is beneficial
            let (data, compressed_size) = if contents.len() > self.compress_threshold {
                let compressed = compress_prepend_size(&contents);
                // Only use compression if it saves >10%
                if compressed.len() < contents.len() * 9 / 10 {
                    (compressed.clone(), compressed.len() as u64)
                } else {
                    (contents.clone(), contents.len() as u64)
                }
            } else {
                (contents.clone(), contents.len() as u64)
            };

            // Calculate content hash
            let hash = blake3::hash(&contents);
            let hash_hex = format!("{}", hash.to_hex());

            entries.push(DxpFileEntry {
                path: clean_path,
                size: contents.len() as u64,
                compressed_size,
                hash: hash_hex,
                data,
            });
        }

        // Sort entries by path for faster binary search
        entries.sort_by(|a, b| a.path.cmp(&b.path));

        // Create .dxp file with metadata
        let dxp = DxpFile {
            version: 1,
            metadata: self.extract_metadata(&package_json),
            entries,
        };

        // Write to disk
        dxp.write(output)?;

        Ok(())
    }

    /// Extract metadata from package.json into binary format
    fn extract_metadata(
        &self,
        package_json: &Option<serde_json::Value>,
    ) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        if let Some(pkg) = package_json {
            // Extract key fields
            if let Some(name) = pkg.get("name").and_then(|v| v.as_str()) {
                metadata.insert("name".to_string(), name.to_string());
            }
            if let Some(version) = pkg.get("version").and_then(|v| v.as_str()) {
                metadata.insert("version".to_string(), version.to_string());
            }
            if let Some(main) = pkg.get("main").and_then(|v| v.as_str()) {
                metadata.insert("main".to_string(), main.to_string());
            }
            if let Some(module) = pkg.get("module").and_then(|v| v.as_str()) {
                metadata.insert("module".to_string(), module.to_string());
            }
            if let Some(description) = pkg.get("description").and_then(|v| v.as_str()) {
                metadata.insert("description".to_string(), description.to_string());
            }

            // Serialize dependencies
            if let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_object()) {
                let deps_json = serde_json::to_string(deps).unwrap_or_default();
                metadata.insert("dependencies".to_string(), deps_json);
            }
        }

        metadata
    }
}

impl Default for PackageConverter {
    fn default() -> Self {
        Self::new()
    }
}
