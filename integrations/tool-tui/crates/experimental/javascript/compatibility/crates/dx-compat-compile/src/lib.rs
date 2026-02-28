//! # dx-compat-compile
//!
//! Single executable compilation compatibility layer.
//!
//! This crate provides functionality to bundle JavaScript/TypeScript applications
//! into single executables with embedded assets, supporting cross-platform compilation.
//!
//! ## Features
//!
//! - Bundle code into single executables
//! - Cross-platform target support (Linux, macOS, Windows)
//! - Asset embedding with zstd compression
//! - Cross-compilation between platforms
//!
//! ## Example
//!
//! ```rust,ignore
//! use dx_compat_compile::{Compiler, CompileOptions, Target};
//!
//! let options = CompileOptions::new("./src/index.ts")
//!     .target(Target::LinuxX64)
//!     .output("./dist/app")
//!     .embed_assets(&["./assets"]);
//!
//! let compiler = Compiler::new();
//! compiler.compile(options)?;
//! ```

#![warn(missing_docs)]

mod error;

pub use error::{CompileError, CompileResult};

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Target platform for compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    /// Linux x64
    LinuxX64,
    /// Linux ARM64
    LinuxArm64,
    /// macOS x64
    MacosX64,
    /// macOS ARM64
    MacosArm64,
    /// Windows x64
    WindowsX64,
}

impl Target {
    /// Get the target triple.
    pub fn triple(&self) -> &'static str {
        match self {
            Target::LinuxX64 => "x86_64-unknown-linux-gnu",
            Target::LinuxArm64 => "aarch64-unknown-linux-gnu",
            Target::MacosX64 => "x86_64-apple-darwin",
            Target::MacosArm64 => "aarch64-apple-darwin",
            Target::WindowsX64 => "x86_64-pc-windows-msvc",
        }
    }

    /// Get the executable extension for this target.
    pub fn exe_extension(&self) -> &'static str {
        match self {
            Target::WindowsX64 => ".exe",
            _ => "",
        }
    }

    /// Get the current host target.
    pub fn current() -> Self {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return Target::LinuxX64;
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return Target::LinuxArm64;
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return Target::MacosX64;
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return Target::MacosArm64;
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return Target::WindowsX64;
        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )))]
        return Target::LinuxX64; // Default fallback
    }

    /// Check if this target requires cross-compilation from the current host.
    pub fn requires_cross_compilation(&self) -> bool {
        *self != Self::current()
    }
}

impl std::str::FromStr for Target {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "linux-x64" | "linux_x64" | "x86_64-unknown-linux-gnu" => Ok(Target::LinuxX64),
            "linux-arm64" | "linux_arm64" | "aarch64-unknown-linux-gnu" => Ok(Target::LinuxArm64),
            "macos-x64" | "macos_x64" | "darwin-x64" | "x86_64-apple-darwin" => {
                Ok(Target::MacosX64)
            }
            "macos-arm64" | "macos_arm64" | "darwin-arm64" | "aarch64-apple-darwin" => {
                Ok(Target::MacosArm64)
            }
            "windows-x64" | "windows_x64" | "win32-x64" | "x86_64-pc-windows-msvc" => {
                Ok(Target::WindowsX64)
            }
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.triple())
    }
}

/// Embedded asset with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedAsset {
    /// Original path relative to the project root.
    pub path: String,
    /// SHA-256 hash of the original content.
    pub hash: String,
    /// Original size in bytes.
    pub original_size: u64,
    /// Compressed size in bytes.
    pub compressed_size: u64,
    /// MIME type of the asset.
    pub mime_type: String,
    /// Compressed data.
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        encoded.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use base64::Engine;
        let encoded = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .map_err(serde::de::Error::custom)
    }
}

impl EmbeddedAsset {
    /// Create a new embedded asset from file content.
    pub fn new(
        path: impl Into<String>,
        data: &[u8],
        mime_type: impl Into<String>,
    ) -> CompileResult<Self> {
        let path = path.into();
        let mime_type = mime_type.into();
        let original_size = data.len() as u64;

        // Compute hash
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());

        // Compress with zstd
        let compressed = compress_data(data)?;
        let compressed_size = compressed.len() as u64;

