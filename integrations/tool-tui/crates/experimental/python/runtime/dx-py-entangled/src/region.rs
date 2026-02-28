//! Shared memory region for cross-process access

use crate::EntangledError;
use memmap2::{MmapMut, MmapOptions};
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Region header size
const HEADER_SIZE: usize = 64;

/// Shared memory region header
#[repr(C)]
struct RegionHeader {
    /// Magic bytes "ENTG"
    magic: [u8; 4],
    /// Version
    version: u32,
    /// Total size
    size: u64,
    /// Current allocation offset
    alloc_offset: AtomicU64,
    /// Number of objects
    object_count: AtomicU64,
    /// Reserved
    _reserved: [u8; 32],
}

/// Shared memory region for cross-process access
pub struct SharedMemoryRegion {
    /// Region name
    name: String,
    /// Memory-mapped file
    mmap: MmapMut,
    /// File handle
    _file: File,
    /// Path to the backing file (kept for cleanup/debugging)
    #[allow(dead_code)]
    path: PathBuf,
    /// Allocation mutex
    alloc_mutex: Mutex<()>,
}

impl SharedMemoryRegion {
    /// Create a new shared memory region
    pub fn create(name: &str, size: usize) -> Result<Self, EntangledError> {
        let size = size.max(HEADER_SIZE * 2);
        let path = Self::get_path(name);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        file.set_len(size as u64)?;

        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        // Initialize header
        let header = unsafe { &mut *(mmap.as_mut_ptr() as *mut RegionHeader) };
        header.magic = *b"ENTG";
        header.version = 1;
        header.size = size as u64;
        header.alloc_offset = AtomicU64::new(HEADER_SIZE as u64);
        header.object_count = AtomicU64::new(0);

        mmap.flush()?;

        Ok(Self {
            name: name.to_string(),
            mmap,
            _file: file,
            path,
            alloc_mutex: Mutex::new(()),
        })
    }

    /// Open an existing shared memory region
    pub fn open(name: &str) -> Result<Self, EntangledError> {
        let path = Self::get_path(name);

        if !path.exists() {
            return Err(EntangledError::RegionNotFound(name.to_string()));
        }

        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        // Validate header
        let header = unsafe { &*(mmap.as_ptr() as *const RegionHeader) };
        if &header.magic != b"ENTG" {
            return Err(EntangledError::RegionNotFound(name.to_string()));
        }

        Ok(Self {
            name: name.to_string(),
            mmap,
            _file: file,
            path,
            alloc_mutex: Mutex::new(()),
        })
    }

    /// Get the path for a region name
    fn get_path(name: &str) -> PathBuf {
        let temp_dir = std::env::temp_dir();
        temp_dir.join("dx-py-entangled").join(format!("{}.shm", name))
    }

    /// Allocate space in the region
    pub fn allocate(&self, size: usize, alignment: usize) -> Result<u64, EntangledError> {
        let _guard = self.alloc_mutex.lock();

        let header = self.header();
        let alignment = alignment.max(8);

        loop {
            let current = header.alloc_offset.load(Ordering::Acquire);
            let aligned = (current as usize + alignment - 1) & !(alignment - 1);
            let new_offset = aligned + size;

            if new_offset > header.size as usize {
                return Err(EntangledError::RegionFull);
            }

            if header
                .alloc_offset
                .compare_exchange(current, new_offset as u64, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                header.object_count.fetch_add(1, Ordering::Relaxed);
                return Ok(aligned as u64);
            }
        }
    }

