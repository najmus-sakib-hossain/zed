//! # Safety Validation Utilities
//!
//! This module provides zero-cost abstractions for common safety checks
//! required when working with unsafe code. It is designed to provide
//! consistent validation patterns for zero-copy operations.
//!
//! ## Attribution
//!
//! This code is inlined from the `dx-safety` crate to enable standalone
//! publishability of `dx-serializer` without path dependencies.
//! Original source: `crates/dx/dx-safety/src/lib.rs`
//!
//! ## Usage
//!
//! ```rust
//! use serializer::safety::{check_bounds, check_alignment, check_cast, SafetyError};
//!
//! fn safe_read<T: Copy>(buffer: &[u8]) -> Result<&T, SafetyError> {
//!     // Validate before any unsafe operation
//!     check_cast::<T>(buffer)?;
//!     
//!     // SAFETY: We verified size >= size_of::<T>() and alignment matches
//!     Ok(unsafe { &*(buffer.as_ptr() as *const T) })
//! }
//!
//! // Example usage
//! let data = [1u8, 0, 0, 0]; // Little-endian 1
//! let value: &u32 = safe_read(&data).unwrap();
//! assert_eq!(*value, 1);
//! ```

use core::fmt;
use core::mem::{align_of, size_of};

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Error type for safety validation failures.
///
/// All variants include context information to aid debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyError {
    /// Buffer is too small for the requested type.
    ///
    /// Contains the number of bytes needed and the actual buffer size.
    BufferTooSmall {
        /// Minimum bytes required
        needed: usize,
        /// Actual buffer size
        actual: usize,
    },

    /// Pointer is not properly aligned for the requested type.
    ///
    /// Contains the required alignment and the actual misalignment offset.
    Misaligned {
        /// Required alignment in bytes
        needed: usize,
        /// Actual offset from aligned address (ptr % needed)
        actual: usize,
    },

    /// Offset would exceed buffer bounds.
    ///
    /// Contains the requested offset and the buffer length.
    OffsetOutOfBounds {
        /// Requested offset
        offset: usize,
        /// Buffer length
        length: usize,
    },

    /// Integer overflow occurred during size calculation.
    ///
    /// This typically happens when multiplying `count * size_of::<T>()`.
    Overflow,
}

impl fmt::Display for SafetyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall { needed, actual } => {
                write!(f, "buffer too small: needed {} bytes, got {}", needed, actual)
            }
            Self::Misaligned { needed, actual } => {
                write!(
                    f,
                    "pointer misaligned: needed {} byte alignment, offset is {}",
                    needed, actual
                )
            }
            Self::OffsetOutOfBounds { offset, length } => {
                write!(f, "offset {} out of bounds for length {}", offset, length)
            }
            Self::Overflow => write!(f, "integer overflow in size calculation"),
        }
    }
}

impl std::error::Error for SafetyError {}

// ============================================================================
// BOUNDS CHECKING UTILITIES
// ============================================================================

/// Check that a slice has sufficient length for type T.
///
/// This is a zero-cost check when the slice is large enough.
///
/// # Example
///
/// ```rust
/// use serializer::safety::{check_size, SafetyError};
///
/// let buffer = [0u8; 8];
/// assert!(check_size::<u64>(&buffer).is_ok());
/// assert!(check_size::<u128>(&buffer).is_err());
/// ```
#[inline(always)]
pub fn check_size<T>(slice: &[u8]) -> Result<(), SafetyError> {
    let needed = size_of::<T>();
    if slice.len() < needed {
        Err(SafetyError::BufferTooSmall {
            needed,
            actual: slice.len(),
        })
    } else {
        Ok(())
    }
}

/// Check that offset + size doesn't exceed buffer length.
///
/// This function also checks for integer overflow when adding offset + size.
///
/// # Example
///
/// ```rust
/// use serializer::safety::{check_bounds, SafetyError};
///
/// // Valid: offset 0, size 4, length 8
/// assert!(check_bounds(0, 4, 8).is_ok());
///
/// // Invalid: offset 6, size 4, length 8 (6 + 4 = 10 > 8)
/// assert!(matches!(
///     check_bounds(6, 4, 8),
///     Err(SafetyError::OffsetOutOfBounds { .. })
/// ));
///
/// // Overflow: very large values
/// assert!(matches!(
///     check_bounds(usize::MAX, 1, 100),
///     Err(SafetyError::Overflow)
/// ));
/// ```
#[inline(always)]
pub fn check_bounds(offset: usize, size: usize, length: usize) -> Result<(), SafetyError> {
    match offset.checked_add(size) {
        Some(end) if end <= length => Ok(()),
        Some(_) => Err(SafetyError::OffsetOutOfBounds { offset, length }),
        None => Err(SafetyError::Overflow),
    }
}