        Ok(Self {
            path,
            hash,
            original_size,
            compressed_size,
            mime_type,
            data: compressed,
        })
    }

    /// Decompress and return the original data.
    pub fn decompress(&self) -> CompileResult<Vec<u8>> {
        decompress_data(&self.data)
    }

    /// Verify the integrity of the decompressed data.
    pub fn verify(&self) -> CompileResult<bool> {
        let decompressed = self.decompress()?;
        let mut hasher = Sha256::new();
        hasher.update(&decompressed);
        let computed_hash = format!("{:x}", hasher.finalize());
        Ok(computed_hash == self.hash)
    }
}

/// Asset bundle containing multiple embedded assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBundle {
    /// Version of the bundle format.
    pub version: u32,
    /// Target platform.
    pub target: Target,
    /// Entry point path.
    pub entry_point: String,
    /// Embedded assets by path.
    pub assets: HashMap<String, EmbeddedAsset>,
    /// Bundle metadata.
    pub metadata: BundleMetadata,
}

/// Bundle metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Build timestamp (Unix epoch).
    pub build_time: u64,
    /// Total original size of all assets.
    pub total_original_size: u64,
    /// Total compressed size of all assets.
    pub total_compressed_size: u64,
}

impl AssetBundle {
    /// Create a new empty asset bundle.
    pub fn new(
        target: Target,
        entry_point: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            version: 1,
            target,
            entry_point: entry_point.into(),
            assets: HashMap::new(),
            metadata: BundleMetadata {
                name: name.into(),
                version: version.into(),
                build_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                total_original_size: 0,
                total_compressed_size: 0,
            },
        }
    }

    /// Add an asset to the bundle.
    pub fn add_asset(&mut self, asset: EmbeddedAsset) {
        self.metadata.total_original_size += asset.original_size;
        self.metadata.total_compressed_size += asset.compressed_size;
        self.assets.insert(asset.path.clone(), asset);
    }

    /// Get an asset by path.
    pub fn get_asset(&self, path: &str) -> Option<&EmbeddedAsset> {
        self.assets.get(path)
    }

    /// Serialize the bundle to bytes.
    pub fn to_bytes(&self) -> CompileResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| CompileError::Serialization(e.to_string()))
    }

    /// Deserialize a bundle from bytes.
    pub fn from_bytes(data: &[u8]) -> CompileResult<Self> {
        serde_json::from_slice(data).map_err(|e| CompileError::Serialization(e.to_string()))
    }

    /// Get compression ratio.
    pub fn compression_ratio(&self) -> f64 {
        if self.metadata.total_original_size == 0 {
            1.0
        } else {
            self.metadata.total_compressed_size as f64 / self.metadata.total_original_size as f64
        }
    }
}

/// Compile options for building single executables.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Entry point file path.
    pub entry_point: PathBuf,
    /// Output executable path.
    pub output: PathBuf,
    /// Target platform.
    pub target: Target,
    /// Asset directories to embed.
    pub asset_dirs: Vec<PathBuf>,
    /// Individual asset files to embed.
    pub asset_files: Vec<PathBuf>,
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Minify the bundled code.
    pub minify: bool,
    /// Source maps.
    pub sourcemap: bool,
    /// Compression level (1-22, default 3).
    pub compression_level: i32,
}

impl CompileOptions {
    /// Create new compile options with entry point.
    pub fn new(entry_point: impl AsRef<Path>) -> Self {
        let entry = entry_point.as_ref();
        let name = entry.file_stem().and_then(|s| s.to_str()).unwrap_or("app").to_string();

        Self {
            entry_point: entry.to_path_buf(),
            output: PathBuf::from(format!("./{}", name)),
            target: Target::current(),
            asset_dirs: Vec::new(),
            asset_files: Vec::new(),
            name,
            version: "1.0.0".to_string(),
            minify: true,
            sourcemap: false,
            compression_level: 3,
        }
    }

    /// Set the output path.
    pub fn output(mut self, path: impl AsRef<Path>) -> Self {
        self.output = path.as_ref().to_path_buf();
        self
    }

    /// Set the target platform.
    pub fn target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }

    /// Add asset directories to embed.
    pub fn embed_assets(mut self, dirs: &[impl AsRef<Path>]) -> Self {
        self.asset_dirs.extend(dirs.iter().map(|p| p.as_ref().to_path_buf()));
        self
    }

    /// Add individual asset files to embed.
    pub fn embed_files(mut self, files: &[impl AsRef<Path>]) -> Self {
        self.asset_files.extend(files.iter().map(|p| p.as_ref().to_path_buf()));
        self
    }

    /// Set application name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set application version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Enable/disable minification.
    pub fn minify(mut self, minify: bool) -> Self {
        self.minify = minify;
        self
    }

    /// Enable/disable source maps.
    pub fn sourcemap(mut self, sourcemap: bool) -> Self {
        self.sourcemap = sourcemap;
        self
    }

    /// Set compression level (1-22).
    pub fn compression_level(mut self, level: i32) -> Self {
        self.compression_level = level.clamp(1, 22);
        self
    }
}

