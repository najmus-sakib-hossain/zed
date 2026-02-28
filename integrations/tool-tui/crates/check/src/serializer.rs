//! DX Serializer Integration
//!
//! This module provides integration with dx-serializer for configuration and rule loading.
//! It supports the holographic format with ONE format, THREE representations:
//! - **LLM Format**: Token-efficient text stored on disk (`.sr` files or `dx` config files)
//! - **HUMAN Format**: Readable TOML-like format shown in editors (`.human` files)
//! - **MACHINE Format**: Binary zero-copy format for runtime (`.machine` files in `.dx/serializer/`)

use serializer::{DxDocument, DxLlmValue, IndexMap, deserialize, serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default cache directories
pub const SERIALIZER_CACHE_DIR: &str = ".dx/serializer";
pub const CHECK_CACHE_DIR: &str = ".dx/check";

/// Serializer errors
#[derive(Debug, Error)]
pub enum SerializerError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse LLM format: {0}")]
    ParseError(String),

    #[error("Failed to serialize: {0}")]
    SerializeError(String),

    #[error("Invalid configuration: {field} - {message}")]
    ValidationError { field: String, message: String },

    #[error("Cache error: {0}")]
    CacheError(String),
}

/// Wrapper for dx-serializer operations
pub struct DxSerializerWrapper {
    /// Cache directory for compiled MACHINE format
    cache_dir: PathBuf,
}

impl DxSerializerWrapper {
    /// Create a new serializer wrapper with the specified cache directory
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Load a configuration or rule file from disk (LLM format)
    ///
    /// This reads `.sr` files or `dx` config files and parses them into a `DxDocument`.
    pub fn load_from_disk(&self, path: &Path) -> Result<DxDocument, SerializerError> {
        let content = std::fs::read_to_string(path)?;
        deserialize(&content).map_err(|e| SerializerError::ParseError(e.to_string()))
    }

