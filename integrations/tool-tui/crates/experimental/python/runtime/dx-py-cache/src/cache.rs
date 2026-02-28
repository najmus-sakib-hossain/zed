//! Reactive cache implementation

use dashmap::DashMap;
use memmap2::{Mmap, MmapOptions};
use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::entry::{CacheEntry, CompilationTier};

/// Error types for cache operations
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cache is corrupted")]
    Corrupted,

    #[error("Entry not found")]
    NotFound,

    #[error("Cache is full")]
    Full,

    #[error("Invalid path")]
    InvalidPath,
}

const CACHE_MAGIC: [u8; 8] = *b"DXPYCACH";
const CACHE_VERSION: u32 = 1;
const HEADER_SIZE: usize = 32;
const INDEX_ENTRY_SIZE: usize = CacheEntry::serialized_size() + 256; // entry + path

/// Reactive bytecode cache with O(1) lookup
pub struct ReactiveCache {
    /// Cache file path (unused but kept for future use)
    #[allow(dead_code)]
    path: PathBuf,
    /// Index mapping path hash to cache entry
    index: DashMap<u64, (String, CacheEntry)>,
    /// Memory-mapped cache data
    mmap: Option<RwLock<Mmap>>,
    /// Cache file for writing
    file: RwLock<File>,
    /// Current data offset for new entries
    data_offset: AtomicU64,
    /// Maximum cache size
    max_size: u64,
}

impl ReactiveCache {
    /// Open or create a cache file
    pub fn open<P: AsRef<Path>>(path: P, max_size: u64) -> Result<Self, CacheError> {
        let path = path.as_ref().to_path_buf();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        let metadata = file.metadata()?;
        let file_size = metadata.len();

        let mut cache = Self {
            path,
            index: DashMap::new(),
            mmap: None,
            file: RwLock::new(file),
            data_offset: AtomicU64::new(HEADER_SIZE as u64),
            max_size,
        };

        if file_size > 0 {
            cache.load_index()?;
        } else {
            cache.initialize()?;
        }

        Ok(cache)
    }

