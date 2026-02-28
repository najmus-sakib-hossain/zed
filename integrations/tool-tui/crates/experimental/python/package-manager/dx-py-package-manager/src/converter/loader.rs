//! Bytecode loading for DPP packages
//!
//! Loads pre-compiled bytecode during package installation and validates
//! against source hashes.

use super::bytecode::PythonVersion;
use std::collections::HashMap;

/// Loaded bytecode entry
#[derive(Debug, Clone)]
pub struct LoadedBytecode {
    /// Source file path
    pub source_path: String,
    /// Source hash for validation
    pub source_hash: [u8; 32],
    /// Target Python version
    pub python_version: PythonVersion,
    /// Compiled bytecode
    pub bytecode: Vec<u8>,
    /// Whether the bytecode has been validated
    pub validated: bool,
}

impl LoadedBytecode {
    /// Create a new loaded bytecode entry
    pub fn new(
        source_path: String,
        source_hash: [u8; 32],
        python_version: PythonVersion,
        bytecode: Vec<u8>,
    ) -> Self {
        Self {
            source_path,
            source_hash,
            python_version,
            bytecode,
            validated: false,
        }
    }

    /// Validate bytecode against source content
    pub fn validate(&mut self, source_content: &[u8]) -> bool {
        let hash = blake3::hash(source_content);
        self.validated = hash.as_bytes() == &self.source_hash;
        self.validated
    }

    /// Check if bytecode is valid for the given Python version
    pub fn is_compatible(&self, target_version: &PythonVersion) -> bool {
        // Bytecode is compatible if it was compiled for the same or older minor version
        // within the same major version
        self.python_version.major == target_version.major
            && self.python_version.minor <= target_version.minor
    }
}

/// Bytecode loader for DPP packages
pub struct BytecodeLoader {
    /// Target Python version
    target_version: PythonVersion,
    /// Loaded bytecode cache (path -> bytecode)
    cache: HashMap<String, LoadedBytecode>,
    /// Whether to validate bytecode against source
    validate_on_load: bool,
}

impl BytecodeLoader {
    /// Create a new bytecode loader
    pub fn new(target_version: PythonVersion) -> Self {
        Self {
            target_version,
            cache: HashMap::new(),
            validate_on_load: true,
        }
    }

