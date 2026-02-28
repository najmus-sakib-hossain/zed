//! Zero-copy teleportation types.

use dx_security::{SafetyError, check_alignment, check_bounds};
use std::mem::{align_of, size_of};

/// Layout information for teleportable types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportLayout {
    /// Size of the type in bytes.
    pub size: usize,
    /// Alignment requirement in bytes.
    pub align: usize,
    /// Checksum for layout verification.
    pub checksum: u64,
}

impl TeleportLayout {
    /// Create a new layout.
    pub const fn new(size: usize, align: usize, checksum: u64) -> Self {
        Self {
            size,
            align,
            checksum,
        }
    }
}

/// Marker trait for zero-copy transferable types.
///
/// # Safety
///
/// This trait is unsafe because implementors must guarantee:
/// 1. The type uses `#[repr(C)]` for stable memory layout
/// 2. The type contains no pointers or references
/// 3. The type is `Copy` (no drop semantics)
/// 4. The layout is identical on server and WASM client
pub unsafe trait Teleportable: Copy + 'static {
    /// Layout information for this type.
    const LAYOUT: TeleportLayout;
}

/// Buffer for writing teleportable values.
pub struct TeleportBuffer {
    /// Main data buffer.
    buffer: Vec<u8>,
    /// Current write position.
    position: usize,
    /// String table buffer.
    strings: Vec<u8>,
}

impl TeleportBuffer {
    /// Create a new teleport buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            position: 0,
            strings: Vec::new(),
        }
    }

    /// Write a teleportable value to the buffer.
    pub fn write<T: Teleportable>(&mut self, value: &T) {
        let size = size_of::<T>();
        let align = align_of::<T>();

        // Align the position
        let padding = (align - (self.position % align)) % align;
        self.buffer.resize(self.position + padding + size, 0);
        self.position += padding;

        // SAFETY: We're creating a byte slice from a Teleportable value.
        // - T: Teleportable guarantees the type is #[repr(C)] with stable layout
        // - T: Copy guarantees no drop semantics
        // - size is computed from size_of::<T>() which is correct for the type
        // - The value reference is valid for the duration of this call
        let bytes = unsafe { std::slice::from_raw_parts(value as *const T as *const u8, size) };
        self.buffer[self.position..self.position + size].copy_from_slice(bytes);
        self.position += size;
    }

    /// Write a slice of teleportable values to the buffer.
    pub fn write_slice<T: Teleportable>(&mut self, values: &[T]) {
        for value in values {
            self.write(value);
        }
    }

    /// Write a string to the string table.
    ///
    /// Returns (offset, length) for later retrieval.
    pub fn write_string(&mut self, s: &str) -> (u32, u32) {
        let offset = self.strings.len() as u32;
        let len = s.len() as u32;
        self.strings.extend_from_slice(s.as_bytes());
        (offset, len)
    }

    /// Finalize the buffer and return the complete byte slice.
    ///
    /// The format is:
    /// - [data section]
    /// - [string table offset: u32]
    /// - [string table]
    pub fn finalize(&mut self) -> &[u8] {
        // Write string table offset
        let string_table_offset = self.position as u32;
        self.buffer.extend_from_slice(&string_table_offset.to_le_bytes());
        self.position += 4;

        // Append string table
        self.buffer.extend_from_slice(&self.strings);
        self.position += self.strings.len();

        &self.buffer[..self.position]
    }

    /// Get the current buffer contents without finalizing.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.position]
    }

    /// Get the current position.
    pub fn position(&self) -> usize {
        self.position
    }
}

/// Reader for teleportable values (zero-copy).
pub struct TeleportReader<'a> {
    /// Buffer to read from.
    buffer: &'a [u8],
    /// Current read position.
    position: usize,
    /// String table offset.
    string_table_offset: usize,
}

/// Error type for teleport read operations.
#[derive(Debug)]
pub enum TeleportError {
    /// Safety validation failed (bounds, alignment)
    Safety(SafetyError),
    /// Invalid UTF-8 in string data
    InvalidUtf8(std::str::Utf8Error),
    /// Buffer too small for the requested read
    BufferTooSmall { needed: usize, available: usize },
}

impl From<SafetyError> for TeleportError {
    fn from(err: SafetyError) -> Self {
        TeleportError::Safety(err)
    }
}