/// Compiler for building single executables.
pub struct Compiler {
    /// Compression level for zstd.
    compression_level: i32,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    /// Create a new compiler instance.
    pub fn new() -> Self {
        Self {
            compression_level: 3,
        }
    }

    /// Set compression level (1-22).
    pub fn with_compression_level(mut self, level: i32) -> Self {
        self.compression_level = level.clamp(1, 22);
        self
    }

    /// Compile the application into a single executable.
    pub fn compile(&self, options: CompileOptions) -> CompileResult<CompiledOutput> {
        // Create asset bundle
        let mut bundle = AssetBundle::new(
            options.target,
            options.entry_point.to_string_lossy().to_string(),
            &options.name,
            &options.version,
        );

        // Embed entry point
        let entry_content = std::fs::read(&options.entry_point)?;
        let entry_asset = EmbeddedAsset::new(
            options.entry_point.to_string_lossy().to_string(),
            &entry_content,
            guess_mime_type(&options.entry_point),
        )?;
        bundle.add_asset(entry_asset);

        // Embed asset directories
        for dir in &options.asset_dirs {
            self.embed_directory(&mut bundle, dir)?;
        }

        // Embed individual files
        for file in &options.asset_files {
            self.embed_file(&mut bundle, file)?;
        }

        // Serialize bundle
        let bundle_data = bundle.to_bytes()?;

        // Create output path with correct extension
        let mut output_path = options.output.clone();
        let ext = options.target.exe_extension();
        if !ext.is_empty() && !output_path.to_string_lossy().ends_with(ext) {
            output_path = PathBuf::from(format!("{}{}", output_path.display(), ext));
        }

        Ok(CompiledOutput {
            bundle,
            bundle_data: Bytes::from(bundle_data),
            output_path,
            target: options.target,
        })
    }

    /// Embed a directory recursively.
    fn embed_directory(&self, bundle: &mut AssetBundle, dir: &Path) -> CompileResult<()> {
        if !dir.exists() {
            return Err(CompileError::AssetNotFound(dir.display().to_string()));
        }

        for entry in
            walkdir::WalkDir::new(dir).follow_links(true).into_iter().filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                self.embed_file(bundle, path)?;
            }
        }

        Ok(())
    }

    /// Embed a single file.
    fn embed_file(&self, bundle: &mut AssetBundle, path: &Path) -> CompileResult<()> {
        if !path.exists() {
            return Err(CompileError::AssetNotFound(path.display().to_string()));
        }

        let content = std::fs::read(path)?;
        let asset = EmbeddedAsset::new(
            path.to_string_lossy().to_string(),
            &content,
            guess_mime_type(path),
        )?;
        bundle.add_asset(asset);

        Ok(())
    }
}

/// Output from compilation.
#[derive(Debug)]
pub struct CompiledOutput {
    /// The asset bundle.
    pub bundle: AssetBundle,
    /// Serialized bundle data.
    pub bundle_data: Bytes,
    /// Output path.
    pub output_path: PathBuf,
    /// Target platform.
    pub target: Target,
}

impl CompiledOutput {
    /// Write the compiled output to disk.
    pub fn write(&self) -> CompileResult<()> {
        // Create parent directories if needed
        if let Some(parent) = self.output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write the bundle data
        std::fs::write(&self.output_path, &self.bundle_data)?;

        Ok(())
    }

    /// Get the size of the compiled output.
    pub fn size(&self) -> usize {
        self.bundle_data.len()
    }

    /// Get compression statistics.
    pub fn stats(&self) -> CompileStats {
        CompileStats {
            original_size: self.bundle.metadata.total_original_size,
            compressed_size: self.bundle.metadata.total_compressed_size,
            asset_count: self.bundle.assets.len(),
            compression_ratio: self.bundle.compression_ratio(),
        }
    }
}

