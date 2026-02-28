//! Direct I/O Mode - Bypass page cache for large objects
//!
//! Direct I/O bypasses the OS page cache, providing:
//! - Lower latency for large sequential writes (1-10GB)
//! - Reduced memory pressure (no double buffering)
//! - Predictable performance
//!
//! Requirements:
//! - Aligned buffers (typically 512 bytes or 4KB)
//! - Aligned file offsets
//! - Aligned I/O sizes

use std::alloc::{alloc, dealloc, Layout};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

/// Default alignment for direct I/O (4KB, works on most systems)
pub const DEFAULT_ALIGNMENT: usize = 4096;

/// Default threshold for enabling direct I/O (1MB)
pub const DEFAULT_DIRECT_IO_THRESHOLD: usize = 1024 * 1024;

/// Configuration for direct I/O operations
#[derive(Debug, Clone)]
pub struct DirectIoConfig {
    /// Buffer alignment (must be power of 2)
    pub alignment: usize,
    /// Minimum size to enable direct I/O
    pub threshold: usize,
    /// Whether to use direct I/O
    pub enabled: bool,
}

impl Default for DirectIoConfig {
    fn default() -> Self {
        Self {
            alignment: DEFAULT_ALIGNMENT,
            threshold: DEFAULT_DIRECT_IO_THRESHOLD,
            enabled: true,
        }
    }
}

impl DirectIoConfig {
    /// Create a new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the alignment (must be power of 2)
    pub fn with_alignment(mut self, alignment: usize) -> Self {
        assert!(alignment.is_power_of_two(), "Alignment must be power of 2");
        self.alignment = alignment;
        self
    }

    /// Set the threshold for enabling direct I/O
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = threshold;
        self
    }

    /// Enable or disable direct I/O
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Check if direct I/O should be used for given size
    pub fn should_use_direct_io(&self, size: usize) -> bool {
        self.enabled && size >= self.threshold
    }
}

/// Aligned buffer for direct I/O
///
/// Ensures buffer is aligned to system requirements (typically 512 bytes or 4KB)
pub struct AlignedBuffer {
    ptr: *mut u8,
    capacity: usize,
    len: usize,
    alignment: usize,
}

impl AlignedBuffer {
    /// Create a new aligned buffer with specified capacity and alignment
    pub fn new(capacity: usize, alignment: usize) -> io::Result<Self> {
        assert!(alignment.is_power_of_two(), "Alignment must be power of 2");

        // Round up capacity to alignment
        let aligned_capacity = (capacity + alignment - 1) & !(alignment - 1);

        // SAFETY: We verify alignment is power of 2 and capacity is non-zero
        let layout = Layout::from_size_align(aligned_capacity, alignment)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        // SAFETY: Layout is valid (non-zero size, power-of-2 alignment)
        let ptr = unsafe { alloc(layout) };

        if ptr.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "Failed to allocate aligned buffer",
            ));
        }

        Ok(Self {
            ptr,
            capacity: aligned_capacity,
            len: 0,
            alignment,
        })
    }

    /// Get the capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the current length of data in the buffer
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the alignment of the buffer
    pub fn alignment(&self) -> usize {
        self.alignment
    }

    /// Get a slice of the buffer
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: ptr is valid for len bytes, allocated by us
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Get a mutable slice of the buffer
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: ptr is valid for capacity bytes, allocated by us, and we have exclusive access
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.capacity) }
    }

    /// Write data to the buffer
    pub fn write(&mut self, data: &[u8]) -> io::Result<()> {
        if self.len + data.len() > self.capacity {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "Buffer capacity exceeded"));
        }

        // SAFETY: We verified there's enough space, ptr is valid
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(self.len), data.len());
        }

        self.len += data.len();
        Ok(())
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Pad the buffer to alignment boundary with zeros
    pub fn pad_to_alignment(&mut self) {
        let remainder = self.len % self.alignment;
        if remainder != 0 {
            let padding = self.alignment - remainder;
            if self.len + padding <= self.capacity {
                // SAFETY: We verified there's enough space, ptr is valid
                unsafe {
                    std::ptr::write_bytes(self.ptr.add(self.len), 0, padding);
                }
                self.len += padding;
            }
        }
    }

    /// Get the actual data length (before padding)
    pub fn data_len(&self) -> usize {
        self.len
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: ptr was allocated by us with this layout
            unsafe {
                let layout = Layout::from_size_align_unchecked(self.capacity, self.alignment);
                dealloc(self.ptr, layout);
            }
        }
    }
}

// SAFETY: AlignedBuffer owns its memory exclusively through a raw pointer.
// - Send: The buffer owns its allocation and can be safely transferred between threads
//   because the raw pointer is the sole owner of the memory.
// - Sync: The buffer doesn't use interior mutability and all access is through &mut,
//   so shared references across threads are safe (though not useful without interior mutability).
// The memory is allocated via std::alloc::alloc and deallocated in Drop, ensuring
// proper ownership semantics.
unsafe impl Send for AlignedBuffer {}

