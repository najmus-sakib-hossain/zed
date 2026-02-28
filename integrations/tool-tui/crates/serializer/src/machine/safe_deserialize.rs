//! Safe zero-copy deserialization with bounds and alignment checking.
//!
//! This module provides a safe wrapper around the zero-copy deserialization
//! functionality, ensuring all pointer operations are validated before execution.
//!
//! # Example
//!
//! ```rust
//! use serializer::machine::SafeDeserializer;
//!
//! // Create a buffer with some u32 values
//! let values: [u32; 4] = [1, 2, 3, 4];
//! let buffer: &[u8] = bytemuck::cast_slice(&values);
//!
//! let mut deserializer = SafeDeserializer::new(buffer);
//!
//! // Read a single u32 with full validation
//! let first: &u32 = deserializer.read().unwrap();
//! assert_eq!(*first, 1);
//!
//! // Read remaining values as a slice
//! let rest: &[u32] = deserializer.read_slice(3).unwrap();
//! assert_eq!(rest, &[2, 3, 4]);
//! ```

use crate::safety::{check_alignment, check_bounds, check_cast, SafetyError};
use std::mem::size_of;

/// Safe zero-copy deserializer that validates before casting.
///
/// This struct wraps a byte buffer and provides safe methods for
/// reading typed values with full bounds and alignment checking.
#[derive(Debug)]
pub struct SafeDeserializer<'a> {
    /// The underlying byte buffer
    buffer: &'a [u8],
    /// Current read position
    position: usize,
}