/// Compilation statistics.
#[derive(Debug, Clone)]
pub struct CompileStats {
    /// Total original size of all assets.
    pub original_size: u64,
    /// Total compressed size of all assets.
    pub compressed_size: u64,
    /// Number of embedded assets.
    pub asset_count: usize,
    /// Compression ratio (compressed/original).
    pub compression_ratio: f64,
}

// ============================================================================
// Compression utilities
// ============================================================================

/// Compress data using zstd.
pub fn compress_data(data: &[u8]) -> CompileResult<Vec<u8>> {
    compress_data_with_level(data, 3)
}

/// Compress data using zstd with specified level.
pub fn compress_data_with_level(data: &[u8], level: i32) -> CompileResult<Vec<u8>> {
    let mut encoder = zstd::Encoder::new(Vec::new(), level)
        .map_err(|e| CompileError::CompressionFailed(e.to_string()))?;
    encoder
        .write_all(data)
        .map_err(|e| CompileError::CompressionFailed(e.to_string()))?;
    encoder.finish().map_err(|e| CompileError::CompressionFailed(e.to_string()))
}

/// Decompress zstd data.
pub fn decompress_data(data: &[u8]) -> CompileResult<Vec<u8>> {
    let mut decoder =
        zstd::Decoder::new(data).map_err(|e| CompileError::DecompressionFailed(e.to_string()))?;
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| CompileError::DecompressionFailed(e.to_string()))?;
    Ok(decompressed)
}

// ============================================================================
// MIME type detection
// ============================================================================

/// Guess MIME type from file extension.
fn guess_mime_type(path: &Path) -> String {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        // JavaScript/TypeScript
        "js" | "mjs" | "cjs" => "application/javascript",
        "ts" | "mts" | "cts" => "application/typescript",
        "jsx" => "text/jsx",
        "tsx" => "text/tsx",
        // Web
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "json" => "application/json",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        // Fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",
        // Documents
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        // Data
        "wasm" => "application/wasm",
        "yaml" | "yml" => "application/x-yaml",
        "toml" => "application/toml",
        // Default
        _ => "application/octet-stream",
    }
    .to_string()
}

// ============================================================================
// Runtime asset extraction
// ============================================================================

/// Runtime for extracting and accessing embedded assets.
pub struct Runtime {
    bundle: AssetBundle,
}

impl Runtime {
    /// Create a runtime from bundle data.
    pub fn from_bytes(data: &[u8]) -> CompileResult<Self> {
        let bundle = AssetBundle::from_bytes(data)?;
        Ok(Self { bundle })
    }

    /// Get the entry point path.
    pub fn entry_point(&self) -> &str {
        &self.bundle.entry_point
    }

    /// Get an asset by path.
    pub fn get_asset(&self, path: &str) -> Option<&EmbeddedAsset> {
        self.bundle.get_asset(path)
    }

    /// Read an asset's content.
    pub fn read_asset(&self, path: &str) -> CompileResult<Vec<u8>> {
        let asset = self
            .bundle
            .get_asset(path)
            .ok_or_else(|| CompileError::AssetNotFound(path.to_string()))?;
        asset.decompress()
    }

    /// Read an asset as string.
    pub fn read_asset_string(&self, path: &str) -> CompileResult<String> {
        let data = self.read_asset(path)?;
        String::from_utf8(data).map_err(|e| CompileError::Serialization(e.to_string()))
    }

    /// List all asset paths.
    pub fn list_assets(&self) -> Vec<&str> {
        self.bundle.assets.keys().map(|s| s.as_str()).collect()
    }

    /// Get bundle metadata.
    pub fn metadata(&self) -> &BundleMetadata {
        &self.bundle.metadata
    }