/// Check bounds for reading `count` elements of type T starting at `offset`.
///
/// This combines overflow checking for `count * size_of::<T>()` with bounds checking.
///
/// # Example
///
/// ```rust
/// use serializer::safety::{check_slice_bounds, SafetyError};
///
/// let buffer = [0u8; 32];
///
/// // Valid: 4 u64s starting at offset 0 = 32 bytes
/// assert!(check_slice_bounds::<u64>(0, 4, buffer.len()).is_ok());
///
/// // Invalid: 5 u64s = 40 bytes > 32
/// assert!(check_slice_bounds::<u64>(0, 5, buffer.len()).is_err());
/// ```
#[inline(always)]
pub fn check_slice_bounds<T>(
    offset: usize,
    count: usize,
    length: usize,
) -> Result<(), SafetyError> {
    let size = size_of::<T>().checked_mul(count).ok_or(SafetyError::Overflow)?;
    check_bounds(offset, size, length)
}

// ============================================================================
// ALIGNMENT CHECKING UTILITIES
// ============================================================================

/// Check that a pointer is properly aligned for type T.
///
/// # Example
///
/// ```rust
/// use serializer::safety::{check_alignment, SafetyError};
///
/// let aligned: [u64; 2] = [0, 0];
/// let ptr = aligned.as_ptr() as *const u8;
///
/// // u64 requires 8-byte alignment
/// assert!(check_alignment::<u64>(ptr).is_ok());
///
/// // Offset by 1 byte - now misaligned for u64
/// let misaligned = unsafe { ptr.add(1) };
/// assert!(matches!(
///     check_alignment::<u64>(misaligned),
///     Err(SafetyError::Misaligned { needed: 8, actual: 1 })
/// ));
/// ```
#[inline(always)]
pub fn check_alignment<T>(ptr: *const u8) -> Result<(), SafetyError> {
    let needed = align_of::<T>();
    let actual = ptr as usize % needed;
    if actual != 0 {
        Err(SafetyError::Misaligned { needed, actual })
    } else {
        Ok(())
    }
}

/// Combined check for size and alignment before casting a slice to type T.
///
/// This is the primary validation function for zero-copy deserialization.
///
/// # Example
///
/// ```rust
/// use serializer::safety::{check_cast, SafetyError};
///
/// #[repr(C)]
/// struct Header {
///     magic: u32,
///     version: u32,
/// }
///
/// fn read_header(buffer: &[u8]) -> Result<&Header, SafetyError> {
///     check_cast::<Header>(buffer)?;
///     // SAFETY: We verified size and alignment
///     Ok(unsafe { &*(buffer.as_ptr() as *const Header) })
/// }
///
/// // Example usage with properly aligned buffer
/// let data = [0x44u8, 0x58, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
/// let header = read_header(&data).unwrap();
/// ```
#[inline(always)]
pub fn check_cast<T>(slice: &[u8]) -> Result<(), SafetyError> {
    check_size::<T>(slice)?;
    check_alignment::<T>(slice.as_ptr())?;
    Ok(())
}

// ============================================================================
// SAFE POINTER UTILITIES
// ============================================================================

/// Safely read a value of type T from a byte slice with full validation.
///
/// This is a convenience function that combines validation and reading.
///
/// # Safety
///
/// The type T must be valid for any bit pattern (e.g., primitive types,
/// `#[repr(C)]` structs with no padding requirements).
///
/// # Example
///
/// ```rust
/// use serializer::safety::safe_read;
///
/// let buffer = 42u64.to_le_bytes();
/// let value: &u64 = safe_read(&buffer).unwrap();
/// assert_eq!(*value, 42);
/// ```
#[inline]
pub fn safe_read<T: Copy>(slice: &[u8]) -> Result<&T, SafetyError> {
    check_cast::<T>(slice)?;
    // SAFETY: We verified size >= size_of::<T>() and alignment matches align_of::<T>()
    Ok(unsafe { &*(slice.as_ptr() as *const T) })
}

