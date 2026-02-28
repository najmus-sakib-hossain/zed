//! AOT (Ahead-of-Time) Compilation Cache
//!
//! This module provides persistent caching of compiled code to disk,
//! enabling faster startup times by avoiding recompilation of hot functions.
//!
//! ## Cache Structure
//!
//! The AOT cache uses content-addressable storage based on source hash:
//! ```text
//! cache_dir/
//!   {source_hash_prefix}/
//!     {source_hash}_{func_name}.dxao
//! ```
//!
//! ## File Format
//!
//! Each `.dxao` file contains:
//! - Header with magic number, version, and metadata
//! - Compiled native code
//! - Relocation information for position-independent loading

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Magic number for AOT cache files: "DXAO"
pub const AOT_MAGIC: [u8; 4] = [b'D', b'X', b'A', b'O'];

/// Current AOT cache format version
pub const AOT_VERSION: u32 = 1;

/// Errors that can occur during AOT cache operations
#[derive(Debug, Error)]
pub enum AotError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid cache file: {0}")]
    InvalidCacheFile(String),

    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },

    #[error("Source hash mismatch")]
    SourceHashMismatch,

    #[error("Cache directory not found: {0}")]
    CacheDirNotFound(PathBuf),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// AOT cache file header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AotCacheHeader {
    /// Magic number: "DXAO"
    pub magic: [u8; 4],
    /// Format version
    pub version: u32,
    /// BLAKE3 hash of the source code
    pub source_hash: [u8; 32],
    /// Offset to compiled code section
    pub code_offset: u32,
    /// Size of compiled code
    pub code_size: u32,
    /// Offset to relocation section
    pub reloc_offset: u32,
    /// Number of relocations
    pub reloc_count: u32,
    /// Reserved for future use
    pub reserved: [u8; 16],
}

impl AotCacheHeader {
    /// Create a new header with the given source hash
    pub fn new(source_hash: [u8; 32]) -> Self {
        Self {
            magic: AOT_MAGIC,
            version: AOT_VERSION,
            source_hash,
            code_offset: std::mem::size_of::<Self>() as u32,
            code_size: 0,
            reloc_offset: 0,
            reloc_count: 0,
            reserved: [0; 16],
        }
    }

    /// Validate the header
    pub fn validate(&self) -> Result<(), AotError> {
        if self.magic != AOT_MAGIC {
            return Err(AotError::InvalidCacheFile("Invalid magic number".to_string()));
        }
        if self.version != AOT_VERSION {
            return Err(AotError::VersionMismatch {
                expected: AOT_VERSION,
                found: self.version,
            });
        }
        Ok(())
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Self>());
        bytes.extend_from_slice(&self.magic);
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.source_hash);
        bytes.extend_from_slice(&self.code_offset.to_le_bytes());
        bytes.extend_from_slice(&self.code_size.to_le_bytes());
        bytes.extend_from_slice(&self.reloc_offset.to_le_bytes());
        bytes.extend_from_slice(&self.reloc_count.to_le_bytes());
        bytes.extend_from_slice(&self.reserved);
        bytes
    }

    /// Deserialize header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AotError> {
        if bytes.len() < std::mem::size_of::<Self>() {
            return Err(AotError::InvalidCacheFile("Header too short".to_string()));
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);

        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        let mut source_hash = [0u8; 32];
        source_hash.copy_from_slice(&bytes[8..40]);

        let code_offset = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);
        let code_size = u32::from_le_bytes([bytes[44], bytes[45], bytes[46], bytes[47]]);
        let reloc_offset = u32::from_le_bytes([bytes[48], bytes[49], bytes[50], bytes[51]]);
        let reloc_count = u32::from_le_bytes([bytes[52], bytes[53], bytes[54], bytes[55]]);

        let mut reserved = [0u8; 16];
        reserved.copy_from_slice(&bytes[56..72]);

        Ok(Self {
            magic,
            version,
            source_hash,
            code_offset,
            code_size,
            reloc_offset,
            reloc_count,
            reserved,
        })
    }
}

/// Relocation entry for position-independent code
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Relocation {
    /// Offset within the code section
    pub offset: u32,
    /// Type of relocation
    pub reloc_type: RelocationType,
    /// Symbol index or addend
    pub addend: i64,
}

