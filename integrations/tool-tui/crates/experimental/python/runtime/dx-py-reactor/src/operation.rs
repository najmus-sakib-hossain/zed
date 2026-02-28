//! I/O operation types

use crate::io_buffer::IoBuffer;
use bitflags::bitflags;
use std::net::SocketAddr;

#[cfg(unix)]
use std::os::unix::io::RawFd;

#[cfg(windows)]
use std::os::windows::io::RawHandle;

/// File descriptor type (platform-specific)
#[cfg(unix)]
pub type Fd = RawFd;

#[cfg(windows)]
pub type Fd = RawHandle;

/// I/O operation types supported by the reactor.
#[derive(Debug)]
pub enum IoOperation {
    /// Read from a file descriptor into a buffer.
    Read {
        /// File descriptor to read from
        fd: Fd,
        /// Buffer to read into
        buf: IoBuffer,
        /// Offset in the file (0 for sockets/pipes)
        offset: u64,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Write from a buffer to a file descriptor.
    Write {
        /// File descriptor to write to
        fd: Fd,
        /// Buffer containing data to write
        buf: IoBuffer,
        /// Offset in the file (0 for sockets/pipes)
        offset: u64,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Accept a single connection on a listening socket.
    Accept {
        /// Listening socket file descriptor
        fd: Fd,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Accept multiple connections (multi-shot).
    /// One submission generates multiple completions.
    AcceptMulti {
        /// Listening socket file descriptor
        fd: Fd,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Connect to a remote address.
    Connect {
        /// Socket file descriptor
        fd: Fd,
        /// Remote address to connect to
        addr: SocketAddr,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Send data on a socket.
    Send {
        /// Socket file descriptor
        fd: Fd,
        /// Buffer containing data to send
        buf: IoBuffer,
        /// Send flags
        flags: SendFlags,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Send data with zero-copy (Linux io_uring only).
    /// The buffer must remain valid until completion.
    SendZeroCopy {
        /// Socket file descriptor
        fd: Fd,
        /// Buffer containing data to send
        buf: IoBuffer,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Receive data from a socket.
    Recv {
        /// Socket file descriptor
        fd: Fd,
        /// Buffer to receive into
        buf: IoBuffer,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Close a file descriptor.
    Close {
        /// File descriptor to close
        fd: Fd,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Sync file data to disk.
    Fsync {
        /// File descriptor to sync
        fd: Fd,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Sync file data to disk (data only, no metadata).
    Fdatasync {
        /// File descriptor to sync
        fd: Fd,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Timeout operation (for implementing deadlines).
    Timeout {
        /// Duration in nanoseconds
        duration_ns: u64,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Cancel a pending operation.
    Cancel {
        /// User data of the operation to cancel
        target_user_data: u64,
        /// User data for this cancel operation
        user_data: u64,
    },

    /// No-op (useful for waking up the reactor).
    Nop {
        /// User data to identify this operation
        user_data: u64,
    },

    /// Poll for readability/writability.
    Poll {
        /// File descriptor to poll
        fd: Fd,
        /// Events to poll for
        events: PollEvents,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Read from a file at a specific offset (pread).
    Pread {
        /// File descriptor to read from
        fd: Fd,
        /// Buffer to read into
        buf: IoBuffer,
        /// Offset in the file
        offset: u64,
        /// User data to identify this operation
        user_data: u64,
    },

    /// Write to a file at a specific offset (pwrite).
    Pwrite {
        /// File descriptor to write to
        fd: Fd,
        /// Buffer containing data to write
        buf: IoBuffer,
        /// Offset in the file
        offset: u64,
        /// User data to identify this operation
        user_data: u64,
    },
}

impl IoOperation {
    /// Get the user data for this operation.
    pub fn user_data(&self) -> u64 {
        match self {
            IoOperation::Read { user_data, .. } => *user_data,
            IoOperation::Write { user_data, .. } => *user_data,
            IoOperation::Accept { user_data, .. } => *user_data,
            IoOperation::AcceptMulti { user_data, .. } => *user_data,
            IoOperation::Connect { user_data, .. } => *user_data,
            IoOperation::Send { user_data, .. } => *user_data,
            IoOperation::SendZeroCopy { user_data, .. } => *user_data,
            IoOperation::Recv { user_data, .. } => *user_data,
            IoOperation::Close { user_data, .. } => *user_data,
            IoOperation::Fsync { user_data, .. } => *user_data,
            IoOperation::Fdatasync { user_data, .. } => *user_data,
            IoOperation::Timeout { user_data, .. } => *user_data,
            IoOperation::Cancel { user_data, .. } => *user_data,
            IoOperation::Nop { user_data, .. } => *user_data,
            IoOperation::Poll { user_data, .. } => *user_data,
            IoOperation::Pread { user_data, .. } => *user_data,
            IoOperation::Pwrite { user_data, .. } => *user_data,
        }
    }

    /// Get the file descriptor for this operation (if applicable).
    pub fn fd(&self) -> Option<Fd> {
        match self {
            IoOperation::Read { fd, .. } => Some(*fd),
            IoOperation::Write { fd, .. } => Some(*fd),
            IoOperation::Accept { fd, .. } => Some(*fd),
            IoOperation::AcceptMulti { fd, .. } => Some(*fd),
            IoOperation::Connect { fd, .. } => Some(*fd),
            IoOperation::Send { fd, .. } => Some(*fd),
            IoOperation::SendZeroCopy { fd, .. } => Some(*fd),
            IoOperation::Recv { fd, .. } => Some(*fd),
            IoOperation::Close { fd, .. } => Some(*fd),
            IoOperation::Fsync { fd, .. } => Some(*fd),
            IoOperation::Fdatasync { fd, .. } => Some(*fd),
            IoOperation::Poll { fd, .. } => Some(*fd),
            IoOperation::Pread { fd, .. } => Some(*fd),
            IoOperation::Pwrite { fd, .. } => Some(*fd),
            IoOperation::Timeout { .. } => None,
            IoOperation::Cancel { .. } => None,
            IoOperation::Nop { .. } => None,
        }
    }

    /// Check if this is a multi-shot operation.
    pub fn is_multishot(&self) -> bool {
        matches!(self, IoOperation::AcceptMulti { .. })
    }
}

bitflags! {
    /// Flags for send operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SendFlags: u32 {
        /// Zero-copy send (data is not copied to kernel)
        const ZEROCOPY = 0x01;
        /// Don't generate SIGPIPE on broken pipe
        const NOSIGNAL = 0x02;
        /// Don't route, use interface addresses
        const DONTROUTE = 0x04;
        /// Send out-of-band data
        const OOB = 0x08;
        /// More data coming (cork)
        const MORE = 0x10;
    }
}

bitflags! {
    /// Events for poll operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PollEvents: u32 {
        /// Data available to read
        const READABLE = 0x01;
        /// Ready for writing
        const WRITABLE = 0x02;
        /// Error condition
        const ERROR = 0x04;
        /// Hang up
        const HUP = 0x08;
        /// Invalid request
        const NVAL = 0x10;
        /// Priority data available
        const PRI = 0x20;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_user_data() {
        let op = IoOperation::Nop { user_data: 42 };
        assert_eq!(op.user_data(), 42);
    }

    #[test]
    fn test_operation_fd() {
        #[cfg(unix)]
        {
            let op = IoOperation::Read {
                fd: 5,
                buf: IoBuffer::new(1024),
                offset: 0,
                user_data: 1,
            };
            assert_eq!(op.fd(), Some(5));
        }

        let op = IoOperation::Nop { user_data: 1 };
        assert_eq!(op.fd(), None);
    }

    #[test]
    fn test_multishot() {
        #[cfg(unix)]
        {
            let op = IoOperation::AcceptMulti {
                fd: 5,
                user_data: 1,
            };
            assert!(op.is_multishot());

            let op = IoOperation::Accept {
                fd: 5,
                user_data: 1,
            };
            assert!(!op.is_multishot());
        }
    }

    #[test]
    fn test_send_flags() {
        let flags = SendFlags::ZEROCOPY | SendFlags::NOSIGNAL;
        assert!(flags.contains(SendFlags::ZEROCOPY));
        assert!(flags.contains(SendFlags::NOSIGNAL));
        assert!(!flags.contains(SendFlags::DONTROUTE));
    }
}
