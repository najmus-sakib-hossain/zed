//! Memory safety validation utilities

use std::mem::{align_of, size_of};

/// Safety validation error types
#[derive(Debug, thiserror::Error)]
pub enum SafetyError {
    /// Buffer bounds check failed
    #[error(
        "Buffer bounds check failed: offset {offset} + size {size} > buffer length {buffer_len}"
    )]
    BoundsCheck {
        offset: usize,
        size: usize,
        buffer_len: usize,
    },

    /// Pointer alignment check failed
    #[error(
        "Pointer alignment check failed: address {address:#x} not aligned to {alignment} bytes"
    )]
    AlignmentCheck { address: usize, alignment: usize },

    /// Integer overflow in size calculation
    #[error("Integer overflow in size calculation")]
    Overflow,
}

/// Check that a memory access is within bounds
pub fn check_bounds(offset: usize, size: usize, buffer_len: usize) -> Result<(), SafetyError> {
    let end = offset.checked_add(size).ok_or(SafetyError::Overflow)?;

    if end > buffer_len {
        return Err(SafetyError::BoundsCheck {
            offset,
            size,
            buffer_len,
        });
    }

    Ok(())
}

/// Check that a pointer is properly aligned for type T
pub fn check_alignment<T>(ptr: *const u8) -> Result<(), SafetyError> {
    let address = ptr as usize;
    let alignment = align_of::<T>();

    if address % alignment != 0 {
        return Err(SafetyError::AlignmentCheck { address, alignment });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_check_valid() {
        assert!(check_bounds(0, 10, 20).is_ok());
        assert!(check_bounds(5, 10, 20).is_ok());
        assert!(check_bounds(10, 10, 20).is_ok());
    }

    #[test]
    fn test_bounds_check_invalid() {
        assert!(check_bounds(0, 21, 20).is_err());
        assert!(check_bounds(15, 10, 20).is_err());
        assert!(check_bounds(20, 1, 20).is_err());
    }

    #[test]
    fn test_bounds_check_overflow() {
        assert!(check_bounds(usize::MAX, 1, 100).is_err());
    }

    #[test]
    fn test_alignment_check() {
        let buffer = [0u8; 16];
        let ptr = buffer.as_ptr();

        // u8 is always aligned
        assert!(check_alignment::<u8>(ptr).is_ok());

        // u64 requires 8-byte alignment
        if ptr as usize % 8 == 0 {
            assert!(check_alignment::<u64>(ptr).is_ok());
        } else {
            assert!(check_alignment::<u64>(ptr).is_err());
        }
    }
}