impl std::fmt::Display for TeleportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safety(err) => write!(f, "safety error: {}", err),
            Self::InvalidUtf8(err) => write!(f, "invalid UTF-8: {}", err),
            Self::BufferTooSmall { needed, available } => {
                write!(f, "buffer too small: needed {} bytes, have {}", needed, available)
            }
        }
    }
}

impl std::error::Error for TeleportError {}

impl<'a> TeleportReader<'a> {
    /// Create a new reader from a finalized buffer.
    pub fn new(buffer: &'a [u8]) -> Self {
        // Read string table offset from the end of the data section
        let string_table_offset = if buffer.len() >= 4 {
            // Find the string table offset marker
            // For simplicity, we'll scan backwards for it
            // In a real implementation, this would be at a known position
            0 // Placeholder - actual implementation would parse the format
        } else {
            buffer.len()
        };

        Self {
            buffer,
            position: 0,
            string_table_offset,
        }
    }

    /// Create a reader with explicit string table offset.
    pub fn with_string_table(buffer: &'a [u8], string_table_offset: usize) -> Self {
        Self {
            buffer,
            position: 0,
            string_table_offset,
        }
    }

    /// Read a teleportable value (zero-copy reference) with full safety validation.
    ///
    /// # Errors
    ///
    /// Returns `TeleportError::Safety` if bounds or alignment checks fail.
    pub fn read<T: Teleportable>(&mut self) -> Result<&'a T, TeleportError> {
        let size = size_of::<T>();
        let align = align_of::<T>();

        // Align the position
        let padding = (align - (self.position % align)) % align;
        let aligned_position = self.position + padding;

        // Validate bounds using dx-safety
        check_bounds(aligned_position, size, self.buffer.len())?;

        // Validate alignment
        let ptr = self.buffer[aligned_position..].as_ptr();
        check_alignment::<T>(ptr)?;

        self.position = aligned_position + size;

        // SAFETY: We've verified:
        // - aligned_position + size <= buffer.len() (via check_bounds)
        // - ptr is aligned to align_of::<T>() (via check_alignment)
        // - T: Teleportable guarantees the type is safe for zero-copy access
        Ok(unsafe { &*(ptr as *const T) })
    }

    /// Read a teleportable value, returning None on failure (legacy API).
    pub fn read_opt<T: Teleportable>(&mut self) -> Option<&'a T> {
        self.read().ok()
    }

    /// Read a slice of teleportable values (zero-copy reference) with full safety validation.
    ///
    /// # Errors
    ///
    /// Returns `TeleportError::Safety` if bounds or alignment checks fail.
    pub fn read_slice<T: Teleportable>(&mut self, count: usize) -> Result<&'a [T], TeleportError> {
        let size = size_of::<T>();
        let align = align_of::<T>();
        let total_size = size.checked_mul(count).ok_or(SafetyError::Overflow)?;

        // Align the position
        let padding = (align - (self.position % align)) % align;
        let aligned_position = self.position + padding;

        // Validate bounds using dx-safety
        check_bounds(aligned_position, total_size, self.buffer.len())?;

        // Validate alignment
        let ptr = self.buffer[aligned_position..].as_ptr();
        check_alignment::<T>(ptr)?;

        self.position = aligned_position + total_size;

        // SAFETY: We've verified:
        // - aligned_position + total_size <= buffer.len() (via check_bounds)
        // - ptr is aligned to align_of::<T>() (via check_alignment)
        // - T: Teleportable guarantees the type is safe for zero-copy access
        Ok(unsafe { std::slice::from_raw_parts(ptr as *const T, count) })
    }

    /// Read a slice of teleportable values, returning None on failure (legacy API).
    pub fn read_slice_opt<T: Teleportable>(&mut self, count: usize) -> Option<&'a [T]> {
        self.read_slice(count).ok()
    }

    /// Read a string from the string table with bounds checking.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is out of bounds or not valid UTF-8.
    pub fn read_string(&self, offset: u32, len: u32) -> Result<&'a str, TeleportError> {
        let start = self.string_table_offset + offset as usize;
        let end = start.checked_add(len as usize).ok_or(SafetyError::Overflow)?;

        // Validate bounds
        check_bounds(start, len as usize, self.buffer.len())?;

        std::str::from_utf8(&self.buffer[start..end]).map_err(TeleportError::InvalidUtf8)
    }

    /// Read a string from the string table, returning None on failure (legacy API).
    pub fn read_string_opt(&self, offset: u32, len: u32) -> Option<&'a str> {
        self.read_string(offset, len).ok()
    }

    /// Get the current read position.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the remaining bytes in the buffer.
    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.position)
    }

    /// Reset the read position.
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Seek to a specific position with bounds checking.
    ///
    /// # Errors
    ///
    /// Returns an error if the position is out of bounds.
    pub fn seek(&mut self, position: usize) -> Result<(), TeleportError> {
        check_bounds(position, 0, self.buffer.len())?;
        self.position = position;
        Ok(())
    }
}