/// Safely read a slice of values from a byte buffer with full validation.
///
/// # Safety
///
/// The type T must be valid for any bit pattern.
///
/// # Example
///
/// ```rust
/// use serializer::safety::{safe_read_slice, SafetyError};
///
/// // Create a properly aligned buffer
/// let values: [u32; 4] = [1, 2, 3, 4];
/// let bytes: &[u8] = bytemuck::cast_slice(&values);
/// let read_values: &[u32] = safe_read_slice(bytes, 4).unwrap();
/// assert_eq!(read_values, &[1, 2, 3, 4]);
/// ```
#[inline]
pub fn safe_read_slice<T: Copy>(slice: &[u8], count: usize) -> Result<&[T], SafetyError> {
    check_slice_bounds::<T>(0, count, slice.len())?;
    check_alignment::<T>(slice.as_ptr())?;
    // SAFETY: We verified bounds and alignment
    Ok(unsafe { core::slice::from_raw_parts(slice.as_ptr() as *const T, count) })
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_size_success() {
        let buffer = [0u8; 16];
        assert!(check_size::<u8>(&buffer).is_ok());
        assert!(check_size::<u16>(&buffer).is_ok());
        assert!(check_size::<u32>(&buffer).is_ok());
        assert!(check_size::<u64>(&buffer).is_ok());
        assert!(check_size::<u128>(&buffer).is_ok());
    }

    #[test]
    fn test_check_size_failure() {
        let buffer = [0u8; 4];
        assert!(matches!(
            check_size::<u64>(&buffer),
            Err(SafetyError::BufferTooSmall {
                needed: 8,
                actual: 4
            })
        ));
    }

    #[test]
    fn test_check_bounds_success() {
        assert!(check_bounds(0, 4, 8).is_ok());
        assert!(check_bounds(4, 4, 8).is_ok());
        assert!(check_bounds(0, 8, 8).is_ok());
        assert!(check_bounds(0, 0, 0).is_ok());
    }

    #[test]
    fn test_check_bounds_failure() {
        assert!(matches!(
            check_bounds(5, 4, 8),
            Err(SafetyError::OffsetOutOfBounds {
                offset: 5,
                length: 8
            })
        ));
    }

    #[test]
    fn test_check_bounds_overflow() {
        assert!(matches!(check_bounds(usize::MAX, 1, 100), Err(SafetyError::Overflow)));
    }

    #[test]
    fn test_check_alignment_success() {
        let aligned: [u64; 2] = [0, 0];
        let ptr = aligned.as_ptr() as *const u8;
        assert!(check_alignment::<u64>(ptr).is_ok());
        assert!(check_alignment::<u32>(ptr).is_ok());
        assert!(check_alignment::<u16>(ptr).is_ok());
        assert!(check_alignment::<u8>(ptr).is_ok());
    }

    #[test]
    fn test_check_alignment_failure() {
        let aligned: [u64; 2] = [0, 0];
        let ptr = aligned.as_ptr() as *const u8;
        // SAFETY: We're just testing alignment, not dereferencing
        let misaligned = unsafe { ptr.add(1) };

        let result = check_alignment::<u64>(misaligned);
        assert!(matches!(
            result,
            Err(SafetyError::Misaligned {
                needed: 8,
                actual: 1
            })
        ));
    }

    #[test]
    fn test_safe_read() {
        let value: u64 = 0x1234567890ABCDEF;
        let bytes = value.to_le_bytes();

        let read_value: &u64 = safe_read(&bytes).unwrap();
        assert_eq!(*read_value, value);
    }

    #[test]
    fn test_safe_read_slice() {
        let values: [u32; 4] = [1, 2, 3, 4];
        // SAFETY: Creating a byte view of the array
        let bytes = unsafe { core::slice::from_raw_parts(values.as_ptr() as *const u8, 16) };

        let read_values: &[u32] = safe_read_slice(bytes, 4).unwrap();
        assert_eq!(read_values, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_error_display() {
        let err = SafetyError::BufferTooSmall {
            needed: 8,
            actual: 4,
        };
        assert_eq!(err.to_string(), "buffer too small: needed 8 bytes, got 4");

        let err = SafetyError::Misaligned {
            needed: 8,
            actual: 3,
        };
        assert_eq!(err.to_string(), "pointer misaligned: needed 8 byte alignment, offset is 3");

        let err = SafetyError::OffsetOutOfBounds {
            offset: 10,
            length: 8,
        };
        assert_eq!(err.to_string(), "offset 10 out of bounds for length 8");

        let err = SafetyError::Overflow;
        assert_eq!(err.to_string(), "integer overflow in size calculation");
    }
}
