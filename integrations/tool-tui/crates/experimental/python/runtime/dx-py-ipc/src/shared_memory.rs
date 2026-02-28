//! Shared memory arena for zero-copy IPC
//!
//! Provides cross-process shared memory for large array transfers.

use memmap2::{MmapMut, MmapOptions};
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};

use crate::protocol::{ArrayDtype, ArrayMetadata};

/// Error types for shared memory operations
#[derive(Debug, thiserror::Error)]
pub enum SharedMemoryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Arena is full")]
    ArenaFull,

    #[error("Invalid offset")]
    InvalidOffset,

    #[error("Invalid handle")]
    InvalidHandle,

    #[error("Size mismatch")]
    SizeMismatch,
}

/// Shared memory arena for allocating objects
pub struct SharedMemoryArena {
    /// Memory-mapped region
    mmap: MmapMut,
    /// Current allocation offset (bump allocator)
    offset: Mutex<usize>,
    /// Arena capacity
    capacity: usize,
    /// Arena name for cross-process access
    name: String,
    /// Backing file (kept open)
    _file: File,
}

impl SharedMemoryArena {
    /// Create a new shared memory arena
    pub fn create(name: &str, capacity: usize) -> Result<Self, SharedMemoryError> {
        let path = Self::get_path(name);

        // Create or truncate the file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        // Set the file size
        file.set_len(capacity as u64)?;

        // Memory map the file
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(Self {
            mmap,
            offset: Mutex::new(0),
            capacity,
            name: name.to_string(),
            _file: file,
        })
    }

    /// Open an existing shared memory arena
    pub fn open(name: &str) -> Result<Self, SharedMemoryError> {
        let path = Self::get_path(name);

        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        let capacity = file.metadata()?.len() as usize;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(Self {
            mmap,
            offset: Mutex::new(0),
            capacity,
            name: name.to_string(),
            _file: file,
        })
    }

    /// Get the platform-specific path for shared memory
    fn get_path(name: &str) -> std::path::PathBuf {
        #[cfg(target_os = "windows")]
        {
            std::env::temp_dir().join(format!("dx_py_shm_{}", name))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::path::PathBuf::from(format!("/dev/shm/dx_py_{}", name))
        }
    }

    /// Allocate space in the arena
    pub fn alloc(&self, size: usize, align: usize) -> Result<usize, SharedMemoryError> {
        let mut offset = self.offset.lock();

        // Align the offset
        let aligned = (*offset + align - 1) & !(align - 1);
        let new_offset = aligned + size;

        if new_offset > self.capacity {
            return Err(SharedMemoryError::ArenaFull);
        }

        *offset = new_offset;
        Ok(aligned)
    }

    /// Write data to the arena at a specific offset
    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<(), SharedMemoryError> {
        if offset + data.len() > self.capacity {
            return Err(SharedMemoryError::InvalidOffset);
        }

        self.mmap[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Get a slice of data from the arena
    pub fn get(&self, offset: usize, len: usize) -> Result<&[u8], SharedMemoryError> {
        if offset + len > self.capacity {
            return Err(SharedMemoryError::InvalidOffset);
        }

        Ok(&self.mmap[offset..offset + len])
    }

    /// Get a mutable slice of data from the arena
    pub fn get_mut(&mut self, offset: usize, len: usize) -> Result<&mut [u8], SharedMemoryError> {
        if offset + len > self.capacity {
            return Err(SharedMemoryError::InvalidOffset);
        }

        Ok(&mut self.mmap[offset..offset + len])
    }

    /// Get the arena name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the arena capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the current allocation offset
    pub fn used(&self) -> usize {
        *self.offset.lock()
    }

    /// Reset the arena (clear all allocations)
    pub fn reset(&self) {
        *self.offset.lock() = 0;
    }

    /// Flush changes to disk
    pub fn flush(&self) -> Result<(), SharedMemoryError> {
        self.mmap.flush()?;
        Ok(())
    }
}

/// Handle to a shared array in shared memory
#[derive(Debug, Clone)]
pub struct SharedArrayHandle {
    /// Arena name
    pub arena_name: String,
    /// Offset in the arena
    pub offset: usize,
    /// Array metadata
    pub metadata: ArrayMetadata,
}

impl SharedArrayHandle {
    /// Create a shared array from data
    pub fn from_array(
        arena: &mut SharedMemoryArena,
        data: &[u8],
        metadata: ArrayMetadata,
    ) -> Result<Self, SharedMemoryError> {
        if data.len() != metadata.byte_size() {
            return Err(SharedMemoryError::SizeMismatch);
        }

        // Allocate space with 64-byte alignment for SIMD
        let offset = arena.alloc(data.len(), 64)?;

        // Copy data to shared memory
        arena.write(offset, data)?;

        Ok(Self {
            arena_name: arena.name().to_string(),
            offset,
            metadata,
        })
    }

    /// Get the array data as a slice
    pub fn as_slice<'a>(
        &self,
        arena: &'a SharedMemoryArena,
    ) -> Result<&'a [u8], SharedMemoryError> {
        arena.get(self.offset, self.metadata.byte_size())
    }