// Implement Teleportable for primitive types
macro_rules! impl_teleportable_primitive {
    ($($ty:ty),*) => {
        $(
            unsafe impl Teleportable for $ty {
                const LAYOUT: TeleportLayout = TeleportLayout::new(
                    size_of::<$ty>(),
                    align_of::<$ty>(),
                    0, // Checksum computed at compile time in real impl
                );
            }
        )*
    };
}

impl_teleportable_primitive!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

// ============================================================================
// Example Teleportable Types
// ============================================================================

/// Example teleportable user type.
///
/// This demonstrates how to create a teleportable struct with string references.
/// Strings are stored in a separate string table and referenced by offset/length.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportableUser {
    /// User ID.
    pub id: u64,
    /// Offset into string table for name.
    pub name_offset: u32,
    /// Length of name string.
    pub name_len: u32,
    /// User age.
    pub age: u8,
    /// Whether user is active.
    pub active: u8,
    /// Padding for alignment.
    pub _pad: [u8; 6],
}

// Compile-time assertion that TeleportableUser is 24 bytes
const _: () = assert!(std::mem::size_of::<TeleportableUser>() == 24);

unsafe impl Teleportable for TeleportableUser {
    const LAYOUT: TeleportLayout = TeleportLayout::new(
        std::mem::size_of::<TeleportableUser>(),
        std::mem::align_of::<TeleportableUser>(),
        0x5553_4552, // "USER" as checksum
    );
}

impl TeleportableUser {
    /// Create a new teleportable user.
    pub fn new(id: u64, name_offset: u32, name_len: u32, age: u8, active: bool) -> Self {
        Self {
            id,
            name_offset,
            name_len,
            age,
            active: if active { 1 } else { 0 },
            _pad: [0; 6],
        }
    }

    /// Check if user is active.
    pub fn is_active(&self) -> bool {
        self.active != 0
    }
}

/// Example teleportable point type for 2D coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeleportablePoint {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
}

// Compile-time assertion that TeleportablePoint is 16 bytes
const _: () = assert!(std::mem::size_of::<TeleportablePoint>() == 16);

unsafe impl Teleportable for TeleportablePoint {
    const LAYOUT: TeleportLayout = TeleportLayout::new(
        std::mem::size_of::<TeleportablePoint>(),
        std::mem::align_of::<TeleportablePoint>(),
        0x504F_494E, // "POIN" as checksum
    );
}

impl TeleportablePoint {
    /// Create a new point.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculate distance from origin.
    pub fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

/// Example teleportable timestamp type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportableTimestamp {
    /// Seconds since Unix epoch.
    pub secs: i64,
    /// Nanoseconds within the second.
    pub nanos: u32,
    /// Padding for alignment.
    pub _pad: u32,
}

// Compile-time assertion that TeleportableTimestamp is 16 bytes
const _: () = assert!(std::mem::size_of::<TeleportableTimestamp>() == 16);

unsafe impl Teleportable for TeleportableTimestamp {
    const LAYOUT: TeleportLayout = TeleportLayout::new(
        std::mem::size_of::<TeleportableTimestamp>(),
        std::mem::align_of::<TeleportableTimestamp>(),
        0x5449_4D45, // "TIME" as checksum
    );
}

impl TeleportableTimestamp {
    /// Create a new timestamp.
    pub fn new(secs: i64, nanos: u32) -> Self {
        Self {
            secs,
            nanos,
            _pad: 0,
        }
    }

    /// Create a timestamp from seconds only.
    pub fn from_secs(secs: i64) -> Self {
        Self::new(secs, 0)
    }
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_teleport_buffer_write_read() {
        let mut buffer = TeleportBuffer::new(256);

        let point = TeleportablePoint::new(1.5, 2.5);
        buffer.write(&point);

        let bytes = buffer.as_bytes();
        let mut reader = TeleportReader::new(bytes);

        let read_point: &TeleportablePoint = reader.read().unwrap();
        assert_eq!(read_point.x, 1.5);
        assert_eq!(read_point.y, 2.5);
    }