/// Types of relocations
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationType {
    /// Absolute 64-bit address
    Abs64 = 0,
    /// PC-relative 32-bit offset
    Rel32 = 1,
    /// Runtime helper function
    RuntimeHelper = 2,
}

impl Relocation {
    /// Serialize relocation to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self.offset.to_le_bytes());
        bytes.push(self.reloc_type as u8);
        bytes.extend_from_slice(&[0u8; 3]); // padding
        bytes.extend_from_slice(&self.addend.to_le_bytes());
        bytes
    }

    /// Deserialize relocation from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AotError> {
        if bytes.len() < 16 {
            return Err(AotError::InvalidCacheFile("Relocation entry too short".to_string()));
        }

        let offset = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let reloc_type = match bytes[4] {
            0 => RelocationType::Abs64,
            1 => RelocationType::Rel32,
            2 => RelocationType::RuntimeHelper,
            _ => {
                return Err(AotError::InvalidCacheFile(format!(
                    "Unknown relocation type: {}",
                    bytes[4]
                )))
            }
        };
        let addend = i64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);

        Ok(Self {
            offset,
            reloc_type,
            addend,
        })
    }
}

/// Compiled code with relocation information
#[derive(Debug, Clone)]
pub struct CachedCode {
    /// Native machine code
    pub code: Vec<u8>,
    /// Relocations to apply when loading
    pub relocations: Vec<Relocation>,
    /// Source hash for validation
    pub source_hash: [u8; 32],
}

impl CachedCode {
    /// Create new cached code
    pub fn new(code: Vec<u8>, source_hash: [u8; 32]) -> Self {
        Self {
            code,
            relocations: Vec::new(),
            source_hash,
        }
    }

    /// Add a relocation
    pub fn add_relocation(&mut self, reloc: Relocation) {
        self.relocations.push(reloc);
    }

    /// Apply relocations to make code executable at the given base address
    ///
    /// # Safety
    /// The code buffer must be writable and the base_addr must be the actual
    /// address where the code will be executed.
    pub fn apply_relocations(
        &mut self,
        base_addr: usize,
        helper_table: &RuntimeHelperTable,
    ) -> Result<(), AotError> {
        for reloc in &self.relocations {
            let offset = reloc.offset as usize;
            if offset + 8 > self.code.len() {
                return Err(AotError::InvalidCacheFile(format!(
                    "Relocation offset {} out of bounds",
                    offset
                )));
            }

            match reloc.reloc_type {
                RelocationType::Abs64 => {
                    // Write absolute 64-bit address
                    let addr = (base_addr as i64 + reloc.addend) as u64;
                    self.code[offset..offset + 8].copy_from_slice(&addr.to_le_bytes());
                }
                RelocationType::Rel32 => {
                    // Write PC-relative 32-bit offset
                    let pc = base_addr + offset + 4; // PC after the instruction
                    let target = (base_addr as i64 + reloc.addend) as usize;
                    let rel_offset = (target as i64 - pc as i64) as i32;
                    self.code[offset..offset + 4].copy_from_slice(&rel_offset.to_le_bytes());
                }
                RelocationType::RuntimeHelper => {
                    // Look up runtime helper address
                    let helper_addr =
                        helper_table.get_helper(reloc.addend as usize).ok_or_else(|| {
                            AotError::InvalidCacheFile(format!(
                                "Unknown runtime helper index: {}",
                                reloc.addend
                            ))
                        })?;
                    self.code[offset..offset + 8].copy_from_slice(&helper_addr.to_le_bytes());
                }
            }
        }
        Ok(())
    }

    /// Get the code as a slice
    pub fn as_slice(&self) -> &[u8] {
        &self.code
    }

    /// Get the code size
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Check if code is empty
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }
}

/// Table of runtime helper function addresses
#[derive(Debug, Clone, Default)]
pub struct RuntimeHelperTable {
    helpers: Vec<usize>,
}

impl RuntimeHelperTable {
    /// Create a new empty helper table
    pub fn new() -> Self {
        Self {
            helpers: Vec::new(),
        }
    }

    /// Register a runtime helper and return its index
    pub fn register(&mut self, addr: usize) -> usize {
        let idx = self.helpers.len();
        self.helpers.push(addr);
        idx
    }

    /// Get a helper address by index
    pub fn get_helper(&self, idx: usize) -> Option<usize> {
        self.helpers.get(idx).copied()
    }

