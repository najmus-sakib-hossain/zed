//! I/O completion types

use bitflags::bitflags;
use std::io;

/// Result of a completed I/O operation.
#[derive(Debug)]
pub struct Completion {
    /// User data from the original operation
    pub user_data: u64,
    /// Result of the operation (bytes transferred or error)
    pub result: io::Result<usize>,
    /// Additional flags for the completion
    pub flags: CompletionFlags,
}

impl Completion {
    /// Create a successful completion.
    pub fn success(user_data: u64, bytes: usize) -> Self {
        Self {
            user_data,
            result: Ok(bytes),
            flags: CompletionFlags::empty(),
        }
    }

    /// Create a successful completion with flags.
    pub fn success_with_flags(user_data: u64, bytes: usize, flags: CompletionFlags) -> Self {
        Self {
            user_data,
            result: Ok(bytes),
            flags,
        }
    }

    /// Create an error completion.
    pub fn error(user_data: u64, err: io::Error) -> Self {
        Self {
            user_data,
            result: Err(err),
            flags: CompletionFlags::empty(),
        }
    }

    /// Create an error completion from a raw OS error code.
    pub fn from_raw_error(user_data: u64, errno: i32) -> Self {
        Self {
            user_data,
            result: Err(io::Error::from_raw_os_error(errno)),
            flags: CompletionFlags::empty(),
        }
    }

    /// Check if this completion indicates success.
    #[inline]
    pub fn is_success(&self) -> bool {
        self.result.is_ok()
    }

    /// Check if this completion indicates an error.
    #[inline]
    pub fn is_error(&self) -> bool {
        self.result.is_err()
    }

    /// Check if more completions are coming (for multi-shot operations).
    #[inline]
    pub fn has_more(&self) -> bool {
        self.flags.contains(CompletionFlags::MORE)
    }

    /// Get the number of bytes transferred (0 if error).
    #[inline]
    pub fn bytes(&self) -> usize {
        self.result.as_ref().copied().unwrap_or(0)
    }

    /// Get the error (if any).
    #[inline]
    pub fn get_error(&self) -> Option<&io::Error> {
        self.result.as_ref().err()
    }

    /// Take the result, consuming the completion.
    #[inline]
    pub fn into_result(self) -> io::Result<usize> {
        self.result
    }
}

bitflags! {
    /// Flags for I/O completions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct CompletionFlags: u32 {
        /// More completions coming for multi-shot operation
        const MORE = 0x01;
        /// Buffer was provided by kernel (buffer selection)
        const BUFFER_SELECT = 0x02;
        /// Notification only (no data transferred)
        const NOTIF = 0x04;
        /// Socket has been shut down
        const SOCK_NONEMPTY = 0x08;
    }
}

impl CompletionFlags {
    /// Create flags from io_uring CQE flags.
    #[cfg(target_os = "linux")]
    pub fn from_io_uring(flags: u32) -> Self {
        let mut result = Self::empty();

        // IORING_CQE_F_MORE = 1 << 1
        if flags & (1 << 1) != 0 {
            result |= Self::MORE;
        }

        // IORING_CQE_F_BUFFER = 1 << 0
        if flags & (1 << 0) != 0 {
            result |= Self::BUFFER_SELECT;
        }

        // IORING_CQE_F_NOTIF = 1 << 3
        if flags & (1 << 3) != 0 {
            result |= Self::NOTIF;
        }

        // IORING_CQE_F_SOCK_NONEMPTY = 1 << 2
        if flags & (1 << 2) != 0 {
            result |= Self::SOCK_NONEMPTY;
        }

        result
    }
}

/// A batch of completions for efficient processing.
#[derive(Debug, Default)]
pub struct CompletionBatch {
    completions: Vec<Completion>,
}

impl CompletionBatch {
    /// Create a new empty batch.
    pub fn new() -> Self {
        Self {
            completions: Vec::new(),
        }
    }

    /// Create a batch with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            completions: Vec::with_capacity(capacity),
        }
    }

    /// Add a completion to the batch.
    pub fn push(&mut self, completion: Completion) {
        self.completions.push(completion);
    }

    /// Get the number of completions in the batch.
    pub fn len(&self) -> usize {
        self.completions.len()
    }

    /// Check if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.completions.is_empty()
    }

    /// Clear the batch.
    pub fn clear(&mut self) {
        self.completions.clear();
    }

    /// Iterate over completions.
    pub fn iter(&self) -> impl Iterator<Item = &Completion> {
        self.completions.iter()
    }

    /// Drain completions from the batch.
    pub fn drain(&mut self) -> impl Iterator<Item = Completion> + '_ {
        self.completions.drain(..)
    }

    /// Convert to a Vec of completions.
    pub fn into_vec(self) -> Vec<Completion> {
        self.completions
    }
}

impl IntoIterator for CompletionBatch {
    type Item = Completion;
    type IntoIter = std::vec::IntoIter<Completion>;

    fn into_iter(self) -> Self::IntoIter {
        self.completions.into_iter()
    }
}

impl<'a> IntoIterator for &'a CompletionBatch {
    type Item = &'a Completion;
    type IntoIter = std::slice::Iter<'a, Completion>;

    fn into_iter(self) -> Self::IntoIter {
        self.completions.iter()
    }
}

impl FromIterator<Completion> for CompletionBatch {
    fn from_iter<I: IntoIterator<Item = Completion>>(iter: I) -> Self {
        Self {
            completions: iter.into_iter().collect(),
        }
    }
}

impl Extend<Completion> for CompletionBatch {
    fn extend<I: IntoIterator<Item = Completion>>(&mut self, iter: I) {
        self.completions.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_success() {
        let c = Completion::success(42, 1024);
        assert!(c.is_success());
        assert!(!c.is_error());
        assert_eq!(c.user_data, 42);
        assert_eq!(c.bytes(), 1024);
    }

    #[test]
    fn test_completion_error() {
        let c = Completion::error(42, io::Error::new(io::ErrorKind::NotFound, "not found"));
        assert!(!c.is_success());
        assert!(c.is_error());
        assert_eq!(c.bytes(), 0);
    }

    #[test]
    fn test_completion_flags() {
        let c = Completion::success_with_flags(1, 100, CompletionFlags::MORE);
        assert!(c.has_more());
    }

    #[test]
    fn test_completion_batch() {
        let mut batch = CompletionBatch::new();
        batch.push(Completion::success(1, 100));
        batch.push(Completion::success(2, 200));

        assert_eq!(batch.len(), 2);

        let vec = batch.into_vec();
        assert_eq!(vec.len(), 2);
    }
}