    /// Save a configuration or rule to disk (LLM format)
    pub fn save_to_disk(&self, doc: &DxDocument, path: &Path) -> Result<(), SerializerError> {
        let content = serialize(doc);
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load and compile to MACHINE format with caching
    ///
    /// This loads the LLM format from disk, compiles it to MACHINE format,
    /// and caches the result in `.dx/serializer/` for fast loading.
    pub fn load_with_cache(&self, source_path: &Path) -> Result<DxDocument, SerializerError> {
        // Ensure cache directory exists
        std::fs::create_dir_all(&self.cache_dir)?;

        // Check if cached MACHINE format exists and is valid
        let cache_path = self.get_cache_path(source_path);
        if self.is_cache_valid(source_path, &cache_path)? {
            // Load from cache (MACHINE format)
            return self.load_from_cache(&cache_path);
        }

        // Load from source (LLM format)
        let doc = self.load_from_disk(source_path)?;

        // Compile and cache MACHINE format
        self.save_to_cache(&doc, &cache_path)?;

        Ok(doc)
    }

    /// Get the cache path for a source file
    fn get_cache_path(&self, source_path: &Path) -> PathBuf {
        let file_name = source_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

        // Create a hash of the full path to avoid collisions
        let hash = blake3::hash(source_path.to_string_lossy().as_bytes());
        let hash_str = hash.to_hex();

        self.cache_dir
            .join(format!("{}-{}.machine", file_name, &hash_str.as_str()[..8]))
    }

    /// Check if the cache is valid (source file hasn't changed)
    fn is_cache_valid(
        &self,
        source_path: &Path,
        cache_path: &Path,
    ) -> Result<bool, SerializerError> {
        if !cache_path.exists() {
            return Ok(false);
        }

        let source_meta = std::fs::metadata(source_path)?;
        let cache_meta = std::fs::metadata(cache_path)?;

        // Cache is valid if it's newer than the source
        Ok(cache_meta.modified()? > source_meta.modified()?)
    }

    /// Load from cache (MACHINE format)
    fn load_from_cache(&self, cache_path: &Path) -> Result<DxDocument, SerializerError> {
        // For now, we'll load the LLM format from cache
        // In a full implementation, this would load the binary MACHINE format
        let content = std::fs::read_to_string(cache_path)?;
        deserialize(&content).map_err(|e| SerializerError::ParseError(e.to_string()))
    }

    /// Save to cache (MACHINE format)
    fn save_to_cache(&self, doc: &DxDocument, cache_path: &Path) -> Result<(), SerializerError> {
        // For now, we'll save the LLM format to cache
        // In a full implementation, this would compile to binary MACHINE format
        let content = serialize(doc);
        std::fs::write(cache_path, content)?;
        Ok(())
    }

    /// Convert a `DxDocument` to an `IndexMap` for easier access
    #[must_use]
    pub fn doc_to_map(&self, doc: &DxDocument) -> IndexMap<String, DxLlmValue> {
        doc.context.clone()
    }

    /// Convert an `IndexMap` to a `DxDocument`
    #[must_use]
    pub fn map_to_doc(&self, map: IndexMap<String, DxLlmValue>) -> DxDocument {
        let mut doc = DxDocument::new();
        doc.context = map;
        doc
    }

    /// Get a string value from a document
    #[must_use]
    pub fn get_string(&self, doc: &DxDocument, key: &str) -> Option<String> {
        match doc.context.get(key) {
            Some(DxLlmValue::Str(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// Get a number value from a document
    #[must_use]
    pub fn get_number(&self, doc: &DxDocument, key: &str) -> Option<f64> {
        match doc.context.get(key) {
            Some(DxLlmValue::Num(n)) => Some(*n),
            _ => None,
        }
    }

    /// Get a boolean value from a document
    #[must_use]
    pub fn get_bool(&self, doc: &DxDocument, key: &str) -> Option<bool> {
        match doc.context.get(key) {
            Some(DxLlmValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Get an array value from a document
    #[must_use]
    pub fn get_array(&self, doc: &DxDocument, key: &str) -> Option<Vec<DxLlmValue>> {
        match doc.context.get(key) {
            Some(DxLlmValue::Arr(arr)) => Some(arr.clone()),
            _ => None,
        }
    }

    /// Get an object value from a document
    #[must_use]
    pub fn get_object(&self, doc: &DxDocument, key: &str) -> Option<IndexMap<String, DxLlmValue>> {
        match doc.context.get(key) {
            Some(DxLlmValue::Obj(obj)) => Some(obj.clone()),
            _ => None,
        }
    }
}

/// Configuration loader using dx-serializer
pub struct ConfigLoader {
    serializer: DxSerializerWrapper,
}

impl ConfigLoader {
    /// Create a new configuration loader
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            serializer: DxSerializerWrapper::new(cache_dir),
        }
    }

    /// Load configuration from a `dx` file or `.sr` file
    pub fn load_config(&self, path: &Path) -> Result<DxDocument, SerializerError> {
        self.serializer.load_with_cache(path)
    }

    /// Save configuration to a file
    pub fn save_config(&self, doc: &DxDocument, path: &Path) -> Result<(), SerializerError> {
        self.serializer.save_to_disk(doc, path)
    }
}

/// Rule loader using dx-serializer
pub struct RuleLoader {
    serializer: DxSerializerWrapper,
}

impl RuleLoader {
    /// Create a new rule loader
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            serializer: DxSerializerWrapper::new(cache_dir),
        }
    }

    /// Load a rule from a `.sr` file
    pub fn load_rule(&self, path: &Path) -> Result<DxDocument, SerializerError> {
        self.serializer.load_with_cache(path)
    }

    /// Load all rules from a directory
    pub fn load_rules_from_dir(&self, dir: &Path) -> Result<Vec<DxDocument>, SerializerError> {
        let mut rules = Vec::new();

        if !dir.exists() {
            return Ok(rules);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Only load .sr files
            if path.extension().and_then(|s| s.to_str()) == Some("sr") {
                match self.load_rule(&path) {
                    Ok(rule) => rules.push(rule),
                    Err(e) => {
                        eprintln!("Warning: Failed to load rule from {path:?}: {e}");
                    }
                }
            }
        }

        Ok(rules)
    }

    /// Save a rule to a `.sr` file
    pub fn save_rule(&self, doc: &DxDocument, path: &Path) -> Result<(), SerializerError> {
        self.serializer.save_to_disk(doc, path)
    }
}

/// Set up cache directory structure
///
/// Creates the following directories:
/// - `.dx/serializer/` - Compiled MACHINE format cache
/// - `.dx/check/` - Score history and analysis results
pub fn setup_cache_directories(root: &Path) -> Result<(), SerializerError> {
    let serializer_cache = root.join(SERIALIZER_CACHE_DIR);
    let check_cache = root.join(CHECK_CACHE_DIR);

    std::fs::create_dir_all(&serializer_cache)?;
    std::fs::create_dir_all(&check_cache)?;

    Ok(())
}

/// Get the default serializer cache directory for a project root
#[must_use]
pub fn get_serializer_cache_dir(root: &Path) -> PathBuf {
    root.join(SERIALIZER_CACHE_DIR)
}

/// Get the default check cache directory for a project root
#[must_use]
pub fn get_check_cache_dir(root: &Path) -> PathBuf {
    root.join(CHECK_CACHE_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_serializer_wrapper_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let wrapper = DxSerializerWrapper::new(temp_dir.path().to_path_buf());

        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("test".to_string()));
        doc.context.insert("version".to_string(), DxLlmValue::Str("1.0.0".to_string()));
        doc.context.insert("enabled".to_string(), DxLlmValue::Bool(true));

        let file_path = temp_dir.path().join("test.sr");
        wrapper.save_to_disk(&doc, &file_path).unwrap();

        let loaded = wrapper.load_from_disk(&file_path).unwrap();
        assert_eq!(wrapper.get_string(&loaded, "name"), Some("test".to_string()));
        assert_eq!(wrapper.get_string(&loaded, "version"), Some("1.0.0".to_string()));
        assert_eq!(wrapper.get_bool(&loaded, "enabled"), Some(true));
    }

    #[test]
    fn test_config_loader() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join(".dx/serializer");
        let loader = ConfigLoader::new(cache_dir);

        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("dx-check".to_string()));

        let config_path = temp_dir.path().join("dx");
        loader.save_config(&doc, &config_path).unwrap();

        let loaded = loader.load_config(&config_path).unwrap();
        assert!(loaded.context.contains_key("name"));
    }

    #[test]
    fn test_rule_loader() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join(".dx/serializer");
        let rules_dir = temp_dir.path().join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();

        let loader = RuleLoader::new(cache_dir);

        // Create a test rule
        let mut rule = DxDocument::new();
        rule.context
            .insert("rule".to_string(), DxLlmValue::Str("no-console".to_string()));
        rule.context
            .insert("category".to_string(), DxLlmValue::Str("linting".to_string()));
        rule.context
            .insert("severity".to_string(), DxLlmValue::Str("warning".to_string()));

        let rule_path = rules_dir.join("no-console.sr");
        loader.save_rule(&rule, &rule_path).unwrap();

        // Load all rules from directory
        let rules = loader.load_rules_from_dir(&rules_dir).unwrap();
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn test_cache_invalidation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join(".dx/serializer");
        let wrapper = DxSerializerWrapper::new(cache_dir.clone());

        let mut doc = DxDocument::new();
        doc.context.insert("value".to_string(), DxLlmValue::Num(1.0));

        let source_path = temp_dir.path().join("test.sr");
        wrapper.save_to_disk(&doc, &source_path).unwrap();

        // First load - should create cache
        let loaded1 = wrapper.load_with_cache(&source_path).unwrap();
        assert_eq!(wrapper.get_number(&loaded1, "value"), Some(1.0));

        // Modify source file
        std::thread::sleep(std::time::Duration::from_millis(10));
        doc.context.insert("value".to_string(), DxLlmValue::Num(2.0));
        wrapper.save_to_disk(&doc, &source_path).unwrap();

        // Second load - should invalidate cache and reload
        let loaded2 = wrapper.load_with_cache(&source_path).unwrap();
        assert_eq!(wrapper.get_number(&loaded2, "value"), Some(2.0));
    }

    #[test]
    fn test_setup_cache_directories() {
        let temp_dir = TempDir::new().unwrap();
        setup_cache_directories(temp_dir.path()).unwrap();

        let serializer_cache = temp_dir.path().join(SERIALIZER_CACHE_DIR);
        let check_cache = temp_dir.path().join(CHECK_CACHE_DIR);

        assert!(serializer_cache.exists());
        assert!(check_cache.exists());
    }

    #[test]
    fn test_get_cache_dirs() {
        let root = Path::new("/project");
        let serializer_cache = get_serializer_cache_dir(root);
        let check_cache = get_check_cache_dir(root);

        assert_eq!(serializer_cache, root.join(SERIALIZER_CACHE_DIR));
        assert_eq!(check_cache, root.join(CHECK_CACHE_DIR));
    }
}