    /// Set whether to validate bytecode on load
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate_on_load = validate;
        self
    }

    /// Get the target Python version
    pub fn target_version(&self) -> &PythonVersion {
        &self.target_version
    }

    /// Load bytecode from DPP bytecode section
    pub fn load_from_section(&mut self, data: &[u8]) -> Result<usize, BytecodeLoadError> {
        let mut offset = 0;

        // Read entry count
        if data.len() < 4 {
            return Err(BytecodeLoadError::InvalidFormat("Data too short".to_string()));
        }
        let count = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        let mut loaded = 0;

        for _ in 0..count {
            // Read source path
            if offset + 2 > data.len() {
                return Err(BytecodeLoadError::InvalidFormat("Unexpected end of data".to_string()));
            }
            let path_len =
                u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
            offset += 2;

            if offset + path_len > data.len() {
                return Err(BytecodeLoadError::InvalidFormat(
                    "Path length exceeds data".to_string(),
                ));
            }
            let source_path = String::from_utf8_lossy(&data[offset..offset + path_len]).to_string();
            offset += path_len;

            // Read source hash (32 bytes)
            if offset + 32 > data.len() {
                return Err(BytecodeLoadError::InvalidFormat(
                    "Hash length exceeds data".to_string(),
                ));
            }
            let mut source_hash = [0u8; 32];
            source_hash.copy_from_slice(&data[offset..offset + 32]);
            offset += 32;

            // Read Python version
            if offset + 4 > data.len() {
                return Err(BytecodeLoadError::InvalidFormat(
                    "Version length exceeds data".to_string(),
                ));
            }
            let version_u32 = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let python_version = PythonVersion::from_u32(version_u32);
            offset += 4;

            // Read bytecode length and content
            if offset + 8 > data.len() {
                return Err(BytecodeLoadError::InvalidFormat(
                    "Bytecode length exceeds data".to_string(),
                ));
            }
            let bytecode_len =
                u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap()) as usize;
            offset += 8;

            if offset + bytecode_len > data.len() {
                return Err(BytecodeLoadError::InvalidFormat(
                    "Bytecode content exceeds data".to_string(),
                ));
            }
            let bytecode = data[offset..offset + bytecode_len].to_vec();
            offset += bytecode_len;

            // Check version compatibility
            let entry =
                LoadedBytecode::new(source_path.clone(), source_hash, python_version, bytecode);

            if entry.is_compatible(&self.target_version) {
                self.cache.insert(source_path, entry);
                loaded += 1;
            }
        }

        Ok(loaded)
    }

    /// Get bytecode for a source path
    pub fn get(&self, source_path: &str) -> Option<&LoadedBytecode> {
        self.cache.get(source_path)
    }

    /// Get mutable bytecode for a source path
    pub fn get_mut(&mut self, source_path: &str) -> Option<&mut LoadedBytecode> {
        self.cache.get_mut(source_path)
    }

    /// Validate bytecode against source content
    pub fn validate(
        &mut self,
        source_path: &str,
        source_content: &[u8],
    ) -> Result<bool, BytecodeLoadError> {
        let entry = self
            .cache
            .get_mut(source_path)
            .ok_or_else(|| BytecodeLoadError::NotFound(source_path.to_string()))?;

        Ok(entry.validate(source_content))
    }

    /// Validate all loaded bytecode against source files
    pub fn validate_all<F>(&mut self, source_reader: F) -> Vec<ValidationResult>
    where
        F: Fn(&str) -> Option<Vec<u8>>,
    {
        let mut results = Vec::new();

        for (path, entry) in &mut self.cache {
            let result = if let Some(content) = source_reader(path) {
                if entry.validate(&content) {
                    ValidationResult::Valid(path.clone())
                } else {
                    ValidationResult::HashMismatch(path.clone())
                }
            } else {
                ValidationResult::SourceNotFound(path.clone())
            };
            results.push(result);
        }

        results
    }

    /// Get all loaded bytecode entries
    pub fn entries(&self) -> impl Iterator<Item = (&String, &LoadedBytecode)> {
        self.cache.iter()
    }

    /// Get the number of loaded entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the loader is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear all loaded bytecode
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Select the best bytecode for a given Python version from multiple entries
    pub fn select_best_version<'a>(
        entries: &'a [LoadedBytecode],
        target: &PythonVersion,
    ) -> Option<&'a LoadedBytecode> {
        entries
            .iter()
            .filter(|e| e.is_compatible(target))
            .max_by_key(|e| (e.python_version.major, e.python_version.minor))
    }
}

impl Default for BytecodeLoader {
    fn default() -> Self {
        Self::new(PythonVersion::default())
    }
}

/// Bytecode load error
#[derive(Debug, thiserror::Error)]
pub enum BytecodeLoadError {
    #[error("Invalid bytecode format: {0}")]
    InvalidFormat(String),

    #[error("Bytecode not found for path: {0}")]
    NotFound(String),

    #[error("Hash mismatch for path: {0}")]
    HashMismatch(String),