    /// Get the array data as a typed slice
    pub fn as_typed_slice<'a, T>(
        &self,
        arena: &'a SharedMemoryArena,
    ) -> Result<&'a [T], SharedMemoryError> {
        let bytes = self.as_slice(arena)?;
        let len = bytes.len() / std::mem::size_of::<T>();

        // Safety: We ensure alignment during allocation
        let ptr = bytes.as_ptr() as *const T;
        Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
    }

    /// Serialize the handle for cross-process transfer
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Arena name length and data
        let name_bytes = self.arena_name.as_bytes();
        bytes.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name_bytes);

        // Offset
        bytes.extend_from_slice(&(self.offset as u64).to_le_bytes());

        // Metadata
        bytes.push(self.metadata.dtype as u8);
        bytes.push(self.metadata.ndim);
        for i in 0..8 {
            bytes.extend_from_slice(&self.metadata.shape[i].to_le_bytes());
        }
        for i in 0..8 {
            bytes.extend_from_slice(&self.metadata.strides[i].to_le_bytes());
        }
        bytes.extend_from_slice(&self.metadata.size.to_le_bytes());

        bytes
    }

    /// Deserialize a handle from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SharedMemoryError> {
        if bytes.len() < 4 {
            return Err(SharedMemoryError::InvalidHandle);
        }

        let mut pos = 0;

        // Arena name
        let name_len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        pos += 4;

        if pos + name_len > bytes.len() {
            return Err(SharedMemoryError::InvalidHandle);
        }

        let arena_name = String::from_utf8_lossy(&bytes[pos..pos + name_len]).to_string();
        pos += name_len;

        // Offset
        if pos + 8 > bytes.len() {
            return Err(SharedMemoryError::InvalidHandle);
        }
        let offset = u64::from_le_bytes([
            bytes[pos],
            bytes[pos + 1],
            bytes[pos + 2],
            bytes[pos + 3],
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        pos += 8;

        // Metadata
        if pos + 2 + 64 + 64 + 8 > bytes.len() {
            return Err(SharedMemoryError::InvalidHandle);
        }

        let dtype = ArrayDtype::from_u8(bytes[pos]).ok_or(SharedMemoryError::InvalidHandle)?;
        pos += 1;
        let ndim = bytes[pos];
        pos += 1;

        let mut shape = [0u64; 8];
        for item in &mut shape {
            *item = u64::from_le_bytes([
                bytes[pos],
                bytes[pos + 1],
                bytes[pos + 2],
                bytes[pos + 3],
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]);
            pos += 8;
        }

        let mut strides = [0i64; 8];
        for item in &mut strides {
            *item = i64::from_le_bytes([
                bytes[pos],
                bytes[pos + 1],
                bytes[pos + 2],
                bytes[pos + 3],
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]);
            pos += 8;
        }

        let size = u64::from_le_bytes([
            bytes[pos],
            bytes[pos + 1],
            bytes[pos + 2],
            bytes[pos + 3],
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]);

        Ok(Self {
            arena_name,
            offset,
            metadata: ArrayMetadata {
                dtype,
                ndim,
                shape,
                strides,
                size,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_create_and_write() {
        let mut arena = SharedMemoryArena::create("test_arena", 4096).unwrap();

        let data = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let offset = arena.alloc(8, 8).unwrap();
        arena.write(offset, &data).unwrap();

        let read = arena.get(offset, 8).unwrap();
        assert_eq!(read, &data);

        // Cleanup
        std::fs::remove_file(SharedMemoryArena::get_path("test_arena")).ok();
    }

    #[test]
    fn test_shared_array_handle() {
        let mut arena = SharedMemoryArena::create("test_array", 4096).unwrap();

        let data: Vec<u8> = (0..64).collect();
        let metadata = ArrayMetadata::new(ArrayDtype::UInt8, &[64]);

        let handle = SharedArrayHandle::from_array(&mut arena, &data, metadata).unwrap();

        let read = handle.as_slice(&arena).unwrap();
        assert_eq!(read, &data[..]);

        // Cleanup
        std::fs::remove_file(SharedMemoryArena::get_path("test_array")).ok();
    }

    #[test]
    fn test_handle_serialization() {
        let metadata = ArrayMetadata::new(ArrayDtype::Float64, &[10, 20]);
        let handle = SharedArrayHandle {
            arena_name: "test".to_string(),
            offset: 1024,
            metadata,
        };

        let bytes = handle.to_bytes();
        let restored = SharedArrayHandle::from_bytes(&bytes).unwrap();

        assert_eq!(restored.arena_name, "test");
        assert_eq!(restored.offset, 1024);
        assert_eq!(restored.metadata.dtype, ArrayDtype::Float64);
        assert_eq!(restored.metadata.ndim, 2);
    }
}