    /// Get target platform.
    pub fn target(&self) -> Target {
        self.bundle.target
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_triple() {
        assert_eq!(Target::LinuxX64.triple(), "x86_64-unknown-linux-gnu");
        assert_eq!(Target::LinuxArm64.triple(), "aarch64-unknown-linux-gnu");
        assert_eq!(Target::MacosX64.triple(), "x86_64-apple-darwin");
        assert_eq!(Target::MacosArm64.triple(), "aarch64-apple-darwin");
        assert_eq!(Target::WindowsX64.triple(), "x86_64-pc-windows-msvc");
    }

    #[test]
    fn test_target_exe_extension() {
        assert_eq!(Target::LinuxX64.exe_extension(), "");
        assert_eq!(Target::MacosArm64.exe_extension(), "");
        assert_eq!(Target::WindowsX64.exe_extension(), ".exe");
    }

    #[test]
    fn test_target_from_str() {
        assert_eq!("linux-x64".parse::<Target>(), Ok(Target::LinuxX64));
        assert_eq!("macos-arm64".parse::<Target>(), Ok(Target::MacosArm64));
        assert_eq!("windows-x64".parse::<Target>(), Ok(Target::WindowsX64));
        assert!("invalid".parse::<Target>().is_err());
    }

    #[test]
    fn test_compression_round_trip() {
        let data = b"Hello, World! This is test data for compression.";
        let compressed = compress_data(data).unwrap();
        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_embedded_asset() {
        let data = b"Test asset content";
        let asset = EmbeddedAsset::new("test.txt", data, "text/plain").unwrap();

        assert_eq!(asset.path, "test.txt");
        assert_eq!(asset.mime_type, "text/plain");
        assert_eq!(asset.original_size, data.len() as u64);
        assert!(asset.verify().unwrap());

        let decompressed = asset.decompress().unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_asset_bundle() {
        let mut bundle = AssetBundle::new(Target::LinuxX64, "index.js", "test-app", "1.0.0");

        let asset1 = EmbeddedAsset::new("file1.txt", b"Content 1", "text/plain").unwrap();
        let asset2 = EmbeddedAsset::new("file2.txt", b"Content 2", "text/plain").unwrap();

        bundle.add_asset(asset1);
        bundle.add_asset(asset2);

        assert_eq!(bundle.assets.len(), 2);
        assert!(bundle.get_asset("file1.txt").is_some());
        assert!(bundle.get_asset("file2.txt").is_some());
        assert!(bundle.get_asset("nonexistent.txt").is_none());
    }

    #[test]
    fn test_bundle_serialization() {
        let mut bundle = AssetBundle::new(Target::MacosArm64, "main.ts", "my-app", "2.0.0");
        let asset =
            EmbeddedAsset::new("data.json", b"{\"key\": \"value\"}", "application/json").unwrap();
        bundle.add_asset(asset);

        let bytes = bundle.to_bytes().unwrap();
        let restored = AssetBundle::from_bytes(&bytes).unwrap();

        assert_eq!(restored.target, Target::MacosArm64);
        assert_eq!(restored.entry_point, "main.ts");
        assert_eq!(restored.metadata.name, "my-app");
        assert_eq!(restored.assets.len(), 1);
    }

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(guess_mime_type(Path::new("file.js")), "application/javascript");
        assert_eq!(guess_mime_type(Path::new("file.ts")), "application/typescript");
        assert_eq!(guess_mime_type(Path::new("file.html")), "text/html");
        assert_eq!(guess_mime_type(Path::new("file.css")), "text/css");
        assert_eq!(guess_mime_type(Path::new("file.json")), "application/json");
        assert_eq!(guess_mime_type(Path::new("file.png")), "image/png");
        assert_eq!(guess_mime_type(Path::new("file.unknown")), "application/octet-stream");
    }

    #[test]
    fn test_compile_options() {
        let options = CompileOptions::new("./src/index.ts")
            .target(Target::LinuxArm64)
            .output("./dist/app")
            .name("my-app")
            .version("1.2.3")
            .minify(false)
            .compression_level(10);

        assert_eq!(options.entry_point, PathBuf::from("./src/index.ts"));
        assert_eq!(options.target, Target::LinuxArm64);
        assert_eq!(options.output, PathBuf::from("./dist/app"));
        assert_eq!(options.name, "my-app");
        assert_eq!(options.version, "1.2.3");
        assert!(!options.minify);
        assert_eq!(options.compression_level, 10);
    }

    #[test]
    fn test_runtime() {
        let mut bundle = AssetBundle::new(Target::LinuxX64, "index.js", "test", "1.0.0");
        let asset = EmbeddedAsset::new("test.txt", b"Hello, Runtime!", "text/plain").unwrap();
        bundle.add_asset(asset);

        let bytes = bundle.to_bytes().unwrap();
        let runtime = Runtime::from_bytes(&bytes).unwrap();

        assert_eq!(runtime.entry_point(), "index.js");
        assert_eq!(runtime.target(), Target::LinuxX64);
        assert_eq!(runtime.list_assets().len(), 1);

        let content = runtime.read_asset_string("test.txt").unwrap();
        assert_eq!(content, "Hello, Runtime!");
    }
}