    /// Create a table with standard runtime helpers
    pub fn with_standard_helpers() -> Self {
        use crate::helpers::*;
        let mut table = Self::new();
        table.register(rt_string_concat as usize);
        table.register(rt_string_repeat as usize);
        table.register(rt_string_compare as usize);
        table.register(rt_power as usize);
        table.register(rt_contains as usize);
        table.register(rt_contains_list as usize);
        table.register(rt_contains_dict as usize);
        table.register(rt_contains_string as usize);
        table
    }
}

/// Standard helper indices
pub mod helper_indices {
    pub const STRING_CONCAT: usize = 0;
    pub const STRING_REPEAT: usize = 1;
    pub const STRING_COMPARE: usize = 2;
    pub const POWER: usize = 3;
    pub const CONTAINS: usize = 4;
    pub const CONTAINS_LIST: usize = 5;
    pub const CONTAINS_DICT: usize = 6;
    pub const CONTAINS_STRING: usize = 7;
}

/// AOT compilation cache manager
pub struct AotCache {
    /// Cache directory path
    cache_dir: PathBuf,
    /// In-memory cache of loaded code
    loaded: HashMap<String, CachedCode>,
}

impl AotCache {
    /// Create a new AOT cache with the given directory
    pub fn new(cache_dir: PathBuf) -> Result<Self, AotError> {
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        Ok(Self {
            cache_dir,
            loaded: HashMap::new(),
        })
    }

    /// Create a cache with the default directory
    pub fn default_cache() -> Result<Self, AotError> {
        let cache_dir = Self::default_cache_dir()?;
        Self::new(cache_dir)
    }

    /// Get the default cache directory
    pub fn default_cache_dir() -> Result<PathBuf, AotError> {
        // Use platform-specific cache directory
        #[cfg(target_os = "windows")]
        let base = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        #[cfg(not(target_os = "windows"))]
        let base = std::env::var("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".cache"))
                .unwrap_or_else(|_| PathBuf::from("."))
        });