    #[error("Incompatible Python version: expected {expected}, got {got}")]
    IncompatibleVersion { expected: String, got: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// Bytecode is valid
    Valid(String),
    /// Source hash doesn't match
    HashMismatch(String),
    /// Source file not found
    SourceNotFound(String),
}

impl ValidationResult {
    /// Check if the result is valid
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid(_))
    }

    /// Get the path
    pub fn path(&self) -> &str {
        match self {
            ValidationResult::Valid(p) => p,
            ValidationResult::HashMismatch(p) => p,
            ValidationResult::SourceNotFound(p) => p,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converter::bytecode::BytecodeCompiler;

    #[test]
    fn test_loaded_bytecode_validation() {
        let source = b"def hello(): pass";
        let hash = *blake3::hash(source).as_bytes();

        let mut entry = LoadedBytecode::new(
            "test.py".to_string(),
            hash,
            PythonVersion::new(3, 12, 0),
            vec![1, 2, 3],
        );

        assert!(!entry.validated);
        assert!(entry.validate(source));
        assert!(entry.validated);

        // Wrong content should fail
        let mut entry2 = LoadedBytecode::new(
            "test.py".to_string(),
            hash,
            PythonVersion::new(3, 12, 0),
            vec![1, 2, 3],
        );
        assert!(!entry2.validate(b"different content"));
    }

    #[test]
    fn test_version_compatibility() {
        let entry = LoadedBytecode::new(
            "test.py".to_string(),
            [0u8; 32],
            PythonVersion::new(3, 11, 0),
            vec![],
        );

        // Same version is compatible
        assert!(entry.is_compatible(&PythonVersion::new(3, 11, 0)));

        // Newer minor version is compatible
        assert!(entry.is_compatible(&PythonVersion::new(3, 12, 0)));

        // Older minor version is not compatible
        assert!(!entry.is_compatible(&PythonVersion::new(3, 10, 0)));

        // Different major version is not compatible
        assert!(!entry.is_compatible(&PythonVersion::new(4, 0, 0)));
    }

    #[test]
    fn test_bytecode_loader_roundtrip() {
        let source = b"def main(): pass";
        let mut compiler = BytecodeCompiler::new(PythonVersion::new(3, 12, 0));
        let compiled = compiler.compile("test.py", source);

        // Serialize bytecode section
        let mut section = Vec::new();
        section.extend_from_slice(&1u32.to_le_bytes()); // count

        let path_bytes = compiled.source_path.as_bytes();
        section.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
        section.extend_from_slice(path_bytes);
        section.extend_from_slice(&compiled.source_hash);
        section.extend_from_slice(&compiled.python_version.to_u32().to_le_bytes());
        section.extend_from_slice(&(compiled.bytecode.len() as u64).to_le_bytes());
        section.extend_from_slice(&compiled.bytecode);

        // Load bytecode
        let mut loader = BytecodeLoader::new(PythonVersion::new(3, 12, 0));
        let loaded = loader.load_from_section(&section).unwrap();

        assert_eq!(loaded, 1);
        assert!(loader.get("test.py").is_some());

        // Validate
        let entry = loader.get_mut("test.py").unwrap();
        assert!(entry.validate(source));
    }

    #[test]
    fn test_select_best_version() {
        let entries = vec![
            LoadedBytecode::new(
                "test.py".to_string(),
                [0u8; 32],
                PythonVersion::new(3, 10, 0),
                vec![],
            ),
            LoadedBytecode::new(
                "test.py".to_string(),
                [0u8; 32],
                PythonVersion::new(3, 11, 0),
                vec![],
            ),
            LoadedBytecode::new(
                "test.py".to_string(),
                [0u8; 32],
                PythonVersion::new(3, 12, 0),
                vec![],
            ),
        ];

        // For Python 3.12, should select 3.12 bytecode
        let best = BytecodeLoader::select_best_version(&entries, &PythonVersion::new(3, 12, 0));
        assert!(best.is_some());
        assert_eq!(best.unwrap().python_version.minor, 12);

        // For Python 3.11, should select 3.11 bytecode
        let best = BytecodeLoader::select_best_version(&entries, &PythonVersion::new(3, 11, 0));
        assert!(best.is_some());
        assert_eq!(best.unwrap().python_version.minor, 11);

        // For Python 3.9, no compatible bytecode
        let best = BytecodeLoader::select_best_version(&entries, &PythonVersion::new(3, 9, 0));
        assert!(best.is_none());
    }
}