impl<'a> SafeDeserializer<'a> {
    /// Create a new SafeDeserializer from a byte buffer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::machine::SafeDeserializer;
    ///
    /// let buffer = [0u8; 64];
    /// let deserializer = SafeDeserializer::new(&buffer);
    /// assert_eq!(deserializer.remaining(), 64);
    /// ```
    #[inline]
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }

    /// Create a new SafeDeserializer starting at a specific offset.
    ///
    /// # Errors
    ///
    /// Returns an error if the offset is out of bounds.
    #[inline]
    pub fn with_offset(buffer: &'a [u8], offset: usize) -> Result<Self, SafetyError> {
        check_bounds(offset, 0, buffer.len())?;
        Ok(Self {
            buffer,
            position: offset,
        })
    }

    /// Read a value of type T with full safety validation.
    ///
    /// This method validates that:
    /// - The remaining buffer has at least `size_of::<T>()` bytes
    /// - The current position is properly aligned for type T
    ///
    /// # Errors
    ///
    /// Returns `SafetyError::BufferTooSmall` if there aren't enough bytes.
    /// Returns `SafetyError::Misaligned` if the pointer is not properly aligned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::machine::SafeDeserializer;
    ///
    /// // Read a u32 value from a buffer
    /// let value: u32 = 0x12345678;
    /// let buffer: &[u8] = bytemuck::bytes_of(&value);
    ///
    /// let mut deserializer = SafeDeserializer::new(buffer);
    /// let read_value: &u32 = deserializer.read().unwrap();
    /// assert_eq!(*read_value, 0x12345678);
    /// ```
    #[inline]
    pub fn read<T: Copy>(&mut self) -> Result<&'a T, SafetyError> {
        let slice = &self.buffer[self.position..];

        // Validate before any unsafe operation
        check_cast::<T>(slice)?;

        // SAFETY: We verified via check_cast that:
        // - slice.len() >= size_of::<T>() (sufficient bytes available)
        // - slice.as_ptr() is aligned to align_of::<T>() (proper alignment)
        // Therefore, casting the pointer to *const T and dereferencing is safe.
        let value = unsafe { &*(slice.as_ptr() as *const T) };
        self.position += size_of::<T>();
        Ok(value)
    }

    /// Read a value of type T without advancing the position.
    ///
    /// This is useful for peeking at headers or magic bytes.
    #[inline]
    pub fn peek<T: Copy>(&self) -> Result<&'a T, SafetyError> {
        let slice = &self.buffer[self.position..];
        check_cast::<T>(slice)?;

        // SAFETY: We verified via check_cast that:
        // - slice.len() >= size_of::<T>() (sufficient bytes available)
        // - slice.as_ptr() is aligned to align_of::<T>() (proper alignment)
        // Therefore, casting the pointer to *const T and dereferencing is safe.
        Ok(unsafe { &*(slice.as_ptr() as *const T) })
    }

    /// Read a slice of values with bounds checking.
    ///
    /// # Errors
    ///
    /// Returns `SafetyError::BufferTooSmall` if there aren't enough bytes.
    /// Returns `SafetyError::Misaligned` if the pointer is not properly aligned.
    /// Returns `SafetyError::Overflow` if `count * size_of::<T>()` overflows.
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::machine::SafeDeserializer;
    ///
    /// let values: [u32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let buffer: &[u8] = bytemuck::cast_slice(&values);
    ///
    /// let mut deserializer = SafeDeserializer::new(buffer);
    /// let slice: &[u32] = deserializer.read_slice(10).unwrap();
    /// assert_eq!(slice, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    /// ```
    #[inline]
    pub fn read_slice<T: Copy>(&mut self, count: usize) -> Result<&'a [T], SafetyError> {
        let size = size_of::<T>().checked_mul(count).ok_or(SafetyError::Overflow)?;

        let slice = &self.buffer[self.position..];
        check_bounds(0, size, slice.len())?;
        check_alignment::<T>(slice.as_ptr())?;

        // SAFETY: We verified via check_bounds and check_alignment that:
        // - slice.len() >= count * size_of::<T>() (sufficient bytes for all elements)
        // - slice.as_ptr() is aligned to align_of::<T>() (proper alignment)
        // Therefore, creating a slice of T from the pointer is safe.
        let values = unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const T, count) };
        self.position += size;
        Ok(values)
    }

    /// Read raw bytes without type casting.
    ///
    /// This is useful for reading variable-length data like strings.
    #[inline]
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], SafetyError> {
        check_bounds(self.position, len, self.buffer.len())?;

        let bytes = &self.buffer[self.position..self.position + len];
        self.position += len;
        Ok(bytes)
    }

    /// Read a UTF-8 string of the specified length.
    ///
    /// # Errors
    ///
    /// Returns `SafetyError::BufferTooSmall` if there aren't enough bytes.
    /// Returns a UTF-8 validation error (wrapped) if the bytes are not valid UTF-8.
    #[inline]
    pub fn read_str(&mut self, len: usize) -> Result<&'a str, DeserializeError> {
        let bytes = self.read_bytes(len)?;
        std::str::from_utf8(bytes).map_err(DeserializeError::InvalidUtf8)
    }

    /// Skip a number of bytes without reading.
    #[inline]
    pub fn skip(&mut self, len: usize) -> Result<(), SafetyError> {
        check_bounds(self.position, len, self.buffer.len())?;
        self.position += len;
        Ok(())
    }

    /// Get the current read position.
    #[inline]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the remaining bytes in the buffer.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.position)
    }

    /// Check if there are at least `n` bytes remaining.
    #[inline]
    pub fn has_remaining(&self, n: usize) -> bool {
        self.remaining() >= n
    }

    /// Reset the position to the beginning.
    #[inline]
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Seek to a specific position.
    ///
    /// # Errors
    ///
    /// Returns an error if the position is out of bounds.
    #[inline]
    pub fn seek(&mut self, position: usize) -> Result<(), SafetyError> {
        check_bounds(position, 0, self.buffer.len())?;
        self.position = position;
        Ok(())
    }

    /// Get a reference to the underlying buffer.
    #[inline]
    pub fn buffer(&self) -> &'a [u8] {
        self.buffer
    }

    /// Get a reference to the remaining buffer (from current position).
    #[inline]
    pub fn remaining_buffer(&self) -> &'a [u8] {
        &self.buffer[self.position..]
    }
}

/// Error type for deserialization operations.
#[derive(Debug)]
pub enum DeserializeError {
    /// Safety validation failed (bounds, alignment, overflow)
    Safety(SafetyError),
    /// Invalid UTF-8 in string data
    InvalidUtf8(std::str::Utf8Error),
    /// Invalid magic bytes
    InvalidMagic { expected: u32, actual: u32 },
    /// Version mismatch
    VersionMismatch { expected: u32, actual: u32 },
}

impl From<SafetyError> for DeserializeError {
    fn from(err: SafetyError) -> Self {
        DeserializeError::Safety(err)
    }
}

impl std::fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safety(err) => write!(f, "safety error: {}", err),
            Self::InvalidUtf8(err) => write!(f, "invalid UTF-8: {}", err),
            Self::InvalidMagic { expected, actual } => {
                write!(f, "invalid magic: expected 0x{:08X}, got 0x{:08X}", expected, actual)
            }
            Self::VersionMismatch { expected, actual } => {
                write!(f, "version mismatch: expected {}, got {}", expected, actual)
            }
        }
    }
}

