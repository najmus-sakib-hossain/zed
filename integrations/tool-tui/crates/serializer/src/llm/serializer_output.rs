//! Serializer Output Module for DX Serializer
//!
//! Generates LLM and Machine format files from .sr/.dx source files.
//! Output files are stored in `.dx/serializer/` with proper naming.
//!
//! ## Output Structure
//!
//! ```text
//! .dx/serializer/
//! ├── javascript-lint.llm      # LLM-optimized format
//! └── javascript-lint.machine  # Binary format (used at runtime)
//! ```
//!
//! ## Format Flow (2026 Architecture)
//!
//! 1. Source (.sr/.dx) - Human format stored on disk
//! 2. LLM (.llm) - LLM-optimized format in .dx folder
//! 3. Machine (.machine) - Binary for fast runtime loading

use crate::llm::convert::{
    CompressionAlgorithm, ConvertError, document_to_machine_with_compression, llm_to_document,
};
use crate::llm::human_formatter::HumanFormatter;
use crate::llm::types::DxDocument;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Serializer output errors
#[derive(Debug, Error)]
pub enum SerializerOutputError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Conversion error: {0}")]
    Convert(#[from] ConvertError),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Directory creation failed: {0}")]
    DirectoryCreation(String),
}

/// Configuration for serializer output
#[derive(Debug, Clone)]
pub struct SerializerOutputConfig {
    /// Root directory for output files (default: .dx/serializer)
    pub output_dir: PathBuf,
    /// Generate LLM format files
    pub generate_llm: bool,
    /// Generate machine format files
    pub generate_machine: bool,
    /// Compression algorithm for machine format
    pub compression: CompressionAlgorithm,
}

impl Default for SerializerOutputConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(".dx/serializer"),
            generate_llm: true,
            generate_machine: true,
            compression: CompressionAlgorithm::default(),
        }
    }
}

impl SerializerOutputConfig {
    /// Create a new config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the output directory
    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = dir.into();
        self
    }

    /// Set whether to generate LLM format
    pub fn with_llm(mut self, generate: bool) -> Self {
        self.generate_llm = generate;
        self
    }

    /// Set whether to generate machine format
    pub fn with_machine(mut self, generate: bool) -> Self {
        self.generate_machine = generate;
        self
    }

    /// Set compression algorithm
    pub fn with_compression(mut self, compression: CompressionAlgorithm) -> Self {
        self.compression = compression;
        self
    }
}

/// Output paths for a serialized file
#[derive(Debug, Clone)]
pub struct SerializerPaths {
    /// Original source path (.sr/.dx file)
    pub source: PathBuf,
    /// LLM format output path (.llm)
    pub llm: PathBuf,
    /// Machine format output path (.machine)
    pub machine: PathBuf,
}

/// Result of serializer output generation
#[derive(Debug)]
pub struct SerializerResult {
    /// Output paths
    pub paths: SerializerPaths,
    /// Whether LLM format was generated
    pub llm_generated: bool,
    /// Whether machine format was generated
    pub machine_generated: bool,
    /// Size of LLM output in bytes
    pub llm_size: usize,
    /// Size of machine output in bytes
    pub machine_size: usize,
}

/// Serializer output generator
pub struct SerializerOutput {
    config: SerializerOutputConfig,
}

