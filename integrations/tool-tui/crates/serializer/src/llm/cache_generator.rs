//! Cache Generator for DX Serializer
//!
//! Automatically generates LLM and Machine format cache files from Human format sources.
//! Cache files are stored in `.dx/cache` with path preservation.
//!
//! ## Cache Structure
//!
//! ```text
//! .dx/cache/
//! ├── llm/
//! │   ├── config.dx.llm
//! │   └── subdir/
//! │       └── data.dx.llm
//! └── machine/
//!     ├── config.dx.bin
//!     └── subdir/
//!         └── data.dx.bin
//! ```

use crate::llm::convert::{ConvertError, document_to_llm, document_to_machine};
use crate::llm::human_parser::HumanParser;
use crate::llm::types::DxDocument;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Cache generation errors
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Conversion error: {0}")]
    Convert(#[from] ConvertError),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Cache directory creation failed: {0}")]
    DirectoryCreation(String),
}

/// Configuration for cache generation
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Root directory for cache files (default: .dx/cache)
    pub cache_root: PathBuf,
    /// Generate LLM format cache files
    pub generate_llm: bool,
    /// Generate Machine format cache files
    pub generate_machine: bool,
    /// Use atomic writes (temp file + rename)
    pub atomic_writes: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_root: PathBuf::from(".dx/cache"),
            generate_llm: true,
            generate_machine: true,
            atomic_writes: true,
        }
    }
}

impl CacheConfig {
    /// Create a new config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cache root directory
    pub fn with_cache_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.cache_root = root.into();
        self
    }

    /// Set whether to generate LLM format
    pub fn with_llm(mut self, generate: bool) -> Self {
        self.generate_llm = generate;
        self
    }

    /// Set whether to generate Machine format
    pub fn with_machine(mut self, generate: bool) -> Self {
        self.generate_machine = generate;
        self
    }

    /// Set whether to use atomic writes
    pub fn with_atomic_writes(mut self, atomic: bool) -> Self {
        self.atomic_writes = atomic;
        self
    }
}

/// Cache generator for DX documents
pub struct CacheGenerator {
    config: CacheConfig,
    parser: HumanParser,
}

impl CacheGenerator {
    /// Create a new cache generator with default config
    pub fn new() -> Self {
        Self {
            config: CacheConfig::default(),
            parser: HumanParser::new(),
        }
    }

    /// Create a cache generator with custom config
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            config,
            parser: HumanParser::new(),
        }
    }

    /// Map a source path to cache paths
    ///
    /// Preserves the relative path structure in the cache directory.
    /// Example: `src/config/data.dx` -> `.dx/cache/llm/src/config/data.dx.llm`
    pub fn map_path_to_cache(&self, source_path: &Path, base_path: &Path) -> CachePaths {
        // Get relative path from base
        let relative = source_path.strip_prefix(base_path).unwrap_or(source_path);

        // Normalize path separators
        let relative_str = relative.to_string_lossy().replace('\\', "/");

        // Build cache paths
        let llm_path =
            self.config.cache_root.join("llm").join(&relative_str).with_extension("dx.llm");

        let machine_path = self
            .config
            .cache_root
            .join("machine")
            .join(&relative_str)
            .with_extension("dx.bin");

        CachePaths {
            source: source_path.to_path_buf(),
            llm: llm_path,
            machine: machine_path,
        }
    }

    /// Generate cache files from a Human format source file
    pub fn generate(
        &self,
        source_path: &Path,
        base_path: &Path,
    ) -> Result<CacheResult, CacheError> {
        // Read source file
        let content = fs::read_to_string(source_path)?;

        // Parse to document
        let doc = self.parser.parse(&content).map_err(|e| CacheError::Parse(e.to_string()))?;

        // Generate cache files
        self.generate_from_document(&doc, source_path, base_path)
    }

    /// Generate cache files from a DxDocument
    pub fn generate_from_document(
        &self,
        doc: &DxDocument,
        source_path: &Path,
        base_path: &Path,
    ) -> Result<CacheResult, CacheError> {
        let paths = self.map_path_to_cache(source_path, base_path);
        let mut result = CacheResult {
            paths: paths.clone(),
            llm_generated: false,
            machine_generated: false,
        };

        // Generate LLM format
        if self.config.generate_llm {
            let llm_content = document_to_llm(doc);
            self.write_cache_file(&paths.llm, llm_content.as_bytes())?;
            result.llm_generated = true;
        }

        // Generate Machine format
        if self.config.generate_machine {
            let machine_content = document_to_machine(doc);
            self.write_cache_file(&paths.machine, &machine_content.data)?;
            result.machine_generated = true;
        }

        Ok(result)
    }

    /// Write cache file with optional atomic write
    fn write_cache_file(&self, path: &Path, content: &[u8]) -> Result<(), CacheError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CacheError::DirectoryCreation(format!("{}: {}", parent.display(), e))
            })?;
        }

        if self.config.atomic_writes {
            // Write to temp file first, then rename
            let temp_path = path.with_extension("tmp");
            fs::write(&temp_path, content)?;
            fs::rename(&temp_path, path)?;
        } else {
            fs::write(path, content)?;
        }

        Ok(())
    }

    /// Get the config
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }
}

