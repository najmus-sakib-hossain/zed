//! Code storage with memory-mapped pages

use crate::PccError;
use memmap2::{MmapMut, MmapOptions};
use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Initial code cache size (1MB)
const INITIAL_SIZE: usize = 1024 * 1024;

/// Maximum code cache size (256MB)
const MAX_SIZE: usize = 256 * 1024 * 1024;

/// Code storage with memory-mapped pages
pub struct CodeStorage {
    /// Memory-mapped code cache (or in-memory buffer)
    mmap: RwLock<Option<MmapMut>>,
    /// In-memory buffer (for testing)
    memory: RwLock<Option<Vec<u8>>>,
    /// Current allocation offset
    offset: AtomicU64,
    /// Current capacity
    capacity: AtomicU64,
    /// File backing the mmap
    file: RwLock<Option<File>>,
    /// Path to the cache file (unused but kept for future use)
    #[allow(dead_code)]
    path: Option<std::path::PathBuf>,
}

impl CodeStorage {
    /// Create a new in-memory code storage
    pub fn new() -> Self {
        let initial_buffer = vec![0u8; INITIAL_SIZE];
        Self {
            mmap: RwLock::new(None),
            memory: RwLock::new(Some(initial_buffer)),
            offset: AtomicU64::new(8), // Start after header
            capacity: AtomicU64::new(INITIAL_SIZE as u64),
            file: RwLock::new(None),
            path: None,
        }
    }

    /// Open or create a file-backed code storage
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PccError> {
        let path = path.as_ref();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let metadata = file.metadata()?;
        let size = if metadata.len() == 0 {
            // Initialize new file
            file.set_len(INITIAL_SIZE as u64)?;
            INITIAL_SIZE
        } else {
            metadata.len() as usize
        };

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        // Read offset from header (first 8 bytes)
        let offset = if size >= 8 {
            u64::from_le_bytes(mmap[0..8].try_into().unwrap())
        } else {
            8 // Start after header
        };

        Ok(Self {
            mmap: RwLock::new(Some(mmap)),
            memory: RwLock::new(None),
            offset: AtomicU64::new(offset.max(8)),
            capacity: AtomicU64::new(size as u64),
            file: RwLock::new(Some(file)),
            path: Some(path.to_path_buf()),
        })
    }

    /// Allocate space for code with alignment
    pub fn allocate(&self, size: usize, alignment: usize) -> Result<u64, PccError> {
        let alignment = alignment.max(8);

        loop {
            let current = self.offset.load(Ordering::Acquire);
            let aligned = (current as usize + alignment - 1) & !(alignment - 1);
            let new_offset = aligned + size;

            let capacity = self.capacity.load(Ordering::Acquire) as usize;
            if new_offset > capacity {
                self.grow(new_offset)?;
            }

            if self
                .offset
                .compare_exchange(current, new_offset as u64, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(aligned as u64);
            }
        }
    }

    /// Grow the code cache
    fn grow(&self, min_size: usize) -> Result<(), PccError> {
        let current_capacity = self.capacity.load(Ordering::Acquire) as usize;
        if min_size <= current_capacity {
            return Ok(());
        }

        let new_capacity = (min_size * 2).min(MAX_SIZE);
        if new_capacity > MAX_SIZE {
            return Err(PccError::CacheFull);
        }

        // Check if we're using mmap or in-memory
        let mut mmap_guard = self.mmap.write();
        let mut memory_guard = self.memory.write();
        let file_guard = self.file.write();

        if let Some(ref file) = *file_guard {
            file.set_len(new_capacity as u64)?;

            // Remap
            let new_mmap = unsafe { MmapOptions::new().map_mut(file)? };
            *mmap_guard = Some(new_mmap);
        } else if let Some(ref mut memory) = *memory_guard {
            // In-memory: resize buffer
            memory.resize(new_capacity, 0);
        }

        self.capacity.store(new_capacity as u64, Ordering::Release);
        Ok(())
    }