    /// Initialize a new cache file
    fn initialize(&self) -> Result<(), CacheError> {
        let mut file = self.file.write();

        // Write header
        file.write_all(&CACHE_MAGIC)?;
        file.write_all(&CACHE_VERSION.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?; // entry_count
        file.write_all(&(HEADER_SIZE as u64).to_le_bytes())?; // data_offset
        file.write_all(&0u64.to_le_bytes())?; // data_size

        file.flush()?;

        Ok(())
    }

    /// Load the index from an existing cache file
    fn load_index(&mut self) -> Result<(), CacheError> {
        let file = self.file.read();

        // Memory map the file
        let mmap = unsafe { MmapOptions::new().map(&*file)? };

        // Validate header
        if mmap.len() < HEADER_SIZE {
            return Err(CacheError::Corrupted);
        }

        if mmap[0..8] != CACHE_MAGIC {
            return Err(CacheError::Corrupted);
        }

        let version = u32::from_le_bytes([mmap[8], mmap[9], mmap[10], mmap[11]]);
        if version > CACHE_VERSION {
            return Err(CacheError::Corrupted);
        }

        let entry_count = u32::from_le_bytes([mmap[12], mmap[13], mmap[14], mmap[15]]);
        let data_offset = u64::from_le_bytes([
            mmap[16], mmap[17], mmap[18], mmap[19], mmap[20], mmap[21], mmap[22], mmap[23],
        ]);

        self.data_offset.store(data_offset, Ordering::SeqCst);

        // Load index entries
        let mut pos = HEADER_SIZE;
        for _ in 0..entry_count {
            if pos + INDEX_ENTRY_SIZE > mmap.len() {
                break;
            }

            // Read path length and path
            let path_len = u16::from_le_bytes([mmap[pos], mmap[pos + 1]]) as usize;
            pos += 2;

            if pos + path_len + CacheEntry::serialized_size() > mmap.len() {
                break;
            }

            let path_str = String::from_utf8_lossy(&mmap[pos..pos + path_len]).to_string();
            pos += 256 - 2; // Fixed path field size

            // Read entry
            let entry_bytes: &[u8] = &mmap[pos..pos + CacheEntry::serialized_size()];
            if let Some(entry) = CacheEntry::from_bytes(entry_bytes) {
                let path_hash = Self::hash_path(&path_str);
                self.index.insert(path_hash, (path_str, entry));
            }

            pos += CacheEntry::serialized_size();
        }

        self.mmap = Some(RwLock::new(mmap));

        Ok(())
    }

    /// Hash a path for O(1) lookup
    #[inline]
    fn hash_path(path: &str) -> u64 {
        let hash = blake3::hash(path.as_bytes());
        let bytes = hash.as_bytes();
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }

    /// Get a cached entry by path
    pub fn get(&self, path: &str) -> Option<CacheEntry> {
        let path_hash = Self::hash_path(path);
        self.index.get(&path_hash).map(|r| r.1.clone())
    }

    /// Get cached data by path
    pub fn get_data(&self, path: &str) -> Option<Vec<u8>> {
        let entry = self.get(path)?;

        // Try to read from mmap first
        if let Some(ref mmap_lock) = self.mmap {
            let mmap = mmap_lock.read();
            let start = entry.data_offset as usize;
            let end = start + entry.data_size as usize;

            if end <= mmap.len() {
                return Some(mmap[start..end].to_vec());
            }
        }

        // Fall back to reading from file
        use std::io::{Read, Seek, SeekFrom};
        let mut file = self.file.write();
        if file.seek(SeekFrom::Start(entry.data_offset)).is_ok() {
            let mut data = vec![0u8; entry.data_size as usize];
            if file.read_exact(&mut data).is_ok() {
                return Some(data);
            }
        }

        None
    }

    /// Check if an entry is valid (quick check using mtime)
    pub fn is_valid_quick(&self, path: &str, current_mtime: u64) -> bool {
        if let Some(entry) = self.get(path) {
            entry.is_valid_quick(current_mtime)
        } else {
            false
        }
    }

    /// Check if an entry is valid (full check using content hash)
    pub fn validate_full(&self, path: &str, source_content: &[u8]) -> bool {
        if let Some(entry) = self.get(path) {
            entry.validate_full(source_content)
        } else {
            false
        }
    }

    /// Store a new cache entry
    pub fn store(
        &self,
        path: &str,
        source_content: &[u8],
        compiled_data: &[u8],
        tier: CompilationTier,
        source_mtime: u64,
    ) -> Result<(), CacheError> {
        let source_hash = *blake3::hash(source_content).as_bytes();

        // Allocate space for the data
        let data_offset = self.allocate(compiled_data.len())?;

        // Write the data
        {
            let mut file = self.file.write();
            file.seek(SeekFrom::Start(data_offset))?;
            file.write_all(compiled_data)?;
            file.flush()?;
        }

        // Create and store the entry
        let entry = CacheEntry::new(
            source_hash,
            data_offset,
            compiled_data.len() as u32,
            tier,
            source_mtime,
        );

        let path_hash = Self::hash_path(path);
        self.index.insert(path_hash, (path.to_string(), entry));

        Ok(())
    }

    /// Allocate space in the cache
    fn allocate(&self, size: usize) -> Result<u64, CacheError> {
        let aligned_size = (size + 63) & !63; // 64-byte alignment
        let offset = self.data_offset.fetch_add(aligned_size as u64, Ordering::SeqCst);

        if offset + aligned_size as u64 > self.max_size {
            return Err(CacheError::Full);
        }

        // Ensure file is large enough
        {
            let file = self.file.read();
            let required_size = offset + aligned_size as u64;
            if file.metadata()?.len() < required_size {
                drop(file);
                let file = self.file.write();
                file.set_len(required_size)?;
            }
        }

        Ok(offset)
    }

    /// Invalidate a cache entry
    pub fn invalidate(&self, path: &str) {
        let path_hash = Self::hash_path(path);
        self.index.remove(&path_hash);
    }

    /// Flush the cache to disk
    pub fn flush(&self) -> Result<(), CacheError> {
        // Write index to file
        let mut file = self.file.write();

        // Update header
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&CACHE_MAGIC)?;
        file.write_all(&CACHE_VERSION.to_le_bytes())?;
        file.write_all(&(self.index.len() as u32).to_le_bytes())?;
        file.write_all(&self.data_offset.load(Ordering::SeqCst).to_le_bytes())?;
        file.write_all(&0u64.to_le_bytes())?; // data_size placeholder

        // Write index entries
        for entry in self.index.iter() {
            let (path, cache_entry) = entry.value();

            // Write path (fixed 256 bytes)
            let path_bytes = path.as_bytes();
            let path_len = path_bytes.len().min(254) as u16;
            file.write_all(&path_len.to_le_bytes())?;
            file.write_all(&path_bytes[..path_len as usize])?;

            // Pad to 254 bytes
            let padding = 254 - path_len as usize;
            file.write_all(&vec![0u8; padding])?;

            // Write entry
            file.write_all(&cache_entry.to_bytes())?;
        }

        file.flush()?;

        // Note: Read-only mmap doesn't need flushing
        // The file handle flush above is sufficient

        Ok(())
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.index.clear();
        self.data_offset.store(HEADER_SIZE as u64, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_cache_create() {
        let temp = NamedTempFile::new().unwrap();
        let cache = ReactiveCache::open(temp.path(), 1024 * 1024).unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_store_and_get() {
        let temp = NamedTempFile::new().unwrap();
        let cache = ReactiveCache::open(temp.path(), 1024 * 1024).unwrap();

        let source = b"print('hello')";
        let compiled = b"compiled bytecode";

        cache
            .store("test.py", source, compiled, CompilationTier::Interpreter, 12345)
            .unwrap();

        let entry = cache.get("test.py").unwrap();
        assert_eq!(entry.tier, CompilationTier::Interpreter);
        assert_eq!(entry.source_mtime, 12345);

        let data = cache.get_data("test.py").unwrap();
        assert_eq!(data, compiled);
    }

    #[test]
    fn test_cache_invalidate() {
        let temp = NamedTempFile::new().unwrap();
        let cache = ReactiveCache::open(temp.path(), 1024 * 1024).unwrap();

        cache
            .store("test.py", b"source", b"data", CompilationTier::Interpreter, 0)
            .unwrap();
        assert!(cache.get("test.py").is_some());

        cache.invalidate("test.py");
        assert!(cache.get("test.py").is_none());
    }

    #[test]
    fn test_cache_validation() {
        let temp = NamedTempFile::new().unwrap();
        let cache = ReactiveCache::open(temp.path(), 1024 * 1024).unwrap();

        let source = b"print('hello')";
        cache
            .store("test.py", source, b"data", CompilationTier::Interpreter, 100)
            .unwrap();

        // Quick validation
        assert!(cache.is_valid_quick("test.py", 100));
        assert!(!cache.is_valid_quick("test.py", 101));

        // Full validation
        assert!(cache.validate_full("test.py", source));
        assert!(!cache.validate_full("test.py", b"different"));
    }
}