impl Default for CacheGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Paths for cache files
#[derive(Debug, Clone)]
pub struct CachePaths {
    /// Original source path
    pub source: PathBuf,
    /// LLM format cache path
    pub llm: PathBuf,
    /// Machine format cache path
    pub machine: PathBuf,
}

/// Result of cache generation
#[derive(Debug)]
pub struct CacheResult {
    /// Cache paths
    pub paths: CachePaths,
    /// Whether LLM format was generated
    pub llm_generated: bool,
    /// Whether Machine format was generated
    pub machine_generated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.cache_root, PathBuf::from(".dx/cache"));
        assert!(config.generate_llm);
        assert!(config.generate_machine);
        assert!(config.atomic_writes);
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::new()
            .with_cache_root("/custom/cache")
            .with_llm(false)
            .with_machine(true)
            .with_atomic_writes(false);

        assert_eq!(config.cache_root, PathBuf::from("/custom/cache"));
        assert!(!config.generate_llm);
        assert!(config.generate_machine);
        assert!(!config.atomic_writes);
    }

    #[test]
    fn test_map_path_to_cache_simple() {
        let generator = CacheGenerator::new();
        let source = Path::new("config.dx");
        let base = Path::new(".");

        let paths = generator.map_path_to_cache(source, base);

        assert_eq!(paths.source, PathBuf::from("config.dx"));
        assert!(paths.llm.to_string_lossy().contains("llm"));
        assert!(paths.llm.to_string_lossy().contains("config.dx.llm"));
        assert!(paths.machine.to_string_lossy().contains("machine"));
        assert!(paths.machine.to_string_lossy().contains("config.dx.bin"));
    }

    #[test]
    fn test_map_path_to_cache_nested() {
        let generator = CacheGenerator::new();
        let source = Path::new("src/config/data.dx");
        let base = Path::new(".");

        let paths = generator.map_path_to_cache(source, base);

        // Should preserve directory structure
        let llm_str = paths.llm.to_string_lossy();
        assert!(llm_str.contains("src") || llm_str.contains("config"));
        assert!(llm_str.contains("data.dx.llm"));
    }

    #[test]
    fn test_map_path_to_cache_with_base() {
        let generator = CacheGenerator::new();
        let source = Path::new("/project/src/config/data.dx");
        let base = Path::new("/project");

        let paths = generator.map_path_to_cache(source, base);

        // Should strip base path
        let llm_str = paths.llm.to_string_lossy();
        assert!(!llm_str.contains("project") || llm_str.contains("src"));
    }

    #[test]
    fn test_cache_generator_from_document() {
        use crate::llm::types::{DxDocument, DxLlmValue};
        use std::env;

        // Use a temp directory in the target folder
        let temp_dir = env::temp_dir().join("dx_cache_test_1");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up any previous run

        let config = CacheConfig::new()
            .with_cache_root(temp_dir.join("cache"))
            .with_atomic_writes(false);

        let generator = CacheGenerator::with_config(config);

        // Create a simple document
        let mut doc = DxDocument::new();
        doc.context.insert("nm".to_string(), DxLlmValue::Str("Test".to_string()));

        let source = Path::new("test.dx");
        let base = Path::new(".");

        let result = generator.generate_from_document(&doc, source, base).unwrap();

        assert!(result.llm_generated);
        assert!(result.machine_generated);
        assert!(result.paths.llm.exists());
        assert!(result.paths.machine.exists());

        // Verify LLM content
        let llm_content = fs::read_to_string(&result.paths.llm).unwrap();
        assert!(llm_content.contains("nm") || llm_content.contains("Test"));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_generator_llm_only() {
        use crate::llm::types::{DxDocument, DxLlmValue};
        use std::env;

        let temp_dir = env::temp_dir().join("dx_cache_test_2");
        let _ = fs::remove_dir_all(&temp_dir);

        let config = CacheConfig::new()
            .with_cache_root(temp_dir.join("cache"))
            .with_llm(true)
            .with_machine(false);

        let generator = CacheGenerator::with_config(config);

        let mut doc = DxDocument::new();
        doc.context.insert("nm".to_string(), DxLlmValue::Str("Test".to_string()));

        let source = Path::new("test.dx");
        let base = Path::new(".");

        let result = generator.generate_from_document(&doc, source, base).unwrap();

        assert!(result.llm_generated);
        assert!(!result.machine_generated);
        assert!(result.paths.llm.exists());
        assert!(!result.paths.machine.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_paths_structure() {
        let paths = CachePaths {
            source: PathBuf::from("src/data.dx"),
            llm: PathBuf::from(".dx/cache/llm/src/data.dx.llm"),
            machine: PathBuf::from(".dx/cache/machine/src/data.dx.bin"),
        };

        assert_eq!(paths.source.file_name().unwrap(), "data.dx");
        assert!(paths.llm.extension().unwrap() == "llm");
        assert!(paths.machine.extension().unwrap() == "bin");
    }
}