    /// Write code at the given offset
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<(), PccError> {
        let offset = offset as usize;

        // Try mmap first
        let mmap_guard = self.mmap.read();
        if let Some(ref mmap) = *mmap_guard {
            if offset + data.len() > mmap.len() {
                return Err(PccError::CacheFull);
            }

            // Safety: we have exclusive access through the lock
            let mmap_ptr = mmap.as_ptr() as *mut u8;
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), mmap_ptr.add(offset), data.len());
            }
            return Ok(());
        }
        drop(mmap_guard);

        // Try in-memory buffer
        let mut memory_guard = self.memory.write();
        if let Some(ref mut memory) = *memory_guard {
            if offset + data.len() > memory.len() {
                return Err(PccError::CacheFull);
            }
            memory[offset..offset + data.len()].copy_from_slice(data);
            return Ok(());
        }

        Err(PccError::NotFound)
    }

    /// Read code at the given offset
    pub fn read(&self, offset: u64, size: usize) -> Result<Vec<u8>, PccError> {
        let offset = offset as usize;

        // Try mmap first
        let mmap_guard = self.mmap.read();
        if let Some(ref mmap) = *mmap_guard {
            if offset + size > mmap.len() {
                return Err(PccError::NotFound);
            }
            return Ok(mmap[offset..offset + size].to_vec());
        }
        drop(mmap_guard);

        // Try in-memory buffer
        let memory_guard = self.memory.read();
        if let Some(ref memory) = *memory_guard {
            if offset + size > memory.len() {
                return Err(PccError::NotFound);
            }
            return Ok(memory[offset..offset + size].to_vec());
        }

        Err(PccError::NotFound)
    }

    /// Get a pointer to code at the given offset
    pub fn get_ptr(&self, offset: u64) -> Option<*const u8> {
        let offset = offset as usize;

        // Try mmap first
        let mmap_guard = self.mmap.read();
        if let Some(ref mmap) = *mmap_guard {
            if offset < mmap.len() {
                return Some(unsafe { mmap.as_ptr().add(offset) });
            }
        }
        drop(mmap_guard);

        // Try in-memory buffer
        let memory_guard = self.memory.read();
        if let Some(ref memory) = *memory_guard {
            if offset < memory.len() {
                return Some(unsafe { memory.as_ptr().add(offset) });
            }
        }

        None
    }

    /// Flush changes to disk
    pub fn flush(&self) -> Result<(), PccError> {
        // Write current offset to header
        let offset = self.offset.load(Ordering::Acquire);
        self.write(0, &offset.to_le_bytes())?;

        let mmap_guard = self.mmap.read();
        if let Some(ref mmap) = *mmap_guard {
            mmap.flush()?;
        }
        // In-memory storage doesn't need flushing
        Ok(())
    }

    /// Get current allocation offset
    pub fn current_offset(&self) -> u64 {
        self.offset.load(Ordering::Acquire)
    }

    /// Get current capacity
    pub fn capacity(&self) -> u64 {
        self.capacity.load(Ordering::Acquire)
    }

    /// Reset the storage (for testing)
    pub fn reset(&self) {
        self.offset.store(8, Ordering::Release);
    }
}

impl Default for CodeStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_storage_creation() {
        let temp = NamedTempFile::new().unwrap();
        let storage = CodeStorage::open(temp.path()).unwrap();

        assert!(storage.capacity() >= INITIAL_SIZE as u64);
        assert_eq!(storage.current_offset(), 8); // After header
    }

    #[test]
    fn test_allocate_and_write() {
        let temp = NamedTempFile::new().unwrap();
        let storage = CodeStorage::open(temp.path()).unwrap();

        let offset = storage.allocate(100, 16).unwrap();
        assert!(offset >= 8);
        assert_eq!(offset % 16, 0); // Aligned

        let data = vec![0xCC; 100]; // INT3 instructions
        storage.write(offset, &data).unwrap();

        let read_back = storage.read(offset, 100).unwrap();
        assert_eq!(read_back, data);
    }

    #[test]
    fn test_multiple_allocations() {
        let temp = NamedTempFile::new().unwrap();
        let storage = CodeStorage::open(temp.path()).unwrap();

        let off1 = storage.allocate(64, 8).unwrap();
        let off2 = storage.allocate(128, 16).unwrap();
        let off3 = storage.allocate(256, 32).unwrap();

        assert!(off2 > off1);
        assert!(off3 > off2);
        assert_eq!(off2 % 16, 0);
        assert_eq!(off3 % 32, 0);
    }

    #[test]
    fn test_flush_and_reopen() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();

        {
            let storage = CodeStorage::open(&path).unwrap();
            let offset = storage.allocate(100, 8).unwrap();
            storage.write(offset, &[0xAB; 100]).unwrap();
            storage.flush().unwrap();
        }

        {
            let storage = CodeStorage::open(&path).unwrap();
            // Offset should be preserved
            assert!(storage.current_offset() > 8);
        }
    }
}