        Ok(base.join("dx-py").join("aot"))
    }

    /// Get the cache file path for a function
    fn cache_path(&self, source_hash: &[u8; 32], func_name: &str) -> PathBuf {
        // Use first 2 bytes of hash as subdirectory for better filesystem performance
        let prefix = format!("{:02x}{:02x}", source_hash[0], source_hash[1]);
        let hash_str = hex_encode(source_hash);
        let filename = format!("{}_{}.dxao", hash_str, sanitize_name(func_name));
        self.cache_dir.join(prefix).join(filename)
    }

    /// Get cached compiled code for a function
    pub fn get(&mut self, source_hash: &[u8; 32], func_name: &str) -> Option<&CachedCode> {
        let key = cache_key(source_hash, func_name);

        // Check in-memory cache first
        if self.loaded.contains_key(&key) {
            return self.loaded.get(&key);
        }

        // Try to load from disk
        let path = self.cache_path(source_hash, func_name);
        if let Ok(cached) = self.load_from_disk(&path, source_hash) {
            self.loaded.insert(key.clone(), cached);
            return self.loaded.get(&key);
        }

        None
    }

    /// Store compiled code in cache
    pub fn put(
        &mut self,
        source_hash: &[u8; 32],
        func_name: &str,
        code: &CachedCode,
    ) -> Result<(), AotError> {
        let path = self.cache_path(source_hash, func_name);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to disk
        self.save_to_disk(&path, code)?;

        // Update in-memory cache
        let key = cache_key(source_hash, func_name);
        self.loaded.insert(key, code.clone());

        Ok(())
    }

    /// Invalidate cache for a source file
    pub fn invalidate(&mut self, source_hash: &[u8; 32]) -> Result<usize, AotError> {
        let prefix = format!("{:02x}{:02x}", source_hash[0], source_hash[1]);
        let hash_str = hex_encode(source_hash);
        let prefix_dir = self.cache_dir.join(&prefix);

        let mut removed = 0;

        if prefix_dir.exists() {
            for entry in fs::read_dir(&prefix_dir)? {
                let entry = entry?;
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(&hash_str) {
                    fs::remove_file(entry.path())?;
                    removed += 1;
                }
            }
        }

        // Remove from in-memory cache
        self.loaded.retain(|k, _| !k.starts_with(&hash_str));

        Ok(removed)
    }

    /// Clear all cached code
    pub fn clear(&mut self) -> Result<(), AotError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
        }
        self.loaded.clear();
        Ok(())
    }

    /// Load cached code from disk
    fn load_from_disk(
        &self,
        path: &Path,
        expected_hash: &[u8; 32],
    ) -> Result<CachedCode, AotError> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        // Parse header
        let header = AotCacheHeader::from_bytes(&data)?;
        header.validate()?;

        // Verify source hash
        if &header.source_hash != expected_hash {
            return Err(AotError::SourceHashMismatch);
        }

        // Extract code
        let code_start = header.code_offset as usize;
        let code_end = code_start + header.code_size as usize;
        if code_end > data.len() {
            return Err(AotError::InvalidCacheFile("Code section extends beyond file".to_string()));
        }
        let code = data[code_start..code_end].to_vec();

        // Extract relocations
        let mut relocations = Vec::with_capacity(header.reloc_count as usize);
        let reloc_start = header.reloc_offset as usize;
        for i in 0..header.reloc_count as usize {
            let offset = reloc_start + i * 16;
            if offset + 16 > data.len() {
                return Err(AotError::InvalidCacheFile(
                    "Relocation section extends beyond file".to_string(),
                ));
            }
            let reloc = Relocation::from_bytes(&data[offset..offset + 16])?;
            relocations.push(reloc);
        }

        Ok(CachedCode {
            code,
            relocations,
            source_hash: header.source_hash,
        })
    }

    /// Save cached code to disk
    fn save_to_disk(&self, path: &Path, cached: &CachedCode) -> Result<(), AotError> {
        let mut header = AotCacheHeader::new(cached.source_hash);
        header.code_size = cached.code.len() as u32;
        header.reloc_offset = header.code_offset + header.code_size;
        header.reloc_count = cached.relocations.len() as u32;

        let mut file = File::create(path)?;

        // Write header
        file.write_all(&header.to_bytes())?;

        // Write code
        file.write_all(&cached.code)?;

        // Write relocations
        for reloc in &cached.relocations {
            file.write_all(&reloc.to_bytes())?;
        }

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Ok(subentries) = fs::read_dir(entry.path()) {
                        for subentry in subentries.flatten() {
                            if let Ok(meta) = subentry.metadata() {
                                total_size += meta.len();
                                file_count += 1;
                            }
                        }
                    }
                }
            }
        }

        CacheStats {
            file_count,
            total_size,
            loaded_count: self.loaded.len(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cache files on disk
    pub file_count: usize,
    /// Total size of cache files in bytes
    pub total_size: u64,
    /// Number of entries loaded in memory
    pub loaded_count: usize,
}

// Helper functions

fn cache_key(source_hash: &[u8; 32], func_name: &str) -> String {
    format!("{}_{}", hex_encode(source_hash), func_name)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Compute BLAKE3 hash of source code
pub fn hash_source(source: &str) -> [u8; 32] {
    // Simple hash implementation - in production would use blake3 crate
    let mut hash = [0u8; 32];
    let bytes = source.as_bytes();

    if bytes.is_empty() {
        return hash;
    }

    // Use FNV-1a hash with better mixing
    let mut h1: u64 = 0xcbf29ce484222325;
    let mut h2: u64 = 0x84222325cbf29ce4;
    let mut h3: u64 = 0x22325cbf29ce4842;
    let mut h4: u64 = 0x325cbf29ce484222;

    for (i, &byte) in bytes.iter().enumerate() {
        let b = byte as u64;
        h1 ^= b;
        h1 = h1.wrapping_mul(0x100000001b3);
        h2 ^= b.wrapping_add(i as u64);
        h2 = h2.wrapping_mul(0x1000000001b3);
        h3 ^= b.wrapping_mul((i as u64).wrapping_add(1));
        h3 = h3.wrapping_mul(0x10000000001b3);
        h4 ^= b ^ ((i as u64) << 8);
        h4 = h4.wrapping_mul(0x100000000001b3);
    }

    // Spread the hash values across 32 bytes
    hash[0..8].copy_from_slice(&h1.to_le_bytes());
    hash[8..16].copy_from_slice(&h2.to_le_bytes());
    hash[16..24].copy_from_slice(&h3.to_le_bytes());
    hash[24..32].copy_from_slice(&h4.to_le_bytes());

    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_header_roundtrip() {
        let source_hash = [0x42u8; 32];
        let header = AotCacheHeader::new(source_hash);

        let bytes = header.to_bytes();
        let parsed = AotCacheHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.magic, AOT_MAGIC);
        assert_eq!(parsed.version, AOT_VERSION);
        assert_eq!(parsed.source_hash, source_hash);
    }

    #[test]
    fn test_header_validation() {
        let mut header = AotCacheHeader::new([0u8; 32]);
        assert!(header.validate().is_ok());

        header.magic = [0, 0, 0, 0];
        assert!(header.validate().is_err());

        header.magic = AOT_MAGIC;
        header.version = 999;
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_relocation_roundtrip() {
        let reloc = Relocation {
            offset: 0x1234,
            reloc_type: RelocationType::Abs64,
            addend: -42,
        };

        let bytes = reloc.to_bytes();
        let parsed = Relocation::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.offset, reloc.offset);
        assert_eq!(parsed.reloc_type, reloc.reloc_type);
        assert_eq!(parsed.addend, reloc.addend);
    }

    #[test]
    fn test_cache_put_get() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = AotCache::new(temp_dir.path().to_path_buf()).unwrap();

        let source_hash = hash_source("def foo(): return 42");
        let code = CachedCode::new(vec![0x90, 0x90, 0xC3], source_hash);

        cache.put(&source_hash, "foo", &code).unwrap();

        let loaded = cache.get(&source_hash, "foo").unwrap();
        assert_eq!(loaded.code, code.code);
        assert_eq!(loaded.source_hash, source_hash);
    }

    #[test]
    fn test_cache_invalidate() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = AotCache::new(temp_dir.path().to_path_buf()).unwrap();

        let source_hash = hash_source("def bar(): return 1");
        let code = CachedCode::new(vec![0x90], source_hash);

        cache.put(&source_hash, "bar", &code).unwrap();
        assert!(cache.get(&source_hash, "bar").is_some());

        cache.invalidate(&source_hash).unwrap();

        // Create new cache instance to test disk invalidation
        let mut cache2 = AotCache::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(cache2.get(&source_hash, "bar").is_none());
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = AotCache::new(temp_dir.path().to_path_buf()).unwrap();

        let hash1 = hash_source("def a(): pass");
        let hash2 = hash_source("def b(): pass");

        cache.put(&hash1, "a", &CachedCode::new(vec![1], hash1)).unwrap();
        cache.put(&hash2, "b", &CachedCode::new(vec![2], hash2)).unwrap();

        cache.clear().unwrap();

        assert!(cache.get(&hash1, "a").is_none());
        assert!(cache.get(&hash2, "b").is_none());
    }

    #[test]
    fn test_hash_source() {
        let hash1 = hash_source("def foo(): return 1");
        let hash2 = hash_source("def foo(): return 2");
        let hash3 = hash_source("def foo(): return 1");

        assert_ne!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = AotCache::new(temp_dir.path().to_path_buf()).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.loaded_count, 0);

        let hash = hash_source("test");
        cache.put(&hash, "test", &CachedCode::new(vec![0x90; 100], hash)).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.file_count, 1);
        assert!(stats.total_size > 0);
        assert_eq!(stats.loaded_count, 1);
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("foo"), "foo");
        assert_eq!(sanitize_name("foo.bar"), "foo_bar");
        assert_eq!(sanitize_name("foo<bar>"), "foo_bar_");
        assert_eq!(sanitize_name("__init__"), "__init__");
    }

    #[test]
    fn test_cached_code_with_relocations() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = AotCache::new(temp_dir.path().to_path_buf()).unwrap();

        let source_hash = hash_source("def with_relocs(): pass");
        let mut code = CachedCode::new(vec![0x48, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0, 0xC3], source_hash);
        code.add_relocation(Relocation {
            offset: 2,
            reloc_type: RelocationType::Abs64,
            addend: 0x12345678,
        });

        cache.put(&source_hash, "with_relocs", &code).unwrap();

        // Clear in-memory cache and reload from disk
        cache.loaded.clear();

        let loaded = cache.get(&source_hash, "with_relocs").unwrap();
        assert_eq!(loaded.code, code.code);
        assert_eq!(loaded.relocations.len(), 1);
        assert_eq!(loaded.relocations[0].offset, 2);
        assert_eq!(loaded.relocations[0].reloc_type, RelocationType::Abs64);
        assert_eq!(loaded.relocations[0].addend, 0x12345678);
    }
}