    /// Write data at an offset
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<(), EntangledError> {
        let offset = offset as usize;
        if offset + data.len() > self.mmap.len() {
            return Err(EntangledError::RegionFull);
        }

        let ptr = self.mmap.as_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.add(offset), data.len());
        }

        Ok(())
    }

    /// Read data at an offset
    pub fn read(&self, offset: u64, size: usize) -> Result<&[u8], EntangledError> {
        let offset = offset as usize;
        if offset + size > self.mmap.len() {
            return Err(EntangledError::ObjectNotFound);
        }

        Ok(&self.mmap[offset..offset + size])
    }

    /// Get a mutable pointer to data at an offset
    pub fn get_mut_ptr(&self, offset: u64) -> *mut u8 {
        unsafe { (self.mmap.as_ptr() as *mut u8).add(offset as usize) }
    }

    /// Get a pointer to data at an offset
    pub fn get_ptr(&self, offset: u64) -> *const u8 {
        unsafe { self.mmap.as_ptr().add(offset as usize) }
    }

    /// Get the region header
    fn header(&self) -> &RegionHeader {
        unsafe { &*(self.mmap.as_ptr() as *const RegionHeader) }
    }

    /// Get region name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get region size
    pub fn size(&self) -> u64 {
        self.header().size
    }

    /// Get current allocation offset
    pub fn allocated(&self) -> u64 {
        self.header().alloc_offset.load(Ordering::Acquire)
    }

    /// Get object count
    pub fn object_count(&self) -> u64 {
        self.header().object_count.load(Ordering::Acquire)
    }

    /// Flush changes to disk
    pub fn flush(&self) -> Result<(), EntangledError> {
        self.mmap.flush()?;
        Ok(())
    }

    /// Memory barrier for consistency
    pub fn memory_barrier(&self) {
        std::sync::atomic::fence(Ordering::SeqCst);
    }
}

impl Drop for SharedMemoryRegion {
    fn drop(&mut self) {
        // Optionally clean up the file
        // For now, we leave it for other processes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

    /// Default size for test regions
    const DEFAULT_SIZE: usize = 1024 * 1024; // 1MB

    // Global counter for unique test names
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_name(prefix: &str) -> String {
        let counter = TEST_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
        format!("{}_{}_{}", prefix, std::process::id(), counter)
    }

    #[allow(dead_code)]
    fn cleanup_region(name: &str) {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("dx-py-entangled").join(format!("{}.shm", name));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_create_region() {
        let name = unique_name("test_create");
        let region = SharedMemoryRegion::create(&name, DEFAULT_SIZE).unwrap();

        assert_eq!(region.name(), name);
        assert!(region.size() >= DEFAULT_SIZE as u64);
        assert_eq!(region.object_count(), 0);

        // Cleanup
        let path = region.path.clone();
        drop(region);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_allocate_and_write() {
        let name = unique_name("test_alloc");
        let region = SharedMemoryRegion::create(&name, DEFAULT_SIZE).unwrap();

        let offset = region.allocate(100, 8).unwrap();
        assert!(offset >= HEADER_SIZE as u64);

        let data = vec![0xAB; 100];
        region.write(offset, &data).unwrap();

        let read_back = region.read(offset, 100).unwrap();
        assert_eq!(read_back, &data[..]);

        // Cleanup
        let path = region.path.clone();
        drop(region);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_open_existing() {
        let name = unique_name("test_open");

        let path = {
            let region = SharedMemoryRegion::create(&name, DEFAULT_SIZE).unwrap();
            let offset = region.allocate(50, 8).unwrap();
            region.write(offset, &[0xCD; 50]).unwrap();
            region.flush().unwrap();
            region.path.clone()
        };

        let region = SharedMemoryRegion::open(&name).unwrap();
        assert_eq!(region.object_count(), 1);

        // Cleanup
        drop(region);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_alignment() {
        let name = unique_name("test_align");
        let region = SharedMemoryRegion::create(&name, DEFAULT_SIZE).unwrap();

        let off1 = region.allocate(10, 8).unwrap();
        let off2 = region.allocate(10, 16).unwrap();
        let off3 = region.allocate(10, 32).unwrap();

        assert_eq!(off1 % 8, 0);
        assert_eq!(off2 % 16, 0);
        assert_eq!(off3 % 32, 0);

        // Cleanup
        let path = region.path.clone();
        drop(region);
        std::fs::remove_file(path).ok();
    }
}