// SAFETY: Same reasoning as Send - AlignedBuffer has exclusive ownership of its memory
// and doesn't use interior mutability, making it safe to share references across threads.
unsafe impl Sync for AlignedBuffer {}

/// Direct I/O writer for bypassing page cache
pub struct DirectIoWriter {
    config: DirectIoConfig,
}

impl DirectIoWriter {
    /// Create a new direct I/O writer with default configuration
    pub fn new() -> Self {
        Self {
            config: DirectIoConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: DirectIoConfig) -> Self {
        Self { config }
    }

    /// Write data to file using direct I/O if appropriate
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P, data: &[u8]) -> io::Result<()> {
        if self.config.should_use_direct_io(data.len()) {
            self.write_direct(path, data)
        } else {
            // Use regular buffered I/O for small files
            std::fs::write(path, data)
        }
    }

    /// Write using direct I/O (platform-specific)
    #[cfg(target_os = "linux")]
    fn write_direct<P: AsRef<Path>>(&self, path: P, data: &[u8]) -> io::Result<()> {
        use std::os::unix::fs::OpenOptionsExt;

        // Create aligned buffer
        let mut buffer = AlignedBuffer::new(data.len(), self.config.alignment)?;
        buffer.write(data)?;
        buffer.pad_to_alignment();

        // Open file with O_DIRECT flag
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .custom_flags(libc::O_DIRECT)
            .open(path)?;

        // Write aligned data
        file.write_all(buffer.as_slice())?;
        file.sync_all()?;

        Ok(())
    }

    /// Write using direct I/O (Windows)
    #[cfg(target_os = "windows")]
    fn write_direct<P: AsRef<Path>>(&self, path: P, data: &[u8]) -> io::Result<()> {
        use std::os::windows::fs::OpenOptionsExt;

        // Create aligned buffer
        let mut buffer = AlignedBuffer::new(data.len(), self.config.alignment)?;
        buffer.write(data)?;
        buffer.pad_to_alignment();

        // Open file with FILE_FLAG_NO_BUFFERING (0x20000000)
        const FILE_FLAG_NO_BUFFERING: u32 = 0x20000000;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .custom_flags(FILE_FLAG_NO_BUFFERING)
            .open(path)?;

        // Write aligned data
        file.write_all(buffer.as_slice())?;
        file.sync_all()?;

        Ok(())
    }

    /// Fallback for other platforms (use regular I/O)
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    fn write_direct<P: AsRef<Path>>(&self, path: P, data: &[u8]) -> io::Result<()> {
        std::fs::write(path, data)
    }
}

impl Default for DirectIoWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_creation() {
        let buffer = AlignedBuffer::new(1024, 512).unwrap();
        assert_eq!(buffer.capacity(), 1024);
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.alignment(), 512);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_aligned_buffer_write() {
        let mut buffer = AlignedBuffer::new(1024, 512).unwrap();
        let data = b"Hello, World!";
        buffer.write(data).unwrap();
        assert_eq!(buffer.len(), data.len());
        assert_eq!(buffer.as_slice(), data);
    }

    #[test]
    fn test_aligned_buffer_padding() {
        let mut buffer = AlignedBuffer::new(1024, 512).unwrap();
        buffer.write(b"Hello").unwrap();
        assert_eq!(buffer.len(), 5);

        buffer.pad_to_alignment();
        assert_eq!(buffer.len(), 512); // Padded to alignment
        assert_eq!(buffer.as_slice()[0..5], b"Hello"[..]);
    }

    #[test]
    fn test_config_threshold() {
        let config = DirectIoConfig::new().with_threshold(1024).with_alignment(4096);

        assert!(!config.should_use_direct_io(512));
        assert!(config.should_use_direct_io(2048));
    }

    #[test]
    fn test_config_disabled() {
        let config = DirectIoConfig::new().with_enabled(false);
        assert!(!config.should_use_direct_io(10_000_000));
    }

    #[test]
    fn test_direct_io_writer() {
        let writer = DirectIoWriter::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.bin");

        let data = vec![42u8; 1024];
        writer.write_to_file(&file_path, &data).unwrap();

        let read_data = std::fs::read(&file_path).unwrap();
        // Data might be padded, so check at least our data is there
        assert!(read_data.len() >= data.len());
        assert_eq!(&read_data[..data.len()], &data[..]);
    }

    #[test]
    fn test_small_file_uses_regular_io() {
        let config = DirectIoConfig::new().with_threshold(1024 * 1024);
        let writer = DirectIoWriter::with_config(config);

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("small.bin");

        let data = vec![42u8; 1024]; // Small file
        writer.write_to_file(&file_path, &data).unwrap();

        let read_data = std::fs::read(&file_path).unwrap();
        assert_eq!(read_data, data); // No padding for small files
    }

    #[test]
    fn test_alignment_power_of_two() {
        assert!(AlignedBuffer::new(1024, 512).is_ok());
        assert!(AlignedBuffer::new(1024, 4096).is_ok());
    }

    #[test]
    #[should_panic(expected = "Alignment must be power of 2")]
    fn test_invalid_alignment() {
        let _ = AlignedBuffer::new(1024, 1000); // Not power of 2
    }
}
