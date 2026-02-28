//! Completion result structure.

/// Unified completion result across all I/O backends.
///
/// This structure represents the result of an asynchronous I/O operation,
/// providing a consistent interface regardless of the underlying platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Completion {
    /// User-provided identifier for this operation.
    /// This is the same value passed when the operation was submitted.
    pub user_data: u64,

    /// Operation result.
    /// - Positive values: number of bytes transferred
    /// - Zero: operation completed with no data (e.g., connection closed)
    /// - Negative values: error code (negated errno on Unix)
    pub result: i32,

    /// Backend-specific flags.
    /// The meaning of these flags depends on the I/O backend:
    /// - io_uring: CQE flags (IORING_CQE_F_*)
    /// - kqueue: kevent flags
    /// - IOCP: completion flags
    pub flags: u32,
}

impl Completion {
    /// Create a new completion result.
    pub fn new(user_data: u64, result: i32, flags: u32) -> Self {
        Self {
            user_data,
            result,
            flags,
        }
    }

    /// Check if the operation completed successfully.
    pub fn is_success(&self) -> bool {
        self.result >= 0
    }

    /// Check if the operation failed.
    pub fn is_error(&self) -> bool {
        self.result < 0
    }

    /// Get the number of bytes transferred (if successful).
    pub fn bytes_transferred(&self) -> Option<usize> {
        if self.result >= 0 {
            Some(self.result as usize)
        } else {
            None
        }
    }

    /// Get the error code (if failed).
    pub fn error_code(&self) -> Option<i32> {
        if self.result < 0 {
            Some(-self.result)
        } else {
            None
        }
    }
}