    #[test]
    fn test_teleport_reader_bounds_check() {
        let buffer = [0u8; 4]; // Too small for TeleportablePoint (16 bytes)
        let mut reader = TeleportReader::new(&buffer);

        let result: Result<&TeleportablePoint, _> = reader.read();
        assert!(result.is_err());
    }

    #[test]
    fn test_teleport_reader_string_bounds() {
        let buffer = b"Hello";
        let reader = TeleportReader::with_string_table(buffer, 0);

        // Valid read
        let s = reader.read_string(0, 5).unwrap();
        assert_eq!(s, "Hello");

        // Out of bounds
        let result = reader.read_string(0, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_teleport_reader_seek() {
        let buffer = [0u8; 32];
        let mut reader = TeleportReader::new(&buffer);

        // Valid seek
        assert!(reader.seek(16).is_ok());
        assert_eq!(reader.position(), 16);

        // Out of bounds seek
        assert!(reader.seek(100).is_err());
    }
}

#[cfg(test)]
mod props {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property 3: Pointer Bounds Safety
        ///
        /// Validates that TeleportReader never allows out-of-bounds reads
        /// regardless of buffer size or read position.
        #[test]
        fn pointer_bounds_safety(
            buffer_size in 0usize..1024,
            read_count in 0usize..100,
        ) {
            let buffer = vec![0u8; buffer_size];
            let mut reader = TeleportReader::new(&buffer);

            // Try to read multiple u64 values
            let mut successful_reads = 0;
            for _ in 0..read_count {
                match reader.read::<u64>() {
                    Ok(_) => successful_reads += 1,
                    Err(_) => break, // Stop on first error
                }
            }

            // Verify we never read past the buffer
            prop_assert!(reader.position() <= buffer_size);

            // Verify the number of successful reads is bounded
            let max_possible = buffer_size / size_of::<u64>();
            prop_assert!(successful_reads <= max_possible);
        }

        /// Property: Slice reads respect bounds
        #[test]
        fn slice_bounds_safety(
            buffer_size in 0usize..1024,
            slice_count in 0usize..100,
        ) {
            let buffer = vec![0u8; buffer_size];
            let mut reader = TeleportReader::new(&buffer);

            let result: Result<&[u32], _> = reader.read_slice(slice_count);

            let required_size = size_of::<u32>() * slice_count;
            if buffer_size >= required_size {
                // Should succeed if buffer is large enough
                prop_assert!(result.is_ok() || buffer_size < required_size);
            } else {
                // Should fail if buffer is too small
                prop_assert!(result.is_err());
            }
        }

        /// Property: String reads respect bounds
        #[test]
        fn string_bounds_safety(
            buffer_size in 0usize..256,
            offset in 0u32..256,
            len in 0u32..256,
        ) {
            let buffer = vec![b'a'; buffer_size];
            let reader = TeleportReader::with_string_table(&buffer, 0);

            let result = reader.read_string(offset, len);

            let end = (offset as usize).saturating_add(len as usize);
            if end <= buffer_size {
                // Should succeed if within bounds
                prop_assert!(result.is_ok());
            } else {
                // Should fail if out of bounds
                prop_assert!(result.is_err());
            }
        }

        /// Property: Seek respects bounds
        #[test]
        fn seek_bounds_safety(
            buffer_size in 0usize..1024,
            seek_position in 0usize..2048,
        ) {
            let buffer = vec![0u8; buffer_size];
            let mut reader = TeleportReader::new(&buffer);

            let result = reader.seek(seek_position);

            if seek_position <= buffer_size {
                prop_assert!(result.is_ok());
                prop_assert_eq!(reader.position(), seek_position);
            } else {
                prop_assert!(result.is_err());
            }
        }

        /// Property: Round-trip write/read preserves data
        #[test]
        fn round_trip_preserves_data(x: f64, y: f64) {
            let mut buffer = TeleportBuffer::new(256);

            let point = TeleportablePoint::new(x, y);
            buffer.write(&point);

            let bytes = buffer.as_bytes();
            let mut reader = TeleportReader::new(bytes);

            let read_point: &TeleportablePoint = reader.read().expect("read should succeed");

            // Handle NaN specially (NaN != NaN)
            if x.is_nan() {
                prop_assert!(read_point.x.is_nan());
            } else {
                prop_assert_eq!(read_point.x, x);
            }

            if y.is_nan() {
                prop_assert!(read_point.y.is_nan());
            } else {
                prop_assert_eq!(read_point.y, y);
            }
        }
    }
}