impl std::error::Error for DeserializeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUtf8(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u32() {
        let buffer = 0x12345678u32.to_le_bytes();
        let mut deserializer = SafeDeserializer::new(&buffer);

        let value: &u32 = deserializer.read().unwrap();
        assert_eq!(*value, 0x12345678);
        assert_eq!(deserializer.position(), 4);
    }

    #[test]
    fn test_read_u64() {
        let buffer = 0x123456789ABCDEFu64.to_le_bytes();
        let mut deserializer = SafeDeserializer::new(&buffer);

        let value: &u64 = deserializer.read().unwrap();
        assert_eq!(*value, 0x123456789ABCDEF);
        assert_eq!(deserializer.position(), 8);
    }

    #[test]
    fn test_read_buffer_too_small() {
        let buffer = [0u8; 4];
        let mut deserializer = SafeDeserializer::new(&buffer);

        let result: Result<&u64, _> = deserializer.read();
        assert!(matches!(
            result,
            Err(SafetyError::BufferTooSmall {
                needed: 8,
                actual: 4
            })
        ));
    }

    #[test]
    fn test_read_slice() {
        let values: [u32; 4] = [1, 2, 3, 4];
        // SAFETY: Creating a byte view of the array for testing
        let buffer = unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u8, 16) };

        let mut deserializer = SafeDeserializer::new(buffer);
        let read_values: &[u32] = deserializer.read_slice(4).unwrap();

        assert_eq!(read_values, &[1, 2, 3, 4]);
        assert_eq!(deserializer.position(), 16);
    }

    #[test]
    fn test_read_bytes() {
        let buffer = b"Hello, World!";
        let mut deserializer = SafeDeserializer::new(buffer);

        let bytes = deserializer.read_bytes(5).unwrap();
        assert_eq!(bytes, b"Hello");
        assert_eq!(deserializer.position(), 5);
    }

    #[test]
    fn test_read_str() {
        let buffer = b"Hello, World!";
        let mut deserializer = SafeDeserializer::new(buffer);

        let s = deserializer.read_str(5).unwrap();
        assert_eq!(s, "Hello");
    }

    #[test]
    fn test_peek() {
        let buffer = 0x12345678u32.to_le_bytes();
        let deserializer = SafeDeserializer::new(&buffer);

        let value: &u32 = deserializer.peek().unwrap();
        assert_eq!(*value, 0x12345678);
        // Position should not change
        assert_eq!(deserializer.position(), 0);
    }

    #[test]
    fn test_skip() {
        let buffer = [0u8; 16];
        let mut deserializer = SafeDeserializer::new(&buffer);

        deserializer.skip(8).unwrap();
        assert_eq!(deserializer.position(), 8);
        assert_eq!(deserializer.remaining(), 8);
    }

    #[test]
    fn test_seek() {
        let buffer = [0u8; 16];
        let mut deserializer = SafeDeserializer::new(&buffer);

        deserializer.seek(10).unwrap();
        assert_eq!(deserializer.position(), 10);

        // Seek out of bounds should fail
        let result = deserializer.seek(20);
        assert!(result.is_err());
    }

    #[test]
    fn test_with_offset() {
        let buffer = [0u8; 16];

        let deserializer = SafeDeserializer::with_offset(&buffer, 8).unwrap();
        assert_eq!(deserializer.position(), 8);

        // Out of bounds offset should fail
        let result = SafeDeserializer::with_offset(&buffer, 20);
        assert!(result.is_err());
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestHeader {
        magic: u32,
        version: u32,
        size: u32,
        flags: u32,
    }

    #[test]
    fn test_read_struct() {
        let header = TestHeader {
            magic: 0x44585A00,
            version: 1,
            size: 100,
            flags: 0,
        };

        // SAFETY: Creating a byte view of the struct for testing
        let buffer =
            unsafe { std::slice::from_raw_parts(&header as *const TestHeader as *const u8, 16) };

        let mut deserializer = SafeDeserializer::new(buffer);
        let read_header: &TestHeader = deserializer.read().unwrap();

        assert_eq!(read_header.magic, 0x44585A00);
        assert_eq!(read_header.version, 1);
        assert_eq!(read_header.size, 100);
        assert_eq!(read_header.flags, 0);
    }
}