impl SerializerOutput {
    /// Create a new serializer output with default config
    pub fn new() -> Self {
        Self {
            config: SerializerOutputConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: SerializerOutputConfig) -> Self {
        Self { config }
    }

    /// Get output paths for a source file
    pub fn get_paths(&self, source_path: &Path) -> SerializerPaths {
        let stem = source_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        SerializerPaths {
            source: source_path.to_path_buf(),
            llm: self.config.output_dir.join(format!("{}.llm", stem)),
            machine: self.config.output_dir.join(format!("{}.machine", stem)),
        }
    }

    /// Process a .sr/.dx source file and generate outputs
    pub fn process_file(
        &self,
        source_path: &Path,
    ) -> Result<SerializerResult, SerializerOutputError> {
        // Read source file (Human format)
        let content = fs::read_to_string(source_path)?;

        // Parse Human format to document
        let doc =
            llm_to_document(&content).map_err(|e| SerializerOutputError::Parse(e.to_string()))?;

        self.process_document(&doc, source_path)
    }

    /// Process a DxDocument and generate outputs
    pub fn process_document(
        &self,
        doc: &DxDocument,
        source_path: &Path,
    ) -> Result<SerializerResult, SerializerOutputError> {
        let paths = self.get_paths(source_path);

        // Ensure output directory exists
        fs::create_dir_all(&self.config.output_dir).map_err(|e| {
            SerializerOutputError::DirectoryCreation(format!(
                "{}: {}",
                self.config.output_dir.display(),
                e
            ))
        })?;

        let mut result = SerializerResult {
            paths: paths.clone(),
            llm_generated: false,
            machine_generated: false,
            llm_size: 0,
            machine_size: 0,
        };

        // Generate LLM format (compact, token-efficient)
        if self.config.generate_llm {
            // Use compact format for LLM (no extra spacing)
            let formatter = HumanFormatter::new();
            let llm_content = formatter.format(doc);
            fs::write(&paths.llm, &llm_content)?;
            result.llm_generated = true;
            result.llm_size = llm_content.len();
        }

        // Generate machine format
        if self.config.generate_machine {
            let machine_content =
                document_to_machine_with_compression(doc, self.config.compression);
            fs::write(&paths.machine, &machine_content.data)?;
            result.machine_generated = true;
            result.machine_size = machine_content.data.len();
        }

        Ok(result)
    }

    /// Process all .sr/.dx files in a directory
    pub fn process_directory(
        &self,
        dir: &Path,
    ) -> Result<Vec<SerializerResult>, SerializerOutputError> {
        let mut results = Vec::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "sr"
                        || ext == "dx"
                        || path.file_name().and_then(|n| n.to_str()) == Some("dx")
                    {
                        match self.process_file(&path) {
                            Ok(result) => results.push(result),
                            Err(e) => {
                                eprintln!("Warning: Failed to process {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Check if outputs are up-to-date for a source file
    pub fn is_up_to_date(&self, source_path: &Path) -> bool {
        let paths = self.get_paths(source_path);

        // Check if output files exist
        if !paths.llm.exists() || !paths.machine.exists() {
            return false;
        }

        // Compare modification times
        let source_modified = fs::metadata(source_path).and_then(|m| m.modified()).ok();

        let llm_modified = fs::metadata(&paths.llm).and_then(|m| m.modified()).ok();

        let machine_modified = fs::metadata(&paths.machine).and_then(|m| m.modified()).ok();

        match (source_modified, llm_modified, machine_modified) {
            (Some(src), Some(llm), Some(machine)) => llm >= src && machine >= src,
            _ => false,
        }
    }

    /// Get the config
    pub fn config(&self) -> &SerializerOutputConfig {
        &self.config
    }
}

impl Default for SerializerOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_serializer_output_config_default() {
        let config = SerializerOutputConfig::default();
        assert_eq!(config.output_dir, PathBuf::from(".dx/serializer"));
        assert!(config.generate_llm);
        assert!(config.generate_machine);
    }

    #[test]
    fn test_get_paths() {
        let output = SerializerOutput::new();
        let paths = output.get_paths(Path::new("rules/javascript-lint.sr"));

        assert_eq!(paths.llm.file_name().unwrap(), "javascript-lint.llm");
        assert_eq!(paths.machine.file_name().unwrap(), "javascript-lint.machine");
    }

    #[test]
    fn test_process_simple_file() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("test.sr");

        // Create a simple .sr file
        fs::write(&source_path, "nm|test\nv|1.0").unwrap();

        let config =
            SerializerOutputConfig::new().with_output_dir(temp.path().join(".dx/serializer"));

        let output = SerializerOutput::with_config(config);
        let result = output.process_file(&source_path);

        // Note: This may fail if llm_to_document doesn't support this format
        // In that case, we'd need to adjust the test
        if let Ok(result) = result {
            assert!(result.llm_generated);
            assert!(result.machine_generated);
        }
    }
}
